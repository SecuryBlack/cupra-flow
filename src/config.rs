use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

/// Configuracion principal de CupraFlow
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub version: Option<String>,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub service: ServiceConfig,
    pub loadbalancer: LoadBalancerConfig,
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub bind_address: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub name: String,
    pub description: String,
    pub startup: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoadBalancerConfig {
    pub enabled: bool,
    pub algorithm: String,
    pub health_check_interval: u64,
    pub backends: Vec<Backend>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Backend {
    pub name: String,
    pub address: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateConfig {
    pub channel: String,
    pub check_on_startup: bool,
    pub check_interval: u64,
    pub github_repo: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            server: ServerConfig {
                port: 8080,
                bind_address: "0.0.0.0".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
            },
            service: ServiceConfig {
                name: "CupraFlow".to_string(),
                description: "Agente de gestion de red y balanceo de carga".to_string(),
                startup: "auto".to_string(),
            },
            loadbalancer: LoadBalancerConfig {
                enabled: false,
                algorithm: "round_robin".to_string(),
                health_check_interval: 30,
                backends: vec![],
            },
            update: UpdateConfig {
                channel: "stable".to_string(),
                check_on_startup: true,
                check_interval: 24,
                github_repo: "securyblack/cupra-flow".to_string(),
            },
        }
    }
}

impl Config {
    /// Carga la configuracion desde un archivo TOML. Si no existe, usa la
    /// configuracion por defecto (comportamiento de
    /// `sb_agent_core::config::load`, no algo que haya que replicar aqui).
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            warn!("Archivo de configuracion no encontrado: {:?}", path);
            info!("Usando configuracion por defecto");
        }

        let config: Config = sb_agent_core::config::load(path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let _ = sb_agent_core::config::sync_version_field(path, env!("CARGO_PKG_VERSION"));
        info!("Configuracion cargada desde: {:?}", path);
        Ok(config)
    }

    /// Inicializa el suscriptor de tracing segun la config (stdout/stderr)
    pub fn init_logging(&self) -> anyhow::Result<()> {
        let env_filter = self.build_env_filter();

        match self.logging.format.to_lowercase().as_str() {
            "json" => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .json()
                    .init();
            }
            "compact" => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .compact()
                    .init();
            }
            _ => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .pretty()
                    .init();
            }
        }

        Ok(())
    }

    /// Inicializa logging a archivo (para modo servicio Windows sin consola)
    #[cfg(windows)]
    pub fn init_logging_file(&self) -> anyhow::Result<()> {
        let env_filter = self.build_env_filter();
        let log_dir = r"C:\ProgramData\CupraFlow";
        std::fs::create_dir_all(log_dir)?;

        let file_appender = tracing_appender::rolling::daily(log_dir, "cupraflow.log");

        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(file_appender)
            .with_ansi(false)
            .init();

        Ok(())
    }

    /// Prioridad: `RUST_LOG` (si está definida) > `self.logging.level` > `"info"`.
    /// Antes ignoraba `logging.level` por completo — el nivel de `config.toml`
    /// se cargaba pero nunca se aplicaba a nada (mismo bug que se encontró y
    /// arregló en FerroSentry al retrofitear sobre sb-agent-core).
    fn build_env_filter(&self) -> tracing_subscriber::EnvFilter {
        let level = std::env::var("RUST_LOG").unwrap_or_else(|_| self.logging.level.clone());
        tracing_subscriber::EnvFilter::new(level)
    }
}
