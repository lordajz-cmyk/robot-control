//! Robotstyrning — klientprogrammet.
//!
//! Flöde (beslutat i spec): svart anslutningsskärm med ett textfält i
//! mitten (robotnamn eller IP) -> helskärm med kamerabild -> AKTIVERA-knapp
//! ovanpå bilden -> färgad ram runt hela fönstret (grön/gul/röd, lila vid
//! dosa-tapp) -> statusinfo i hörnet (hastighet, signal, ping, batteri).
//! Kan minimeras/flyttas som ett vanligt fönster, tvingar inte helskärm.
//! Språk: svenska.

mod gamepad;
mod net;
mod settings;
mod video;

use std::time::{Duration, Instant};

use eframe::egui;
use gamepad::GamepadReader;
use net::{ConnectionState, NetLink};
use relay_protocol::{ControlCommand, LinkQuality};

const ACTIVATE_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("kunde inte starta tokio-runtime");
    let rt_handle = rt.handle().clone();
    // Håll runtimen vid liv i bakgrunden så länge appen kör.
    std::thread::spawn(move || rt.block_on(std::future::pending::<()>()));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Robotstyrning",
        options,
        Box::new(move |_cc| Box::new(App::new(rt_handle))),
    )
}

enum Screen {
    Connect,
    Driving,
    Settings,
}

/// Styr bara vilka krav som gäller för att AKTIVERA ska gå att trycka —
/// INTE vad som visas på skärmen. Kamerabild (eller en framtida
/// VR-ström) renderas oavsett läge; se draw_driving_screen. VESC-kravet
/// (drift/styr svarar på bussen) gäller i båda lägena — det handlar om
/// att roboten mekaniskt går att styra alls, inte om föraren ser den.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrivingMode {
    /// Kräver kamerabild för att AKTIVERA ska gå att trycka.
    Cam,
    /// "Line of sight" — föraren ser roboten direkt, kamerakravet släpps.
    Los,
}

impl Default for DrivingMode {
    fn default() -> Self {
        DrivingMode::Cam
    }
}

struct App {
    screen: Screen,
    name_input: String,
    known_robots: Vec<String>, // laddas/sparas via `directories`-mappen
    net: NetLink,
    gamepad: Option<GamepadReader>,
    activated: bool,
    last_activity: Instant,
    gamepad_connected: bool,
    settings: settings::SettingsView,
    lights_on: bool,
    rt_handle: tokio::runtime::Handle,
    /// Standardporten robotd lyssnar på (se robotd:s config.json, `bind_addr`).
    robot_port: u16,
    /// Sätts när ett anslutningsförsök pågår; flyttas till `known_robots`
    /// bara vid lyckad anslutning (Fel 6, testrapport 2026-09-19 — sparades
    /// tidigare direkt vid försök, så felstavningar hamnade i listan).
    pending_target: Option<String>,
    /// Visas på anslutningsskärmen efter en nekad/avbruten anslutning
    /// (Fel 5 — syntes tidigare ingenstans i UI:t).
    connect_error: Option<String>,
    driving_mode: DrivingMode,
    /// Kameraströmmen (egen TCP-anslutning + egen tråd, se video.rs).
    video: video::VideoReceiver,
    /// robotd:s videoport (`video.port` i robotd:s config.json).
    video_port: u16,
    video_texture: Option<egui::TextureHandle>,
    /// Förarens Max (0..1) som i RControlStation, sparas mellan körningar.
    max_output: f32,
}

impl App {
    fn new(rt_handle: tokio::runtime::Handle) -> Self {
        Self {
            screen: Screen::Connect,
            name_input: String::new(),
            known_robots: load_known_robots(),
            net: NetLink::new(),
            gamepad: GamepadReader::new().ok(),
            activated: false,
            last_activity: Instant::now(),
            gamepad_connected: true,
            settings: settings::SettingsView::new(),
            lights_on: false,
            rt_handle,
            robot_port: 9000,
            pending_target: None,
            connect_error: None,
            driving_mode: DrivingMode::Cam,
            video: video::VideoReceiver::new(),
            video_port: 9001,
            video_texture: None,
            max_output: load_max_output(),
        }
    }

    fn border_color(&self) -> egui::Color32 {
        if !self.gamepad_connected {
            return egui::Color32::from_rgb(180, 80, 220); // lila: dosa frånkopplad
        }
        match self.net.link_quality(Instant::now()) {
            LinkQuality::Green => egui::Color32::from_rgb(60, 200, 90),
            LinkQuality::Yellow => egui::Color32::from_rgb(230, 190, 40),
            LinkQuality::Red => egui::Color32::from_rgb(220, 60, 60),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Fel 1 (testrapport 2026-09-19): måste köras HÄR, inte bara i
        // draw_driving_screen, annars slutar klienten läsa nätverket så
        // fort en annan skärm (t.ex. Settings) visas — kön fylls och hela
        // nätverksuppgiften fastnar på `send().await`.
        self.net.poll_updates();

        match self.net.state {
            ConnectionState::ConnectedAsDriver | ConnectionState::ConnectedAsViewer => {
                // Fel 6: robotnamnet sparas i listan först nu, vid bekräftat
                // lyckad anslutning — inte redan när användaren bara försökte.
                if let Some(target) = self.pending_target.take() {
                    if !self.known_robots.contains(&target) {
                        self.known_robots.push(target);
                        save_known_robots(&self.known_robots);
                    }
                }
                self.connect_error = None;
            }
            ConnectionState::Denied | ConnectionState::Disconnected => {
                // Fel 5: nekad/avbruten anslutning gav tidigare ingen
                // återkoppling i UI:t alls — fastnade på körskärmen utan
                // text. Nu: tillbaka till anslutningsskärmen med orsaken.
                if !matches!(self.screen, Screen::Connect) {
                    self.video.stop();
                    self.video_texture = None;
                    self.connect_error = Some(
                        self.net
                            .deny_reason
                            .clone()
                            .unwrap_or_else(|| "Anslutningen bröts.".to_string()),
                    );
                    self.screen = Screen::Connect;
                    self.pending_target = None;
                }
            }
            ConnectionState::Connecting => {}
        }

        match self.screen {
            Screen::Connect => self.draw_connect_screen(ctx),
            Screen::Driving => self.draw_driving_screen(ctx),
            Screen::Settings => {
                let mut close = false;
                self.settings.draw(ctx, &mut self.net, &mut || close = true);
                if close {
                    self.screen = Screen::Driving;
                }
            }
        }
        ctx.request_repaint_after(Duration::from_millis(33)); // ~30fps UI-loop
    }
}

impl App {
    fn draw_connect_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 60.0);

                    let field = egui::TextEdit::singleline(&mut self.name_input)
                        .hint_text("Robotnamn eller IP-adress")
                        .desired_width(320.0)
                        .text_color(egui::Color32::WHITE);
                    let response = ui.add(field);

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.connect_to(self.name_input.clone());
                    }

                    if let Some(err) = &self.connect_error {
                        ui.add_space(12.0);
                        ui.colored_label(egui::Color32::from_rgb(220, 90, 90), err);
                    }

                    ui.add_space(20.0);

                    if !self.known_robots.is_empty() {
                        ui.label(
                            egui::RichText::new("Tidigare anslutna").color(egui::Color32::GRAY),
                        );
                        for name in self.known_robots.clone() {
                            if ui
                                .add(egui::Button::new(
                                    egui::RichText::new(&name).color(egui::Color32::WHITE),
                                ))
                                .clicked()
                            {
                                self.connect_to(name);
                            }
                        }
                    }
                });
            });
    }

    fn connect_to(&mut self, target: String) {
        if target.trim().is_empty() {
            return;
        }
        self.connect_error = None;
        self.pending_target = Some(target.clone());
        // Alltid CAM som startläge vid varje ny anslutning — medvetet
        // vald standard, minns inte senast använda läget (spec-diskussion
        // 2026-09-19).
        self.driving_mode = DrivingMode::Cam;
        // Direktanslutning över WireGuard-tunneln — inget relä, ingen
        // parkopplingskod. `target` är robotens VPN-IP eller ett namn som
        // löses via klientens /etc/hosts (se PROJECT_SPEC.md §14).
        self.net.connect(&self.rt_handle, target.clone(), self.robot_port, false, None);
        // Videon på en egen anslutning till samma värd, så en tung bildruta
        // aldrig kan fördröja styrkommandon på styr-anslutningen.
        self.video_texture = None;
        self.video.start(target, self.video_port);
        self.screen = Screen::Driving;
    }

    fn draw_driving_screen(&mut self, ctx: &egui::Context) {
        // Poll gamepad varje bildruta.
        let mut gp_state = gamepad::GamepadState::default();
        if let Some(gp) = &mut self.gamepad {
            gp_state = gp.poll();
            self.gamepad_connected = gp_state.connected;
            if gp_state.connected
                && (gp_state.throttle.abs() > 0.02 || gp_state.steering.abs() > 0.02)
            {
                self.last_activity = Instant::now();
            }
        }

        // (poll_updates() körs nu centralt i App::update(), oavsett skärm —
        // se Fel 1 i testrapporten 2026-09-19: den fick tidigare bara köras
        // härifrån, vilket gjorde att nätverket slutade läsas så fort
        // Settings-vyn var öppen.)

        if self.activated && self.last_activity.elapsed() > ACTIVATE_IDLE_TIMEOUT {
            self.activated = false;
        }
        if !self.gamepad_connected {
            self.activated = false;
        }

        // Skicka styrkommando varje bildruta. Gas/styr nollställs om vi
        // inte är aktiverade (dosan låst), men tillval (lastarm/tilt) och
        // belysning skickas alltid, precis som beslutat (accessoarer är
        // inte låsta bakom AKTIVERA).
        let cmd = ControlCommand {
            throttle: if self.activated { gp_state.throttle } else { 0.0 },
            steering: if self.activated { gp_state.steering } else { 0.0 },
            accessory_a: gp_state.accessory_a,
            accessory_b: gp_state.accessory_b,
            lights: self.lights_on,
            activated: self.activated,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            max_output: Some(self.max_output),
        };
        self.net.send_control(cmd);

        let border = self.border_color();

        // Ny kamerabild -> textur. Bara den senaste bilden uppdateras; om
        // avkodningen ligger före UI:t visas alltid den färskaste.
        if let Some(f) = self.video.take_new_frame() {
            let img = egui::ColorImage::from_rgb([f.width, f.height], &f.rgb);
            match &mut self.video_texture {
                Some(tex) => tex.set(img, egui::TextureOptions::LINEAR),
                None => {
                    self.video_texture =
                        Some(ctx.load_texture("kamera", img, egui::TextureOptions::LINEAR))
                }
            }
        }
        let video_live = self.video.is_live();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_gray(20)))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(10));
                match &self.video_texture {
                    Some(tex) => {
                        // Rita bilden så stor som möjligt utan att förvränga den.
                        let tex_size = tex.size_vec2();
                        let scale = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                        let dest = egui::Rect::from_center_size(rect.center(), tex_size * scale);
                        ui.painter().image(
                            tex.id(),
                            dest,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        if !video_live {
                            // En stillastående bild ser levande ut — säg det
                            // rakt ut när strömmen fryst eller brutits.
                            ui.painter().rect_filled(
                                dest,
                                0.0,
                                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
                            );
                            ui.painter().text(
                                rect.center() + egui::vec2(0.0, 90.0),
                                egui::Align2::CENTER_CENTER,
                                "BILDEN FRUSEN – ingen ny bild från roboten",
                                egui::FontId::proportional(26.0),
                                egui::Color32::from_rgb(255, 120, 120),
                            );
                        }
                    }
                    None => {
                        ui.painter().text(
                            rect.center() + egui::vec2(0.0, 90.0),
                            egui::Align2::CENTER_CENTER,
                            format!("Ingen bild ännu ({})", self.video.status_line()),
                            egui::FontId::proportional(24.0),
                            egui::Color32::GRAY,
                        );
                    }
                }

                // Statusinfo, övre vänstra hörnet.
                if self.driving_mode == DrivingMode::Los {
                    // Ren visuell markör om aktivt läge — påverkar inte
                    // kamerabilden/videoströmmen alls, den renderas oavsett
                    // (spec-diskussion 2026-09-19: kameran ska kunna
                    // strömmas/spelas in även när man själv kör LOS, t.ex.
                    // med VR-glasögon på).
                    egui::Area::new("los_badge".into())
                        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 16.0))
                        .show(ctx, |ui| {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgba_unmultiplied(180, 80, 220, 220))
                                .inner_margin(egui::Margin::symmetric(14.0, 6.0))
                                .rounding(6.0)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("LOS-läge").color(egui::Color32::WHITE).strong());
                                });
                        });
                }

                egui::Area::new("status".into())
                    .fixed_pos(rect.left_top() + egui::vec2(16.0, 16.0))
                    .show(ctx, |ui| {
                        ui.colored_label(egui::Color32::WHITE, self.status_text());
                    });

                // Belysningsknapp + körläge + inställningar, övre högra hörnet.
                egui::Area::new("lighting".into())
                    .fixed_pos(rect.right_top() + egui::vec2(-290.0, 16.0))
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            let lights_label = if self.lights_on { "💡 Belysning PÅ" } else { "💡 Belysning" };
                            if ui.button(lights_label).clicked() {
                                self.lights_on = !self.lights_on;
                            }

                            // Läget låst medan AKTIVERAD — ska inte gå att
                            // byta mitt i körning, bara innan/efter.
                            let mode_label = match self.driving_mode {
                                DrivingMode::Cam => "📷 CAM",
                                DrivingMode::Los => "👁 LOS",
                            };
                            if ui
                                .add_enabled(!self.activated, egui::Button::new(mode_label))
                                .on_hover_text("Växla mellan CAM (kräver kamerabild) och LOS (fri sikt, inget kamerakrav)")
                                .clicked()
                            {
                                self.driving_mode = match self.driving_mode {
                                    DrivingMode::Cam => DrivingMode::Los,
                                    DrivingMode::Los => DrivingMode::Cam,
                                };
                            }

                            if ui.button("⚙").on_hover_text("Inställningar").clicked() {
                                self.screen = Screen::Settings;
                            }
                        });
                        ui.horizontal(|ui| {
                            let resp = ui
                                .add(
                                    egui::DragValue::new(&mut self.max_output)
                                        .clamp_range(0.05..=1.0)
                                        .speed(0.005)
                                        .fixed_decimals(2)
                                        .prefix("Max: "),
                                )
                                .on_hover_text(
                                    "Värdet som skickas vid fullt spakutslag (som Max i RControlStation). \
                                     Dra eller dubbelklicka och skriv. Gäller direkt, även under körning.",
                                );
                            if resp.changed() {
                                save_max_output(self.max_output);
                            }
                        });
                    });

                if !self.activated {
                    egui::Area::new("activate".into())
                        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                        .show(ctx, |ui| {
                            if !self.gamepad_connected {
                                // Fix efter regressionstest 2026-09-19 (Dator
                                // 2): utan det här visade knappen bara
                                // AKTIVERA, gick att trycka, men föll
                                // omedelbart tillbaka varje bildruta eftersom
                                // `activated` nollställs utan dosa (spec §6)
                                // — kändes trasigt istället för avsiktligt.
                                ui.vertical_centered(|ui| {
                                    let frame = egui::Frame::none()
                                        .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                                        .inner_margin(egui::Margin::symmetric(24.0, 16.0))
                                        .rounding(8.0);
                                    frame.show(ui, |ui| {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(200, 140, 220),
                                            egui::RichText::new("Ingen dosa ansluten").size(20.0).strong(),
                                        );
                                    });
                                });
                                return;
                            }
                            let btn = egui::Button::new(
                                egui::RichText::new("AKTIVERA").size(32.0).strong(),
                            )
                            .fill(egui::Color32::from_rgb(40, 140, 60))
                            .min_size(egui::vec2(220.0, 70.0));
                            if ui.add_enabled(self.can_activate(), btn).clicked() {
                                self.activated = true;
                                self.last_activity = Instant::now();
                            }
                            if let Some(why) = self.activate_blocker() {
                                ui.colored_label(egui::Color32::from_rgb(255, 170, 90), why);
                            }
                        });
                }
            });

        // Rita ramen SIST, på ett eget lager ovanpå allt annat innehåll.
        // Tidigare låg den som Frame-stroke bakom kamerabildens
        // rect_filled, så den syntes aldrig (Fel 2, testrapport 2026-09-19).
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("border")))
            .rect_stroke(ctx.screen_rect(), 0.0, egui::Stroke::new(6.0, border));
    }

    /// Kamerabild + drift-/styr-VESC måste vara OK innan Aktivera går att
    /// trycka. GPS/temp/övrigt varnar bara, blockerar inte.
    /// VESC-kravet (drift/styr svarar) gäller ALLTID, oavsett läge — det
    /// handlar om att roboten mekaniskt går att styra, inte om föraren ser
    /// den. Kamerakravet gäller bara i CAM-läge; i LOS-läge släpps det,
    /// eftersom föraren då litar på egen sikt istället för skärmen.
    fn can_activate(&self) -> bool {
        self.activate_blocker().is_none()
    }

    /// Varför AKTIVERA inte går att trycka, eller `None` om allt är OK.
    fn activate_blocker(&self) -> Option<String> {
        // VESC-kravet gäller i båda lägena: alla VESC som ska finnas på
        // roboten måste ha skickat CAN-status nyss (enligt färsk status från robotd).
        let Some(status) = self.net.fresh_status() else {
            return Some("Väntar på status från roboten…".to_string());
        };
        if let Some(err) = &status.last_error {
            if status.vescs_responding.is_empty() {
                return Some(err.clone());
            }
        }
        let missing: Vec<u8> = status
            .vescs_expected
            .iter()
            .copied()
            .filter(|id| !status.vescs_responding.contains(id))
            .collect();
        if !missing.is_empty() {
            return Some(format!("VESC svarar inte: {missing:?}"));
        }
        if status.vescs_expected.is_empty() && status.vescs_responding.is_empty() {
            return Some("Ingen VESC svarar".to_string());
        }
        // Kamerakravet gäller bara i CAM-läge: en NY bild måste ha kommit nyligen.
        if self.driving_mode == DrivingMode::Cam && !self.video.is_live() {
            return Some("Ingen kamerabild – byt till LOS för att köra utan".to_string());
        }
        None
    }

    fn status_text(&self) -> String {
        let status = self.net.fresh_status();
        let speed = status
            .and_then(|s| s.speed_kmh)
            .map(|v| format!("{v:.1} km/h"))
            .unwrap_or_else(|| "–".to_string());
        let battery = match (status.and_then(|s| s.battery_voltage), status.and_then(|s| s.battery_percent)) {
            (Some(v), Some(p)) => format!("{v:.1} V ({p:.0}%)"),
            (Some(v), None) => format!("{v:.1} V"),
            (None, Some(p)) => format!("{p:.0}%"),
            (None, None) => "–".to_string(),
        };
        let vesc = match status {
            Some(s) if !s.vescs_expected.is_empty() => {
                let ok = s.vescs_expected.iter().filter(|id| s.vescs_responding.contains(id)).count();
                format!("{ok}/{} svarar", s.vescs_expected.len())
            }
            Some(s) => format!("{} svarar", s.vescs_responding.len()),
            None => "–".to_string(),
        };
        let temp = status
            .and_then(|s| s.vesc_temps_c.iter().copied().reduce(f32::max))
            .map(|t| format!("{t:.0} °C"))
            .unwrap_or_else(|| "–".to_string());
        let ping = self
            .net
            .last_ping_ms
            .map(|v| format!("{v} ms"))
            .unwrap_or_else(|| "–".to_string());
        let mut text = format!(
            "Hastighet: {speed}\nBatteri: {battery}\nVESC: {vesc}\nTemp: {temp}\nPing: {ping}\nVideo: {}",
            self.video.status_line()
        );
        if let Some(err) = status.and_then(|s| s.last_error.as_ref()) {
            text.push_str(&format!("\n⚠ {err}"));
        }
        text
    }
}

fn max_output_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("se", "robot-control", "robotstyrning")
        .map(|d| d.config_dir().join("max.json"))
}

/// Förarens Max, 0.45 om inget sparats (det som används på RobAnt).
fn load_max_output() -> f32 {
    max_output_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<f32>(&s).ok())
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(0.05, 1.0))
        .unwrap_or(0.45)
}

fn save_max_output(v: f32) {
    if let Some(path) = max_output_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, format!("{v:.2}"));
    }
}

fn known_robots_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("se", "robot-control", "robotstyrning")
        .map(|d| d.config_dir().join("kanda_robotar.json"))
}

fn load_known_robots() -> Vec<String> {
    known_robots_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_known_robots(names: &[String]) {
    if let Some(path) = known_robots_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(names) {
            let _ = std::fs::write(path, json);
        }
    }
}
