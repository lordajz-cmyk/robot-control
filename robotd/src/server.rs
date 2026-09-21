//! robotd:s WebSocket-server. Lyssnar direkt på en port (rekommenderat:
//! bind till robotens WireGuard-IP, t.ex. 192.168.200.8, eller 0.0.0.0 med
//! en brandväggsregel som bara släpper in på wg0 — se
//! PROJECT_SPEC.md §14 och Sakerhetsanalys-rekommendationen om `ufw`).
//!
//! Ersätter den gamla relay_client.rs (som ringde HEM till en central
//! reläserver). Nu är det tvärtom: robotd är servern, klienten ansluter
//! rakt in över VPN-tunneln. En-förare-i-taget och tillfälliga titta-koder
//! hanteras lokalt här, ingen extern registry behövs.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use relay_protocol::{ClientMessage, ControlCommand, RobotMessage, StatusUpdate};
use tokio::sync::{mpsc, Mutex};

struct ViewCode {
    code: String,
    expires_at: Instant,
}

#[derive(Default)]
struct SharedState {
    /// Finns en förare just nu? (enkel närvaromarkör, se `connect`)
    driver_connected: bool,
    view_codes: Vec<ViewCode>,
}

#[derive(Clone)]
struct AppState {
    shared: Arc<Mutex<SharedState>>,
    /// Styrkommandon (bara från den aktiva föraren) vidare till huvudloopen.
    control_tx: mpsc::Sender<ControlCommand>,
    /// Övriga förfrågningar (VESC-skanning/profil) vidare till huvudloopen.
    request_tx: mpsc::Sender<RobotRequest>,
    /// Senaste status, spegling för nya klienter direkt vid anslutning.
    latest_status: Arc<Mutex<Option<StatusUpdate>>>,
}

/// Förfrågningar från en klient som huvudloopen (main.rs) hanterar och
/// svarar på via `response_tx` i samma struct.
pub enum RobotRequest {
    ScanVescBus { respond_to: mpsc::Sender<RobotMessage> },
    SaveVescProfile { profile: relay_protocol::VescProfileMsg, respond_to: mpsc::Sender<RobotMessage> },
}

pub struct ServerHandles {
    pub incoming_control: mpsc::Receiver<ControlCommand>,
    pub incoming_requests: mpsc::Receiver<RobotRequest>,
    /// main.rs anropar den här för att sprida en ny StatusUpdate till alla
    /// anslutna klienter (drivrutinen håller koll på vilka som är kopplade).
    pub status_broadcast: mpsc::Sender<StatusUpdate>,
}

/// Startar servern på `bind_addr` (t.ex. "192.168.200.8:9000" eller
/// "0.0.0.0:9000"). Körs i en egen task; returnerar handtag för
/// huvudloopen i main.rs.
pub fn spawn(bind_addr: String) -> ServerHandles {
    let (control_tx, control_rx) = mpsc::channel::<ControlCommand>(32);
    let (request_tx, request_rx) = mpsc::channel::<RobotRequest>(16);
    let (status_tx, mut status_rx) = mpsc::channel::<StatusUpdate>(16);

    let latest_status = Arc::new(Mutex::new(None));
    let broadcast_targets: Arc<Mutex<Vec<mpsc::Sender<RobotMessage>>>> = Arc::new(Mutex::new(Vec::new()));

    let state = AppState {
        shared: Arc::new(Mutex::new(SharedState::default())),
        control_tx,
        request_tx,
        latest_status: latest_status.clone(),
    };

    // Sprid varje ny status till alla anslutna klienters kanaler.
    {
        let latest_status = latest_status.clone();
        let broadcast_targets = broadcast_targets.clone();
        tokio::spawn(async move {
            while let Some(status) = status_rx.recv().await {
                *latest_status.lock().await = Some(status.clone());
                let targets = broadcast_targets.lock().await;
                for t in targets.iter() {
                    let _ = t.send(RobotMessage::Status(status.clone())).await;
                }
            }
        });
    }

    {
        let state = state.clone();
        let broadcast_targets = broadcast_targets.clone();
        tokio::spawn(async move {
            let app = Router::new()
                .route("/ws", get(move |ws: WebSocketUpgrade, State(s): State<AppState>| {
                    let broadcast_targets = broadcast_targets.clone();
                    async move { ws.on_upgrade(move |socket| handle_socket(socket, s, broadcast_targets)) }
                }))
                .with_state(state);

            match tokio::net::TcpListener::bind(&bind_addr).await {
                Ok(listener) => {
                    tracing::info!("robotd lyssnar på {bind_addr}");
                    if let Err(e) = axum::serve(listener, app).await {
                        tracing::error!("servern stannade: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Kunde inte binda {bind_addr}: {e}. Kontrollera att adressen finns \
                         (t.ex. att WireGuard-tunneln är uppe) och att porten är ledig."
                    );
                }
            }
        });
    }

    ServerHandles {
        incoming_control: control_rx,
        incoming_requests: request_rx,
        status_broadcast: status_tx,
    }
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    broadcast_targets: Arc<Mutex<Vec<mpsc::Sender<RobotMessage>>>>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<RobotMessage>(32);

    // Vänta på första meddelandet: måste vara Connect.
    let Some(Ok(Message::Text(text))) = ws_rx.next().await else { return };
    let Ok(ClientMessage::Connect { as_viewer, view_code }) = serde_json::from_str(&text) else {
        return;
    };

    let is_driver = if as_viewer {
        let mut ok = true;
        if let Some(code) = &view_code {
            let mut shared = state.shared.lock().await;
            let now = Instant::now();
            shared.view_codes.retain(|c| c.expires_at > now);
            ok = shared.view_codes.iter().any(|c| &c.code == code);
        }
        if !ok {
            tracing::info!("Åskådaranslutning nekad: ogiltig eller utgången titta-kod.");
            let _ = out_tx.send(RobotMessage::ConnectDenied {
                reason: "ogiltig eller utgången titta-kod".to_string(),
            }).await;
            return;
        }
        false
    } else {
        let mut shared = state.shared.lock().await;
        if shared.driver_connected {
            // Syntes tidigare bara hos klienten (Dator 1:s förslag efter
            // regressionstestet 2026-09-19) — nu loggas avslaget här också,
            // så det går att felsöka från robotd-sidan utan att fråga
            // föraren vad de såg.
            tracing::info!("Förar-anslutning nekad: roboten körs redan av någon annan.");
            let _ = out_tx.send(RobotMessage::ConnectDenied {
                reason: "roboten körs redan av någon annan".to_string(),
            }).await;
            return;
        }
        shared.driver_connected = true;
        true
    };

    let _ = out_tx.send(if is_driver {
        RobotMessage::ConnectedAsDriver
    } else {
        RobotMessage::ConnectedAsViewer
    }).await;

    // Skicka senaste kända status direkt så nya klienter inte väntar tomt.
    if let Some(status) = state.latest_status.lock().await.clone() {
        let _ = out_tx.send(RobotMessage::Status(status)).await;
    }

    broadcast_targets.lock().await.push(out_tx.clone());

    // Om ingen giltig ClientMessage kommit på STALE_TIMEOUT: släpp
    // förar-platsen. Löser risken båda testomgångarna 2026-09-19 flaggade —
    // att `driver_connected` annars kan fastna om VPN-tunneln dör tyst
    // (utan en riktig TCP-close), eftersom WebSocket-läsningen då aldrig
    // returnerar `None`/`Close` av sig själv.
    const STALE_TIMEOUT: Duration = Duration::from_secs(5);
    let mut last_msg_at = Instant::now();
    let mut stale_check = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            Some(msg) = out_rx.recv() => {
                let text = serde_json::to_string(&msg).unwrap();
                if ws_tx.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            _ = stale_check.tick() => {
                if last_msg_at.elapsed() > STALE_TIMEOUT {
                    tracing::warn!("Klient (förare={is_driver}) tyst i >{STALE_TIMEOUT:?}, kopplar ner.");
                    break;
                }
            }
            incoming = ws_rx.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        last_msg_at = Instant::now();
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Control(cmd)) if is_driver => {
                                let _ = state.control_tx.send(cmd).await;
                            }
                            Ok(ClientMessage::ScanVescBus) => {
                                let _ = state.request_tx.send(RobotRequest::ScanVescBus {
                                    respond_to: out_tx.clone(),
                                }).await;
                            }
                            Ok(ClientMessage::SaveVescProfile(profile)) => {
                                let _ = state.request_tx.send(RobotRequest::SaveVescProfile {
                                    profile,
                                    respond_to: out_tx.clone(),
                                }).await;
                            }
                            Ok(ClientMessage::RequestViewCode) => {
                                let code = random_code();
                                let ttl = Duration::from_secs(1800);
                                state.shared.lock().await.view_codes.push(ViewCode {
                                    code: code.clone(),
                                    expires_at: Instant::now() + ttl,
                                });
                                let _ = out_tx.send(RobotMessage::ViewCode {
                                    code,
                                    ttl_secs: ttl.as_secs() as u32,
                                }).await;
                            }
                            Ok(ClientMessage::Ping { sent_ms }) => {
                                let _ = out_tx.send(RobotMessage::Pong { sent_ms }).await;
                            }
                            Ok(ClientMessage::Control(_)) => {
                                // Inte förare — skickar styrkommandon utan att få lov.
                                tracing::warn!("Icke-förare försökte skicka Control, ignoreras.");
                            }
                            Ok(other) => {
                                // Samma kompileringsfel som Dator 1 hittade i
                                // mock-robotd (E0004) fanns här också —
                                // `Connect`/`VideoSignal` saknade arm efter att
                                // den tysta `_ => {}` togs bort. `Connect`
                                // skickas bara som första meddelandet (hanteras
                                // separat ovanför loopen) — kommer den igen här
                                // är det avvikande, logga det. `VideoSignal`
                                // är inte ihopkopplat än (väntar på WebRTC,
                                // spec §12).
                                tracing::warn!("Meddelande som robotd inte hanterar här, ignoreras: {other:?}");
                            }
                            Err(e) => {
                                // Tidigare: tyst `_ => {}` här (upptäckt av
                                // testerna 2026-09-19 — trasiga/okända
                                // meddelanden gav aldrig svar eller loggrad,
                                // vilket t.ex. gjorde en profil med NaN
                                // omöjlig att felsöka). Nu loggas det.
                                tracing::warn!("Kunde inte tolka meddelande från klient: {e}");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::warn!("websocket-fel från klient: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    if is_driver {
        state.shared.lock().await.driver_connected = false;
    }
    broadcast_targets.lock().await.retain(|t| !t.same_channel(&out_tx));
}

fn random_code() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..6).map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char).collect()
}
