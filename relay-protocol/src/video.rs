//! Videoströmmen robotd -> klient. Går över en EGEN TCP-anslutning (port
//! `bind_port + 1`, alltså 9001 med standardkonfigurationen), inte över
//! styr-WebSocketen — en stor bildruta ska aldrig kunna fördröja ett
//! styr- eller stoppkommando som köar bakom den i samma TCP-ström.
//!
//! Trådformat (allt big-endian):
//!
//! ```text
//! servern skickar först:  "RVD1"                       (4 byte, protokollversion)
//! därefter, för varje bild:
//!   u32  payload_len      antal byte H.264 som följer (<= MAX_PAYLOAD)
//!   u8   flags            bit 0 = nyckelbild (innehåller IDR)
//!   u64  capture_ms       ms sedan epoch när robotd fick bilden
//!   [u8] payload          EN komplett H.264 access unit i Annex-B-format
//! ```
//!
//! Servern börjar alltid en ny klient på en nyckelbild, och hoppar över till
//! nästa nyckelbild om klienten hamnar efter — så fördröjningen växer aldrig.

pub const MAGIC: [u8; 4] = *b"RVD1";
pub const HEADER_LEN: usize = 13;
/// Övre gräns för en bild. En 1080p-nyckelbild från en webbkamera ryms med
/// god marginal; allt över detta är en trasig ström.
pub const MAX_PAYLOAD: usize = 4 * 1024 * 1024;

const FLAG_KEYFRAME: u8 = 0b0000_0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    pub payload_len: u32,
    pub keyframe: bool,
    pub capture_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoError {
    FrameTooLarge(u32),
}

impl std::fmt::Display for VideoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoError::FrameTooLarge(n) => {
                write!(f, "bildruta på {n} byte överstiger gränsen {MAX_PAYLOAD}")
            }
        }
    }
}

impl std::error::Error for VideoError {}

impl FrameHeader {
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut b = [0u8; HEADER_LEN];
        b[0..4].copy_from_slice(&self.payload_len.to_be_bytes());
        b[4] = if self.keyframe { FLAG_KEYFRAME } else { 0 };
        b[5..13].copy_from_slice(&self.capture_ms.to_be_bytes());
        b
    }

    pub fn decode(b: &[u8; HEADER_LEN]) -> Result<Self, VideoError> {
        let payload_len = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        if payload_len as usize > MAX_PAYLOAD {
            return Err(VideoError::FrameTooLarge(payload_len));
        }
        Ok(Self {
            payload_len,
            keyframe: b[4] & FLAG_KEYFRAME != 0,
            capture_ms: u64::from_be_bytes([b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12]]),
        })
    }
}

/// H.264 NAL-typer vi bryr oss om.
const NAL_IDR: u8 = 5;
const NAL_AUD: u8 = 9;

/// Hittar alla NAL-enheter i en Annex-B-ström. Returnerar
/// `(start, nal_typ)` där `start` är första byten i startkoden (inklusive en
/// ledande nollbyte vid 4-bytes-startkod, `00 00 00 01`).
fn nal_units(buf: &[u8]) -> Vec<(usize, u8)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < buf.len() {
        if buf[i] == 0 && buf[i + 1] == 0 && buf[i + 2] == 1 {
            let start = if i > 0 && buf[i - 1] == 0 { i - 1 } else { i };
            out.push((start, buf[i + 3] & 0x1f));
            i += 3;
        } else {
            i += 1;
        }
    }
    out
}

/// Innehåller den här access unit en IDR-bild (dvs. går att börja avkoda på)?
pub fn contains_idr(au: &[u8]) -> bool {
    nal_units(au).iter().any(|&(_, t)| t == NAL_IDR)
}

/// Delar upp en rå H.264-Annex-B-ström (t.ex. från `ffmpeg -f h264 pipe:1`)
/// i access units. Kräver att strömmen innehåller Access Unit Delimiters
/// (NAL-typ 9) före varje bild — `ffmpeg` gör det med
/// `-bsf:v h264_metadata=aud=insert`. Att dela på AUD är robust mot
/// att en bild består av flera slices, och mot att data kommer i godtyckliga
/// bitar (pipe-läsningar).
#[derive(Default)]
pub struct AuSplitter {
    buf: Vec<u8>,
}

impl AuSplitter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Matar in nya bytes. Returnerar alla access units som nu är kompletta.
    /// (En access unit är komplett först när nästa börjar, så den senaste
    /// bilden väntar alltid en bildruta i bufferten.)
    pub fn push(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        self.buf.extend_from_slice(data);

        let auds: Vec<usize> = nal_units(&self.buf)
            .into_iter()
            .filter(|&(_, t)| t == NAL_AUD)
            .map(|(pos, _)| pos)
            .collect();

        let mut out = Vec::new();
        if auds.len() >= 2 {
            for w in auds.windows(2) {
                out.push(self.buf[w[0]..w[1]].to_vec());
            }
            // Behåll den ofärdiga sista bilden, släng ev. skräp före första AUD.
            self.buf.drain(..*auds.last().unwrap());
        } else if let Some(&first) = auds.first() {
            // Skräp före första AUD behövs inte.
            if first > 0 {
                self.buf.drain(..first);
            }
        }

        // Skydd mot en ström utan AUD (fel ffmpeg-flaggor): växer bufferten
        // förbi gränsen utan att någon bild kan brytas ut, börja om istället
        // för att äta minne.
        if self.buf.len() > MAX_PAYLOAD {
            self.buf.clear();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let h = FrameHeader { payload_len: 12345, keyframe: true, capture_ms: 1_700_000_000_123 };
        assert_eq!(FrameHeader::decode(&h.encode()), Ok(h));
        let h = FrameHeader { payload_len: 0, keyframe: false, capture_ms: 0 };
        assert_eq!(FrameHeader::decode(&h.encode()), Ok(h));
    }

    #[test]
    fn header_rejects_oversized_frames() {
        let h = FrameHeader { payload_len: (MAX_PAYLOAD + 1) as u32, keyframe: false, capture_ms: 0 };
        assert_eq!(
            FrameHeader::decode(&h.encode()),
            Err(VideoError::FrameTooLarge((MAX_PAYLOAD + 1) as u32))
        );
    }

    // Byggstenar: AUD (typ 9), SPS (7), PPS (8), IDR (5), icke-IDR-slice (1).
    fn aud() -> Vec<u8> { vec![0, 0, 0, 1, 0x09, 0xf0] }
    fn key_au() -> Vec<u8> {
        let mut v = aud();
        v.extend_from_slice(&[0, 0, 0, 1, 0x67, 1, 2, 3]); // SPS
        v.extend_from_slice(&[0, 0, 1, 0x68, 4, 5]);       // PPS, 3-bytes-startkod
        v.extend_from_slice(&[0, 0, 0, 1, 0x65, 9, 9, 9, 9]); // IDR
        v
    }
    fn delta_au() -> Vec<u8> {
        let mut v = aud();
        v.extend_from_slice(&[0, 0, 1, 0x41, 7, 7, 7]); // icke-IDR
        v
    }

    fn stream() -> (Vec<u8>, Vec<Vec<u8>>) {
        let aus = vec![key_au(), delta_au(), delta_au(), key_au()];
        (aus.concat(), aus)
    }

    #[test]
    fn splits_whole_stream_and_holds_back_last_au() {
        let (bytes, aus) = stream();
        let mut s = AuSplitter::new();
        let got = s.push(&bytes);
        assert_eq!(got, aus[..3].to_vec()); // sista väntar tills nästa AUD kommer
        // Nästa AUD släpper ut den sista.
        let more = s.push(&aud());
        assert_eq!(more, vec![aus[3].clone()]);
    }

    #[test]
    fn splits_identically_for_any_chunking() {
        let (bytes, aus) = stream();
        for chunk in [1usize, 2, 3, 5, 7, 64] {
            let mut s = AuSplitter::new();
            let mut got = Vec::new();
            for c in bytes.chunks(chunk) {
                got.extend(s.push(c));
            }
            got.extend(s.push(&aud())); // släpp ut den sista
            assert_eq!(got, aus, "chunk-storlek {chunk}");
        }
    }

    #[test]
    fn drops_garbage_before_first_aud() {
        let mut bytes = vec![0xde, 0xad, 0xbe, 0xef, 0x55];
        bytes.extend(delta_au());
        bytes.extend(aud());
        let mut s = AuSplitter::new();
        assert_eq!(s.push(&bytes), vec![delta_au()]);
    }

    #[test]
    fn detects_idr() {
        assert!(contains_idr(&key_au()));
        assert!(!contains_idr(&delta_au()));
        assert!(!contains_idr(&[]));
    }

    #[test]
    fn stream_without_aud_does_not_grow_forever() {
        let mut s = AuSplitter::new();
        let junk = vec![0x11u8; 1024 * 1024];
        for _ in 0..8 {
            assert!(s.push(&junk).is_empty());
        }
        assert!(s.buf.len() <= MAX_PAYLOAD);
    }
}
