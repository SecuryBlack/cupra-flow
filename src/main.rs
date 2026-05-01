mod cli;
mod config;
mod service;

use anyhow::Result;
use clap::Parser;
use tracing::info;

use cli::{Cli, Commands};
use config::Config;

#[cfg(windows)]
fn main() {
    // En Windows, intentamos primero ejecutar como servicio (SCM).
    // Si el proceso NO fue lanzado por el SCM (error 1063), caemos a modo consola.
    match service::start() {
        Ok(_) => {
            // Modo servicio: nunca retorna hasta que el servicio se detiene
        }
        Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(1063) => {
            // ERROR_FAILED_SERVICE_CONTROLLER_CONNECT: no somos un servicio, modo consola
            if let Err(e) = run_console() {
                eprintln!("[cupraflow] error: {e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("[cupraflow] service error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(windows)]
fn run_console() -> Result<()> {
    let args = Cli::parse();
    let config = Config::from_file(&args.config)?;
    let _ = config.init_logging();

    info!(version = env!("CARGO_PKG_VERSION"), "CupraFlow modo consola");

    match args.command {
        Commands::Install => {
            info!("Instalando servicio CupraFlow...");
            service::install(&config)?;
        }
        Commands::Uninstall => {
            info!("Desinstalando servicio CupraFlow...");
            service::uninstall(&config)?;
        }
        Commands::Start => {
            info!("Iniciando servicio CupraFlow via SCM...");
            service::start_service(&config)?;
        }
        Commands::Stop => {
            info!("Deteniendo servicio CupraFlow via SCM...");
            service::stop_service(&config)?;
        }
        Commands::Status => {
            info!("Consultando estado del servicio...");
            println!("Estado del servicio CupraFlow:");
            println!("  Nombre:      {}", config.service.name);
            println!("  Descripcion: {}", config.service.description);
            println!("  Estado:      Usa 'sc query {}' para ver estado real", config.service.name);
            println!("  API:         {}:{}", config.server.bind_address, config.server.port);
        }
        Commands::Version => {
            println!("CupraFlow v{}", env!("CARGO_PKG_VERSION"));
            println!("{}", env!("CARGO_PKG_DESCRIPTION"));
            println!("Repositorio: {}", env!("CARGO_PKG_REPOSITORY"));
            println!("Soporte: Windows / Linux");
            println!("Stack: Rust + clap + serde + tracing + windows-service");
        }
        Commands::Check => {
            info!("Validando configuracion...");
            println!("Configuracion cargada correctamente:");
            println!(
                "  Server:      {}:{}",
                config.server.bind_address, config.server.port
            );
            println!(
                "  Logging:     nivel={}, formato={}",
                config.logging.level, config.logging.format
            );
            println!(
                "  Service:     nombre={}, startup={}",
                config.service.name, config.service.startup
            );
            println!("  LB enabled:  {}", config.loadbalancer.enabled);
            println!(
                "  Update:      canal={}, intervalo={}h",
                config.update.channel, config.update.check_interval
            );
            if config.loadbalancer.enabled {
                println!("  Backends:");
                for backend in &config.loadbalancer.backends {
                    println!(
                        "    - {} -> {} (weight: {})",
                        backend.name, backend.address, backend.weight
                    );
                }
            }
            info!("Configuracion validada correctamente");
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn main() -> Result<()> {
    let args = Cli::parse();
    let config = Config::from_file(&args.config)?;
    let _ = config.init_logging();

    info!(version = env!("CARGO_PKG_VERSION"), "CupraFlow iniciado");

    match args.command {
        Commands::Install => service::install(&config)?,
        Commands::Uninstall => service::uninstall(&config)?,
        Commands::Start => service::start_service(&config)?,
        Commands::Stop => service::stop_service(&config)?,
        Commands::Status => {
            println!("Estado del servicio CupraFlow:");
            println!("  Nombre:      {}", config.service.name);
            println!("  Descripcion: {}", config.service.description);
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
            println!("Configuracion cargada correctamente:");
            println!(
                "  Server:      {}:{}",
                config.server.bind_address, config.server.port
            );
            println!(
                "  Logging:     nivel={}, formato={}",
                config.logging.level, config.logging.format
            );
            println!(
                "  Service:     nombre={}, startup={}",
                config.service.name, config.service.startup
            );
            println!("  LB enabled:  {}", config.loadbalancer.enabled);
            if config.loadbalancer.enabled {
                println!("  Backends:");
                for backend in &config.loadbalancer.backends {
                    println!(
                        "    - {} -> {} (weight: {})",
                        backend.name, backend.address, backend.weight
                    );
                }
            }
        }
    }

    Ok(())
}
