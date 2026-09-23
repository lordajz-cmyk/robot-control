mod board_config;
mod can_iface;
mod car_client_link;
mod config;
mod display;
mod gpio_lighting;
mod ramp;
mod serial_bridge;
mod server;
mod video;
mod vesc_can;
mod watchdog;

use std::path::Path;
use std::time::{Duration, Instant};

use config::RobotConfig;
use ramp::RateLimiter;
use relay_protocol::{RobotMessage, StatusUpdate, VescSighting};
use server::RobotRequest;
use watchdog::{Watchdog, WatchdogState};

const CONFIG_PATH: &str = "/etc/robotd/config.json";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = RobotConfig::load_or_default(Path::new(CONFIG_PATH));
    tracing::info!("robotd startar som '{}'", cfg.robot_id);

    // Nätverket: robotd LYSSNAR direkt på sin WireGuard-IP (se
    // PROJECT_SPEC.md §14) — ingen central reläserver längre. `bind_addr`
    // sätts i /etc/robotd/config.json, t.ex. "192.168.200.8:9000".
    let mut server = server::spawn(cfg.bind_addr.clone());

    // Länken till det redan körande Car_Client (se PROJECT_SPEC.md §13) i en
    // egen uppgift. Den tar Car_Client bara medan en klient är ansluten och
    // släpper den annars, så att RControlStation kan användas när ingen kör
    // med robotstyrning (Car_Client tar bara en klient åt gången).
    let link = match cfg.car_client_addr.parse() {
        Ok(addr) => Some(car_client_link::spawn(
            car_client_link::LinkParams {
                addr,
                car_id: cfg.car_client_id,
                speed_activity: cfg.drive.speed_activity,
                steering_activity: cfg.drive.steering_activity,
            },
            server.clients_connected.clone(),
        )),
        Err(e) => {
            tracing::error!("Ogiltig car_client_addr i config: {e} — ingen körning möjlig.");
            None
        }
    };
    let car_client_ok = || link.as_ref().is_some_and(|l| l.status.borrow().connected);

    let mut lighting = match gpio_lighting::LightingRelay::new(cfg.lighting_gpio_pin, false) {
        Ok(l) => {
            tracing::info!("Belysningsrelä redo på GPIO{}.", cfg.lighting_gpio_pin);
            Some(l)
        }
        Err(e) => {
            tracing::warn!("Ingen belysningsstyrning tillgänglig ({e}) — fortsätter utan.");
            None
        }
    };
    let mut lights_on = false;

    let mut status_display = match display::StatusDisplay::new("/dev/i2c-1") {
        Ok(d) => {
            tracing::info!("OLED-display redo.");
            Some(d)
        }
        Err(e) => {
            tracing::warn!("Ingen OLED-display tillgänglig ({e}) — fortsätter utan.");
            None
        }
    };
    if let Some(d) = &mut status_display {
        let _ = d.show(&display::DisplayStatus {
            pairing_code: None,
            internet_ok: check_internet(),
            car_client_ok: car_client_ok(),
            usb_ok: check_usb(&cfg.usb_device_path),
            battery_percent: None, // väntar på §13
            last_error: None,
        });
    }

    // Kameraströmmen (egen port, se video.rs). Ett fel där får aldrig stoppa
    // styrningen, så den startas fristående och loggar bara.
    video::spawn(cfg.video.clone(), &cfg.bind_addr);

    let mut throttle_ramp = RateLimiter::new(2.0);
    let mut steering_ramp = RateLimiter::new(4.0);
    let mut watchdog = Watchdog::new(cfg.watchdog_soft_ms, cfg.watchdog_hard_ms);

    let mut tick = tokio::time::interval(Duration::from_millis(20)); // 50Hz styrloop
    let mut last_tick = Instant::now();
    // OLED:en behöver inte uppdateras 50 ggr/s — den sitter bakom ett
    // stängt lock, var 2:a sekund räcker gott för att vara "levande" när
    // man väl öppnar den.
    let mut display_tick = tokio::time::interval(Duration::from_secs(2));
    // Senaste spakvärden från föraren (-1..1), nollade om AKTIVERA inte är på.
    let mut target_throttle = 0.0f32;
    let mut target_steering = 0.0f32;
    // För körloggen: senaste kommandots AKTIVERA-flagga, antal kommandon sedan
    // förra loggraden och watchdog-läget, så att det syns vad som händer.
    let mut last_activated = false;
    let mut cmds_since_log: u32 = 0;
    let mut last_wd_state = WatchdogState::HardStop;
    let mut last_drive_log = Instant::now();
    // Förarens max från klienten (Max-reglaget), om den skickar något.
    let mut client_max: Option<f32> = None;
    // Status till klienterna två gånger per sekund.
    let mut status_tick = tokio::time::interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let now = Instant::now();
                let dt = now.duration_since(last_tick).as_secs_f32();
                last_tick = now;

                let wd_state = watchdog.state(now);
                if wd_state != last_wd_state {
                    tracing::info!("Watchdog: {last_wd_state:?} -> {wd_state:?}");
                    last_wd_state = wd_state;
                }
                match wd_state {
                    WatchdogState::Ok => {
                        throttle_ramp.step(target_throttle, dt);
                        steering_ramp.step(target_steering, dt);
                    }
                    WatchdogState::SoftBrake => {
                        throttle_ramp.step(0.0, dt);
                        steering_ramp.step(0.0, dt);
                    }
                    WatchdogState::HardStop => {
                        throttle_ramp.reset(0.0);
                        steering_ramp.reset(0.0);
                    }
                }

                if let Some(l) = &link {
                    let sp = drive_setpoint(&cfg.drive, client_max, throttle_ramp.current(), steering_ramp.current());
                    // Körlogg en gång per sekund medan någon är ansluten.
                    if now.duration_since(last_drive_log) >= Duration::from_secs(1) {
                        if cmds_since_log > 0 || !sp.is_zero() {
                            tracing::info!(
                                "Körning: {cmds_since_log} kommandon/s, AKTIVERA={last_activated}, \
                                 spak gas={target_throttle:.2} styr={target_steering:.2} -> \
                                 skickar fart={:.3} styr={:.3}, Car_Client {}",
                                sp.speed,
                                sp.steering,
                                if l.status.borrow().connected { "ansluten" } else { "EJ ansluten" },
                            );
                        }
                        cmds_since_log = 0;
                        last_drive_log = now;
                    }
                    l.setpoint.send_if_modified(|cur| {
                        let changed = *cur != sp;
                        *cur = sp;
                        changed
                    });
                }

                // TODO: bygg en riktig StatusUpdate från Car_Client-telemetri
                // (hastighet, batteri, GPS, VESC-temp) och skicka den via
                // server.status_broadcast, istället för att inte skicka
                // någon status alls just nu.
            }
            _ = status_tick.tick() => {
                if let Some(l) = &link {
                    let status = build_status(&l.status.borrow(), &l.telemetry.borrow(), &cfg.known_vesc_ids);
                    // Aldrig vänta på nätverket här — hellre tappa en status.
                    let _ = server.status_broadcast.try_send(status);
                }
            }
            _ = display_tick.tick() => {
                if let Some(d) = &mut status_display {
                    let _ = d.show(&display::DisplayStatus {
                        pairing_code: None,
                        internet_ok: check_internet(),
                        car_client_ok: car_client_ok(),
                        usb_ok: check_usb(&cfg.usb_device_path),
                        battery_percent: None, // väntar på §13
                        last_error: None,
                    });
                }
            }
            Some(cmd) = server.incoming_control.recv() => {
                watchdog.command_received(Instant::now());
                cmds_since_log += 1;
                last_activated = cmd.activated;
                tracing::debug!("Kommando mottaget: throttle={} steering={}", cmd.throttle, cmd.steering);
                // Klienten nollar redan när AKTIVERA är av, men robotd litar inte på det.
                let clean = |v: f32| if cmd.activated && v.is_finite() { v.clamp(-1.0, 1.0) } else { 0.0 };
                target_throttle = clean(cmd.throttle);
                target_steering = clean(cmd.steering);
                client_max = cmd.max_output.filter(|m| m.is_finite());

                if cmd.lights != lights_on {
                    if let Some(l) = &mut lighting {
                        l.set(cmd.lights);
                    }
                    lights_on = cmd.lights;
                }
            }
            Some(req) = server.incoming_requests.recv() => {
                match req {
                    RobotRequest::ScanVescBus { respond_to } => {
                        // Egen uppgift med egen kortvarig anslutning: en skanning
                        // kan ta flera sekunder och ska inte blockera huvudloopen.
                        // Vid fel svarar vi med tom lista (så Settings-vyn inte
                        // hänger sig i "Skannar...") och loggar orsaken.
                        tracing::info!("Settings-vy begärde VESC-skanning — frågar styrkortet (CMD_GET_VESC_STATUS).");
                        let known = cfg.known_vesc_ids.clone();
                        let scan = link.as_ref().map(|l| l.scan.clone());
                        tokio::spawn(async move {
                            let found: Vec<u8> = match scan_vesc_bus(scan).await {
                                Ok(entries) => {
                                    tracing::info!("VESC-skanning klar: {entries:?}");
                                    entries.iter().filter(|e| e.is_fresh()).map(|e| e.can_id).collect()
                                }
                                Err(e) => {
                                    tracing::warn!("VESC-skanning misslyckades: {e}");
                                    Vec::new()
                                }
                            };
                            let sightings = merge_scan_with_known(&found, &known);
                            tracing::info!(
                                "Skickar {} VESC till Settings-vyn ({} svarade, resten är kända men svarade inte).",
                                sightings.len(),
                                found.len()
                            );
                            let _ = respond_to.send(RobotMessage::VescBusResult(sightings)).await;
                        });
                    }
                    RobotRequest::ReadActuators { respond_to } => {
                        let config = link.as_ref().map(|l| l.config.clone());
                        tokio::spawn(async move {
                            let r = config_request(config, car_client_link::ConfigRequest::Read).await;
                            let _ = respond_to.send(match r {
                                Ok(c) => RobotMessage::Actuators(c),
                                Err(e) => RobotMessage::ActuatorError(e),
                            }).await;
                        });
                    }
                    RobotRequest::WriteActuators { config: wanted, respond_to } => {
                        // Under skrivningen skickas inga styrvärden (några sekunder),
                        // så det får bara ske när roboten står still och inte är aktiverad.
                        let moving = throttle_ramp.current() != 0.0 || steering_ramp.current() != 0.0;
                        if last_activated || moving {
                            let _ = respond_to.send(RobotMessage::ActuatorError(
                                "Stäng av AKTIVERA och låt roboten stå still innan du skriver till styrkortet.".to_string(),
                            )).await;
                        } else {
                            tracing::info!("Klienten vill skriva aktuatorer: {wanted:?}");
                            let config = link.as_ref().map(|l| l.config.clone());
                            tokio::spawn(async move {
                                let r = config_request(config, |reply| {
                                    car_client_link::ConfigRequest::Write(wanted, reply)
                                }).await;
                                let _ = respond_to.send(match r {
                                    Ok(c) => RobotMessage::ActuatorsWritten(c),
                                    Err(e) => RobotMessage::ActuatorError(e),
                                }).await;
                            });
                        }
                    }
                    RobotRequest::SaveVescProfile { profile, respond_to } => {
                        let mut new_cfg = cfg.clone();
                        new_cfg.vesc_profile.roller = profile.roller.into_iter()
                            .map(|(id, role)| (id, config::from_msg_role(role)))
                            .collect();
                        if let Err(e) = new_cfg.save(Path::new(CONFIG_PATH)) {
                            tracing::error!("Kunde inte spara VESC-profil: {e}");
                        } else {
                            tracing::info!("VESC-profil sparad ({} roller).", new_cfg.vesc_profile.roller.len());
                        }
                        let _ = respond_to.send(RobotMessage::VescProfileSaved).await;
                    }
                }
            }
        }
    }
}

/// Skanningsresultat + kända ID:n till en lista för Settings-vyn: de som
/// svarade märks `responding: true`, kända som inte svarade `false`.
fn merge_scan_with_known(found: &[u8], known: &[u8]) -> Vec<VescSighting> {
    let mut out: Vec<VescSighting> = found
        .iter()
        .map(|&can_id| VescSighting { can_id, responding: true })
        .collect();
    for &can_id in known {
        if !found.contains(&can_id) {
            out.push(VescSighting { can_id, responding: false });
        }
    }
    out
}

/// Ber länkuppgiften fråga styrkortet om VESC-status.
async fn scan_vesc_bus(
    scan: Option<tokio::sync::mpsc::Sender<car_client_link::ScanReply>>,
) -> Result<Vec<vesc_can::VescStatusEntry>, String> {
    let scan = scan.ok_or("ingen Car_Client-länk (ogiltig car_client_addr)")?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    scan.send(tx).await.map_err(|_| "Car_Client-länken har avslutats")?;
    rx.await.map_err(|_| "Car_Client-länken avbröt skanningen".to_string())?
}

/// Skickar en inställningsfråga till länkuppgiften och väntar på svaret.
async fn config_request(
    config: Option<tokio::sync::mpsc::Sender<car_client_link::ConfigRequest>>,
    make: impl FnOnce(car_client_link::ConfigReply) -> car_client_link::ConfigRequest,
) -> Result<relay_protocol::ActuatorConfig, String> {
    let config = config.ok_or("ingen Car_Client-länk (ogiltig car_client_addr)")?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    config.send(make(tx)).await.map_err(|_| "Car_Client-länken har avslutats")?;
    rx.await.map_err(|_| "Car_Client-länken avbröt".to_string())?
}

/// Rampade spakvärden (-1..1) till värden för styrkortet. Förarens max från
/// klienten gäller för både fart och styrning (som Max-rutan i RControlStation),
/// begränsat av `max_cap`; utan det används config:ens `speed_max`/`steering_max`.
fn drive_setpoint(
    d: &config::DriveConfig,
    client_max: Option<f32>,
    throttle: f32,
    steering: f32,
) -> car_client_link::DriveSetpoint {
    let cap = d.max_cap.clamp(0.0, 1.0);
    let scale = |v: f32, max: f32, invert: bool| {
        let max = client_max.unwrap_or(max).clamp(0.0, cap);
        let v = v.clamp(-1.0, 1.0) * max;
        if invert { -v } else { v }
    };
    car_client_link::DriveSetpoint {
        speed: scale(throttle, d.speed_max, d.invert_speed),
        steering: scale(steering, d.steering_max, d.invert_steering),
    }
}

/// Enkel koll för OLED-displayen: är WireGuard-gränssnittet (`wg0`) uppe?
/// Ingen ping, ingen extern förfrågan — bara att gränssnittet självt
/// rapporterar "up" i kärnan. Räcker för "syns roboten på VPN:et alls",
/// inte en fullständig internetkontroll.
/// StatusUpdate till klienterna från länkens läge och styrkortets telemetri.
fn build_status(
    link: &car_client_link::LinkStatus,
    t: &car_client_link::Telemetry,
    expected: &[u8],
) -> StatusUpdate {
    let board = t.board;
    let fault = board.filter(|b| b.fault_code != 0).map(|b| format!("VESC-fel, kod {}", b.fault_code));
    StatusUpdate {
        speed_kmh: board.map(|b| b.speed_ms.abs() * 3.6),
        battery_percent: board.map(|b| vesc_can::estimate_battery_percent(b.v_in)),
        gps_fix: false,
        vesc_temps_c: board.map(|b| vec![b.temp_mos]).unwrap_or_default(),
        last_error: link.error.clone().or(fault),
        link_quality: relay_protocol::LinkQuality::Green,
        battery_voltage: board.map(|b| b.v_in),
        vescs_responding: t.vescs.iter().filter(|e| e.is_fresh()).map(|e| e.can_id).collect(),
        vescs_expected: expected.to_vec(),
    }
}

fn check_internet() -> bool {
    std::fs::read_to_string("/sys/class/net/wg0/operstate")
        .map(|s| s.trim() == "up")
        .unwrap_or(false)
}

/// Enkel koll för OLED-displayen: finns den förväntade USB-enheten
/// (CarController-kortet) där den ska vara. Bara en "finns filen"-koll,
/// inget djupare (rör inte serieporten, konkurrerar inte med Car_Client).
fn check_usb(device_path: &str) -> bool {
    std::path::Path::new(device_path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kanda_vesc_som_inte_svarade_markeras_tysta() {
        let s = merge_scan_with_known(&[36], &[28, 36, 76]);
        let get = |id| s.iter().find(|v| v.can_id == id).map(|v| v.responding);
        assert_eq!(s.len(), 3);
        assert_eq!(get(36), Some(true));
        assert_eq!(get(28), Some(false));
        assert_eq!(get(76), Some(false));
    }

    #[test]
    fn spakvarden_skalas_och_begransas() {
        let d = config::DriveConfig::default();
        let sp = drive_setpoint(&d, None, 1.0, -0.5);
        assert!((sp.speed - 0.45).abs() < 1e-6);
        assert!((sp.steering + 0.225).abs() < 1e-6);
        // Trasig config kan aldrig ge mer än fullt utslag.
        let wild = config::DriveConfig { speed_max: 7.0, steering_max: 7.0, invert_steering: true, ..d };
        let sp = drive_setpoint(&wild, None, 3.0, 1.0);
        assert_eq!(sp.speed, 1.0);
        assert_eq!(sp.steering, -1.0);
        assert!(drive_setpoint(&d, None, 0.0, 0.0).is_zero());
        // Förarens max gäller för båda, men aldrig över taket.
        let sp = drive_setpoint(&d, Some(0.6), 1.0, 1.0);
        assert!((sp.speed - 0.6).abs() < 1e-6 && (sp.steering - 0.6).abs() < 1e-6);
        let capped = config::DriveConfig { max_cap: 0.5, ..d.clone() };
        assert!((drive_setpoint(&capped, Some(0.9), 1.0, 0.0).speed - 0.5).abs() < 1e-6);
        assert_eq!(drive_setpoint(&d, Some(-3.0), 1.0, 0.0).speed, 0.0);
    }

    #[test]
    fn okand_vesc_som_svarar_tas_med() {
        let s = merge_scan_with_known(&[5], &[28]);
        assert!(s.iter().any(|v| v.can_id == 5 && v.responding));
        assert!(s.iter().any(|v| v.can_id == 28 && !v.responding));
    }
}
