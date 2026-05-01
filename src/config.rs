use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Configuracion principal de CupraFlow
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
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
                github_repo: "sb-mcampoe/cupraflow".to_string(),
            },
        }
    }
}

impl Config {
    /// Carga la configuracion desde un archivo TOML
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            warn!("Archivo de configuracion no encontrado: {:?}", path);
            info!("Usando configuracion por defecto");
            return Ok(Config::default());
        }

        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        info!("Configuracion cargada desde: {:?}", path);
        Ok(config)
    }

    /// Inicializa el suscriptor de tracing segun la config (stdout/stderr)
    pub fn init_logging(&self) -> anyhow::Result<()> {
        let env_filter = Self::build_env_filter();

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
        let env_filter = Self::build_env_filter();
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

    fn build_env_filter() -> tracing_subscriber::EnvFilter {
        let level = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "info".to_string());
        tracing_subscriber::EnvFilter::new(level)
    }
}
