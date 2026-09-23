//! Ansluter till det redan körande `Car_Client` (rise_sdvp) på Pi:n via
//! lokal TCP, istället för att ta över `/dev/car` direkt. Beslut (se
//! PROJECT_SPEC.md §11): Car_Client är den del av nuvarande system som
//! bevisligen redan fungerar (bekräftat på riktig hårdvara: /dev/car ->
//! ttyACM0, STM32 VCP, processen kör med --usetcp) — det ni upplevt som
//! opålitligt är RControlStation/nätverkslagret, inte den här länken. Så
//! `robotd` bygger ovanpå den fungerande delen istället för att
//! reverse-engineera USB-serieprotokollet mot kortet på nytt.
//!
//! Paket-inramningen (start/längd/payload/CRC16/slut) är samma som i
//! `serial_bridge.rs`, verifierad byte för byte mot en fångst 2026-09-16
//! (PROJECT_SPEC.md §13). Styrningen använder samma kommando som
//! RControlStation (`CMD_RC_CONTROL_ADV`), se länkuppgiften längre ner.

use std::net::SocketAddr;

use tokio::net::TcpStream;

use crate::serial_bridge::{encode_packet, try_decode_packet, DecodeError};

/// Port Car_Client lyssnar på, bekräftat mot riktig hårdvara 2026-09-16
/// (loggutskrift: "Trying to start TCP server. Port: 8300").
pub const DEFAULT_CAR_CLIENT_TCP_PORT: u16 = 8_300;

pub struct CarClientLink {
    stream: TcpStream,
    rx_buf: Vec<u8>,
}

impl CarClientLink {
    pub async fn connect(addr: SocketAddr) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("kunde inte ansluta till Car_Client på {addr}: {e}"))?;
        tracing::info!("Ansluten till Car_Client på {addr}");
        Ok(Self {
            stream,
            rx_buf: Vec::new(),
        })
    }

    pub async fn send_raw_payload(&mut self, payload: &[u8]) -> Result<(), String> {
        use tokio::io::AsyncWriteExt;
        let framed = encode_packet(payload);
        self.stream
            .write_all(&framed)
            .await
            .map_err(|e| format!("skrivfel mot Car_Client: {e}"))
    }

    /// Läs in mer data från socketen och försök tolka ut färdiga paket ur
    /// bufferten. Returnerar de payloads som gick att avkoda; ofullständiga
    /// paket ligger kvar i bufferten till nästa anrop.
    pub async fn poll_packets(&mut self) -> Result<Vec<Vec<u8>>, String> {
        use tokio::io::AsyncReadExt;
        let mut tmp = [0u8; 4096];
        let n = self
            .stream
            .read(&mut tmp)
            .await
            .map_err(|e| format!("läsfel mot Car_Client: {e}"))?;
        if n == 0 {
            return Err("Car_Client stängde anslutningen".to_string());
        }
        self.rx_buf.extend_from_slice(&tmp[..n]);

        let mut packets = Vec::new();
        loop {
            match try_decode_packet(&self.rx_buf) {
                Ok((payload, consumed)) => {
                    self.rx_buf.drain(..consumed);
                    packets.push(payload);
                }
                Err(DecodeError::Incomplete) => break,
                Err(DecodeError::BadCrc) | Err(DecodeError::BadFraming) => {
                    // Ramningsantagandet stämde inte — släng första byten och
                    // försök synka om, hellre än att fastna. Loggas tydligt
                    // så det syns direkt vid test mot riktig hårdvara.
                    tracing::warn!(
                        "Kunde inte tolka paket från Car_Client (ramnings-antagandet \
                         kanske fel) — kasserar en byte och försöker synka om."
                    );
                    if self.rx_buf.is_empty() {
                        break;
                    }
                    self.rx_buf.remove(0);
                }
            }
        }
        Ok(packets)
    }
}

// ---------------------------------------------------------------------------
// Länkuppgiften: den enda anslutningen robotd har till Car_Client.
//
// Car_Client tar bara EN TCP-klient åt gången (`TcpServerSimple::newTcpConnection`
// stänger varje ny anslutning medan en annan är öppen). Därför äger en enda
// uppgift anslutningen och sköter både styrning och skanning, och därför kan
// robotd och RControlStation inte vara anslutna samtidigt.

use std::time::Duration;

use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;

use crate::vesc_can::{self, VescStatusEntry};

/// Värden som skickas till styrkortet, redan skalade (t.ex. ±0.15).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DriveSetpoint {
    pub speed: f32,
    pub steering: f32,
}

impl DriveSetpoint {
    pub fn is_zero(&self) -> bool {
        self.speed == 0.0 && self.steering == 0.0
    }
}

/// Omsändning under körning, samma takt som RControlStation (`rcResendTick`).
/// Oftare än så svämmar firmwarens "Activity …"-utskrift över.
const RESEND_ACTIVE: Duration = Duration::from_millis(100);
/// I vila skickas 0 bara då och då, som ett "fortfarande stilla".
const RESEND_IDLE: Duration = Duration::from_secs(1);
const SCAN_TIMEOUT: Duration = Duration::from_secs(3);
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

pub type ScanReply = oneshot::Sender<Result<Vec<VescStatusEntry>, String>>;

pub struct LinkHandle {
    pub setpoint: watch::Sender<DriveSetpoint>,
    pub scan: mpsc::Sender<ScanReply>,
    pub connected: watch::Receiver<bool>,
}

#[derive(Clone, Copy)]
pub struct LinkParams {
    pub addr: SocketAddr,
    pub car_id: u8,
    pub speed_activity: u8,
    pub steering_activity: u8,
}

/// Ska `sp` skickas nu, givet vad som skickades senast? Övergång till stillastående
/// skickas direkt, annars styr omsändningstakten.
fn should_send(sp: DriveSetpoint, last: Option<(DriveSetpoint, Instant)>, now: Instant) -> bool {
    let Some((prev, at)) = last else { return true };
    let elapsed = now.saturating_duration_since(at);
    if sp.is_zero() {
        !prev.is_zero() || elapsed >= RESEND_IDLE
    } else {
        elapsed >= RESEND_ACTIVE
    }
}

pub fn spawn(params: LinkParams) -> LinkHandle {
    let (setpoint_tx, setpoint_rx) = watch::channel(DriveSetpoint::default());
    let (scan_tx, scan_rx) = mpsc::channel(4);
    let (connected_tx, connected_rx) = watch::channel(false);
    tokio::spawn(run(params, setpoint_rx, scan_rx, connected_tx));
    LinkHandle { setpoint: setpoint_tx, scan: scan_tx, connected: connected_rx }
}

async fn run(
    params: LinkParams,
    mut setpoint: watch::Receiver<DriveSetpoint>,
    mut scans: mpsc::Receiver<ScanReply>,
    connected: watch::Sender<bool>,
) {
    let mut failures: u32 = 0;
    loop {
        match CarClientLink::connect(params.addr).await {
            Ok(link) => {
                failures = 0;
                let _ = connected.send(true);
                tracing::info!(
                    "robotd äger nu Car_Client-länken — RControlStation kan inte ansluta samtidigt."
                );
                let started = Instant::now();
                let res = session(link, &params, &mut setpoint, &mut scans).await;
                let _ = connected.send(false);
                match res {
                    Ok(()) => return, // huvudloopen är borta
                    Err(e) if started.elapsed() < Duration::from_secs(2) => tracing::warn!(
                        "Car_Client-länken bröts direkt: {e}. Car_Client tar bara en klient — \
                         är RControlStation ansluten?"
                    ),
                    Err(e) => tracing::warn!("Car_Client-länken bröts: {e}"),
                }
            }
            Err(e) => {
                failures += 1;
                if failures == 1 || failures % 20 == 0 {
                    tracing::warn!(
                        "{e} (försök {failures}). Kör car_client.service? \
                         Försöker igen var {RECONNECT_DELAY:?}."
                    );
                }
            }
        }
        // Vänta innan nästa försök, men svara skanningar direkt så att
        // Settings-vyn inte står och väntar.
        let wait = tokio::time::sleep(RECONNECT_DELAY);
        tokio::pin!(wait);
        loop {
            tokio::select! {
                _ = &mut wait => break,
                req = scans.recv() => match req {
                    Some(reply) => { let _ = reply.send(Err("ingen anslutning till Car_Client".into())); }
                    None => return,
                },
            }
        }
    }
}

/// En anslutning. `Ok(())` = huvudloopen har släppt sina handtag, `Err` = länken bröts.
async fn session(
    mut link: CarClientLink,
    p: &LinkParams,
    setpoint: &mut watch::Receiver<DriveSetpoint>,
    scans: &mut mpsc::Receiver<ScanReply>,
) -> Result<(), String> {
    let mut last_sent: Option<(DriveSetpoint, Instant)> = None;
    let mut pending_scan: Option<(ScanReply, Instant)> = None;
    let mut tick = tokio::time::interval(RESEND_ACTIVE);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = tick.tick() => {}
            changed = setpoint.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
            }
            req = scans.recv(), if pending_scan.is_none() => {
                let Some(reply) = req else { return Ok(()) };
                link.send_raw_payload(&vesc_can::make_vesc_status_request(p.car_id)).await?;
                pending_scan = Some((reply, Instant::now() + SCAN_TIMEOUT));
            }
            packets = link.poll_packets() => {
                for pkt in packets? {
                    if let Some(entries) = vesc_can::parse_vesc_status_reply(&pkt) {
                        if let Some((reply, _)) = pending_scan.take() {
                            let _ = reply.send(Ok(entries));
                        }
                    }
                }
            }
        }

        let now = Instant::now();
        if pending_scan.as_ref().is_some_and(|(_, deadline)| now >= *deadline) {
            let (reply, _) = pending_scan.take().unwrap();
            let _ = reply.send(Err(format!(
                "inget svar på VESC-statusfrågan inom {SCAN_TIMEOUT:?} \
                 (saknar kortets firmware CMD_GET_VESC_STATUS?)"
            )));
        }

        let sp = *setpoint.borrow();
        if should_send(sp, last_sent, now) {
            link.send_raw_payload(&vesc_can::make_rc_control_adv(p.car_id, p.speed_activity, sp.speed))
                .await?;
            link.send_raw_payload(&vesc_can::make_rc_control_adv(p.car_id, p.steering_activity, sp.steering))
                .await?;
            last_sent = Some((sp, now));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn sp(speed: f32, steering: f32) -> DriveSetpoint {
        DriveSetpoint { speed, steering }
    }

    #[test]
    fn forsta_vardet_skickas_alltid() {
        assert!(should_send(sp(0.0, 0.0), None, Instant::now()));
    }

    #[test]
    fn stopp_skickas_direkt_men_vila_bara_varje_sekund() {
        let t = Instant::now();
        assert!(should_send(sp(0.0, 0.0), Some((sp(0.1, 0.0), t)), t));
        assert!(!should_send(sp(0.0, 0.0), Some((sp(0.0, 0.0), t)), t + Duration::from_millis(500)));
        assert!(should_send(sp(0.0, 0.0), Some((sp(0.0, 0.0), t)), t + Duration::from_millis(1000)));
    }

    #[test]
    fn korning_skickas_var_100_ms() {
        let t = Instant::now();
        assert!(!should_send(sp(0.1, 0.0), Some((sp(0.05, 0.0), t)), t + Duration::from_millis(40)));
        assert!(should_send(sp(0.1, 0.0), Some((sp(0.1, 0.0), t)), t + Duration::from_millis(100)));
    }

    /// Läs ramade paket från en falsk Car_Client tills `n` stycken kommit.
    async fn read_packets(sock: &mut tokio::net::TcpStream, n: usize) -> Vec<Vec<u8>> {
        let mut buf = Vec::new();
        let mut out = Vec::new();
        while out.len() < n {
            let mut tmp = [0u8; 256];
            let k = sock.read(&mut tmp).await.unwrap();
            assert!(k > 0, "robotd stängde anslutningen");
            buf.extend_from_slice(&tmp[..k]);
            while let Ok((payload, used)) = try_decode_packet(&buf) {
                buf.drain(..used);
                out.push(payload);
            }
        }
        out
    }

    /// Hela vägen mot en falsk Car_Client: nolla vid anslutning, spakvärden,
    /// direkt nolla vid stopp och en skanning på samma anslutning.
    #[tokio::test]
    async fn styrning_och_skanning_mot_falsk_car_client() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let h = spawn(LinkParams { addr, car_id: 4, speed_activity: 10, steering_activity: 11 });
        let (mut sock, _) = listener.accept().await.unwrap();

        // Först en nolla till båda aktiviteterna.
        let p = read_packets(&mut sock, 2).await;
        assert_eq!(p[0], vesc_can::make_rc_control_adv(4, 10, 0.0));
        assert_eq!(p[1], vesc_can::make_rc_control_adv(4, 11, 0.0));

        h.setpoint.send(sp(0.15, -0.05)).unwrap();
        let p = read_packets(&mut sock, 2).await;
        assert_eq!(p[0], vesc_can::make_rc_control_adv(4, 10, 0.15));
        assert_eq!(p[1], vesc_can::make_rc_control_adv(4, 11, -0.05));

        // Stopp ska komma direkt, inte vänta på omsändningen.
        let t = std::time::Instant::now();
        h.setpoint.send(sp(0.0, 0.0)).unwrap();
        let p = loop {
            let p = read_packets(&mut sock, 2).await;
            if p[0] == vesc_can::make_rc_control_adv(4, 10, 0.0) {
                break p;
            }
        };
        assert_eq!(p[1], vesc_can::make_rc_control_adv(4, 11, 0.0));
        assert!(t.elapsed() < Duration::from_millis(80), "stopp dröjde {:?}", t.elapsed());

        // Skanning: frågan ska komma på samma anslutning, svaret gå tillbaka.
        let (tx, rx) = oneshot::channel();
        h.scan.send(tx).await.unwrap();
        let req = loop {
            let p = read_packets(&mut sock, 1).await;
            if p[0][1] == vesc_can::CMD_GET_VESC_STATUS {
                break p;
            }
        };
        assert_eq!(req[0], vesc_can::make_vesc_status_request(4));
        let mut reply = vec![4, vesc_can::CMD_GET_VESC_STATUS, 28, 0, 50];
        reply.extend([0u8; 8]);
        sock.write_all(&encode_packet(&reply)).await.unwrap();
        let entries = rx.await.unwrap().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].can_id, 28);
        assert!(*h.connected.borrow());
    }

    #[tokio::test]
    async fn skanning_utan_car_client_svarar_med_fel() {
        // Ledig port där ingen lyssnar.
        let addr = TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap();
        let h = spawn(LinkParams { addr, car_id: 4, speed_activity: 10, steering_activity: 11 });
        let (tx, rx) = oneshot::channel();
        h.scan.send(tx).await.unwrap();
        let r = tokio::time::timeout(Duration::from_secs(1), rx).await.unwrap().unwrap();
        assert!(r.is_err());
        assert!(!*h.connected.borrow());
    }
}
