//! Mjuka ramper på väg mot ett målvärde (t.ex. VESC-varvtal), så att den
//! öppna hydrauliska drivningen inte får ryckiga steg vid snabba
//! spak-rörelser eller vid nödstopp/nedbromsning. Ren, testbar matematik,
//! ingen I/O.

pub struct RateLimiter {
    current: f32,
    /// Max förändring per sekund.
    max_rate_per_sec: f32,
}

impl RateLimiter {
    pub fn new(max_rate_per_sec: f32) -> Self {
        Self {
            current: 0.0,
            max_rate_per_sec,
        }
    }

    /// Kliv mot `target`, begränsat av `max_rate_per_sec`, givet att `dt_secs`
    /// har gått sedan förra anropet.
    pub fn step(&mut self, target: f32, dt_secs: f32) -> f32 {
        let max_delta = self.max_rate_per_sec * dt_secs;
        let delta = (target - self.current).clamp(-max_delta, max_delta);
        self.current += delta;
        self.current
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn reset(&mut self, value: f32) {
        self.current = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begransar_forandring_per_sekund() {
        let mut r = RateLimiter::new(1.0); // max 1.0/s
        let v = r.step(1.0, 0.5); // 0.5s -> max delta 0.5
        assert!((v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn naar_malet_till_slut() {
        let mut r = RateLimiter::new(2.0);
        let mut v = 0.0;
        for _ in 0..10 {
            v = r.step(1.0, 0.1);
        }
        assert!((v - 1.0).abs() < 1e-3);
    }
}
