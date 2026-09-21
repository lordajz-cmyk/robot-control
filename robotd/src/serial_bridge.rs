//! Paket-inramning för Car_Client-länken (TCP port 8300).
//!
//! **Status: ramningen är verifierad, byte-exakt, mot riktig trafik**
//! (2026-09-16, tcpdump-fångst på port 8300): `[0x02 start][1 byte
//! längd][payload][CRC16-CCITT poly 0x1021 init 0x0000][0x03 slut]`. Det var
//! ursprungligen en gissning baserad på mönstret i Benjamin Vedders andra
//! seriella protokoll (VESC UART, BLDC-Tool) — nu bekräftad mot faktisk
//! byte-trafik, inte längre en gissning.
//!
//! **Fortfarande okänt:** de specifika kommando-ID:na/typmarkörerna inuti
//! payload för olika meddelandetyper (t.ex. de två bytes `04 3f` som syntes
//! före varje NMEA-position i fångsten — troligen en typ-markör för
//! "GPS-position", men inte bekräftat vad de exakt betyder eller hur andra
//! meddelandetyper som VESC-telemetri/styrkommandon ser ut).

/// CRC16/CCITT (polynom 0x1021, init **0x0000**) — verifierat byte-exakt
/// 2026-09-16 mot en riktig `tcpdump`-fångst av Car_Client-trafik på port
/// 8300 (ett NMEA-paket med känd CRC `9a e9` gav exakt match med init=0x0000,
/// medan det tidigare gissade 0xFFFF gav fel resultat). Inte längre en gissning.
pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Bygg ett paket enligt Vedder-mönstret: [start][längd(1-2 byte)][payload][crc16][slut=0x03].
/// start = 0x02 om payload <= 255 byte, annars 0x03 med 2-byte längd.
pub fn encode_packet(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 5);
    if payload.len() <= 255 {
        out.push(0x02);
        out.push(payload.len() as u8);
    } else {
        out.push(0x03);
        out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    }
    out.extend_from_slice(payload);
    let crc = crc16_ccitt(payload);
    out.extend_from_slice(&crc.to_be_bytes());
    out.push(0x03);
    out
}

#[derive(Debug)]
pub enum DecodeError {
    /// Inte tillräckligt med bytes ännu — vänta på mer data från porten.
    Incomplete,
    BadCrc,
    BadFraming,
}

/// Försöker tolka ett paket ur `buf`. Returnerar (payload, antal bytes
/// konsumerade) vid lyckad avkodning, så anroparen kan dränera bufferten.
pub fn try_decode_packet(buf: &[u8]) -> Result<(Vec<u8>, usize), DecodeError> {
    if buf.is_empty() {
        return Err(DecodeError::Incomplete);
    }
    let (len, header_len): (usize, usize) = match buf[0] {
        0x02 => {
            if buf.len() < 2 {
                return Err(DecodeError::Incomplete);
            }
            (buf[1] as usize, 2)
        }
        0x03 => {
            if buf.len() < 3 {
                return Err(DecodeError::Incomplete);
            }
            (u16::from_be_bytes([buf[1], buf[2]]) as usize, 3)
        }
        _ => return Err(DecodeError::BadFraming),
    };

    let total_len = header_len + len + 2 + 1; // header + payload + crc16 + slutbyte
    if buf.len() < total_len {
        return Err(DecodeError::Incomplete);
    }

    let payload = &buf[header_len..header_len + len];
    let crc_recv = u16::from_be_bytes([buf[header_len + len], buf[header_len + len + 1]]);
    let end_byte = buf[total_len - 1];

    if end_byte != 0x03 {
        return Err(DecodeError::BadFraming);
    }
    if crc16_ccitt(payload) != crc_recv {
        return Err(DecodeError::BadCrc);
    }

    Ok((payload.to_vec(), total_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kodar_och_avkodar_ett_paket() {
        let payload = vec![1, 2, 3, 4, 5];
        let encoded = encode_packet(&payload);
        let (decoded, consumed) = try_decode_packet(&encoded).unwrap();
        assert_eq!(decoded, payload);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn ofullstandigt_buffer_ger_incomplete() {
        let payload = vec![1, 2, 3];
        let encoded = encode_packet(&payload);
        let result = try_decode_packet(&encoded[..encoded.len() - 2]);
        assert!(matches!(result, Err(DecodeError::Incomplete)));
    }

    #[test]
    fn fel_crc_upptacks() {
        let payload = vec![9, 9, 9];
        let mut encoded = encode_packet(&payload);
        let last = encoded.len() - 2;
        encoded[last] ^= 0xFF; // förstör CRC:n
        let result = try_decode_packet(&encoded);
        assert!(matches!(result, Err(DecodeError::BadCrc)));
    }
}
