//! Fordonsdata som räknas fram i robotd (2026-10-05): batteriprocent, om batteriet
//! laddas, effekt, förbrukning (Wh/km) och räckvidd.
//!
//! - **Procent:** ur vilospänningen (LiFePO4-kurva eller linjärt, `VehicleConfig`).
//!   Spänningen sjunker under last, så procenten bestäms när roboten står still och
//!   räknas sedan ner med förbrukad energi under körningen (kräver kapaciteten).
//! - **Laddning:** roboten har stått still i minst `REST_BEFORE` och spänningen har
//!   stigit mer än `CHARGE_SLOPE_V_PER_MIN` under den senaste minuten. Direkt efter
//!   körning stiger spänningen lite av sig själv (batteriet "hämtar sig"), därför
//!   väntar vi först. Medan roboten kör går det inte att avgöra (`None`).
//! - **Effekt:** VESC:ernas motorström × duty × batterispänning ≈ effekten ur batteriet.
//! - **Förbrukning och räckvidd:** energi och sträcka summeras under körningen;
//!   räckvidd = kvarvarande energi / förbrukning. Kräver batteriets kapacitet.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::config::VehicleConfig;
use crate::vesc_can::VescStatusEntry;

/// Hur länge roboten ska ha stått still innan laddning bedöms.
const REST_BEFORE: Duration = Duration::from_secs(90);
/// Mätfönster för laddning.
const CHARGE_WINDOW: Duration = Duration::from_secs(60);
/// Stigning som räknas som laddning (V per minut).
const CHARGE_SLOPE_V_PER_MIN: f32 = 0.05;
/// Under så här låg fart och effekt räknas roboten som stillastående.
const STILL_SPEED_MS: f32 = 0.1;
const STILL_POWER_W: f32 = 30.0;
/// Förbrukning visas först efter så här lång sträcka (mindre ger orimliga värden).
const MIN_DISTANCE_FOR_RATE_M: f64 = 200.0;
/// Längre glapp mellan mätningar än så räknas inte in (länken var nere).
const MAX_DT: Duration = Duration::from_secs(5);
/// Så länge ska roboten stå still innan vilospänningen ger ny procent.
const REST_FOR_PERCENT: Duration = Duration::from_secs(60);

/// LiFePO4-cellens vilospänning (V) mot laddning (%). Kurvan är mycket flat mellan
/// 20 och 90 %, så procenten där är ungefärlig.
const LIFEPO4_CELL: [(f32, f32); 11] = [
    (2.50, 0.0),
    (3.00, 10.0),
    (3.20, 20.0),
    (3.22, 30.0),
    (3.25, 40.0),
    (3.26, 50.0),
    (3.27, 60.0),
    (3.30, 70.0),
    (3.32, 80.0),
    (3.35, 90.0),
    (3.40, 100.0),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BatteryModel {
    LiFePO4 { cells: u32 },
    Linear { empty_v: f32, full_v: f32 },
}

impl BatteryModel {
    pub fn from_config(c: &VehicleConfig) -> Self {
        if c.battery_type.eq_ignore_ascii_case("lifepo4") && c.battery_series_cells > 0 {
            BatteryModel::LiFePO4 { cells: c.battery_series_cells }
        } else {
            BatteryModel::Linear { empty_v: c.battery_empty_v, full_v: c.battery_full_v }
        }
    }

    pub fn percent(&self, v_in: f32) -> f32 {
        match *self {
            BatteryModel::Linear { empty_v, full_v } => battery_percent(v_in, empty_v, full_v),
            BatteryModel::LiFePO4 { cells } => {
                let v = v_in / cells as f32;
                let t = &LIFEPO4_CELL;
                if v <= t[0].0 {
                    return 0.0;
                }
                for w in t.windows(2) {
                    let ((v0, p0), (v1, p1)) = (w[0], w[1]);
                    if v <= v1 {
                        return p0 + (p1 - p0) * (v - v0) / (v1 - v0);
                    }
                }
                100.0
            }
        }
    }
}

pub fn battery_percent(v_in: f32, empty_v: f32, full_v: f32) -> f32 {
    if full_v <= empty_v {
        return 0.0;
    }
    ((v_in - empty_v) / (full_v - empty_v) * 100.0).clamp(0.0, 100.0)
}

/// Effekten ur batteriet (W) från VESC:ernas status: |motorström × duty| × spänning.
pub fn power_from_vescs(vescs: &[VescStatusEntry], v_in: f32) -> f32 {
    vescs
        .iter()
        .filter(|e| e.is_fresh())
        .map(|e| (e.current_a * e.duty).abs() * v_in)
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleFigures {
    pub percent: f32,
    pub charging: Option<bool>,
    pub power_w: f32,
    pub wh_per_km: Option<f32>,
    pub range_km: Option<f32>,
}

#[derive(Debug)]
pub struct VehicleTracker {
    model: BatteryModel,
    capacity_wh: Option<f32>,
    /// Procent vid senaste vila och energin (Wh) som var förbrukad då.
    rest_percent: Option<(f32, f64)>,
    /// (tid, spänning) för laddningsbedömningen.
    volts: VecDeque<(Instant, f32)>,
    still_since: Option<Instant>,
    last: Option<Instant>,
    energy_wh: f64,
    distance_m: f64,
}

impl VehicleTracker {
    pub fn new(model: BatteryModel, capacity_wh: Option<f32>) -> Self {
        Self {
            model,
            capacity_wh,
            rest_percent: None,
            volts: VecDeque::new(),
            still_since: None,
            last: None,
            energy_wh: 0.0,
            distance_m: 0.0,
        }
    }

    /// Ny mätning. `speed_ms` är farten (tecken spelar ingen roll), `power_w` effekten.
    pub fn update(&mut self, now: Instant, v_in: f32, speed_ms: f32, power_w: f32) -> VehicleFigures {
        if let Some(prev) = self.last {
            let dt = now.saturating_duration_since(prev);
            if dt <= MAX_DT {
                let s = dt.as_secs_f64();
                self.energy_wh += power_w.max(0.0) as f64 * s / 3600.0;
                self.distance_m += speed_ms.abs() as f64 * s;
            }
        }
        self.last = Some(now);

        let still = speed_ms.abs() < STILL_SPEED_MS && power_w < STILL_POWER_W;
        if still {
            self.still_since.get_or_insert(now);
        } else {
            self.still_since = None;
            self.volts.clear();
        }
        self.volts.push_back((now, v_in));
        while self.volts.front().is_some_and(|(t, _)| now.saturating_duration_since(*t) > CHARGE_WINDOW) {
            self.volts.pop_front();
        }

        let volt_percent = self.model.percent(v_in);
        if still {
            let rested = self.still_since.is_some_and(|s| now.saturating_duration_since(s) >= REST_FOR_PERCENT);
            if rested || self.rest_percent.is_none() {
                self.rest_percent = Some((volt_percent, self.energy_wh));
            }
        }
        let percent = match (self.rest_percent, self.capacity_wh) {
            (Some((p, e0)), Some(cap)) if cap > 0.0 => {
                (p - ((self.energy_wh - e0) / cap as f64 * 100.0) as f32).clamp(0.0, 100.0)
            }
            _ => volt_percent,
        };
        let charging = self.charging(now);
        let wh_per_km = (self.distance_m >= MIN_DISTANCE_FOR_RATE_M)
            .then(|| (self.energy_wh / (self.distance_m / 1000.0)) as f32)
            .filter(|w| *w > 0.0);
        let range_km = match (self.capacity_wh, wh_per_km) {
            (Some(cap), Some(rate)) => Some(percent / 100.0 * cap / rate),
            _ => None,
        };
        VehicleFigures { percent, charging, power_w, wh_per_km, range_km }
    }

    fn charging(&self, now: Instant) -> Option<bool> {
        let since = self.still_since?;
        if now.saturating_duration_since(since) < REST_BEFORE {
            return None;
        }
        // Bara mätningar efter vilan, och ett fönster på minst halva tiden.
        let pts: Vec<(f32, f32)> = self
            .volts
            .iter()
            .filter(|(t, _)| *t >= since + REST_BEFORE - CHARGE_WINDOW)
            .map(|(t, v)| (now.saturating_duration_since(*t).as_secs_f32(), *v))
            .collect();
        if pts.len() < 5 {
            return None;
        }
        let span = pts.iter().map(|p| p.0).fold(0.0, f32::max);
        if span < CHARGE_WINDOW.as_secs_f32() / 2.0 {
            return None;
        }
        // Minsta kvadrat: lutning i V per sekund (x = sekunder SEDAN, därför minus).
        let n = pts.len() as f32;
        let mx = pts.iter().map(|p| p.0).sum::<f32>() / n;
        let my = pts.iter().map(|p| p.1).sum::<f32>() / n;
        let sxy: f32 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        let sxx: f32 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
        if sxx <= 0.0 {
            return None;
        }
        let v_per_min = -(sxy / sxx) * 60.0;
        Some(v_per_min > CHARGE_SLOPE_V_PER_MIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIN: BatteryModel = BatteryModel::Linear { empty_v: 42.0, full_v: 54.4 };

    #[test]
    fn lifepo4_kurva() {
        let m = BatteryModel::LiFePO4 { cells: 16 };
        assert_eq!(m.percent(38.0), 0.0);
        assert_eq!(m.percent(56.0), 100.0);
        assert!((m.percent(16.0 * 3.26) - 50.0).abs() < 0.1);
        assert!((m.percent(16.0 * 3.21) - 25.0).abs() < 0.1);
        let c = VehicleConfig::default();
        assert_eq!(BatteryModel::from_config(&c), m);
    }

    #[test]
    fn procent_raknas_ner_med_energi_under_korning() {
        let t0 = Instant::now();
        let mut tr = VehicleTracker::new(BatteryModel::LiFePO4 { cells: 16 }, Some(4096.0));
        // Står still på 52,16 V (50 %), kör sedan 1 h med 409,6 W => 10 % av 4096 Wh.
        let f = tr.update(t0, 16.0 * 3.26, 0.0, 0.0);
        assert!((f.percent - 50.0).abs() < 0.1);
        let mut f = f;
        for s in 1..=3600 {
            // Spänningen sjunker under last, men det ska inte synas i procenten.
            f = tr.update(t0 + Duration::from_secs(s), 16.0 * 3.15, 1.0, 409.6);
        }
        assert!((f.percent - 40.0).abs() < 0.2, "{}", f.percent);
    }

    #[test]
    fn procent_linjart_och_klampat() {
        assert_eq!(battery_percent(42.0, 42.0, 54.4), 0.0);
        assert!((battery_percent(48.2, 42.0, 54.4) - 50.0).abs() < 0.1);
        assert_eq!(battery_percent(60.0, 42.0, 54.4), 100.0);
        assert_eq!(battery_percent(50.0, 50.0, 50.0), 0.0);
    }

    #[test]
    fn effekt_fran_vesc() {
        let e = |cur, duty, age| VescStatusEntry { can_id: 1, age_ms: age, rpm: 0, current_a: cur, duty };
        let p = power_from_vescs(&[e(10.0, 0.5, 100), e(-4.0, 0.25, 100), e(99.0, 1.0, 9_000)], 48.0);
        assert!((p - (5.0 + 1.0) * 48.0).abs() < 0.01); // gammal VESC räknas inte
    }

    #[test]
    fn laddning_kannas_igen_efter_vila() {
        let t0 = Instant::now();
        let mut tr = VehicleTracker::new(LIN, None);
        let mut last = None;
        // 3 minuter stilla, spänningen stiger 0,2 V/min (generatorn laddar).
        for s in 0..=180 {
            let v = 48.0 + 0.2 * s as f32 / 60.0;
            last = Some(tr.update(t0 + Duration::from_secs(s), v, 0.0, 5.0));
            if s < 90 {
                assert_eq!(last.unwrap().charging, None, "för tidigt vid {s} s");
            }
        }
        assert_eq!(last.unwrap().charging, Some(true));
    }

    #[test]
    fn ingen_laddning_nar_spanningen_ar_still() {
        let t0 = Instant::now();
        let mut tr = VehicleTracker::new(LIN, None);
        let mut f = None;
        for s in 0..=200 {
            f = Some(tr.update(t0 + Duration::from_secs(s), 48.0 - 0.01 * s as f32 / 60.0, 0.0, 5.0));
        }
        assert_eq!(f.unwrap().charging, Some(false));
    }

    #[test]
    fn under_korning_gar_laddning_inte_att_avgora() {
        let t0 = Instant::now();
        let mut tr = VehicleTracker::new(LIN, None);
        let mut f = None;
        for s in 0..=200 {
            f = Some(tr.update(t0 + Duration::from_secs(s), 48.0, 1.0, 300.0));
        }
        assert_eq!(f.unwrap().charging, None);
    }

    #[test]
    fn forbrukning_och_rackvidd() {
        let t0 = Instant::now();
        let mut tr = VehicleTracker::new(LIN, Some(2400.0));
        let mut f = None;
        // 1 m/s och 360 W i 400 s: 400 m och 40 Wh => 100 Wh/km.
        for s in 0..=400 {
            f = Some(tr.update(t0 + Duration::from_secs(s), 48.2, 1.0, 360.0));
        }
        let f = f.unwrap();
        assert!((f.wh_per_km.unwrap() - 100.0).abs() < 1.0, "{:?}", f.wh_per_km);
        // 50 % av 2400 Wh = 1200 Wh / 100 Wh/km = 12 km
        assert!((f.range_km.unwrap() - 12.0).abs() < 0.3, "{:?}", f.range_km);
    }

    #[test]
    fn ingen_rackvidd_utan_kapacitet_eller_for_kort_stracka() {
        let t0 = Instant::now();
        let mut tr = VehicleTracker::new(LIN, None);
        let mut f = None;
        for s in 0..=400 {
            f = Some(tr.update(t0 + Duration::from_secs(s), 48.2, 1.0, 360.0));
        }
        assert_eq!(f.unwrap().range_km, None);
        let mut tr = VehicleTracker::new(LIN, Some(2400.0));
        let f = tr.update(t0, 48.2, 1.0, 360.0);
        assert_eq!(f.wh_per_km, None);
    }
}
