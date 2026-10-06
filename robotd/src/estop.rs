//! Nödstoppet: läget som Car_Client skriver till `/tmp/nodstopp_status`.
//!
//! Själva knappen (NC-brytare mellan GPIO27 och GND) läses av Car_Client, eftersom
//! alla körkommandon — från både robotd och RControlStation — går den vägen och
//! spärras där. robotd läser bara läget för att visa det i Robotstyrning och för att
//! kräva ny AKTIVERA efter ett nödstopp.
//!
//! Filen innehåller en rad: `<läge> <unix-tid>`, där läget är `ok`, `intryckt`,
//! `sparrad` (knappen utdragen, väntar på ett stoppkommando) eller `fel`.
//! Saknas filen är nödstoppet inte installerat på roboten (`None`).

use std::time::{SystemTime, UNIX_EPOCH};

pub const STATUS_FILE: &str = "/tmp/nodstopp_status";

/// Car_Client skriver filen varje sekund. Äldre än så här = Car_Client hänger/är död.
const MAX_AGE_S: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estop {
    Ok,
    Pressed,
    Latched,
    Fault,
}

impl Estop {
    /// Texten som skickas till klienten i `StatusUpdate::estop`.
    pub fn as_str(self) -> &'static str {
        match self {
            Estop::Ok => "ok",
            Estop::Pressed => "intryckt",
            Estop::Latched => "sparrad",
            Estop::Fault => "fel",
        }
    }
}

/// Läser läget. `None` = inget nödstopp installerat (filen saknas).
pub fn read() -> Option<Estop> {
    let text = std::fs::read_to_string(STATUS_FILE).ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    Some(parse(&text, now))
}

fn parse(text: &str, now: u64) -> Estop {
    let mut parts = text.split_whitespace();
    let state = parts.next().unwrap_or("");
    let written: Option<u64> = parts.next().and_then(|t| t.parse().ok());
    if written.is_none_or(|w| now.saturating_sub(w) > MAX_AGE_S) {
        return Estop::Fault;
    }
    match state {
        "ok" => Estop::Ok,
        "intryckt" => Estop::Pressed,
        "sparrad" => Estop::Latched,
        _ => Estop::Fault,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lagen() {
        assert_eq!(parse("ok 1000\n", 1001), Estop::Ok);
        assert_eq!(parse("intryckt 1000\n", 1000), Estop::Pressed);
        assert_eq!(parse("sparrad 1000", 1002), Estop::Latched);
        assert_eq!(parse("fel 1000", 1000), Estop::Fault);
    }

    #[test]
    fn gammal_eller_trasig_fil_ar_fel() {
        assert_eq!(parse("ok 1000", 1010), Estop::Fault);
        assert_eq!(parse("ok", 1000), Estop::Fault);
        assert_eq!(parse("", 1000), Estop::Fault);
        assert_eq!(parse("konstigt 1000", 1000), Estop::Fault);
    }
}
