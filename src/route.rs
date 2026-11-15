//! Ruta de CupraFlow, independiente del motor (contrato en
//! `sb-wiki/Tecnica/CupraFlow v2 - contrato.md`). `caddy.rs` la traduce.

use serde::Deserialize;
use serde_json::Value;
use std::net::{IpAddr, SocketAddr};

/// Hosts como mucho: el principal y los 10 extra que admite la API.
const MAX_HOSTS: usize = 11;
/// Compañeros como mucho por ruta.
const MAX_PEERS: usize = 10;
/// Prefijos de camino y entradas de `allow_ips` como mucho por ruta.
const MAX_PATHS: usize = 10;
const MAX_ALLOW: usize = 32;
/// Sufijos de los `@id` de los objetos extra: ningún id de ruta puede acabar así.
pub const EXTRA_SUFFIXES: [&str; 3] = ["-peers", "-acme", "-tls"];

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
    /// Prefijos de camino (`/agents/me/`). Con ellos la ruta casa solo esos
    /// caminos, sin quitar el prefijo, y va delante de las de solo nombre.
    #[serde(default)]
    pub paths: Vec<String>,
    /// IPs o redes (CIDR) que pueden entrar, según la IP del socket. El resto
    /// recibe un 403 sin llegar al upstream.
    #[serde(default)]
    pub allow_ips: Vec<String>,
    /// Ruta sin upstream: 301 a este host, con el mismo camino y query.
    #[serde(default)]
    pub redirect_to: Option<String>,
    #[serde(default)]
    pub tls: Tls,
    #[serde(default)]
    pub upstream_protocol: Protocol,
}

/// De dónde sale el certificado de los hosts.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Tls {
    /// La política automática de Caddy (Let's Encrypt / ZeroSSL).
    #[default]
    Acme,
    /// La CA interna de Caddy: para Cloudflare en modo «Full» o nombres no públicos.
    Internal,
}

/// Cómo se habla con el upstream.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Http,
    /// HTTP/2 sin TLS (gRPC), con streams sin límite de tiempo.
    H2c,
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

fn valid_part(p: &str, max: usize) -> bool {
    !p.is_empty() && p.len() <= max && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// `<app>` o `<app>.<nombre>` (otra ruta hacia el mismo contenedor).
pub fn valid_id(id: &str) -> bool {
    let ok = match id.split_once('.') {
        None => valid_part(id, 64),
        Some((app, name)) => valid_part(app, 64) && valid_part(name, 32),
    };
    ok && !EXTRA_SUFFIXES.iter().any(|s| id.ends_with(s))
}

/// IP o red en notación CIDR.
fn valid_cidr(s: &str) -> bool {
    let (ip, len) = match s.split_once('/') {
        Some((ip, len)) => (ip, Some(len)),
        None => (s, None),
    };
    let Ok(ip) = ip.parse::<IpAddr>() else {
        return false;
    };
    let max = if ip.is_ipv4() { 32 } else { 128 };
    match len {
        None => true,
        Some(l) => l.parse::<u8>().is_ok_and(|l| l <= max),
    }
}

/// Camino literal: sin espacios ni control, y sin llaves, que Caddy leería
/// como marcadores (`{env.X}`, `{http.…}`).
fn plain_path(p: &str) -> bool {
    p.starts_with('/')
        && p.len() <= 256
        && !p
            .bytes()
            .any(|b| b <= b' ' || b == 0x7f || b == b'{' || b == b'}')
}

/// Prefijo de `paths`: además, sin `*` (el comodín lo pone CupraFlow).
fn valid_path(p: &str) -> bool {
    plain_path(p) && !p.contains('*')
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
            if a.port() == 0 {
                return Err(bad());
            }
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
        if let Some(to) = &self.redirect_to {
            if !valid_host(to) {
                return Err(format!("invalid redirect_to: {to:?}"));
            }
            if !self.upstreams.is_empty()
                || !self.peers.is_empty()
                || self.health.is_some()
                || !self.redirect_hosts.is_empty()
                || self.upstream_protocol != Protocol::Http
            {
                return Err("a redirect_to route takes no upstreams, peers, health, redirect_hosts or upstream_protocol".into());
            }
        }
        match self.upstreams.as_slice() {
            [] if self.redirect_to.is_some() => {}
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
            if !plain_path(&h.path) {
                return Err(format!("invalid health path: {:?}", h.path));
            }
            if !(1..=3600).contains(&h.interval_secs) {
                return Err("health interval_secs must be 1 to 3600".into());
            }
        }
        if self.paths.len() > MAX_PATHS {
            return Err(format!("a route takes at most {MAX_PATHS} paths"));
        }
        if let Some(p) = self.paths.iter().find(|p| !valid_path(p)) {
            return Err(format!("invalid path: {p:?}"));
        }
        if self.allow_ips.len() > MAX_ALLOW {
            return Err(format!("a route takes at most {MAX_ALLOW} allow_ips"));
        }
        if let Some(ip) = self.allow_ips.iter().find(|ip| !valid_cidr(ip)) {
            return Err(format!("invalid allow_ips entry: {ip:?}"));
        }
        // Lo reenviado llega con la IP del compañero, que la lista rechazaría.
        if !self.allow_ips.is_empty() && !self.peers.is_empty() {
            return Err("allow_ips and peers cannot go together".into());
        }
        // El reenvío verifica el certificado del compañero contra el dominio:
        // uno de la CA interna no pasaría.
        if self.tls == Tls::Internal && !self.peers.is_empty() {
            return Err("tls internal and peers cannot go together".into());
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
            "1.2.3.4:0",
            "[::1]:0",
            "x.com:99999",
            "x.com:",
            "http://x.com",
        ] {
            assert!(Peer::parse(bad).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn accepts_v3_options() {
        let mut v = ok();
        v["id"] = json!("11111111-2222-3333-4444-555555555555.api");
        v["paths"] = json!(["/agents/me/", "/heartbeats/ping/"]);
        v["allow_ips"] = json!(["100.64.0.0/10", "fd7a:115c:a1e0::/48", "203.0.113.7"]);
        v["tls"] = json!("internal");
        v["upstream_protocol"] = json!("h2c");
        assert_eq!(route(v).validate(), Ok(()));

        let r = route(json!({
            "id": "x.www",
            "hosts": ["www.ejemplo.com"],
            "upstreams": [],
            "redirect_to": "ejemplo.com",
        }));
        assert_eq!(r.validate(), Ok(()));
    }

    #[test]
    fn ids() {
        for good in [
            "abc",
            "11111111-2222-3333-4444-555555555555",
            "a.api",
            "a.www-2",
        ] {
            assert!(valid_id(good), "{good}");
        }
        let long = "x".repeat(65);
        for bad in [
            "",
            "a.b.c",
            ".a",
            "a.",
            "a/b",
            "a-tls",
            "a.b-peers",
            "a-acme",
            &long,
        ] {
            assert!(!valid_id(bad), "{bad}");
        }
    }

    #[test]
    fn rejects_bad_v3_options() {
        let cases = [
            ("paths", json!(["agents"])),
            ("paths", json!(["/a*"])),
            ("paths", json!(["/a b"])),
            ("paths", json!(["/{env.HOME}/"])),
            ("paths", json!(["/a}"])),
            ("paths", json!(vec!["/a"; 11])),
            ("allow_ips", json!(["100.64.0.0/33"])),
            ("allow_ips", json!(["::/129"])),
            ("allow_ips", json!(["tailscale"])),
            ("tls", json!("letsencrypt")),
            ("upstream_protocol", json!("grpc")),
            ("redirect_to", json!("bad host")),
            // redirect_to con upstream
            ("redirect_to", json!("ejemplo.com")),
        ];
        for (field, value) in cases {
            let mut v = ok();
            v[field] = value.clone();
            let r: Result<Route, _> = serde_json::from_value(v);
            assert!(
                r.map_or(true, |r| r.validate().is_err()),
                "{field} = {value} should fail"
            );
        }
        let mut v = ok();
        v["allow_ips"] = json!(["100.64.0.0/10"]);
        v["peers"] = json!(["203.0.113.7"]);
        assert!(route(v).validate().is_err(), "allow_ips + peers");
        let mut v = ok();
        v["tls"] = json!("internal");
        v["peers"] = json!(["203.0.113.7"]);
        assert!(route(v).validate().is_err(), "tls internal + peers");
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
            ("health", json!({ "path": "/{http.request.host}" })),
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
