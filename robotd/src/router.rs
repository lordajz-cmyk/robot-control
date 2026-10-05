//! Mobilnätets signal från robotens Teltonika-router (RUT951 m.fl., RutOS 7).
//!
//! Routerns REST-API nås över HTTPS på robotens lokala nät (självsignerat cert):
//! `POST /api/login` ger en nyckel, `GET /api/modems/status` ger signalvärdena.
//! Vi använder `curl` (finns på Pi:n) i stället för ett TLS-bibliotek i robotd.
//! Inloggningen görs om när nyckeln gått ut. Utan lösenord i configen görs inget.

use std::time::Duration;

use serde_json::Value;
use tokio::process::Command;
use tokio::sync::watch;

const POLL: Duration = Duration::from_secs(10);
const CURL_TIMEOUT_S: &str = "6";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RouterSignal {
    pub rssi_dbm: Option<i32>,
    /// T.ex. "4G Telia · RSRP −95 dBm · SINR 12 dB".
    pub info: Option<String>,
}

/// Startar avläsningen om url och lösenord finns. Ger `None` annars.
pub fn spawn(url: String, user: String, password: String) -> Option<watch::Receiver<Option<RouterSignal>>> {
    if url.trim().is_empty() || password.is_empty() {
        return None;
    }
    let (tx, rx) = watch::channel(None);
    tokio::spawn(async move {
        let mut token: Option<String> = None;
        let mut failures = 0u32;
        loop {
            if token.is_none() {
                token = login(&url, &user, &password).await;
            }
            let signal = match &token {
                Some(t) => match modem_status(&url, t).await {
                    Ok(v) => parse_modem_status(&v),
                    Err(_) => {
                        token = None; // utgången nyckel eller nätfel: logga in igen nästa gång
                        None
                    }
                },
                None => None,
            };
            if signal.is_none() {
                failures += 1;
                if failures == 1 || failures % 30 == 0 {
                    tracing::warn!("Routern ({url}) gav ingen signalstyrka (försök {failures}).");
                }
            } else {
                failures = 0;
            }
            if tx.send(signal).is_err() {
                return;
            }
            tokio::time::sleep(POLL).await;
        }
    });
    Some(rx)
}

async fn curl(args: &[&str]) -> Result<Value, String> {
    let out = Command::new("curl")
        .args(["-sk", "-m", CURL_TIMEOUT_S])
        .args(args)
        .output()
        .await
        .map_err(|e| format!("curl: {e}"))?;
    if !out.status.success() {
        return Err(format!("curl avslutades med {}", out.status));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("inte JSON: {e}"))
}

async fn login(url: &str, user: &str, password: &str) -> Option<String> {
    let body = serde_json::json!({ "username": user, "password": password }).to_string();
    let v = curl(&["-X", "POST", "-H", "Content-Type: application/json", "-d", &body, &format!("{url}/api/login")])
        .await
        .ok()?;
    v.pointer("/data/token").and_then(Value::as_str).map(str::to_string)
}

async fn modem_status(url: &str, token: &str) -> Result<Value, String> {
    let v = curl(&["-H", &format!("Authorization: Bearer {token}"), &format!("{url}/api/modems/status")]).await?;
    if v.get("success").and_then(Value::as_bool) == Some(false) {
        return Err("nekad".into());
    }
    Ok(v)
}

/// Tal ur ett fält som kan vara tal eller text ("-71", "-71 dBm").
fn num(o: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|k| match o.get(*k)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.split_whitespace().next()?.parse().ok(),
        _ => None,
    })
}

fn text(o: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| o.get(*k)?.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string))
}

/// Första modemet i svaret från `/api/modems/status`.
pub fn parse_modem_status(v: &Value) -> Option<RouterSignal> {
    let data = v.get("data")?;
    let m = match data {
        Value::Array(a) => a.first()?,
        Value::Object(_) => data,
        _ => return None,
    };
    let rssi = num(m, &["rssi", "signal"]).map(|x| x.round() as i32);
    let rsrp = num(m, &["rsrp"]);
    let sinr = num(m, &["sinr"]);
    let net = text(m, &["conntype", "ntype", "network_type", "net_mode", "service"]);
    let oper = text(m, &["operator", "oper", "provider"]);
    let mut parts: Vec<String> = Vec::new();
    match (net, oper) {
        (Some(n), Some(o)) => parts.push(format!("{n} {o}")),
        (Some(n), None) => parts.push(n),
        (None, Some(o)) => parts.push(o),
        (None, None) => {}
    }
    if let Some(r) = rsrp {
        parts.push(format!("RSRP {r:.0} dBm"));
    }
    if let Some(s) = sinr {
        parts.push(format!("SINR {s:.0} dB"));
    }
    if rssi.is_none() && parts.is_empty() {
        return None;
    }
    Some(RouterSignal { rssi_dbm: rssi, info: (!parts.is_empty()).then(|| parts.join(" · ")) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modemstatus_tolkas() {
        let v: Value = serde_json::from_str(
            r#"{"success":true,"data":[{"id":"1-1","rssi":-71,"rsrp":-95,"rsrq":-10,"sinr":12,
                "operator":"Telia","conntype":"4G (LTE)","state":"connected"}]}"#,
        )
        .unwrap();
        let s = parse_modem_status(&v).unwrap();
        assert_eq!(s.rssi_dbm, Some(-71));
        assert_eq!(s.info.as_deref(), Some("4G (LTE) Telia · RSRP -95 dBm · SINR 12 dB"));
    }

    #[test]
    fn text_och_saknade_falt() {
        let v: Value = serde_json::from_str(r#"{"data":{"rssi":"-80 dBm","ntype":"LTE"}}"#).unwrap();
        let s = parse_modem_status(&v).unwrap();
        assert_eq!(s.rssi_dbm, Some(-80));
        assert_eq!(s.info.as_deref(), Some("LTE"));
        let v: Value = serde_json::from_str(r#"{"success":true,"data":[]}"#).unwrap();
        assert_eq!(parse_modem_status(&v), None);
    }

    #[test]
    fn startar_inte_utan_losenord() {
        assert!(spawn("https://192.168.60.1".into(), "admin".into(), String::new()).is_none());
        assert!(spawn(String::new(), "admin".into(), "x".into()).is_none());
    }
}
