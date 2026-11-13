//! Todo lo específico de Caddy vive aquí (traído de `cromo-forge/src/proxy.rs`
//! 0.7.2). Si un día se cambia de motor, se cambia este módulo.
//!
//! Un Caddy en contenedor (`sb-caddy`, `--network host`) publica las apps en
//! su dominio con HTTPS automático. Las rutas se gestionan solo por la API de
//! administración (`127.0.0.1:2019`), una por app con `@id = "sbapp-<id>"`,
//! así que nunca escribimos un Caddyfile. Mismos nombres que CromoForge: un
//! Caddy que ya montó CromoForge se adopta tal cual.

use crate::route::Route;
use serde_json::{json, Value};
use std::ffi::OsStr;
use std::process::Stdio;
use std::time::Duration;

pub const CONTAINER: &str = "sb-caddy";
// Imagen fija: una actualización de Caddy es un cambio de código, no un `pull` sorpresa.
const IMAGE: &str = "caddy:2.10.0";
const ADMIN: &str = "http://127.0.0.1:2019";
/// Nombre del servidor HTTP de SB dentro de la config de Caddy.
const SERVER: &str = "sb";

async fn docker<I, S>(args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let out = tokio::process::Command::new("docker")
        .args(args)
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| format!("failed to execute docker: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("docker exited with {}", out.status)
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn http(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .map_err(|e| format!("http client: {e}"))
}

/// "running" | "stopped" | "absent", tal cual lo pide `proxy_status`.
pub async fn state() -> &'static str {
    match docker(["inspect", "-f", "{{.State.Running}}", CONTAINER]).await {
        Ok(s) if s == "true" => "running",
        Ok(_) => "stopped",
        Err(_) => "absent",
    }
}

fn route_id(id: &str) -> String {
    format!("sbapp-{id}")
}

/// Ruta de Caddy: hosts → reverse_proxy al upstream. Los hosts con
/// redirección van en una subruta delante, dentro de la misma ruta, así que
/// la app sigue teniendo un solo `@id`. Todos los hosts están en el `match`
/// de fuera, que es de donde Caddy saca los certificados.
pub fn route_json(route: &Route) -> Value {
    let upstreams: Vec<Value> = route
        .upstreams
        .iter()
        .map(|u| json!({ "dial": u.dial }))
        .collect();
    let proxy = json!({ "handler": "reverse_proxy", "upstreams": upstreams });
    let handle = if route.redirect_hosts.is_empty() {
        proxy
    } else {
        json!({
            "handler": "subroute",
            "routes": [
                {
                    "match": [{ "host": route.redirect_hosts }],
                    "handle": [{
                        "handler": "static_response",
                        "status_code": 308,
                        "headers": { "Location": [format!("https://{}{{http.request.uri}}", route.primary())] }
                    }],
                    "terminal": true
                },
                { "handle": [proxy] }
            ]
        })
    };
    json!({
        "@id": route_id(&route.id),
        "match": [{ "host": route.hosts }],
        "handle": [handle],
        "terminal": true
    })
}

/// Config inicial: admin solo en loopback y un servidor vacío en :443. El
/// HTTPS automático de Caddy añade solo el servidor de :80 (redirección y
/// reto ACME HTTP-01) en cuanto aparece una ruta con `host`.
fn initial_config() -> Value {
    json!({
        "admin": { "listen": "127.0.0.1:2019" },
        "apps": { "http": { "servers": { SERVER: { "listen": [":443"], "routes": [] } } } }
    })
}

/// Crea o sustituye la ruta. PATCH sobre `/id/...` reemplaza la ruta
/// existente en sitio; si no existe aún, se añade al final.
pub async fn upsert_route(route: &Route) -> Result<(), String> {
    let client = http(Duration::from_secs(10))?;
    let body = route_json(route).to_string();

    let patched = client
        .patch(format!("{ADMIN}/id/{}", route_id(&route.id)))
        .header("Content-Type", "application/json")
        .body(body.clone())
        .send()
        .await;
    if matches!(&patched, Ok(r) if r.status().is_success()) {
        return Ok(());
    }

    let resp = client
        .post(format!("{ADMIN}/config/apps/http/servers/{SERVER}/routes"))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Caddy admin API unreachable: {e}"))?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        Err(format!(
            "Caddy admin API returned {status}: {}",
            resp.text().await.unwrap_or_default()
        ))
    }
}

/// Borra la ruta. Que no exista no es un error.
pub async fn delete_route(id: &str) -> Result<(), String> {
    let client = http(Duration::from_secs(10))?;
    let resp = client
        .delete(format!("{ADMIN}/id/{}", route_id(id)))
        .send()
        .await
        .map_err(|e| format!("Caddy admin API unreachable: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() || status == 404 || text.contains("unknown object ID") {
        Ok(())
    } else {
        Err(format!("Caddy admin API returned {status}: {text}"))
    }
}

/// Procesos escuchando en :80 y :443 según `ss -ltnpH`. "unknown" si `ss`
/// no puede ver el proceso (sin privilegios).
fn parse_ss(output: &str) -> (Option<String>, Option<String>) {
    let mut port80 = None;
    let mut port443 = None;
    for line in output.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(local) = fields.get(3) else { continue };
        let slot = match local.rsplit(':').next() {
            Some("80") => &mut port80,
            Some("443") => &mut port443,
            _ => continue,
        };
        if slot.is_none() {
            let process = line
                .split("users:((\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .unwrap_or("unknown");
            *slot = Some(process.to_string());
        }
    }
    (port80, port443)
}

async fn listening_ports() -> Result<(Option<String>, Option<String>), String> {
    let out = tokio::process::Command::new("ss")
        .args(["-ltnpH"])
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| format!("failed to execute ss: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ss failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(parse_ss(&String::from_utf8_lossy(&out.stdout)))
}

/// Estado de `proxy_status`. Un puerto ocupado por nuestro propio Caddy
/// cuenta como libre (`null`).
pub async fn status() -> Result<Value, String> {
    let caddy = state().await;
    let (p80, p443) = listening_ports().await?;
    let ours = |p: Option<String>| p.filter(|name| !(caddy == "running" && name == "caddy"));
    Ok(json!({ "caddy": caddy, "port80": ours(p80), "port443": ours(p443) }))
}

/// Espera a que la API de administración responda y, si Caddy arrancó sin
/// config (primera vez; luego `--resume` recupera la autoguardada), carga
/// la inicial.
async fn ensure_config() -> Result<(), String> {
    let client = http(Duration::from_secs(5))?;
    let mut last = String::new();
    for _ in 0..30 {
        // Antes del primer `/load` Caddy escucha en `localhost:2019` (resuelve también a 127.0.0.1).
        match client.get(format!("{ADMIN}/config/")).send().await {
            Ok(r) if r.status().is_success() => {
                let body = r.text().await.unwrap_or_default();
                if body.trim() != "null" {
                    return Ok(());
                }
                let resp = client
                    .post(format!("{ADMIN}/load"))
                    .header("Content-Type", "application/json")
                    .body(initial_config().to_string())
                    .send()
                    .await
                    .map_err(|e| format!("Caddy /load failed: {e}"))?;
                if !resp.status().is_success() {
                    let status = resp.status();
                    return Err(format!(
                        "Caddy /load returned {status}: {}",
                        resp.text().await.unwrap_or_default()
                    ));
                }
                return Ok(());
            }
            Ok(r) => last = format!("HTTP {}", r.status()),
            Err(e) => last = e.to_string(),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(format!("Caddy admin API did not come up: {last}"))
}

/// `proxy_install`: solo con el 80 y el 443 libres. Si `sb-caddy` ya existe
/// lo arranca o lo deja como está.
pub async fn install() -> Result<Value, String> {
    match state().await {
        "running" => {}
        st => {
            let (p80, p443) = listening_ports().await?;
            if p80.is_some() || p443.is_some() {
                let busy: Vec<String> = [("80", p80), ("443", p443)]
                    .into_iter()
                    .filter_map(|(port, p)| p.map(|p| format!("port {port} in use by {p}")))
                    .collect();
                return Err(busy.join(", "));
            }
            if st == "stopped" {
                docker(["start", CONTAINER]).await?;
            } else {
                docker([
                    "run",
                    "-d",
                    "--name",
                    CONTAINER,
                    "--network",
                    "host",
                    "--restart",
                    "unless-stopped",
                    "-v",
                    "sb-caddy-data:/data",
                    // La config autoguardada vive en el mismo volumen que los certificados.
                    "-e",
                    "XDG_CONFIG_HOME=/data/config",
                    IMAGE,
                    "caddy",
                    "run",
                    "--resume",
                ])
                .await?;
            }
        }
    }
    ensure_config().await?;
    status().await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(v: Value) -> Route {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn route_json_shape() {
        let r = route_json(&route(json!({
            "id": "11111111-2222-3333-4444-555555555555",
            "hosts": ["app.ejemplo.com"],
            "upstreams": [{ "dial": "127.0.0.1:20001" }],
        })));
        assert_eq!(r["@id"], "sbapp-11111111-2222-3333-4444-555555555555");
        assert_eq!(r["match"][0]["host"], json!(["app.ejemplo.com"]));
        assert_eq!(r["handle"][0]["handler"], "reverse_proxy");
        assert_eq!(r["handle"][0]["upstreams"][0]["dial"], "127.0.0.1:20001");
        assert_eq!(r["terminal"], true);
    }

    #[test]
    fn route_json_with_redirects() {
        let r = route_json(&route(json!({
            "id": "x",
            "hosts": ["ejemplo.com", "www.ejemplo.com", "otro.com"],
            "redirect_hosts": ["www.ejemplo.com"],
            "upstreams": [{ "dial": "127.0.0.1:20001" }],
        })));
        assert_eq!(
            r["match"][0]["host"],
            json!(["ejemplo.com", "www.ejemplo.com", "otro.com"])
        );
        let routes = &r["handle"][0]["routes"];
        assert_eq!(routes[0]["match"][0]["host"], json!(["www.ejemplo.com"]));
        assert_eq!(routes[0]["handle"][0]["status_code"], 308);
        assert_eq!(
            routes[0]["handle"][0]["headers"]["Location"][0],
            "https://ejemplo.com{http.request.uri}"
        );
        assert_eq!(
            routes[1]["handle"][0]["upstreams"][0]["dial"],
            "127.0.0.1:20001"
        );
    }

    #[test]
    fn initial_config_admin_on_loopback() {
        let c = initial_config();
        assert_eq!(c["admin"]["listen"], "127.0.0.1:2019");
        assert!(c["apps"]["http"]["servers"]["sb"]["routes"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn parse_ss_finds_80_and_443() {
        let out = "LISTEN 0 4096 0.0.0.0:80 0.0.0.0:* users:((\"nginx\",pid=12,fd=6))\n\
                   LISTEN 0 4096 [::]:443 [::]:* users:((\"caddy\",pid=99,fd=7))\n\
                   LISTEN 0 4096 127.0.0.1:8080 0.0.0.0:* users:((\"app\",pid=1,fd=3))\n\
                   LISTEN 0 4096 127.0.0.1:2080 0.0.0.0:*\n";
        assert_eq!(parse_ss(out), (Some("nginx".into()), Some("caddy".into())));
        assert_eq!(
            parse_ss("LISTEN 0 1 *:80 *:*\n"),
            (Some("unknown".into()), None)
        );
        assert_eq!(parse_ss(""), (None, None));
    }
}
