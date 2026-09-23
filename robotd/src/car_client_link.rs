//! Ansluter till det redan körande `Car_Client` (rise_sdvp) på Pi:n via
//! lokal TCP, istället för att ta över `/dev/car` direkt. Beslut (se
//! PROJECT_SPEC.md §11): Car_Client är den del av nuvarande system som
//! bevisligen redan fungerar (bekräftat på riktig hårdvara: /dev/car ->
//! ttyACM0, STM32 VCP, processen kör med --usetcp) — det ni upplevt som
//! opålitligt är RControlStation/nätverkslagret, inte den här länken. Så
//! `robotd` bygger ovanpå den fungerande delen istället för att
//! reverse-engineera USB-serieprotokollet mot kortet på nytt.
//!
//! Paket-inramningen (start/längd/payload/CRC16/slut) återanvänds från
//! `serial_bridge.rs` — Car_Client relayar troligen samma ramning över TCP
//! som över den seriella länken (samma författare, samma mönster i alla
//! hans projekt). **Fortfarande inte verifierat mot faktisk byte-trafik.**
//!
//! Nästa steg för att fylla i det som är kvar (kommando-ID:n för att läsa
//! VESC-telemetri / skicka RC-styrning genom Car_Client): fånga riktig
//! TCP-trafik med `tcpdump`/Wireshark medan RControlStation ansluter till
//! Car_Client som vanligt, t.ex.:
//!
//! ```bash
//! sudo tcpdump -i any -w car_client_capture.pcap tcp
//! # kör RControlStation som vanligt en stund samtidigt, avsluta sen tcpdump
//! ```
//!
//! Det ger oss verifierade paket att matcha mot `serial_bridge`-ramningen,
//! istället för att gissa kommando-ID:n. Klistra in en `xxd`/`hexdump` av
//! några paket här så fyller jag i resten.

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

    /// Fråga styrkortet vilka VESC det hört CAN-status från (`CMD_GET_VESC_STATUS`,
    /// kräver firmware med `firmware/0001-cmd-get-vesc-status.patch`). Bara en
    /// läsfråga. Utan den firmware kommer inget svar och det blir timeout.
    /// Övriga paket som Car_Client skickar under tiden (tillstånd, NMEA) hoppas över.
    pub async fn read_vesc_status(
        &mut self,
        car_id: u8,
        timeout: std::time::Duration,
    ) -> Result<Vec<crate::vesc_can::VescStatusEntry>, String> {
        self.send_raw_payload(&crate::vesc_can::make_vesc_status_request(car_id))
            .await?;
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            let packets = tokio::time::timeout(left, self.poll_packets())
                .await
                .map_err(|_| {
                    format!(
                        "inget svar på VESC-statusfrågan inom {timeout:?} \
                         (kortets firmware saknar troligen CMD_GET_VESC_STATUS)"
                    )
                })??;
            for p in packets {
                if let Some(entries) = crate::vesc_can::parse_vesc_status_reply(&p) {
                    return Ok(entries);
                }
            }
        }
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
