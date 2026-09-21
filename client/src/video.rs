//! Tar emot robotens kameraström (H.264 över en egen TCP-anslutning, se
//! `relay_protocol::video` och `robotd/src/video.rs`), avkodar den och
//! lämnar senaste bilden till UI:t.
//!
//! Allt nätverk och all avkodning sker i en egen OS-tråd — UI-tråden rör bara
//! en mutex för att hämta senaste färdiga bild, så en långsam länk eller en
//! trasig ström aldrig kan frysa fönstret eller gamepad-loopen.
//!
//! Stabilitet:
//! * Ansluter om automatiskt med backoff (robotd kan starta senare än klienten).
//! * Tystnad i mer än `IDLE_LIMIT` = döda länken -> koppla ner och återanslut.
//! * En trasig bildruta återställer avkodaren och strömmen återhämtar sig på
//!   nästa nyckelbild (max ~1 s), inget kraschar.
//! * `is_live()` är sant först när en NY bild kommit nyligen — används både
//!   för "bilden frusen"-varning och för kamerakravet vid AKTIVERA.

use std::io::{ErrorKind, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use openh264::decoder::Decoder;
use openh264::formats::YUVSource; // ger `dimensions()`
use relay_protocol::video::{FrameHeader, HEADER_LEN, MAGIC};

/// Hur gammal senaste bild får vara för att strömmen räknas som levande.
const LIVE_WITHIN: Duration = Duration::from_millis(1500);
/// Ingen data alls så här länge = länken är död.
const IDLE_LIMIT: Duration = Duration::from_secs(4);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// Läsning väcks med jämna mellanrum för att kunna se `stop`-flaggan.
const READ_POLL: Duration = Duration::from_millis(250);

pub struct VideoFrame {
    pub width: usize,
    pub height: usize,
    /// Packad RGB, `width * height * 3` byte.
    pub rgb: Vec<u8>,
}

#[derive(Default)]
struct Inner {
    frame: Option<VideoFrame>,
    last_frame_at: Option<Instant>,
    status: String,
    fps: f32,
    kbps: f32,
    size: (usize, usize),
}

pub struct VideoReceiver {
    inner: Arc<Mutex<Inner>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl VideoReceiver {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    /// Börjar ta emot från `host:port`. Stoppar en ev. tidigare mottagning först.
    pub fn start(&mut self, host: String, port: u16) {
        self.stop();
        self.stop = Arc::new(AtomicBool::new(false));
        *self.inner.lock().unwrap() = Inner { status: "ansluter…".into(), ..Inner::default() };
        let inner = self.inner.clone();
        let stop = self.stop.clone();
        self.thread = Some(
            std::thread::Builder::new()
                .name("video".into())
                .spawn(move || run(&host, port, &inner, &stop))
                .expect("kunde inte starta videotråden"),
        );
    }

    /// Stoppar mottagningen och väntar in tråden (max ~en läsintervall).
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        let mut g = self.inner.lock().unwrap();
        g.frame = None;
        g.last_frame_at = None;
        g.status = "frånkopplad".into();
    }

    /// Hämtar en ny färdig bild om det kommit någon sedan sist (annars `None`).
    pub fn take_new_frame(&self) -> Option<VideoFrame> {
        self.inner.lock().unwrap().frame.take()
    }

    /// Har en ny bild kommit nyligen? Falskt vid frusen/avbruten ström.
    pub fn is_live(&self) -> bool {
        self.inner
            .lock()
            .unwrap()
            .last_frame_at
            .map_or(false, |t| t.elapsed() < LIVE_WITHIN)
    }

    /// En kort rad till statusytan, t.ex. "1024x576 · 14.8 bilder/s · 910 kbit/s".
    pub fn status_line(&self) -> String {
        let g = self.inner.lock().unwrap();
        if g.size.0 > 0 && g.last_frame_at.map_or(false, |t| t.elapsed() < LIVE_WITHIN) {
            format!("{}x{} · {:.1} bilder/s · {:.0} kbit/s", g.size.0, g.size.1, g.fps, g.kbps)
        } else {
            g.status.clone()
        }
    }
}

impl Drop for VideoReceiver {
    fn drop(&mut self) {
        self.stop();
    }
}

fn set_status(inner: &Mutex<Inner>, s: impl Into<String>) {
    inner.lock().unwrap().status = s.into();
}

/// Sover `total`, men vaknar direkt om `stop` sätts.
fn sleep_unless_stopped(total: Duration, stop: &AtomicBool) {
    let end = Instant::now() + total;
    while Instant::now() < end && !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn run(host: &str, port: u16, inner: &Mutex<Inner>, stop: &AtomicBool) {
    let mut backoff = Duration::from_millis(500);
    while !stop.load(Ordering::Relaxed) {
        match connect(host, port) {
            Ok(stream) => {
                backoff = Duration::from_millis(500);
                set_status(inner, "väntar på bild…");
                match receive(stream, inner, stop) {
                    Ok(()) => {} // stop begärd
                    Err(e) => set_status(inner, format!("bildströmmen bröts ({e}), återansluter…")),
                }
            }
            Err(e) => set_status(inner, format!("ingen kamera ({e}), försöker igen…")),
        }
        sleep_unless_stopped(backoff, stop);
        backoff = (backoff * 2).min(Duration::from_secs(3));
    }
}

fn connect(host: &str, port: u16) -> std::io::Result<TcpStream> {
    let mut last_err = std::io::Error::new(ErrorKind::NotFound, "adressen kunde inte slås upp");
    for addr in (host, port).to_socket_addrs()? {
        match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
            Ok(s) => {
                let _ = s.set_nodelay(true);
                s.set_read_timeout(Some(READ_POLL))?;
                return Ok(s);
            }
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

/// Läser exakt `buf.len()` byte, men ger upp (`Err`) om det varit tyst i
/// `IDLE_LIMIT`, och returnerar `Ok(false)` direkt om `stop` sätts.
fn read_exact_or_stop(
    s: &mut TcpStream,
    buf: &mut [u8],
    stop: &AtomicBool,
) -> std::io::Result<bool> {
    let mut got = 0;
    let mut last_data = Instant::now();
    while got < buf.len() {
        if stop.load(Ordering::Relaxed) {
            return Ok(false);
        }
        match s.read(&mut buf[got..]) {
            Ok(0) => return Err(std::io::Error::new(ErrorKind::UnexpectedEof, "roboten stängde anslutningen")),
            Ok(n) => {
                got += n;
                last_data = Instant::now();
            }
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => {
                if last_data.elapsed() > IDLE_LIMIT {
                    return Err(std::io::Error::new(ErrorKind::TimedOut, "ingen data"));
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(true)
}

fn receive(mut s: TcpStream, inner: &Mutex<Inner>, stop: &AtomicBool) -> std::io::Result<()> {
    let mut magic = [0u8; 4];
    if !read_exact_or_stop(&mut s, &mut magic, stop)? {
        return Ok(());
    }
    if magic != MAGIC {
        return Err(std::io::Error::new(ErrorKind::InvalidData, "okänt videoprotokoll (fel version?)"));
    }

    let mut decoder = Decoder::new().map_err(|e| std::io::Error::new(ErrorKind::Other, format!("avkodaren: {e}")))?;
    let mut payload = Vec::new();
    let (mut n_frames, mut n_bytes, mut window) = (0u32, 0usize, Instant::now());

    loop {
        let mut hdr = [0u8; HEADER_LEN];
        if !read_exact_or_stop(&mut s, &mut hdr, stop)? {
            return Ok(());
        }
        let h = FrameHeader::decode(&hdr).map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e.to_string()))?;
        payload.resize(h.payload_len as usize, 0);
        if !read_exact_or_stop(&mut s, &mut payload, stop)? {
            return Ok(());
        }

        match decoder.decode(&payload) {
            Ok(Some(yuv)) => {
                let (w, h_px) = yuv.dimensions();
                let mut rgb = vec![0u8; w * h_px * 3];
                yuv.write_rgb8(&mut rgb);

                n_frames += 1;
                n_bytes += payload.len() + HEADER_LEN;
                let mut g = inner.lock().unwrap();
                g.frame = Some(VideoFrame { width: w, height: h_px, rgb });
                g.last_frame_at = Some(Instant::now());
                g.size = (w, h_px);
                let dt = window.elapsed();
                if dt >= Duration::from_secs(1) {
                    g.fps = n_frames as f32 / dt.as_secs_f32();
                    g.kbps = n_bytes as f32 * 8.0 / dt.as_secs_f32() / 1000.0;
                    n_frames = 0;
                    n_bytes = 0;
                    window = Instant::now();
                }
            }
            Ok(None) => {} // avkodaren behöver mer data (t.ex. första bilden)
            Err(_) => {
                // Trasig bildruta: börja om med en ren avkodare, nästa
                // nyckelbild (max ~1 s) rättar till bilden.
                decoder = Decoder::new()
                    .map_err(|e| std::io::Error::new(ErrorKind::Other, format!("avkodaren: {e}")))?;
            }
        }
    }
}
