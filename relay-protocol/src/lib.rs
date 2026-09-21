//! Delade meddelandetyper mellan robotd och klienten.
//!
//! **Arkitekturändring 2026-09-19:** Tidigare byggde det här på en egen
//! reläserver med Ed25519-parkoppling för att lösa NAT-genomgång och
//! autentisering. Det behövs inte — ni har redan en fungerande, beprövad
//! WireGuard-VPN (fast IP-schema `192.168.200.x`, se PROJECT_SPEC.md §14)
//! som redan löser exakt det: kryptering, autentisering via nycklar,
//! NAT-genomgång. `robotd` lyssnar nu direkt på sin VPN-IP, och klienten
//! ansluter direkt till den — ingen mellanhand.
//!
//! Kvar i det här protokollet: bara applikationslogiken (styrkommandon,
//! status, VESC-mappning, en-förare-i-taget). Ingen kryptografi här längre
//! — WireGuard-tunneln är redan krypterad och autentiserad på nätverksnivå.

pub mod video;

use serde::{Deserialize, Serialize};

/// Skickas av klienten när den ansluter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Första meddelandet. `view_code` krävs bara om `as_viewer` är true
    /// OCH robotd är konfigurerad att kräva kod för åskådare (se spec §4:
    /// tillfälliga titta-koder). En förar-anslutning (as_viewer=false)
    /// litar på att WireGuard redan avgjort att du får vara på nätverket;
    /// robotd nekar bara om någon annan redan kör (en förare i taget).
    Connect { as_viewer: bool, view_code: Option<String> },
    Control(ControlCommand),
    ScanVescBus,
    SaveVescProfile(VescProfileMsg),
    VideoSignal(String),
    /// Klienten begär en ny tillfällig titta-kod att dela vidare.
    RequestViewCode,
    /// Skickas periodiskt av klienten. `sent_ms` ekas tillbaka oförändrad i
    /// `RobotMessage::Pong` så klienten kan räkna ut en riktig
    /// tur-och-retur-tid, istället för den tidigare hårdkodade `Pong(0)`.
    /// Fungerar också som ett levnadstecken för framtida
    /// timeout/keepalive-logik på robotd-sidan.
    Ping { sent_ms: u64 },
}

/// Skickas av robotd till en ansluten klient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RobotMessage {
    ConnectedAsDriver,
    ConnectedAsViewer,
    /// "Roboten körs redan av någon annan" eller ogiltig/utgången titta-kod.
    ConnectDenied { reason: String },
    Status(StatusUpdate),
    VescBusResult(Vec<VescSighting>),
    VescProfileSaved,
    VideoSignal(String),
    ViewCode { code: String, ttl_secs: u32 },
    Pong { sent_ms: u64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ControlCommand {
    /// -1.0..=1.0
    pub throttle: f32,
    /// -1.0..=1.0
    pub steering: f32,
    pub accessory_a: f32, // t.ex. lastarm (L1/L2)
    pub accessory_b: f32, // t.ex. tilt (R1/R2)
    pub lights: bool,
    pub activated: bool,
    /// Millisekunder sedan epoch när klienten skapade kommandot, för
    /// latensmätning och för watchdogens tidsbedömning.
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub speed_kmh: Option<f32>,
    pub battery_percent: Option<f32>,
    pub gps_fix: bool,
    pub vesc_temps_c: Vec<f32>,
    pub last_error: Option<String>,
    pub link_quality: LinkQuality,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LinkQuality {
    Green,
    Yellow,
    Red,
}

/// En VESC som svarat på CAN-bussen, för Settings-vyn i klienten.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VescSighting {
    pub can_id: u8,
    pub responding: bool,
}

/// Roll tilldelad en VESC i Settings-vyn (spec §5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VescRoleMsg {
    DriftVanster,
    DriftHoger,
    Styrning,
    Annat {
        namn: String,
        input: String, // "l1_l2" eller "r1_r2"
        min: f32,
        max: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VescProfileMsg {
    pub roller: std::collections::HashMap<u8, VescRoleMsg>,
}
