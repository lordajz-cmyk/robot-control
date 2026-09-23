//! Klientens nätverkslager — ansluter DIREKT till robotd över
//! WireGuard-tunneln (se PROJECT_SPEC.md §14). Ingen central reläserver,
//! ingen egen kryptografisk parkoppling: WireGuard-tunneln är redan
//! krypterad och bara godkända VPN-peers kan ens nå robotens IP.
//!
//! "Anslut" betyder därför bara: öppna en WebSocket mot
//! `ws://<robotens-vpn-ip>:<port>/ws` och skicka `Connect`. robotd svarar
//! med Driver/Viewer/Denied (en förare i taget, se server.rs på
//! robotd-sidan).

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use relay_protocol::{
    ClientMessage, ControlCommand, LinkQuality, RobotMessage, StatusUpdate, VescProfileMsg,
    VescSighting,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    ConnectedAsDriver,
    ConnectedAsViewer,
    Denied,
}

pub struct NetLink {
    pub state: ConnectionState,
    pub deny_reason: Option<String>,
    pub last_status: Option<StatusUpdate>,
    /// När senaste status kom — äldre status räknas inte som aktuell.
    pub last_status_at: Option<Instant>,
    pub last_ping_ms: Option<u32>,
    last_pong_at: Option<Instant>,
    updates_rx: Option<mpsc::Receiver<LinkUpdate>>,
    control_tx: Option<mpsc::Sender<ControlCommand>>,
    request_tx: Option<mpsc::Sender<ClientMessage>>,
    pub vesc_scan_result: Option<Vec<VescSighting>>,
    pub vesc_profile_saved: bool,
    pub last_view_code: Option<String>,
}

enum LinkUpdate {
    State(ConnectionState, Option<String>),
    Status(StatusUpdate),
    /// Riktig tur-och-retur-tid från ett Ping/Pong-utbyte.
    Pong(u32),
    /// Ett meddelande kom in, men inget vi mätt en RTT för — uppdaterar
    /// bara "senast hört från roboten"-tidsstämpeln som link_quality
    /// bygger på, utan att låtsas ha ett pingvärde.
    Alive,
    VescBusResult(Vec<VescSighting>),
    VescProfileSaved,
    ViewCode(String),
}

impl NetLink {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            deny_reason: None,
            last_status: None,
            last_status_at: None,
            last_ping_ms: None,
            last_pong_at: None,
            updates_rx: None,
            control_tx: None,
            request_tx: None,
            vesc_scan_result: None,
            vesc_profile_saved: false,
            last_view_code: None,
        }
    }

    /// `target` är det som skrevs in på anslutningsskärmen: robotens
    /// VPN-IP (t.ex. "192.168.200.8") eller ett namn som löses via
    /// klientens egen /etc/hosts (t.ex. "drangen" -> 192.168.200.8).
    /// `port` är robotd:s port (default 9000, se robotd:s config.json).
    pub fn connect(&mut self, rt: &tokio::runtime::Handle, target: String, port: u16, as_viewer: bool, view_code: Option<String>) {
        self.state = ConnectionState::Connecting;
        self.last_status = None;
        self.last_status_at = None;
        let (updates_tx, updates_rx) = mpsc::channel::<LinkUpdate>(32);
        let (control_tx, control_rx) = mpsc::channel::<ControlCommand>(32);
        let (request_tx, request_rx) = mpsc::channel::<ClientMessage>(16);
        self.updates_rx = Some(updates_rx);
        self.control_tx = Some(control_tx);
        self.request_tx = Some(request_tx);

        let url = format!("ws://{target}:{port}/ws");

        rt.spawn(async move {
            if let Err(e) = run_connection(&url, as_viewer, view_code, &updates_tx, control_rx, request_rx).await {
                tracing::warn!("anslutning avslutad: {e}");
                let _ = updates_tx.send(LinkUpdate::State(ConnectionState::Disconnected, Some(e))).await;
            }
        });
    }

    /// Kallas från egui:s `update()` varje bildruta — icke-blockerande.
    pub fn poll_updates(&mut self) {
        let Some(rx) = &mut self.updates_rx else { return };
        while let Ok(update) = rx.try_recv() {
            match update {
                LinkUpdate::State(s, reason) => {
                    self.state = s;
                    self.deny_reason = reason;
                }
                LinkUpdate::Status(s) => {
                    self.last_status = Some(s);
                    self.last_status_at = Some(Instant::now());
                }
                LinkUpdate::Pong(ms) => {
                    self.last_ping_ms = Some(ms);
                    self.last_pong_at = Some(Instant::now());
                }
                LinkUpdate::Alive => {
                    self.last_pong_at = Some(Instant::now());
                }
                LinkUpdate::VescBusResult(result) => self.vesc_scan_result = Some(result),
                LinkUpdate::VescProfileSaved => self.vesc_profile_saved = true,
                LinkUpdate::ViewCode(code) => self.last_view_code = Some(code),
            }
        }
    }

    /// Senaste status om den är färsk (robotd skickar två gånger per sekund).
    pub fn fresh_status(&self) -> Option<&StatusUpdate> {
        let at = self.last_status_at?;
        (at.elapsed() < Duration::from_secs(3)).then_some(self.last_status.as_ref()?)
    }

    pub fn link_quality(&self, now: Instant) -> LinkQuality {
        match self.last_pong_at {
            None => LinkQuality::Red,
            Some(t) => {
                let age = now.saturating_duration_since(t);
                // Trösklarna var tidigare snävare än robotens statusintervall
                // (500ms) — bekräftat i test 2026-09-19 att det gjorde att
                // ramen flimrade grön/gul konstant även på en perfekt
                // anslutning. Nu bredare marginal mot det normala intervallet.
                if age > Duration::from_millis(2000) {
                    LinkQuality::Red
                } else if age > Duration::from_millis(700) || self.last_ping_ms.unwrap_or(0) > 150 {
                    LinkQuality::Yellow
                } else {
                    LinkQuality::Green
                }
            }
        }
    }

    pub fn send_control(&self, cmd: ControlCommand) {
        if let Some(tx) = &self.control_tx {
            let _ = tx.try_send(cmd);
        }
    }

    pub fn request_vesc_scan(&self) {
        if let Some(tx) = &self.request_tx {
            let _ = tx.try_send(ClientMessage::ScanVescBus);
        }
    }

    pub fn save_vesc_profile(&self, profile: VescProfileMsg) {
        if let Some(tx) = &self.request_tx {
            let _ = tx.try_send(ClientMessage::SaveVescProfile(profile));
        }
    }

    pub fn request_view_code(&self) {
        if let Some(tx) = &self.request_tx {
            let _ = tx.try_send(ClientMessage::RequestViewCode);
        }
    }
}

async fn run_connection(
    url: &str,
    as_viewer: bool,
    view_code: Option<String>,
    updates_tx: &mpsc::Sender<LinkUpdate>,
    mut control_rx: mpsc::Receiver<ControlCommand>,
    mut request_rx: mpsc::Receiver<ClientMessage>,
) -> Result<(), String> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(|e| format!("kunde inte ansluta till {url}: {e}"))?;
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    send(&mut ws_tx, &ClientMessage::Connect { as_viewer, view_code }).await?;

    match recv::<RobotMessage>(&mut ws_rx).await? {
        RobotMessage::ConnectedAsDriver => {
            let _ = updates_tx.send(LinkUpdate::State(ConnectionState::ConnectedAsDriver, None)).await;
        }
        RobotMessage::ConnectedAsViewer => {
            let _ = updates_tx.send(LinkUpdate::State(ConnectionState::ConnectedAsViewer, None)).await;
        }
        RobotMessage::ConnectDenied { reason } => {
            let _ = updates_tx.send(LinkUpdate::State(ConnectionState::Denied, Some(reason.clone()))).await;
            return Err(reason);
        }
        _ => return Err("oväntat svar på Connect".to_string()),
    }

    let _ = updates_tx.send(LinkUpdate::Alive).await; // anslutningen lyckades, inget uppmätt pingvärde än

    let mut ping_interval = tokio::time::interval(Duration::from_millis(1000));

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                let sent_ms = now_ms();
                send(&mut ws_tx, &ClientMessage::Ping { sent_ms }).await?;
            }
            Some(cmd) = control_rx.recv() => {
                send(&mut ws_tx, &ClientMessage::Control(cmd)).await?;
            }
            Some(req) = request_rx.recv() => {
                send(&mut ws_tx, &req).await?;
            }
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed) = serde_json::from_str::<RobotMessage>(&text) {
                            match parsed {
                                RobotMessage::Pong { sent_ms } => {
                                    let rtt = now_ms().saturating_sub(sent_ms).min(u32::MAX as u64) as u32;
                                    let _ = updates_tx.send(LinkUpdate::Pong(rtt)).await;
                                }
                                RobotMessage::Status(status) => {
                                    let _ = updates_tx.send(LinkUpdate::Alive).await;
                                    let _ = updates_tx.send(LinkUpdate::Status(status)).await;
                                }
                                RobotMessage::VescBusResult(result) => {
                                    let _ = updates_tx.send(LinkUpdate::VescBusResult(result)).await;
                                }
                                RobotMessage::VescProfileSaved => {
                                    let _ = updates_tx.send(LinkUpdate::VescProfileSaved).await;
                                }
                                RobotMessage::ViewCode { code, .. } => {
                                    let _ = updates_tx.send(LinkUpdate::ViewCode(code)).await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => return Err("roboten stängde anslutningen".to_string()),
                    Some(Err(e)) => return Err(format!("websocket-fel: {e}")),
                    _ => {}
                }
            }
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

async fn send<T: serde::Serialize>(
    tx: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Message,
    >,
    msg: &T,
) -> Result<(), String> {
    let text = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    tx.send(Message::Text(text)).await.map_err(|e| e.to_string())
}

async fn recv<T: serde::de::DeserializeOwned>(
    rx: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    >,
) -> Result<T, String> {
    match rx.next().await {
        Some(Ok(Message::Text(text))) => serde_json::from_str(&text).map_err(|e| e.to_string()),
        Some(Ok(_)) => Err("oväntad meddelandetyp".to_string()),
        Some(Err(e)) => Err(e.to_string()),
        None => Err("anslutningen stängdes".to_string()),
    }
}
