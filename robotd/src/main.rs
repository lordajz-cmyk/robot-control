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

    // Ansluter till det redan körande Car_Client (se PROJECT_SPEC.md §13) —
    // det är kortlänken, oberoende av nätverket mot klienten nedan.
    //
    // Upprepade försök, inte bara ett: systemd-tjänsten har redan en
    // 90-sekunders paus innan robotd ens startar (se install_pi.sh, lärdom
    // från RControlStation-erfarenheten — 5G-modem ~90s, CarController
    // ~60s att bli redo), men den pausen är en fast gissning. Om Car_Client
    // av någon anledning tar längre tid en enskild dag ska robotd ändå
    // hitta den istället för att permanent ge upp efter ett enda försök.
    let mut car_client_ok = false;
    match cfg.car_client_addr.parse() {
        Ok(addr) => {
            const MAX_ATTEMPTS: u32 = 10;
            const RETRY_DELAY: Duration = Duration::from_secs(3);
            for attempt in 1..=MAX_ATTEMPTS {
                match car_client_link::CarClientLink::connect(addr).await {
                    Ok(_link) => {
                        tracing::info!("Ansluten till Car_Client-länken (försök {attempt}/{MAX_ATTEMPTS}).");
                        car_client_ok = true;
                        break;
                    }
                    Err(e) if attempt < MAX_ATTEMPTS => {
                        tracing::warn!(
                            "Kunde inte ansluta till Car_Client än (försök {attempt}/{MAX_ATTEMPTS}): {e}. \
                             Försöker igen om {RETRY_DELAY:?}."
                        );
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                    Err(e) => tracing::error!(
                        "Gav upp att ansluta till Car_Client på {addr} efter {MAX_ATTEMPTS} försök: {e}. \
                         Kontrollera att car_client.service kör (systemctl status car_client.service, \
                         eller screen -r car för live-loggen) på Pi:n, och att porten i config.json stämmer. \
                         robotd fortsätter köra ändå (belysning/OLED/nätverk fungerar oberoende)."
                    ),
                }
            }
        }
        Err(e) => tracing::error!("Ogiltig car_client_addr i config: {e}"),
    }

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
            car_client_ok,
            usb_ok: check_usb(&cfg.usb_device_path),
            battery_percent: None, // väntar på §13
            last_error: None,
        });
    }

    // Nätverket: robotd LYSSNAR direkt på sin WireGuard-IP (se
    // PROJECT_SPEC.md §14) — ingen central reläserver längre. `bind_addr`
    // sätts i /etc/robotd/config.json, t.ex. "192.168.200.8:9000".
    let mut server = server::spawn(cfg.bind_addr.clone());

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

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let now = Instant::now();
                let dt = now.duration_since(last_tick).as_secs_f32();
                last_tick = now;

                let target_throttle = 0.0; // sätts av senaste inkommande ControlCommand
                let target_steering = 0.0;

                match watchdog.state(now) {
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

                // TODO: skicka throttle_ramp.current()/steering_ramp.current()
                // som VESC-kommandon via vesc_can::make_set_rpm_frame enligt
                // sparad rollmappning i cfg.vesc_profile.

                // TODO: bygg en riktig StatusUpdate från Car_Client-telemetri
                // (hastighet, batteri, GPS, VESC-temp) och skicka den via
                // server.status_broadcast, istället för att inte skicka
                // någon status alls just nu.
            }
            _ = display_tick.tick() => {
                if let Some(d) = &mut status_display {
                    let _ = d.show(&display::DisplayStatus {
                        pairing_code: None,
                        internet_ok: check_internet(),
                        car_client_ok,
                        usb_ok: check_usb(&cfg.usb_device_path),
                        battery_percent: None, // väntar på §13
                        last_error: None,
                    });
                }
            }
            Some(cmd) = server.incoming_control.recv() => {
                watchdog.command_received(Instant::now());
                tracing::debug!("Kommando mottaget: throttle={} steering={}", cmd.throttle, cmd.steering);

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
                        // TODO: skanna riktiga CAN-bussen via car_client_link
                        // när VESC-telemetrins kommando-ID:n är bekräftade
                        // (se PROJECT_SPEC.md §13). Tills dess: tomt svar,
                        // så Settings-vyn inte hänger sig i "Skannar...".
                        tracing::info!("Settings-vy begärde VESC-skanning — ingen riktig CAN-koppling än.");
                        let result: Vec<VescSighting> = Vec::new();
                        let _ = respond_to.send(RobotMessage::VescBusResult(result)).await;
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

// Tyst varning: används inte just nu men signaturen finns kvar för när
// riktig StatusUpdate byggs ovan (se TODO).
#[allow(dead_code)]
fn _unused(_: StatusUpdate) {}

/// Enkel koll för OLED-displayen: är WireGuard-gränssnittet (`wg0`) uppe?
/// Ingen ping, ingen extern förfrågan — bara att gränssnittet självt
/// rapporterar "up" i kärnan. Räcker för "syns roboten på VPN:et alls",
/// inte en fullständig internetkontroll.
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
