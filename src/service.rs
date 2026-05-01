use std::ffi::OsString;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{info, warn};

#[cfg(windows)]
use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
#[cfg(windows)]
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
#[cfg(windows)]
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
#[cfg(windows)]
use windows_service::{define_windows_service, service_dispatcher};

use crate::config::Config;

const SERVICE_NAME: &str = "CupraFlow";

/// Instala CupraFlow como servicio de Windows.
#[cfg(windows)]
pub fn install(config: &Config) -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(None::<&std::ffi::OsStr>, ServiceManagerAccess::CREATE_SERVICE).map_err(|e| {
        if let windows_service::Error::Winapi(ref io) = e {
            if io.raw_os_error() == Some(5) {
                return anyhow::anyhow!("Permiso denegado. Ejecuta como Administrador para instalar el servicio.");
            }
        }
        anyhow::anyhow!(e)
    })?;

    let executable_path = std::env::current_exe()?;

    let service_info = ServiceInfo {
        name: OsString::from(&config.service.name),
        display_name: OsString::from(&config.service.name),
        service_type: ServiceType::OWN_PROCESS,
        start_type: match config.service.startup.to_lowercase().as_str() {
            "auto" | "automatic" => ServiceStartType::AutoStart,
            "disabled" => ServiceStartType::Disabled,
            _ => ServiceStartType::OnDemand,
        },
        error_control: ServiceErrorControl::Normal,
        executable_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let service = manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(&config.service.description)?;

    info!("Servicio '{}' instalado correctamente", config.service.name);
    println!("[OK] Servicio '{}' instalado.", config.service.name);
    println!("     Inicio: {:?}", service_info.start_type);
    println!("     Ejecutable: {:?}", service_info.executable_path);

    Ok(())
}

/// Desinstala el servicio de Windows.
#[cfg(windows)]
pub fn uninstall(config: &Config) -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(None::<&std::ffi::OsStr>, ServiceManagerAccess::CONNECT).map_err(|e| {
        if let windows_service::Error::Winapi(ref io) = e {
            if io.raw_os_error() == Some(5) {
                return anyhow::anyhow!("Permiso denegado. Ejecuta como Administrador para desinstalar el servicio.");
            }
        }
        anyhow::anyhow!(e)
    })?;

    let service = match manager.open_service(&config.service.name, ServiceAccess::DELETE) {
        Ok(s) => s,
        Err(_) => {
            warn!("El servicio '{}' no existe", config.service.name);
            println!("[INFO] El servicio '{}' no esta instalado.", config.service.name);
            return Ok(());
        }
    };

    service.delete()?;
    info!("Servicio '{}' desinstalado correctamente", config.service.name);
    println!("[OK] Servicio '{}' desinstalado.", config.service.name);

    Ok(())
}

/// Inicia el servicio ya instalado via SCM.
#[cfg(windows)]
pub fn start_service(config: &Config) -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(None::<&std::ffi::OsStr>, ServiceManagerAccess::CONNECT).map_err(|e| {
        if let windows_service::Error::Winapi(ref io) = e {
            if io.raw_os_error() == Some(5) {
                return anyhow::anyhow!("Permiso denegado. Ejecuta como Administrador para iniciar el servicio.");
            }
        }
        anyhow::anyhow!(e)
    })?;
    let service = manager.open_service(&config.service.name, ServiceAccess::START)?;
    service.start(&[] as &[&std::ffi::OsStr])?;
    info!("Servicio '{}' iniciado via SCM", config.service.name);
    println!("[OK] Servicio '{}' iniciado.", config.service.name);
    Ok(())
}

/// Detiene el servicio via SCM.
#[cfg(windows)]
pub fn stop_service(config: &Config) -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(None::<&std::ffi::OsStr>, ServiceManagerAccess::CONNECT).map_err(|e| {
        if let windows_service::Error::Winapi(ref io) = e {
            if io.raw_os_error() == Some(5) {
                return anyhow::anyhow!("Permiso denegado. Ejecuta como Administrador para detener el servicio.");
            }
        }
        anyhow::anyhow!(e)
    })?;
    let service = manager.open_service(&config.service.name, ServiceAccess::STOP)?;
    service.stop()?;
    info!("Servicio '{}' detenido via SCM", config.service.name);
    println!("[OK] Servicio '{}' detenido.", config.service.name);
    Ok(())
}

/// Inicia el servicio como daemon Windows.
/// Debe ser llamada desde `main` cuando el binario es lanzado por el SCM.
#[cfg(windows)]
pub fn start() -> Result<(), windows_service::Error> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

#[cfg(windows)]
define_windows_service!(ffi_service_main, service_main_impl);

#[cfg(windows)]
fn service_main_impl(_arguments: Vec<OsString>) {
    // Buscar config en ubicaciones estandar:
    // 1. Directorio del ejecutable / config.toml
    // 2. Directorio del ejecutable / config / config.toml
    // 3. C:\ProgramData\CupraFlow\config.toml
    // 4. Config por defecto
    let config = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .and_then(|dir| {
            Config::from_file(dir.join("config.toml")).ok()
                .or_else(|| Config::from_file(dir.join("config").join("config.toml")).ok())
        })
        .or_else(|| Config::from_file(r"C:\ProgramData\CupraFlow\config.toml").ok())
        .unwrap_or_default();

    let _ = config.init_logging_file();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let shutdown_tx = std::sync::Mutex::new(Some(shutdown_tx));

    let status_handle = service_control_handler::register(
        SERVICE_NAME,
        move |control_event| match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                if let Ok(mut guard) = shutdown_tx.lock() {
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(());
                    }
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        },
    )
    .expect("failed to register service control handler");

    // Marcar como Running
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
        .expect("failed to set service status Running");

    info!("CupraFlow service running");

    // Bloquear hasta recibir señal de shutdown
    // Usamos un runtime tokio minimo solo para esperar el oneshot
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    rt.block_on(async {
        let _ = shutdown_rx.await;
        info!("Shutdown signal received, stopping service...");
    });

    // Marcar como Stopped
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    });

    info!("CupraFlow service stopped");
}

/// Stub para plataformas no-Windows (no-op).
#[cfg(not(windows))]
pub fn install(_config: &Config) -> anyhow::Result<()> {
    warn!("Instalacion de servicio solo disponible en Windows");
    Ok(())
}

#[cfg(not(windows))]
pub fn uninstall(_config: &Config) -> anyhow::Result<()> {
    warn!("Desinstalacion de servicio solo disponible en Windows");
    Ok(())
}

#[cfg(not(windows))]
pub fn start_service(_config: &Config) -> anyhow::Result<()> {
    warn!("Inicio de servicio solo disponible en Windows");
    Ok(())
}

#[cfg(not(windows))]
pub fn stop_service(_config: &Config) -> anyhow::Result<()> {
    warn!("Detencion de servicio solo disponible en Windows");
    Ok(())
}

#[cfg(not(windows))]
pub fn start() -> Result<(), String> {
    Err("Servicio Windows no disponible en esta plataforma".into())
}
