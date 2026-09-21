//! Reläserver: robotar och klienter ansluter båda UT hit (löser NAT).
//! Auth-handskakningen (Hello -> AuthChallenge -> AuthResponse) och all
//! parkopplings-/förarlogik ligger i `registry.rs`, som är enhetstestad
//! oberoende av det här filens nätverkskod.

mod registry;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use rand::RngCore;
use relay_protocol::{
    verify, ClientToRelay, ControlCommand, RelayToClient, RelayToRobot, RobotId, RobotToRelay,
};
use tokio::sync::{mpsc, Mutex};

use registry::Registry;

/// Kanal in till en ansluten robot (för styrkommandon/video-signalering från
/// dess aktiva förare).
type RobotSink = mpsc::Sender<RelayToRobot>;
/// Kanal in till en ansluten klient (för status/video-signalering från
/// roboten den kör/tittar på).
type ClientSink = mpsc::Sender<RelayToClient>;

#[derive(Default)]
struct Connections {
    robots: HashMap<RobotId, RobotSink>,
    clients: HashMap<String, ClientSink>,
}

#[derive(Clone)]
struct AppState {
    registry: Arc<Mutex<Registry>>,
    conns: Arc<Mutex<Connections>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState {
        registry: Arc::new(Mutex::new(Registry::new())),
        conns: Arc::new(Mutex::new(Connections::default())),
    };

    let app = Router::new()
        .route("/robot/ws", get(robot_ws))
        .route("/client/ws", get(client_ws))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8443")
        .await
        .expect("kunde inte binda port 8443");
    tracing::info!("relay-server lyssnar på 0.0.0.0:8443");
    axum::serve(listener, app).await.unwrap();
}

fn random_nonce() -> [u8; 32] {
    let mut n = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut n);
    n
}

async fn robot_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_robot_socket(socket, state))
}

async fn client_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_client_socket(socket, state))
}

/// Robotens sida: verifiera identitet via signerad utmaning, registrera en
/// utgående kanal så klienter kan skicka styrkommandon till den, och
/// vidarebefordra dess status/video-signalering till den klient som just nu
/// kör/tittar.
async fn handle_robot_socket(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split_sink_stream();

    // 1. Hello: robot presenterar id + publik nyckel.
    let Some(Ok(Message::Text(hello_raw))) = ws_rx.next_msg().await else {
        return;
    };
    let Ok(RobotToRelay::Hello { robot_id, public_key }) = serde_json::from_str(&hello_raw) else {
        tracing::warn!("robot skickade ogiltigt Hello, kopplar ner");
        return;
    };

    // 2. Utmaning -> väntar på signerat svar.
    let nonce = random_nonce();
    let _ = send_to_robot(&mut ws_tx, &RelayToRobot::AuthChallenge { nonce }).await;

    let Some(Ok(Message::Text(resp_raw))) = ws_rx.next_msg().await else {
        return;
    };
    let Ok(RobotToRelay::AuthResponse { signature }) = serde_json::from_str(&resp_raw) else {
        return;
    };
    if !verify(&public_key, &nonce, &signature) {
        let _ = send_to_robot(&mut ws_tx, &RelayToRobot::AuthFailed).await;
        tracing::warn!("robot '{robot_id}' misslyckades autentisering");
        return;
    }
    let _ = send_to_robot(&mut ws_tx, &RelayToRobot::AuthOk).await;
    tracing::info!("robot '{robot_id}' ansluten och autentiserad");

    // 3. Registrera en kanal andra delar av servern kan skicka till den här
    // roboten genom (styrkommandon från aktiv förare, video-signalering).
    let (sink_tx, mut sink_rx) = mpsc::channel::<RelayToRobot>(32);
    state.conns.lock().await.robots.insert(robot_id.clone(), sink_tx);

    loop {
        tokio::select! {
            // Från servern (t.ex. en klients styrkommando) -> ut på socketen.
            Some(msg) = sink_rx.recv() => {
                if send_to_robot(&mut ws_tx, &msg).await.is_err() {
                    break;
                }
            }
            // Från roboten (status, registrera pairing-kod, video-signal) -> hantera.
            maybe_msg = ws_rx.next_msg() => {
                let Some(Ok(Message::Text(text))) = maybe_msg else { break };
                match serde_json::from_str::<RobotToRelay>(&text) {
                    Ok(RobotToRelay::RegisterPairingCode { code, ttl_secs }) => {
                        state.registry.lock().await.register_pairing_code(
                            &robot_id, code, std::time::Duration::from_secs(ttl_secs as u64), Instant::now(),
                        );
                    }
                    Ok(RobotToRelay::RegisterViewCode { code, ttl_secs }) => {
                        state.registry.lock().await.register_view_code(
                            &robot_id, code, std::time::Duration::from_secs(ttl_secs as u64), Instant::now(),
                        );
                    }
                    Ok(RobotToRelay::Status(status)) => {
                        if let Some(driver) = state.registry.lock().await.current_driver(&robot_id) {
                            if let Some(sink) = state.conns.lock().await.clients.get(driver) {
                                let _ = sink.send(RelayToClient::Status(status)).await;
                            }
                        }
                    }
                    Ok(RobotToRelay::VideoSignal { to_client, payload }) => {
                        if let Some(sink) = state.conns.lock().await.clients.get(&to_client) {
                            let _ = sink.send(RelayToClient::VideoSignal { from_robot: robot_id.clone(), payload }).await;
                        }
                    }
                    Ok(RobotToRelay::VescBusResult { to_client, result }) => {
                        if let Some(sink) = state.conns.lock().await.clients.get(&to_client) {
                            let _ = sink.send(RelayToClient::VescBusResult(result)).await;
                        }
                    }
                    Ok(RobotToRelay::VescProfileSaved { to_client }) => {
                        if let Some(sink) = state.conns.lock().await.clients.get(&to_client) {
                            let _ = sink.send(RelayToClient::VescProfileSaved).await;
                        }
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }

    state.conns.lock().await.robots.remove(&robot_id);
    tracing::info!("robot '{robot_id}' frånkopplad");
}

/// Klientens sida: samma auth-mönster, sedan `Connect { robot_id, code }`
/// mot `registry`, som avgör förare/åskådare/nekad. Styrkommandon skickas
/// vidare till robotens kanal.
async fn handle_client_socket(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split_sink_stream();

    let Some(Ok(Message::Text(hello_raw))) = ws_rx.next_msg().await else {
        return;
    };
    let Ok(ClientToRelay::Hello { client_id, public_key }) = serde_json::from_str(&hello_raw) else {
        return;
    };

    let nonce = random_nonce();
    let _ = send_to_client(&mut ws_tx, &RelayToClient::AuthChallenge { nonce }).await;

    let Some(Ok(Message::Text(resp_raw))) = ws_rx.next_msg().await else {
        return;
    };
    let Ok(ClientToRelay::AuthResponse { signature }) = serde_json::from_str(&resp_raw) else {
        return;
    };
    if !verify(&public_key, &nonce, &signature) {
        let _ = send_to_client(&mut ws_tx, &RelayToClient::AuthFailed).await;
        return;
    }
    let _ = send_to_client(&mut ws_tx, &RelayToClient::AuthOk).await;

    let (sink_tx, mut sink_rx) = mpsc::channel::<RelayToClient>(32);
    state.conns.lock().await.clients.insert(client_id.clone(), sink_tx);

    let mut connected_robot: Option<RobotId> = None;

    loop {
        tokio::select! {
            Some(msg) = sink_rx.recv() => {
                if send_to_client(&mut ws_tx, &msg).await.is_err() {
                    break;
                }
            }
            maybe_msg = ws_rx.next_msg() => {
                let Some(Ok(Message::Text(text))) = maybe_msg else { break };
                match serde_json::from_str::<ClientToRelay>(&text) {
                    Ok(ClientToRelay::Connect { robot_id, code }) => {
                        use registry::ConnectOutcome;
                        let outcome = state.registry.lock().await.connect(
                            &robot_id, &client_id, public_key, code.as_deref(), Instant::now(),
                        );
                        match outcome {
                            ConnectOutcome::Driver => {
                                connected_robot = Some(robot_id.clone());
                                let _ = send_to_client(&mut ws_tx, &RelayToClient::ConnectedAsDriver).await;
                            }
                            ConnectOutcome::Viewer => {
                                connected_robot = Some(robot_id.clone());
                                let _ = send_to_client(&mut ws_tx, &RelayToClient::ConnectedAsViewer).await;
                            }
                            ConnectOutcome::Denied(reason) => {
                                let _ = send_to_client(&mut ws_tx, &RelayToClient::ConnectDenied { reason }).await;
                            }
                        }
                    }
                    Ok(ClientToRelay::Control(cmd)) => {
                        if let Some(robot_id) = &connected_robot {
                            if let Some(sink) = state.conns.lock().await.robots.get(robot_id) {
                                let _ = sink.send(RelayToRobot::ControlCommand(cmd)).await;
                            }
                        }
                    }
                    Ok(ClientToRelay::VideoSignal { to_robot, payload }) => {
                        if let Some(sink) = state.conns.lock().await.robots.get(&to_robot) {
                            let _ = sink.send(RelayToRobot::VideoSignal { to_client: client_id.clone(), payload }).await;
                        }
                    }
                    Ok(ClientToRelay::ScanVescBus) => {
                        if let Some(robot_id) = &connected_robot {
                            if let Some(sink) = state.conns.lock().await.robots.get(robot_id) {
                                let _ = sink.send(RelayToRobot::ScanVescBus { requested_by: client_id.clone() }).await;
                            }
                        }
                    }
                    Ok(ClientToRelay::SaveVescProfile(profile)) => {
                        if let Some(robot_id) = &connected_robot {
                            if let Some(sink) = state.conns.lock().await.robots.get(robot_id) {
                                let _ = sink.send(RelayToRobot::SaveVescProfile {
                                    requested_by: client_id.clone(),
                                    profile,
                                }).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }

    if let Some(robot_id) = &connected_robot {
        let mut reg = state.registry.lock().await;
        reg.release_driver(robot_id, &client_id);
        reg.remove_viewer(robot_id, &client_id);
    }
    state.conns.lock().await.clients.remove(&client_id);
    tracing::info!("klient '{client_id}' frånkopplad");
}

async fn send_to_robot(
    tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &RelayToRobot,
) -> Result<(), axum::Error> {
    use futures_util::SinkExt;
    tx.send(Message::Text(serde_json::to_string(msg).unwrap())).await
}

async fn send_to_client(
    tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &RelayToClient,
) -> Result<(), axum::Error> {
    use futures_util::SinkExt;
    tx.send(Message::Text(serde_json::to_string(msg).unwrap())).await
}

/// Litet hjälp-lager ovanpå axum's WebSocket-stream/sink-uppdelning + ett
/// bekvämt `next_msg()` (StreamExt::next kräver att traiten är i scope
/// överallt, det här samlar det på ett ställe).
trait SplitSinkStreamExt {
    fn split_sink_stream(
        self,
    ) -> (
        futures_util::stream::SplitSink<WebSocket, Message>,
        NextMsgStream,
    );
}

impl SplitSinkStreamExt for WebSocket {
    fn split_sink_stream(
        self,
    ) -> (
        futures_util::stream::SplitSink<WebSocket, Message>,
        NextMsgStream,
    ) {
        use futures_util::StreamExt;
        let (tx, rx) = self.split();
        (tx, NextMsgStream(rx))
    }
}

struct NextMsgStream(futures_util::stream::SplitStream<WebSocket>);

impl NextMsgStream {
    async fn next_msg(&mut self) -> Option<Result<Message, axum::Error>> {
        use futures_util::StreamExt;
        self.0.next().await
    }
}
