//! Órdenes de CupraFlow (contrato en `sb-wiki/Tecnica/CupraFlow v1 - contrato.md`):
//! `proxy_install`, `proxy_status`, `route_set`, `route_remove` y `route_list`. Llegan por
//! Nexus (intake firmado) o de CromoForge por el socket local, sin firma.

use crate::{caddy, route::Route};
use sb_agent_core::command_intake::{CommandOutcome, CommandRegistry, ProgressSender};
use serde_json::{json, Value};

/// Nombre con el que CromoForge encuentra el socket local:
/// `command_intake::default_socket_path(LOCAL_AGENT)` = `/run/sb-agent/cupraflow-local-cmd.sock`.
#[cfg(unix)]
pub const LOCAL_AGENT: &str = "cupraflow-local";

pub fn register(registry: &CommandRegistry) {
    registry.register("proxy_install", proxy_install);
    registry.register("proxy_status", proxy_status);
    registry.register("route_set", route_set);
    registry.register("route_remove", route_remove);
    registry.register("route_list", route_list);
}

fn outcome(result: Result<Value, String>) -> CommandOutcome {
    match result {
        Ok(v) => CommandOutcome::ok(v.to_string()),
        Err(e) => CommandOutcome::failed(e),
    }
}

async fn proxy_install(_payload: Value, _p: ProgressSender) -> CommandOutcome {
    outcome(caddy::install().await)
}

async fn route_list(_payload: Value, _p: ProgressSender) -> CommandOutcome {
    outcome(caddy::route_list().await)
}

/// Botón «Actualizar» del panel: lo mismo que la comprobación diaria, ya.
/// Si instala, sale a los 2 s para que systemd arranque el binario nuevo
/// (después de que esta respuesta haya salido).
pub async fn update_now(_payload: Value, _p: ProgressSender) -> CommandOutcome {
    let current = env!("CARGO_PKG_VERSION");
    let cfg = sb_agent_core::updater::UpdaterConfig::new("securyblack", "cupra-flow", "cupraflow", current);
    match tokio::task::spawn_blocking(move || sb_agent_core::updater::check_now(&cfg)).await {
        Ok(Ok(true)) => {
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(2));
                std::process::exit(0);
            });
            CommandOutcome::ok(json!({ "updated": true, "previous_version": current }).to_string())
        }
        Ok(Ok(false)) => CommandOutcome::ok(json!({ "updated": false, "current_version": current }).to_string()),
        Ok(Err(e)) => CommandOutcome::failed(format!("update check failed: {e}")),
        Err(e) => CommandOutcome::failed(format!("update task panicked: {e}")),
    }
}

async fn proxy_status(_payload: Value, _p: ProgressSender) -> CommandOutcome {
    outcome(caddy::status().await)
}

/// Con Caddy parado o ausente no se toca nada y se responde `applied: false`.
async fn route_set(payload: Value, _p: ProgressSender) -> CommandOutcome {
    outcome(
        async {
            let route: Route = serde_json::from_value(payload["route"].clone())
                .map_err(|e| format!("invalid route: {e}"))?;
            route.validate()?;
            let applied = caddy::state().await == "running";
            if applied {
                caddy::upsert_route(&route).await?;
            }
            Ok(json!({ "id": route.id, "applied": applied }))
        }
        .await,
    )
}

async fn route_remove(payload: Value, _p: ProgressSender) -> CommandOutcome {
    outcome(
        async {
            let id = payload["id"].as_str().unwrap_or_default();
            if !crate::route::valid_id(id) {
                return Err(format!("invalid route id: {id:?}"));
            }
            let applied = caddy::state().await == "running";
            if applied {
                caddy::delete_route(id).await?;
            }
            Ok(json!({ "id": id, "applied": applied }))
        }
        .await,
    )
}

/// Socket local para CromoForge: mismo NDJSON que el intake de sb-agent-core
/// (un `CommandEnvelope` → un `CommandResponse`), con el `auth_token`
/// compartido de la máquina pero sin firma, porque la orden no viene de la
/// API. Va aparte del intake normal para no depender de que este acepte
/// órdenes sin firmar (`REQUIRE_SIGNED_COMMANDS`).
#[cfg(unix)]
pub fn spawn_local_server() {
    use sb_agent_core::command_intake::{default_socket_path, CommandEnvelope};
    use std::os::unix::fs::PermissionsExt;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tracing::warn;

    let token = match sb_agent_core::intake_auth::ensure_token() {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "local socket: could not load shared auth token, not starting");
            return;
        }
    };
    let path = default_socket_path(LOCAL_AGENT);
    let _ = std::fs::remove_file(&path);
    let listener = match tokio::net::UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "local socket: bind failed");
            return;
        }
    };
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));

    let registry = CommandRegistry::new();
    register(&registry);
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            let registry = registry.clone();
            let token = token.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                if BufReader::new(read).read_line(&mut line).await.unwrap_or(0) == 0 {
                    return;
                }
                let reply = match serde_json::from_str::<CommandEnvelope>(line.trim_end()) {
                    Err(e) => json!({ "error": format!("malformed envelope: {e}") }),
                    Ok(env) if env.auth_token != token => json!({ "error": "Unauthorized" }),
                    Ok(env) => {
                        // Estas órdenes no mandan progreso: el canal se descarta.
                        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
                        match registry.dispatch(env, tx).await {
                            Ok(resp) => serde_json::to_value(resp).unwrap_or_default(),
                            Err(e) => json!({ "error": format!("{e:?}") }),
                        }
                    }
                };
                let mut out = reply.to_string().into_bytes();
                out.push(b'\n');
                let _ = write.write_all(&out).await;
                let _ = write.shutdown().await;
            });
        }
    });
}
