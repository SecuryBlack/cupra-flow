//! CupraFlow — dueño del Caddy de SB en el servidor. Config, logging,
//! servicio, updater, status socket e intake de órdenes vienen de
//! `sb-agent-core`; aquí solo están las órdenes del proxy (`commands.rs`) y
//! el módulo de Caddy (`caddy.rs`).

mod caddy;
mod commands;
mod config;
mod route;

use config::Config;
use sb_agent_core::{
    cli, command_intake, config as core_config, logging, service, status, updater,
};
use tokio::sync::oneshot;
use tracing::info;

const AGENT_NAME: &str = "cupraflow";
#[cfg(windows)]
const SERVICE_DISPLAY_NAME: &str = "CupraFlow";

async fn run(mut shutdown: oneshot::Receiver<()>) {
    let config_path = core_config::default_config_path(AGENT_NAME);
    let log_dir = config_path
        .parent()
        .expect("config path always has a parent")
        .to_path_buf();
    logging::init(AGENT_NAME, &log_dir, "info");

    let version = env!("CARGO_PKG_VERSION");
    info!("CupraFlow v{version} starting");

    let cfg: Config = core_config::load(&config_path).unwrap_or_else(|e| {
        tracing::error!("failed to load config: {e}");
        std::process::exit(1);
    });
    let _ = core_config::sync_version_field(&config_path, version);

    let status_handle = status::StatusHandle::new(AGENT_NAME, version);
    status::spawn_server(
        status_handle.clone(),
        status::default_socket_path(AGENT_NAME),
    );

    let registry = command_intake::CommandRegistry::new();
    commands::register(&registry);
    command_intake::spawn_server(
        registry,
        command_intake::default_socket_path(AGENT_NAME),
        command_intake::SigningContext {
            server_token: cfg.token.clone().unwrap_or_default(),
            target_agent: AGENT_NAME.to_string(),
        },
    );
    #[cfg(unix)]
    commands::spawn_local_server();

    updater::start_daily_check(updater::UpdaterConfig::new(
        "securyblack",
        "cupra-flow",
        "cupraflow",
        version,
    ));

    status_handle.set_state("running");
    // El estado de Caddy se refresca cada minuto para `cupraflow status`/`top`.
    loop {
        status_handle.set_details(serde_json::json!({ "caddy": caddy::state().await }));
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
            _ = &mut shutdown => {
                info!("shutdown signal received, stopping");
                status_handle.set_state("stopping");
                break;
            }
        }
    }
}

#[cfg(windows)]
fn main() {
    cli::dispatch_common_args(AGENT_NAME, "cupraflow", env!("CARGO_PKG_VERSION"));
    match service::windows::run_service(SERVICE_DISPLAY_NAME, run) {
        Ok(_) => {}
        Err(e) if service::windows::is_not_started_by_scm(&e) => service::run_console(run),
        Err(e) => {
            eprintln!("[cupraflow] service error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    cli::dispatch_common_args(AGENT_NAME, "cupraflow", env!("CARGO_PKG_VERSION"));
    service::run_console(run);
}
