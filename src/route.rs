//! Ruta de CupraFlow, independiente del motor (contrato en
//! `sb-wiki/Tecnica/CupraFlow v2 - contrato.md`). `caddy.rs` la traduce.

use serde::Deserialize;
use serde_json::Value;
use std::net::{IpAddr, SocketAddr};

/// Hosts como mucho: el principal y los 10 extra que admite la API.
const MAX_HOSTS: usize = 11;
/// Compañeros como mucho por ruta.
const MAX_PEERS: usize = 10;

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
    /// Otros servidores de la app (IP o nombre, `host:puerto` opcional; 443
    /// por defecto). Vacío = ruta de un solo nodo, como en v1.
    #[serde(default)]
    pub peers: Vec<String>,
    /// Comprobación activa del upstream local. Solo se usa con `peers`.
    #[serde(default)]
    pub health: Option<Health>,
    /// Se acepta y se ignora: el reparto es siempre «primero el local».
    #[serde(default)]
    pub policy: Value,
}

#[derive(Debug, Deserialize)]
pub struct Health {
    pub path: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
}

fn default_interval() -> u64 {
    10
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

/// Un compañero: host para el `dial` (IPv6 entre corchetes) y puerto HTTPS.
#[derive(Debug, PartialEq)]
pub struct Peer {
    pub host: String,
    pub port: u16,
}

impl Peer {
    pub fn parse(s: &str) -> Result<Peer, String> {
        let bad = || format!("invalid peer: {s:?}");
        if let Ok(a) = s.parse::<SocketAddr>() {
            return Ok(Peer::from_ip(a.ip(), a.port()));
        }
        let unbracketed = s
            .strip_prefix('[')
            .and_then(|r| r.strip_suffix(']'))
            .unwrap_or(s);
        if let Ok(ip) = unbracketed.parse::<IpAddr>() {
            return Ok(Peer::from_ip(ip, 443));
        }
        let (host, port) = match s.split_once(':') {
            Some((h, p)) => (h, p.parse::<u16>().map_err(|_| bad())?),
            None => (s, 443),
        };
        if !valid_host(host) || port == 0 {
            return Err(bad());
        }
        Ok(Peer {
            host: host.to_ascii_lowercase(),
            port,
        })
    }

    fn from_ip(ip: IpAddr, port: u16) -> Peer {
        let host = match ip {
            IpAddr::V4(v4) => v4.to_string(),
            IpAddr::V6(v6) => format!("[{v6}]"),
        };
        Peer { host, port }
    }

    /// El host sin corchetes, para resolverlo.
    pub fn bare_host(&self) -> &str {
        self.host.trim_start_matches('[').trim_end_matches(']')
    }
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
        if self.peers.len() > MAX_PEERS {
            return Err(format!("a route takes at most {MAX_PEERS} peers"));
        }
        let peers = self.peers()?;
        for (i, p) in peers.iter().enumerate() {
            if peers[..i].contains(p) {
                return Err(format!("repeated peer: {}", self.peers[i]));
            }
        }
        if let Some(h) = &self.health {
            if !h.path.starts_with('/') || h.path.bytes().any(|b| b <= b' ' || b == 0x7f) {
                return Err(format!("invalid health path: {:?}", h.path));
            }
            if !(1..=3600).contains(&h.interval_secs) {
                return Err("health interval_secs must be 1 to 3600".into());
            }
        }
        if !self.policy.is_null() {
            tracing::warn!(id = %self.id, "route policy ignored: local first, then peers");
        }
        Ok(())
    }

    pub fn peers(&self) -> Result<Vec<Peer>, String> {
        self.peers.iter().map(|p| Peer::parse(p)).collect()
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
    fn accepts_v1_route() {
        assert_eq!(route(ok()).validate(), Ok(()));
    }

    #[test]
    fn accepts_peers() {
        let mut v = ok();
        v["peers"] = json!(["203.0.113.7", "nodo2.ejemplo.com:8443", "[2001:db8::1]:443"]);
        v["policy"] = json!("round_robin");
        assert_eq!(route(v).validate(), Ok(()));
    }

    #[test]
    fn parses_peers() {
        let p = |s| Peer::parse(s).unwrap();
        assert_eq!(
            p("203.0.113.7"),
            Peer {
                host: "203.0.113.7".into(),
                port: 443
            }
        );
        assert_eq!(p("203.0.113.7:8443").port, 8443);
        assert_eq!(p("Nodo2.Ejemplo.com").host, "nodo2.ejemplo.com");
        assert_eq!(p("2001:db8::1").host, "[2001:db8::1]");
        assert_eq!(
            p("[2001:db8::1]:444"),
            Peer {
                host: "[2001:db8::1]".into(),
                port: 444
            }
        );
        assert_eq!(p("[2001:db8::1]").bare_host(), "2001:db8::1");
        for bad in [
            "",
            "a b",
            "x.com:0",
            "x.com:99999",
            "x.com:",
            "http://x.com",
        ] {
            assert!(Peer::parse(bad).is_err(), "{bad:?} should fail");
        }
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
            ("peers", json!(["a.com", "A.com"])),
            ("peers", json!(["1.2.3.4", "1.2.3.4:443"])),
            ("peers", json!(["bad peer"])),
            ("peers", json!(vec!["1.1.1.1"; 11])),
            ("health", json!({ "path": "health" })),
            ("health", json!({ "path": "/a b" })),
            ("health", json!({ "path": "/", "interval_secs": 0 })),
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
