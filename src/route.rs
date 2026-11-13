//! Ruta de CupraFlow, independiente del motor (contrato en
//! `sb-wiki/Tecnica/CupraFlow v1 - contrato.md`). `caddy.rs` la traduce.

use serde::Deserialize;
use serde_json::Value;

/// Hosts como mucho: el principal y los 10 extra que admite la API.
const MAX_HOSTS: usize = 11;

#[derive(Debug, Deserialize)]
pub struct Route {
    /// El `app.id`; Caddy lo ve como `@id = "sbapp-<id>"`.
    pub id: String,
    /// El primero es el principal.
    pub hosts: Vec<String>,
    /// Hosts que responden 308 al principal.
    #[serde(default)]
    pub redirect_hosts: Vec<String>,
    pub upstreams: Vec<Upstream>,
    /// Fase 2. En v1 se acepta y se ignora.
    #[serde(default)]
    pub health: Value,
    /// Fase 2. En v1 se acepta y se ignora.
    #[serde(default)]
    pub policy: Value,
}

#[derive(Debug, Deserialize)]
pub struct Upstream {
    pub dial: String,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn valid_host(h: &str) -> bool {
    !h.is_empty()
        && h.len() <= 253
        && h.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
}

impl Route {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err(format!("invalid route id: {:?}", self.id));
        }
        if self.hosts.is_empty() || self.hosts.len() > MAX_HOSTS {
            return Err(format!("a route needs 1 to {MAX_HOSTS} hosts"));
        }
        for (i, h) in self.hosts.iter().enumerate() {
            if !valid_host(h) {
                return Err(format!("invalid host: {h:?}"));
            }
            // Caddy compara los hosts sin distinguir mayúsculas.
            if self.hosts[..i].iter().any(|o| o.eq_ignore_ascii_case(h)) {
                return Err(format!("repeated host: {h}"));
            }
        }
        for h in &self.redirect_hosts {
            if !self.hosts[1..].iter().any(|o| o.eq_ignore_ascii_case(h)) {
                return Err(format!(
                    "redirect host {h:?} must be one of the extra hosts"
                ));
            }
        }
        match self.upstreams.as_slice() {
            [u] if u.dial.starts_with("127.0.0.1:")
                && u.dial["127.0.0.1:".len()..].parse::<u16>().is_ok() => {}
            [_] => return Err("the upstream must be 127.0.0.1:<port>".into()),
            [] => return Err("a route needs an upstream".into()),
            _ => return Err("several upstreams need CupraFlow >= 0.3".into()),
        }
        if !self.health.is_null() || !self.policy.is_null() {
            tracing::warn!(id = %self.id, "route health/policy ignored until CupraFlow 0.3");
        }
        Ok(())
    }

    pub fn primary(&self) -> &str {
        &self.hosts[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn route(v: Value) -> Route {
        serde_json::from_value(v).unwrap()
    }

    fn ok() -> Value {
        json!({
            "id": "11111111-2222-3333-4444-555555555555",
            "hosts": ["ejemplo.com", "www.ejemplo.com"],
            "redirect_hosts": ["www.ejemplo.com"],
            "upstreams": [{ "dial": "127.0.0.1:20001" }],
            "health": { "path": "/" },
        })
    }

    #[test]
    fn accepts_v1_route_and_ignores_phase2_fields() {
        assert_eq!(route(ok()).validate(), Ok(()));
    }

    #[test]
    fn rejects_bad_routes() {
        let cases = [
            ("id", json!("sbapp/../x")),
            ("hosts", json!([])),
            ("hosts", json!(["a.com", "a.com"])),
            ("hosts", json!(["ejemplo.com", "EJEMPLO.com"])),
            ("hosts", json!(["a.com", "bad host"])),
            ("redirect_hosts", json!(["ejemplo.com"])),
            ("redirect_hosts", json!(["otro.com"])),
            ("upstreams", json!([])),
            ("upstreams", json!([{ "dial": "10.0.0.2:80" }])),
            ("upstreams", json!([{ "dial": "127.0.0.1:99999" }])),
            (
                "upstreams",
                json!([{ "dial": "127.0.0.1:1" }, { "dial": "127.0.0.1:2" }]),
            ),
        ];
        for (field, value) in cases {
            let mut v = ok();
            v[field] = value.clone();
            assert!(
                route(v).validate().is_err(),
                "{field} = {value} should fail"
            );
        }
    }
}
