//! Lokal säkerhets-watchdog på Pi:n, oberoende av nätverket. Detta är kärnan
//! i skyddet mot 5G-avbrott: om inget giltigt styrkommando kommit in på ett
//! tag, tappar roboten drivning stegvis snarare än att bara fortsätta köra
//! på senaste kommandot. Ren tillståndslogik, testbar utan I/O.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogState {
    /// Allt normalt, senaste kommando är färskt.
    Ok,
    /// >= soft_ms sedan senaste kommando: mjuk nedbromsning påbörjad.
    SoftBrake,
    /// >= hard_ms sedan senaste kommando: hård nollställning, kräver nytt
    /// giltigt kommando (och, i kliuenten, Aktivera igen) innan körning
    /// återupptas.
    HardStop,
}

pub struct Watchdog {
    soft_timeout: Duration,
    hard_timeout: Duration,
    last_command_at: Instant,
}

impl Watchdog {
    pub fn new(soft_ms: u64, hard_ms: u64) -> Self {
        Self {
            soft_timeout: Duration::from_millis(soft_ms),
            hard_timeout: Duration::from_millis(hard_ms),
            last_command_at: Instant::now(),
        }
    }

    pub fn command_received(&mut self, at: Instant) {
        self.last_command_at = at;
    }

    pub fn state(&self, now: Instant) -> WatchdogState {
        let elapsed = now.saturating_duration_since(self.last_command_at);
        if elapsed >= self.hard_timeout {
            WatchdogState::HardStop
        } else if elapsed >= self.soft_timeout {
            WatchdogState::SoftBrake
        } else {
            WatchdogState::Ok
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_direkt_efter_kommando() {
        let now = Instant::now();
        let mut w = Watchdog::new(150, 400);
        w.command_received(now);
        assert_eq!(w.state(now), WatchdogState::Ok);
    }

    #[test]
    fn mjuk_nedbromsning_efter_150ms() {
        let now = Instant::now();
        let mut w = Watchdog::new(150, 400);
        w.command_received(now);
        let later = now + Duration::from_millis(160);
        assert_eq!(w.state(later), WatchdogState::SoftBrake);
    }

    #[test]
    fn hard_stopp_efter_400ms() {
        let now = Instant::now();
        let mut w = Watchdog::new(150, 400);
        w.command_received(now);
        let later = now + Duration::from_millis(450);
        assert_eq!(w.state(later), WatchdogState::HardStop);
    }
}
