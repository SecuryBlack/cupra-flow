mod cli;
mod config;

use anyhow::Result;
use clap::Parser;
use tracing::{info, warn};

use cli::{Cli, Commands};
use config::Config;

fn main() -> Result<()> {
    let args = Cli::parse();

    // Cargar configuracion primero para poder usar el logging configurado
    let config = Config::from_file(&args.config)?;

    // Inicializar tracing/logging
    // Nota: init_logging solo se puede llamar una vez por proceso
    // Si ya esta inicializado (ej. en tests), ignoramos el error
    let _ = config.init_logging();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "CupraFlow iniciado"
    );

    match args.command {
        Commands::Install => {
            info!("Instalando servicio CupraFlow...");
            warn!("Implementacion pendiente: servicio Windows (Hito 1.2)");
            info!(
                service_name = %config.service.name,
                startup = %config.service.startup,
                "Servicio instalado (simulado)"
            );
        }
        Commands::Uninstall => {
            info!("Desinstalando servicio CupraFlow...");
            warn!("Implementacion pendiente: servicio Windows (Hito 1.2)");
            info!("Servicio desinstalado (simulado)");
        }
        Commands::Start => {
            info!("Iniciando servicio CupraFlow...");
            warn!("Implementacion pendiente: servicio Windows (Hito 1.2)");
            info!(
                bind = %config.server.bind_address,
                port = config.server.port,
                "Servicio iniciado (simulado)"
            );
        }
        Commands::Stop => {
            info!("Deteniendo servicio CupraFlow...");
            warn!("Implementacion pendiente: servicio Windows (Hito 1.2)");
            info!("Servicio detenido (simulado)");
        }
        Commands::Status => {
            info!("Consultando estado del servicio...");
            println!("Estado del servicio CupraFlow:");
            println!("  Nombre:      {}", config.service.name);
            println!("  Descripcion: {}", config.service.description);
            println!("  Estado:      No instalado (simulado)");
            println!("  PID:         N/A");
            println!("  Uptime:      N/A");
            println!("  API:         {}:{}", config.server.bind_address, config.server.port);
        }
        Commands::Version => {
            println!("CupraFlow v{}", env!("CARGO_PKG_VERSION"));
            println!("{}", env!("CARGO_PKG_DESCRIPTION"));
            println!("Repositorio: {}", env!("CARGO_PKG_REPOSITORY"));
            println!("Soporte: Windows / Linux");
            println!("Stack: Rust + clap + serde + tracing");
        }
        Commands::Check => {
            info!("Validando configuracion...");
            println!("Configuracion cargada correctamente:");
            println!("  Server:      {}:{}", config.server.bind_address, config.server.port);
            println!("  Logging:     nivel={}, formato={}", config.logging.level, config.logging.format);
            println!("  Service:     nombre={}, startup={}", config.service.name, config.service.startup);
            println!("  LB enabled:  {}", config.loadbalancer.enabled);
            println!("  Update:      canal={}, intervalo={}h", config.update.channel, config.update.check_interval);
            if config.loadbalancer.enabled {
                println!("  Backends:");
                for backend in &config.loadbalancer.backends {
                    println!("    - {} -> {} (weight: {})", backend.name, backend.address, backend.weight);
                }
            }
            info!("Configuracion validada correctamente");
        }
    }

    Ok(())
}
