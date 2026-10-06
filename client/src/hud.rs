//! Instrumentpanelen i körvyn (2026-10-05, kundens önskemål om fordonsdata):
//! batterimätare med laddningsindikator, felkoder i klartext, fart, temperatur,
//! styrvinkel, lutning och kurs, mobilnätets signal och räckvidd.
//!
//! Ritas på en mörk, halvgenomskinlig botten så att den syns mot ljus kamerabild.
//! Värden som robotd inte skickar (äldre robotd, saknad givare) visas som "–".

use eframe::egui::{self, Color32, RichText};
use relay_protocol::StatusUpdate;

const GREEN: Color32 = Color32::from_rgb(110, 235, 140);
const AMBER: Color32 = Color32::from_rgb(255, 190, 70);
const RED: Color32 = Color32::from_rgb(255, 90, 90);
const DIM: Color32 = Color32::from_rgb(170, 180, 190);
const TEXT: Color32 = Color32::from_rgb(225, 235, 230);
const TRACK: Color32 = Color32::from_rgba_premultiplied(70, 80, 90, 200);

pub fn battery_color(percent: f32) -> Color32 {
    if percent < 20.0 {
        RED
    } else if percent < 40.0 {
        AMBER
    } else {
        GREEN
    }
}

/// Antal staplar (0–4) för mobilnätets RSSI.
pub fn signal_bars(rssi_dbm: i32) -> u8 {
    match rssi_dbm {
        r if r >= -65 => 4,
        r if r >= -75 => 3,
        r if r >= -85 => 2,
        r if r >= -95 => 1,
        _ => 0,
    }
}

/// Styrvinkeln i text, t.ex. "V 40 %", "H 25 %" eller "rakt".
pub fn steering_label(percent: f32) -> String {
    if percent.abs() < 3.0 {
        "rakt".to_string()
    } else if percent < 0.0 {
        format!("V {:.0} %", -percent)
    } else {
        format!("H {percent:.0} %")
    }
}

/// Färg för lutning (grader): gul över 15°, röd över 25°.
pub fn tilt_color(deg: f32) -> Color32 {
    let a = deg.abs();
    if a > 25.0 {
        RED
    } else if a > 15.0 {
        AMBER
    } else {
        TEXT
    }
}

fn label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(DIM).size(13.0));
}

/// Hela panelen. `blink` växlar ungefär en gång per sekund (för låg batterinivå).
pub fn show(ui: &mut egui::Ui, s: Option<&StatusUpdate>, ping_ms: Option<u32>, video: &str, blink: bool) {
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 5.0);
    ui.set_min_width(330.0);

    // --- Batteri ---------------------------------------------------------------
    ui.horizontal(|ui| {
        label(ui, "Batteri");
        let pct = s.and_then(|s| s.battery_percent);
        let (w, h) = (150.0, 22.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w + 6.0, h), egui::Sense::hover());
        let body = egui::Rect::from_min_size(rect.min, egui::vec2(w, h));
        let nub = egui::Rect::from_min_size(egui::pos2(body.right(), body.center().y - 5.0), egui::vec2(5.0, 10.0));
        let p = ui.painter();
        p.rect_filled(body, 4.0, TRACK);
        p.rect_filled(nub, 2.0, TRACK);
        if let Some(pct) = pct {
            let mut c = battery_color(pct);
            if pct < 15.0 && blink {
                c = c.linear_multiply(0.35); // blinkar när det nästan är tomt
            }
            let fill = egui::Rect::from_min_size(body.min, egui::vec2(w * pct / 100.0, h));
            p.rect_filled(fill.shrink(2.0), 3.0, c);
            p.text(body.center(), egui::Align2::CENTER_CENTER, format!("{pct:.0} %"), egui::FontId::proportional(15.0), Color32::BLACK);
        } else {
            p.text(body.center(), egui::Align2::CENTER_CENTER, "–", egui::FontId::proportional(15.0), DIM);
        }
        p.rect_stroke(body, 4.0, egui::Stroke::new(1.0_f32, DIM));

        let volt = s.and_then(|s| s.battery_voltage).map(|v| format!("{v:.1} V")).unwrap_or_else(|| "– V".into());
        ui.label(RichText::new(volt).color(TEXT).size(15.0).strong());
        match s.and_then(|s| s.battery_charging) {
            Some(true) => {
                ui.label(RichText::new("⚡ laddar").color(GREEN).size(14.0).strong());
            }
            Some(false) => {
                ui.label(RichText::new("laddar inte").color(DIM).size(13.0));
            }
            None => {}
        }
    });
    if let Some(pct) = s.and_then(|s| s.battery_percent) {
        if pct < 15.0 {
            ui.label(RichText::new("⚠ Lågt batteri – kör hem och ladda").color(RED).size(14.0).strong());
        }
    }

    // --- Räckvidd och effekt ---------------------------------------------------
    ui.horizontal(|ui| {
        label(ui, "Räckvidd");
        let range = match (s.and_then(|s| s.range_km), s.and_then(|s| s.wh_per_km)) {
            (Some(r), Some(w)) => format!("≈ {r:.1} km  ({w:.0} Wh/km)"),
            (None, Some(w)) => format!("–  ({w:.0} Wh/km)"),
            _ => "–".to_string(),
        };
        ui.label(RichText::new(range).color(TEXT).size(15.0));
        if let Some(p) = s.and_then(|s| s.power_w) {
            ui.label(RichText::new(format!("· {p:.0} W")).color(DIM).size(13.0));
        }
    });

    // --- Fart, temperatur, VESC ------------------------------------------------
    ui.horizontal(|ui| {
        label(ui, "Fart");
        let speed = s.and_then(|s| s.speed_kmh).map(|v| format!("{v:.1} km/h")).unwrap_or_else(|| "–".into());
        ui.label(RichText::new(speed).color(TEXT).size(18.0).strong());
        label(ui, "Temp");
        match s.and_then(|s| s.vesc_temps_c.iter().copied().reduce(f32::max)) {
            Some(t) => {
                let c = if t > 80.0 { RED } else if t > 65.0 { AMBER } else { TEXT };
                ui.label(RichText::new(format!("{t:.0} °C")).color(c).size(15.0));
            }
            None => {
                ui.label(RichText::new("–").color(DIM));
            }
        }
        label(ui, "VESC");
        let vesc = match s {
            Some(s) if !s.vescs_expected.is_empty() => {
                let ok = s.vescs_expected.iter().filter(|id| s.vescs_responding.contains(id)).count();
                let c = if ok == s.vescs_expected.len() { TEXT } else { RED };
                RichText::new(format!("{ok}/{}", s.vescs_expected.len())).color(c)
            }
            Some(s) => RichText::new(format!("{}", s.vescs_responding.len())).color(TEXT),
            None => RichText::new("–").color(DIM),
        };
        ui.label(vesc.size(15.0));
    });

    // --- Styrning --------------------------------------------------------------
    ui.horizontal(|ui| {
        label(ui, "Styrning");
        let pct = s.and_then(|s| s.steering_percent);
        let (w, h) = (150.0, 12.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 3.0, TRACK);
        p.line_segment(
            [egui::pos2(rect.center().x, rect.top() - 2.0), egui::pos2(rect.center().x, rect.bottom() + 2.0)],
            egui::Stroke::new(1.0_f32, DIM),
        );
        if let Some(pct) = pct {
            let x = rect.center().x + rect.width() / 2.0 * (pct / 100.0).clamp(-1.0, 1.0);
            let bar = egui::Rect::from_two_pos(egui::pos2(rect.center().x, rect.top() + 2.0), egui::pos2(x, rect.bottom() - 2.0));
            p.rect_filled(bar, 2.0, GREEN);
            p.circle_filled(egui::pos2(x, rect.center().y), 5.0, Color32::WHITE);
        }
        let text = match (pct, s.and_then(|s| s.steering_deg)) {
            (Some(p), Some(d)) => format!("{}  ({d:.0}°)", steering_label(p)),
            _ => "–".to_string(),
        };
        ui.label(RichText::new(text).color(TEXT).size(14.0));
    });

    // --- Lutning och kurs ------------------------------------------------------
    ui.horizontal(|ui| {
        label(ui, "Lutning");
        match (s.and_then(|s| s.roll_deg), s.and_then(|s| s.pitch_deg)) {
            (Some(r), Some(p)) => {
                ui.label(RichText::new(format!("sida {r:+.0}°")).color(tilt_color(r)).size(14.0));
                ui.label(RichText::new(format!("fram/bak {p:+.0}°")).color(tilt_color(p)).size(14.0));
            }
            _ => {
                ui.label(RichText::new("–").color(DIM));
            }
        }
        label(ui, "Kurs");
        let yaw = s.and_then(|s| s.yaw_deg).map(|y| format!("{:.0}°", y.rem_euclid(360.0))).unwrap_or_else(|| "–".into());
        ui.label(RichText::new(yaw).color(TEXT).size(14.0));
    });

    // --- Signal, ping, video ---------------------------------------------------
    ui.horizontal(|ui| {
        label(ui, "Signal");
        let rssi = s.and_then(|s| s.rssi_dbm);
        let bars = rssi.map(signal_bars).unwrap_or(0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(26.0, 16.0), egui::Sense::hover());
        let p = ui.painter();
        for i in 0..4u8 {
            let bh = 4.0 + i as f32 * 4.0;
            let r = egui::Rect::from_min_size(egui::pos2(rect.left() + i as f32 * 6.5, rect.bottom() - bh), egui::vec2(4.5, bh));
            let on = rssi.is_some() && i < bars;
            let c = if !on { TRACK } else if bars <= 1 { RED } else if bars == 2 { AMBER } else { GREEN };
            p.rect_filled(r, 1.0, c);
        }
        let txt = rssi.map(|r| format!("{r} dBm")).unwrap_or_else(|| "–".into());
        ui.label(RichText::new(txt).color(TEXT).size(14.0));
        label(ui, "Ping");
        let ping = ping_ms.map(|v| format!("{v} ms")).unwrap_or_else(|| "–".into());
        ui.label(RichText::new(ping).color(TEXT).size(14.0));
    });
    if let Some(info) = s.and_then(|s| s.signal_info.as_deref()) {
        ui.label(RichText::new(info).color(DIM).size(12.0));
    }
    ui.label(RichText::new(format!("Video: {video}")).color(DIM).size(12.0));

    // --- Nödstopp -------------------------------------------------------------
    if let Some(text) = s.and_then(|s| estop_text(s.estop.as_deref())) {
        egui::Frame::none()
            .fill(Color32::from_rgb(200, 0, 0))
            .rounding(5.0)
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.label(RichText::new(text).color(Color32::WHITE).size(20.0).strong());
            });
    }

    // --- Fel -------------------------------------------------------------------
    if let Some(err) = s.and_then(|s| s.fault_text.clone().or_else(|| s.last_error.clone())) {
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(150, 20, 20, 220))
            .rounding(5.0)
            .inner_margin(egui::Margin::symmetric(10.0, 6.0))
            .show(ui, |ui| {
                ui.label(RichText::new(format!("⚠ {err}")).color(Color32::WHITE).size(15.0).strong());
            });
    }
}

/// Text att visa för nödstoppets läge (från robotd), eller `None` när allt är OK
/// eller nödstopp inte finns på roboten.
pub fn estop_text(state: Option<&str>) -> Option<&'static str> {
    match state? {
        "ok" => None,
        "intryckt" => Some("⛔ NÖDSTOPP INTRYCKT"),
        "sparrad" => Some("Nödstopp utdraget – släpper spärren…"),
        _ => Some("⛔ NÖDSTOPP: knappen går inte att läsa (kabel?)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signalstaplar() {
        assert_eq!(signal_bars(-60), 4);
        assert_eq!(signal_bars(-71), 3);
        assert_eq!(signal_bars(-80), 2);
        assert_eq!(signal_bars(-90), 1);
        assert_eq!(signal_bars(-110), 0);
    }

    #[test]
    fn styrning_i_text() {
        assert_eq!(steering_label(0.0), "rakt");
        assert_eq!(steering_label(-40.0), "V 40 %");
        assert_eq!(steering_label(25.4), "H 25 %");
    }

    #[test]
    fn batterifarger() {
        assert_eq!(battery_color(10.0), RED);
        assert_eq!(battery_color(30.0), AMBER);
        assert_eq!(battery_color(80.0), GREEN);
    }
}
