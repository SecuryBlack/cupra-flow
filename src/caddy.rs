//! Todo lo específico de Caddy vive aquí (traído de `cromo-forge/src/proxy.rs`
//! 0.7.2). Si un día se cambia de motor, se cambia este módulo.
//!
//! Un Caddy en contenedor (`sb-caddy`, `--network host`) publica las apps en
//! su dominio con HTTPS automático. Las rutas se gestionan solo por la API de
//! administración (`127.0.0.1:2019`), una por app con `@id = "sbapp-<id>"`,
//! así que nunca escribimos un Caddyfile. Mismos nombres que CromoForge: un
//! Caddy que ya montó CromoForge se adopta tal cual.

use crate::route::{Peer, Protocol, Route, Tls, EXTRA_SUFFIXES};
use serde_json::{json, Value};
use std::ffi::OsStr;
use std::net::IpAddr;
use std::process::Stdio;
use std::time::Duration;

pub const CONTAINER: &str = "sb-caddy";
// Imagen fija: una actualización de Caddy es un cambio de código, no un `pull` sorpresa.
const IMAGE: &str = "caddy:2.10.0";
const ADMIN: &str = "http://127.0.0.1:2019";
/// Nombre del servidor HTTP de SB dentro de la config de Caddy.
const SERVER: &str = "sb";
/// Servidor de :80 de CupraFlow, solo para reenviar retos ACME a los
/// compañeros. Caddy le añade detrás sus redirecciones a HTTPS.
const SERVER_HTTP: &str = "sb_http";
/// Marca de lo reenviado entre compañeros: quien la recibe solo sirve en local.
const MARKER: &str = "X-Sb-Peer";

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
        // Cada cambio de config reinicia la API de administración y corta las
        // conexiones abiertas: una conexión nueva por petición.
        .pool_max_idle_per_host(0)
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

/// Objetos extra de una ruta con compañeros: (sufijo del `@id`, lista de
/// la config donde se añade, si va delante).
const EXTRAS: [(&str, &str, bool); 3] = [
    ("peers", "apps/http/servers/sb/errors/routes", false),
    ("acme", "apps/http/servers/sb_http/routes", false),
    ("tls", "apps/tls/automation/policies", true),
];

fn extra_id(id: &str, suffix: &str) -> String {
    format!("sbapp-{id}-{suffix}")
}

fn not_marked() -> Value {
    json!([{ "header": { MARKER: ["1"] } }])
}

/// Proxy al upstream local. Con compañeros: salud activa y pasiva, la marca
/// fuera y el X-Forwarded-For que deja `route_json` en `sb_client`.
fn local_proxy(route: &Route) -> Value {
    let upstreams: Vec<Value> = route
        .upstreams
        .iter()
        .map(|u| json!({ "dial": u.dial }))
        .collect();
    let mut proxy = json!({ "handler": "reverse_proxy", "upstreams": upstreams });
    if route.upstream_protocol == Protocol::H2c {
        // gRPC sin TLS. Caddy no pone límite de tiempo a las peticiones ni a
        // los streams (`read_timeout`, `write_timeout` y `stream_timeout`
        // van a 0 por defecto); el volcado inmediato hace falta para los
        // streams bidireccionales.
        proxy["transport"] = json!({ "protocol": "http", "versions": ["h2c", "2"] });
        proxy["flush_interval"] = json!(-1);
    }
    if !route.peers.is_empty() {
        proxy["headers"] = json!({ "request": {
            "set": { "X-Forwarded-For": ["{http.vars.sb_client}"] },
            "delete": [MARKER],
        }});
        // Tras un fallo el local queda fuera 10 s y todo va a los compañeros.
        let mut checks = json!({ "passive": { "fail_duration": "10s" } });
        if let Some(h) = &route.health {
            // Caddy solo da por sano un 2xx.
            checks["active"] = json!({
                "uri": h.path,
                "interval": format!("{}s", h.interval_secs),
                "timeout": "5s",
            });
        }
        proxy["health_checks"] = checks;
    }
    proxy
}

/// Ruta de Caddy: hosts → reverse_proxy al upstream. Los hosts con
/// redirección van en una subruta delante, dentro de la misma ruta, así que
/// la app sigue teniendo un solo `@id`. Todos los hosts están en el `match`
/// de fuera, que es de donde Caddy saca los certificados.
///
/// Con compañeros (`peer_ips`, sus IPs), el X-Forwarded-For que llega con la
/// marca se respeta solo si viene de una de esas IPs; si no, va la IP de
/// quien conecta, como hace Caddy por defecto.
pub fn route_json(route: &Route, peer_ips: &[IpAddr]) -> Value {
    let mut routes = Vec::new();
    if !route.allow_ips.is_empty() {
        // `remote_ip` mira la IP del socket, nunca X-Forwarded-For.
        routes.push(json!({
            "match": [{ "not": [{ "remote_ip": { "ranges": route.allow_ips } }] }],
            "handle": [{ "handler": "static_response", "status_code": 403, "body": "Forbidden" }],
            "terminal": true
        }));
    }
    if !route.redirect_hosts.is_empty() {
        routes.push(json!({
            "match": [{ "host": route.redirect_hosts }],
            "handle": [{
                "handler": "static_response",
                "status_code": 308,
                "headers": { "Location": [format!("https://{}{{http.request.uri}}", route.primary())] }
            }],
            "terminal": true
        }));
    }
    if !route.peers.is_empty() {
        let ips: Vec<String> = peer_ips.iter().map(|ip| ip.to_string()).collect();
        routes.push(
            json!({ "handle": [{ "handler": "vars", "sb_client": "{http.request.remote.host}" }] }),
        );
        routes.push(json!({
            "match": [{ "header": { MARKER: ["1"] }, "remote_ip": { "ranges": ips } }],
            "handle": [{ "handler": "vars", "sb_client": "{http.request.header.X-Forwarded-For}" }]
        }));
    }
    let last = match &route.redirect_to {
        Some(to) => json!({
            "handler": "static_response",
            "status_code": 301,
            "headers": { "Location": [format!("https://{to}{{http.request.uri}}")] }
        }),
        None => local_proxy(route),
    };
    let handle = if routes.is_empty() {
        last
    } else {
        routes.push(json!({ "handle": [last] }));
        json!({ "handler": "subroute", "routes": routes })
    };
    json!({
        "@id": route_id(&route.id),
        "match": [host_match(route)],
        "handle": [handle],
        "terminal": true
    })
}

/// `host` y, si la ruta los tiene, los prefijos de camino (sin quitarlos).
fn host_match(route: &Route) -> Value {
    let mut m = json!({ "host": route.hosts });
    if !route.paths.is_empty() {
        let paths: Vec<String> = route.paths.iter().map(|p| format!("{p}*")).collect();
        m["path"] = json!(paths);
    }
    m
}

/// Ruta de errores de `sb`: la petición sin marca va por HTTPS al primer
/// compañero que conteste, con el dominio como SNI y Host y la marca puesta,
/// solo si es seguro repetirla allí:
/// - 503 (no había upstream local sano): no se intentó, así que llega
///   intacta, con su cuerpo, y la app no la vio. Cualquier método.
/// - 502/504 (se intentó el local y falló): Go ya cerró el cuerpo y la app
///   quizá la procesó. Solo GET y HEAD.
fn peers_route_json(route: &Route, peers: &[Peer]) -> Value {
    let upstreams: Vec<Value> = peers
        .iter()
        .map(|p| json!({ "dial": format!("{}:{}", p.host, p.port) }))
        .collect();
    json!({
        "@id": extra_id(&route.id, "peers"),
        "match": [
            with(host_match(route), json!({
                "not": not_marked(),
                "expression": "{http.error.status_code} == 503"
            })),
            with(host_match(route), json!({
                "not": not_marked(),
                "method": ["GET", "HEAD"],
                "expression": "{http.error.status_code} in [502, 504]"
            }))
        ],
        "handle": [{
            "handler": "reverse_proxy",
            "upstreams": upstreams,
            "transport": {
                "protocol": "http",
                "tls": { "server_name": "{http.request.host}" },
                "dial_timeout": "2s"
            },
            "headers": { "request": { "set": { MARKER: ["1"] } } },
            // Más que `dial_timeout`, para que dé tiempo a probar el siguiente.
            "load_balancing": { "selection_policy": { "policy": "first" }, "try_duration": "5s" },
            "health_checks": { "passive": { "fail_duration": "10s" } }
        }],
        "terminal": true
    })
}

/// `base` con las claves de `more` añadidas.
fn with(mut base: Value, more: Value) -> Value {
    if let (Some(b), Value::Object(m)) = (base.as_object_mut(), more) {
        b.extend(m);
    }
    base
}

/// Reto HTTP-01 que este nodo no conoce (los suyos los responde Caddy antes
/// de las rutas): se pregunta a los compañeros por el 80, al siguiente
/// mientras contesten que no lo tienen (cualquier cosa que no sea 2xx).
fn acme_handler(peers: &[Peer]) -> Value {
    let upstreams: Vec<Value> = peers
        .iter()
        .map(|p| json!({ "dial": format!("{}:80", p.host) }))
        .collect();
    let mut proxy = json!({
        "handler": "reverse_proxy",
        "upstreams": upstreams,
        "transport": { "protocol": "http", "dial_timeout": "2s" },
        "headers": { "request": { "set": { MARKER: ["1"] } } },
        "load_balancing": { "selection_policy": { "policy": "first" }, "try_duration": "5s" }
    });
    if peers.len() > 1 {
        proxy["handle_response"] = json!([{
            "match": { "status_code": [3, 4, 5] },
            "routes": [{ "handle": [acme_handler(&peers[1..])] }]
        }]);
    }
    proxy
}

fn acme_route_json(route: &Route, peers: &[Peer]) -> Value {
    json!({
        "@id": extra_id(&route.id, "acme"),
        "match": [{
            "host": route.hosts,
            "path": ["/.well-known/acme-challenge/*"],
            "not": not_marked()
        }],
        "handle": [acme_handler(peers)],
        "terminal": true
    })
}

/// Política TLS de los hosts. Con `tls: internal` (nunca con compañeros), la
/// CA interna de Caddy. Si no, con compañeros, solo HTTP-01: un reto TLS-ALPN que cae en el otro
/// nodo no se puede reenviar.
fn tls_policy_json(route: &Route) -> Value {
    let issuer = match route.tls {
        Tls::Internal => json!({ "module": "internal" }),
        Tls::Acme => {
            json!({ "module": "acme", "challenges": { "tls-alpn": { "disabled": true } } })
        }
    };
    json!({
        "@id": extra_id(&route.id, "tls"),
        "subjects": route.hosts,
        "issuers": [issuer]
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

fn admin_error(e: reqwest::Error) -> String {
    format!("Caddy admin API unreachable: {e}")
}

async fn check(resp: reqwest::Response) -> Result<String, String> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(text)
    } else {
        Err(format!("Caddy admin API returned {status}: {text}"))
    }
}

/// Crea o sustituye el objeto con ese `@id`. PATCH sobre `/id/...` lo
/// reemplaza en sitio; si no existe aún, se añade a `list` (al final, o
/// delante con `front`).
async fn upsert(
    client: &reqwest::Client,
    id: &str,
    body: &Value,
    list: &str,
    front: bool,
) -> Result<(), String> {
    let body = body.to_string();
    let patched = client
        .patch(format!("{ADMIN}/id/{id}"))
        .header("Content-Type", "application/json")
        .body(body.clone())
        .send()
        .await
        .map_err(admin_error)?;
    let status = patched.status();
    if status.is_success() {
        return Ok(());
    }
    // Solo se añade si no existe: tras cualquier otro fallo (o un timeout,
    // que puede llegar con el cambio ya hecho) un POST dejaría dos objetos
    // con el mismo `@id`.
    let text = patched.text().await.unwrap_or_default();
    if status != 404 && !text.contains("unknown object ID") {
        return Err(format!("Caddy admin API returned {status}: {text}"));
    }
    let req = if front {
        // PUT sobre un índice inserta en esa posición.
        client.put(format!("{ADMIN}/config/{list}/0"))
    } else {
        client.post(format!("{ADMIN}/config/{list}"))
    };
    let resp = req
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(admin_error)?;
    check(resp).await.map(|_| ())
}

/// Borra el objeto con ese `@id`. Que no exista no es un error.
async fn delete(client: &reqwest::Client, id: &str) -> Result<(), String> {
    let resp = client
        .delete(format!("{ADMIN}/id/{id}"))
        .send()
        .await
        .map_err(admin_error)?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() || status == 404 || text.contains("unknown object ID") {
        Ok(())
    } else {
        Err(format!("Caddy admin API returned {status}: {text}"))
    }
}

/// Valor vacío para el trozo `name` de una ruta de la config que no existe,
/// con lo que cuelga de él (`rest`), p. ej. `tls` + `automation/policies` →
/// `{"automation":{"policies":[]}}`. El servidor de :80 lleva su `listen`.
fn empty_at(name: &str, rest: &[&str]) -> Value {
    let mut v = match (name, rest) {
        (SERVER_HTTP, _) => json!({ "listen": [":80"], "routes": [] }),
        (_, []) => json!([]),
        _ => json!({}),
    };
    if let [first, more @ ..] = rest {
        v[*first] = empty_at(first, more);
    }
    v
}

/// Crea `path` en la config si no existe, sin tocar lo que ya hay. Solo lo
/// piden las rutas con compañeros: un Caddy sin ellas queda como estaba.
async fn ensure_path(client: &reqwest::Client, path: &str) -> Result<(), String> {
    let segments: Vec<&str> = path.split('/').collect();
    for i in 1..=segments.len() {
        let prefix = segments[..i].join("/");
        let resp = client
            .get(format!("{ADMIN}/config/{prefix}"))
            .send()
            .await
            .map_err(admin_error)?;
        if check(resp).await?.trim() == "null" {
            let resp = client
                .put(format!("{ADMIN}/config/{prefix}"))
                .header("Content-Type", "application/json")
                .body(empty_at(segments[i - 1], &segments[i..]).to_string())
                .send()
                .await
                .map_err(admin_error)?;
            if resp.status().is_success() {
                return Ok(());
            }
            // Otro `route_set` a la vez puede haberlo creado entre el GET y el PUT.
            let err = check(resp).await.unwrap_err();
            let again = client
                .get(format!("{ADMIN}/config/{prefix}"))
                .send()
                .await
                .map_err(admin_error)?;
            return match check(again).await {
                Ok(t) if t.trim() != "null" => Ok(()),
                _ => Err(err),
            };
        }
    }
    Ok(())
}

/// IPs de los compañeros, para fiarse de su X-Forwarded-For. Los nombres se
/// resuelven aquí: si cambian de IP, vale hasta el siguiente `route_set`.
async fn resolve(peers: &[Peer]) -> Result<Vec<IpAddr>, String> {
    let mut ips = Vec::new();
    for p in peers {
        let addrs = tokio::net::lookup_host((p.bare_host(), p.port))
            .await
            .map_err(|e| format!("cannot resolve peer {}: {e}", p.host))?;
        for a in addrs {
            if !ips.contains(&a.ip()) {
                ips.push(a.ip());
            }
        }
    }
    Ok(ips)
}

/// Los objetos extra que lleva la ruta, en el orden de `EXTRAS`; `None` los
/// que no lleva (se borran si estaban).
fn extras(route: &Route, peers: &[Peer]) -> [Option<Value>; 3] {
    let with_peers = !peers.is_empty();
    [
        with_peers.then(|| peers_route_json(route, peers)),
        with_peers.then(|| acme_route_json(route, peers)),
        (with_peers || route.tls == Tls::Internal).then(|| tls_policy_json(route)),
    ]
}

/// Crea o sustituye la ruta y sus objetos extra (antes que la ruta, para que
/// no haya un momento con salud y sin a dónde ir); quita los que ya no lleva.
/// Una ruta con caminos va delante de todas, para ganar a las de solo nombre.
pub async fn upsert_route(route: &Route) -> Result<(), String> {
    let client = http(Duration::from_secs(10))?;
    let peers = route.peers()?;
    let ips = if peers.is_empty() {
        Vec::new()
    } else {
        resolve(&peers).await?
    };
    for ((suffix, list, front), body) in EXTRAS.into_iter().zip(extras(route, &peers)) {
        let id = extra_id(&route.id, suffix);
        match body {
            Some(body) => {
                ensure_path(&client, list).await?;
                upsert(&client, &id, &body, list, front).await?;
            }
            None => delete(&client, &id).await?,
        }
    }
    let routes = format!("apps/http/servers/{SERVER}/routes");
    let id = route_id(&route.id);
    let body = route_json(route, &ips);
    if route.paths.is_empty() {
        return upsert(&client, &id, &body, &routes, false).await;
    }
    // PATCH la dejaría donde estuviera: se quita y se pone delante.
    // ponytail: entre los dos pasos (milisegundos) la petición cae en la ruta
    // de solo nombre del mismo host, si la hay.
    delete(&client, &id).await?;
    upsert(&client, &id, &body, &routes, true).await
}

/// Borra la ruta y sus objetos extra y, si `id` es una app (sin `.`), las
/// demás rutas de esa app (`<id>.<nombre>`). Que no existan no es un error.
pub async fn delete_route(id: &str) -> Result<(), String> {
    let client = http(Duration::from_secs(10))?;
    let mut ids = vec![id.to_string()];
    if !id.contains('.') {
        ids.extend(sub_routes(&client, id).await?);
    }
    for id in ids {
        delete(&client, &route_id(&id)).await?;
        for (suffix, _, _) in EXTRAS {
            delete(&client, &extra_id(&id, suffix)).await?;
        }
    }
    Ok(())
}

/// Ids de las rutas `<app>.<nombre>` que hay en Caddy.
async fn sub_routes(client: &reqwest::Client, app: &str) -> Result<Vec<String>, String> {
    let resp = client
        .get(format!("{ADMIN}/config/apps/http/servers/{SERVER}/routes"))
        .send()
        .await
        .map_err(admin_error)?;
    let routes: Value = serde_json::from_str(&check(resp).await?).unwrap_or(Value::Null);
    Ok(sub_route_ids(&routes, app))
}

fn sub_route_ids(routes: &Value, app: &str) -> Vec<String> {
    let prefix = format!("{}.", route_id(app));
    routes
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["@id"].as_str()?.strip_prefix("sbapp-"))
        .filter(|id| format!("sbapp-{id}").starts_with(&prefix))
        .filter(|id| !EXTRA_SUFFIXES.iter().any(|s| id.ends_with(s)))
        .map(str::to_string)
        .collect()
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
        let r = route_json(
            &route(json!({
                "id": "11111111-2222-3333-4444-555555555555",
                "hosts": ["app.ejemplo.com"],
                "upstreams": [{ "dial": "127.0.0.1:20001" }],
            })),
            &[],
        );
        assert_eq!(r["@id"], "sbapp-11111111-2222-3333-4444-555555555555");
        assert_eq!(r["match"][0]["host"], json!(["app.ejemplo.com"]));
        assert_eq!(r["handle"][0]["handler"], "reverse_proxy");
        assert_eq!(r["handle"][0]["upstreams"][0]["dial"], "127.0.0.1:20001");
        assert_eq!(r["terminal"], true);
    }

    #[test]
    fn route_json_with_redirects() {
        let r = route_json(
            &route(json!({
                "id": "x",
                "hosts": ["ejemplo.com", "www.ejemplo.com", "otro.com"],
                "redirect_hosts": ["www.ejemplo.com"],
                "upstreams": [{ "dial": "127.0.0.1:20001" }],
            })),
            &[],
        );
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

    fn with_peers() -> Route {
        route(json!({
            "id": "x",
            "hosts": ["ejemplo.com", "www.ejemplo.com"],
            "redirect_hosts": ["www.ejemplo.com"],
            "upstreams": [{ "dial": "127.0.0.1:20001" }],
            "peers": ["203.0.113.7", "nodo3.ejemplo.com:8443"],
            "health": { "path": "/health" },
        }))
    }

    #[test]
    fn route_json_with_peers() {
        let r = with_peers();
        let ips: Vec<IpAddr> = vec![
            "203.0.113.7".parse().unwrap(),
            "198.51.100.3".parse().unwrap(),
        ];
        let j = route_json(&r, &ips);
        let routes = &j["handle"][0]["routes"];
        assert_eq!(routes[0]["handle"][0]["status_code"], 308);
        assert_eq!(
            routes[1]["handle"][0]["sb_client"],
            "{http.request.remote.host}"
        );
        assert_eq!(routes[2]["match"][0]["header"]["X-Sb-Peer"], json!(["1"]));
        assert_eq!(
            routes[2]["match"][0]["remote_ip"]["ranges"],
            json!(["203.0.113.7", "198.51.100.3"])
        );
        let proxy = &routes[3]["handle"][0];
        assert_eq!(proxy["upstreams"], json!([{ "dial": "127.0.0.1:20001" }]));
        assert_eq!(proxy["health_checks"]["active"]["uri"], "/health");
        assert_eq!(proxy["health_checks"]["active"]["interval"], "10s");
        assert_eq!(proxy["headers"]["request"]["delete"], json!(["X-Sb-Peer"]));
        assert_eq!(
            proxy["headers"]["request"]["set"]["X-Forwarded-For"],
            json!(["{http.vars.sb_client}"])
        );
    }

    #[test]
    fn extras_with_peers() {
        let r = with_peers();
        let peers = r.peers().unwrap();

        let e = peers_route_json(&r, &peers);
        assert_eq!(e["@id"], "sbapp-x-peers");
        assert_eq!(e["match"][0]["not"][0]["header"]["X-Sb-Peer"], json!(["1"]));
        assert_eq!(
            e["match"][0]["expression"],
            "{http.error.status_code} == 503"
        );
        assert!(e["match"][0].get("method").is_none());
        assert_eq!(e["match"][1]["method"], json!(["GET", "HEAD"]));
        assert_eq!(
            e["match"][1]["expression"],
            "{http.error.status_code} in [502, 504]"
        );
        let p = &e["handle"][0];
        assert_eq!(
            p["upstreams"],
            json!([{ "dial": "203.0.113.7:443" }, { "dial": "nodo3.ejemplo.com:8443" }])
        );
        assert_eq!(p["transport"]["tls"]["server_name"], "{http.request.host}");
        assert_eq!(p["headers"]["request"]["set"]["X-Sb-Peer"], json!(["1"]));

        let a = acme_route_json(&r, &peers);
        assert_eq!(a["@id"], "sbapp-x-acme");
        assert_eq!(
            a["match"][0]["path"],
            json!(["/.well-known/acme-challenge/*"])
        );
        let first = &a["handle"][0];
        assert_eq!(
            first["upstreams"],
            json!([{ "dial": "203.0.113.7:80" }, { "dial": "nodo3.ejemplo.com:80" }])
        );
        let next = &first["handle_response"][0]["routes"][0]["handle"][0];
        assert_eq!(
            next["upstreams"],
            json!([{ "dial": "nodo3.ejemplo.com:80" }])
        );
        assert!(next.get("handle_response").is_none());

        let t = tls_policy_json(&r);
        assert_eq!(t["subjects"], json!(["ejemplo.com", "www.ejemplo.com"]));
        assert_eq!(t["issuers"][0]["challenges"]["tls-alpn"]["disabled"], true);
    }

    fn v3(extra: Value) -> Route {
        let mut v = json!({
            "id": "a.api",
            "hosts": ["api.ejemplo.com"],
            "upstreams": [{ "dial": "127.0.0.1:20001" }],
        });
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        route(v)
    }

    #[test]
    fn without_v3_fields_nothing_changes() {
        let r = route_json(&v3(json!({})), &[]);
        assert_eq!(
            r,
            json!({
                "@id": "sbapp-a.api",
                "match": [{ "host": ["api.ejemplo.com"] }],
                "handle": [{ "handler": "reverse_proxy", "upstreams": [{ "dial": "127.0.0.1:20001" }] }],
                "terminal": true
            })
        );
        assert_eq!(extras(&v3(json!({})), &[]), [None, None, None]);
    }

    #[test]
    fn paths_match_prefix() {
        let r = route_json(
            &v3(json!({ "paths": ["/agents/me/", "/heartbeats/ping/"] })),
            &[],
        );
        assert_eq!(
            r["match"],
            json!([{ "host": ["api.ejemplo.com"], "path": ["/agents/me/*", "/heartbeats/ping/*"] }])
        );
        // Sin `rewrite`: el prefijo llega al upstream.
        assert_eq!(r["handle"][0]["handler"], "reverse_proxy");
        let mut p = with_peers();
        p.paths = vec!["/x/".into()];
        let e = peers_route_json(&p, &p.peers().unwrap());
        assert_eq!(e["match"][0]["path"], json!(["/x/*"]));
        assert_eq!(e["match"][1]["path"], json!(["/x/*"]));
        assert_eq!(e["match"][1]["method"], json!(["GET", "HEAD"]));
    }

    #[test]
    fn allow_ips_go_first() {
        let r = route_json(&v3(json!({ "allow_ips": ["100.64.0.0/10"] })), &[]);
        let routes = &r["handle"][0]["routes"];
        assert_eq!(
            routes[0]["match"][0]["not"][0]["remote_ip"]["ranges"],
            json!(["100.64.0.0/10"])
        );
        assert_eq!(routes[0]["handle"][0]["status_code"], 403);
        assert_eq!(routes[1]["handle"][0]["handler"], "reverse_proxy");
    }

    #[test]
    fn redirect_to_has_no_upstream() {
        let r = route_json(
            &route(json!({
                "id": "a.www",
                "hosts": ["www.ejemplo.com"],
                "upstreams": [],
                "redirect_to": "ejemplo.com",
            })),
            &[],
        );
        let h = &r["handle"][0];
        assert_eq!(h["status_code"], 301);
        assert_eq!(
            h["headers"]["Location"],
            json!(["https://ejemplo.com{http.request.uri}"])
        );
    }

    #[test]
    fn h2c_upstream() {
        let r = route_json(&v3(json!({ "upstream_protocol": "h2c" })), &[]);
        let p = &r["handle"][0];
        assert_eq!(p["transport"]["versions"], json!(["h2c", "2"]));
        assert_eq!(p["flush_interval"], -1);
    }

    #[test]
    fn internal_tls() {
        let r = v3(json!({ "tls": "internal" }));
        let [peers, acme, tls] = extras(&r, &[]);
        assert!(peers.is_none() && acme.is_none());
        let tls = tls.unwrap();
        assert_eq!(tls["@id"], "sbapp-a.api-tls");
        assert_eq!(tls["subjects"], json!(["api.ejemplo.com"]));
        assert_eq!(tls["issuers"], json!([{ "module": "internal" }]));
    }

    #[test]
    fn finds_sub_routes() {
        let routes = json!([
            { "@id": "sbapp-a" },
            { "@id": "sbapp-a.api" },
            { "@id": "sbapp-a.www" },
            { "@id": "sbapp-ab.api" },
            { "@id": "homepage" },
            {}
        ]);
        assert_eq!(sub_route_ids(&routes, "a"), vec!["a.api", "a.www"]);
        assert!(sub_route_ids(&Value::Null, "a").is_empty());
    }

    #[test]
    fn empty_at_builds_missing_parents() {
        assert_eq!(
            empty_at("tls", &["automation", "policies"]),
            json!({ "automation": { "policies": [] } })
        );
        assert_eq!(
            empty_at("sb_http", &["routes"]),
            json!({ "listen": [":80"], "routes": [] })
        );
        assert_eq!(empty_at("errors", &["routes"]), json!({ "routes": [] }));
        assert_eq!(empty_at("policies", &[]), json!([]));
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
