use clap::{Parser, Subcommand};

/// CupraFlow - Agente de gestion de red y balanceo de carga
#[derive(Parser, Debug)]
#[command(
    name = "cupraflow",
    version = env!("CARGO_PKG_VERSION"),
    about = "Agente de gestion de red y balanceo de carga",
    long_about = None
)]
pub struct Cli {
    /// Ruta al archivo de configuracion
    #[arg(short, long, default_value = "config/config.toml")]
    pub config: String,

    /// Comando a ejecutar
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Instala CupraFlow como servicio de Windows
    Install,
    /// Desinstala el servicio de CupraFlow
    Uninstall,
    /// Inicia el servicio de CupraFlow
    Start,
    /// Detiene el servicio de CupraFlow
    Stop,
    /// Muestra el estado del servicio
    Status,
    /// Muestra informacion de version
    Version,
    /// Valida el archivo de configuracion
    Check,
}
