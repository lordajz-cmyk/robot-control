//! Ren logik för parkoppling, "en förare i taget" och temporära titta-koder.
//! Medvetet fri från nätverkskod (WebSocket etc.) så den går att enhetstesta
//! direkt, utan att starta en server. All tidskänslig logik tar in tiden som
//! parameter i stället för att läsa systemklockan, för deterministiska tester.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use relay_protocol::{PairingCode, RobotId};

#[derive(Debug, Clone)]
struct PairedClient {
    public_key: [u8; 32],
}

#[derive(Debug, Clone)]
struct PendingCode {
    code: PairingCode,
    expires_at: Instant,
    view_only: bool,
}

#[derive(Debug, Default)]
struct RobotEntry {
    /// Klientdatorer som är permanent parkopplade mot roboten.
    paired_clients: HashMap<String, PairedClient>,
    /// Utestående, ej ännu inlösta parkopplings-/titta-koder.
    pending_codes: Vec<PendingCode>,
    /// Vem kör just nu (om någon). Bara en åt gången.
    current_driver: Option<String>,
    /// Vilka som tittar (läs-only) just nu.
    viewers: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Registry {
    robots: HashMap<RobotId, RobotEntry>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectOutcome {
    /// Blev förare.
    Driver,
    /// Blev åskådare (redan upptagen av en förare, eller kopplade in med titta-kod).
    Viewer,
    /// Nekad helt (utgången/okänd kod, eller okänd robot).
    Denied(String),
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    fn entry(&mut self, robot: &RobotId) -> &mut RobotEntry {
        self.robots.entry(robot.clone()).or_default()
    }

    /// `robotctl pair` (eller display-koden) anropar detta på robotd, som
    /// skickar det vidare till servern.
    pub fn register_pairing_code(&mut self, robot: &RobotId, code: PairingCode, ttl: Duration, now: Instant) {
        self.entry(robot).pending_codes.push(PendingCode {
            code,
            expires_at: now + ttl,
            view_only: false,
        });
    }

    pub fn register_view_code(&mut self, robot: &RobotId, code: PairingCode, ttl: Duration, now: Instant) {
        self.entry(robot).pending_codes.push(PendingCode {
            code,
            expires_at: now + ttl,
            view_only: true,
        });
    }

    fn prune_expired(&mut self, robot: &RobotId, now: Instant) {
        if let Some(e) = self.robots.get_mut(robot) {
            e.pending_codes.retain(|c| c.expires_at > now);
        }
    }

    /// En klient med `public_key` försöker ansluta till `robot`, ev. med en engångskod
    /// (första gången) eller utan (om redan permanent parkopplad).
    pub fn connect(
        &mut self,
        robot: &RobotId,
        client_id: &str,
        public_key: [u8; 32],
        code: Option<&str>,
        now: Instant,
    ) -> ConnectOutcome {
        self.prune_expired(robot, now);

        let already_paired = self
            .robots
            .get(robot)
            .map(|e| e.paired_clients.contains_key(client_id))
            .unwrap_or(false);

        if !already_paired {
            let Some(code) = code else {
                return ConnectOutcome::Denied("okänd klient, ingen kod angiven".into());
            };
            let entry = self.entry(robot);
            let matched = entry
                .pending_codes
                .iter()
                .position(|c| c.code == code && c.expires_at > now);
            let Some(idx) = matched else {
                return ConnectOutcome::Denied("ogiltig eller utgången kod".into());
            };
            let view_only = entry.pending_codes[idx].view_only;
            entry.pending_codes.remove(idx);

            if view_only {
                entry.viewers.push(client_id.to_string());
                return ConnectOutcome::Viewer;
            }
            entry
                .paired_clients
                .insert(client_id.to_string(), PairedClient { public_key });
        }

        let entry = self.entry(robot);
        match &entry.current_driver {
            Some(driver) if driver != client_id => {
                // Någon annan kör redan -> neka helt, inget övertagande.
                ConnectOutcome::Denied("roboten körs redan av någon annan".into())
            }
            _ => {
                entry.current_driver = Some(client_id.to_string());
                ConnectOutcome::Driver
            }
        }
    }

    /// Föraren kopplar från / stänger appen -> roboten blir ledig igen.
    pub fn release_driver(&mut self, robot: &RobotId, client_id: &str) {
        if let Some(e) = self.robots.get_mut(robot) {
            if e.current_driver.as_deref() == Some(client_id) {
                e.current_driver = None;
            }
        }
    }

    pub fn remove_viewer(&mut self, robot: &RobotId, client_id: &str) {
        if let Some(e) = self.robots.get_mut(robot) {
            e.viewers.retain(|v| v != client_id);
        }
    }

    pub fn current_driver(&self, robot: &RobotId) -> Option<&str> {
        self.robots.get(robot).and_then(|e| e.current_driver.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pk(n: u8) -> [u8; 32] {
        [n; 32]
    }

    #[test]
    fn ny_klient_utan_kod_nekas() {
        let mut r = Registry::new();
        let now = Instant::now();
        let outcome = r.connect(&"robot-1".into(), "client-a", pk(1), None, now);
        assert_eq!(outcome, ConnectOutcome::Denied("okänd klient, ingen kod angiven".into()));
    }

    #[test]
    fn parkoppling_med_giltig_kod_blir_forare() {
        let mut r = Registry::new();
        let now = Instant::now();
        r.register_pairing_code(&"robot-1".into(), "ABC123".into(), Duration::from_secs(300), now);
        let outcome = r.connect(&"robot-1".into(), "client-a", pk(1), Some("ABC123"), now);
        assert_eq!(outcome, ConnectOutcome::Driver);
    }

    #[test]
    fn utgangen_kod_nekas() {
        let mut r = Registry::new();
        let now = Instant::now();
        r.register_pairing_code(&"robot-1".into(), "ABC123".into(), Duration::from_secs(1), now);
        let later = now + Duration::from_secs(2);
        let outcome = r.connect(&"robot-1".into(), "client-a", pk(1), Some("ABC123"), later);
        assert_eq!(outcome, ConnectOutcome::Denied("ogiltig eller utgången kod".into()));
    }

    #[test]
    fn andra_forare_nekas_ingen_overtagande() {
        let mut r = Registry::new();
        let now = Instant::now();
        r.register_pairing_code(&"robot-1".into(), "A".into(), Duration::from_secs(300), now);
        r.register_pairing_code(&"robot-1".into(), "B".into(), Duration::from_secs(300), now);
        assert_eq!(
            r.connect(&"robot-1".into(), "client-a", pk(1), Some("A"), now),
            ConnectOutcome::Driver
        );
        assert_eq!(
            r.connect(&"robot-1".into(), "client-b", pk(2), Some("B"), now),
            ConnectOutcome::Denied("roboten körs redan av någon annan".into())
        );
    }

    #[test]
    fn roboten_blir_ledig_igen_efter_release() {
        let mut r = Registry::new();
        let now = Instant::now();
        r.register_pairing_code(&"robot-1".into(), "A".into(), Duration::from_secs(300), now);
        r.connect(&"robot-1".into(), "client-a", pk(1), Some("A"), now);
        r.release_driver(&"robot-1".into(), "client-a");
        r.register_pairing_code(&"robot-1".into(), "B".into(), Duration::from_secs(300), now);
        let outcome = r.connect(&"robot-1".into(), "client-b", pk(2), Some("B"), now);
        assert_eq!(outcome, ConnectOutcome::Driver);
    }

    #[test]
    fn titta_kod_ger_bara_askadare() {
        let mut r = Registry::new();
        let now = Instant::now();
        r.register_view_code(&"robot-1".into(), "VIEW1".into(), Duration::from_secs(300), now);
        let outcome = r.connect(&"robot-1".into(), "client-c", pk(3), Some("VIEW1"), now);
        assert_eq!(outcome, ConnectOutcome::Viewer);
    }

    #[test]
    fn redan_parkopplad_klient_kommer_in_utan_kod() {
        let mut r = Registry::new();
        let now = Instant::now();
        r.register_pairing_code(&"robot-1".into(), "A".into(), Duration::from_secs(300), now);
        r.connect(&"robot-1".into(), "client-a", pk(1), Some("A"), now);
        r.release_driver(&"robot-1".into(), "client-a");
        let outcome = r.connect(&"robot-1".into(), "client-a", pk(1), None, now);
        assert_eq!(outcome, ConnectOutcome::Driver);
    }
}
