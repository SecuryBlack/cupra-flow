# CupraFlow - Checklist de Desarrollo

## NOTA IMPORTANTE
**Estado actual:** Proyecto compilable en modo OFFLINE (sin dependencias externas) debido a bug de Cargo/libcurl en Windows con resolucion DNS. 
**Solucion temporal:** Usando solo stdlib de Rust. Las dependencias (clap, tokio, serde) se anadiran cuando se resuelva el problema de red.
**Documentacion:** Ver `docs/CARGO_FIX.md` para soluciones del problema de red.

---

## Fase 1: Fundamentos & Instalador Windows
### Hito 1.1: Estructura base del proyecto [COMPLETADO]
- [x] Crear estructura de carpetas (`/src`, `/config`, `/web`, `/scripts`, `/docs`)
- [x] Elegir stack tecnologico: **Rust 1.75+**
- [x] Crear `Cargo.toml` con metadatos del proyecto (modo offline/stdlib)
- [x] Crear archivo de configuracion base (`config/config.toml`)
- [x] Implementar sistema de logging basico (println! + eprintln!)
- [x] Crear CLI basico con comandos: `install`, `uninstall`, `start`, `stop`, `status`, `version`, `help`
- [x] Compilar exitosamente en release (`cargo build --release`)
- [x] Probar ejecutable: `cupraflow.exe version` funciona
- [x] Inicializar repositorio Git
- [x] Crear `.gitignore` para Rust
- [x] Subir a GitHub (ver `docs/GITHUB_SETUP.md`)

**Pendiente:**
- [ ] Anadir dependencias reales cuando Cargo funcione (clap, tokio, serde, etc.)
- [ ] Reemplazar CLI manual por clap derive macros
- [ ] Implementar logging con tracing

### Hito 1.2: Servicio Windows [PENDIENTE]
- [ ] Implementar wrapper como servicio Windows (usando crate `windows-service`)
- [ ] Crear script de instalacion del servicio (`cupraflow install`)
- [ ] Crear script de desinstalacion (`cupraflow uninstall`)
- [ ] Manejar arranque automatico del servicio
- [ ] Test: Instalar en Windows limpio y verificar que el servicio aparece en `services.msc`

**Bloqueo:** Requiere dependencia `windows-service` (no disponible offline)

### Hito 1.3: Instalador MSI/EXE [PENDIENTE]
- [ ] Elegir herramienta (WiX Toolset / cargo-wix / Inno Setup)
- [ ] Crear script de build del instalador
- [ ] Incluir prerequisitos (VC++ Redistributable si es necesario)
- [ ] Crear flujo de instalacion silenciosa (`/S` o `/quiet`)
- [ ] Test: Instalar en VM Windows sin desarrollo previo
- [ ] Test: Desinstalar completamente sin residuos

---

## Fase 2: Sistema de Actualizacion (GitHub) [PENDIENTE]
### Hito 2.1: Releases en GitHub
- [ ] Crear repositorio GitHub publico/privado
- [ ] Configurar GitHub Actions para builds automaticos
- [ ] Crear estructura de versionado semantico (`v1.0.0`)
- [ ] Generar artefactos de release (binarios + assets)
- [ ] Crear `latest.json` o usar GitHub Releases API para versionado

### Hito 2.2: Auto-Updater
- [ ] Implementar chequeo de version contra GitHub Releases API
- [ ] Descargar nueva version desde GitHub
- [ ] Verificar checksum/firma de los binarios
- [ ] Implementar estrategia de update:
  - [ ] Descargar en paralelo (no bloquear servicio)
  - [ ] Detener servicio
  - [ ] Reemplazar binarios
  - [ ] Reiniciar servicio
- [ ] Rollback automatico si el update falla
- [ ] Soporte para canales: `stable`, `beta`, `dev`
- [ ] Test: Simular update de v1.0.0 a v1.0.1

**Bloqueo:** Requiere tokio + reqwest (no disponibles offline)

---

## Fase 3: Arquitectura & Configuracion [PENDIENTE]
### Hito 3.1: Diseno del Agente
- [ ] Definir arquitectura modular (plugins/modulos)
- [ ] Crear sistema de configuracion dinamica (hot-reload)
- [ ] Implementar API REST interna para control del agente
- [ ] Crear dashboard web basico (React/Vue o simple HTML)
- [ ] Implementar health checks y metricas

### Hito 3.2: Multi-plataforma
- [ ] Abstraer capa de sistema operativo
- [ ] Implementar adaptadores Windows/Linux
- [ ] Manejar permisos de administrador/root
- [ ] Test en Windows Server 2019/2022
- [ ] Test en Ubuntu 22.04 / Debian 12

---

## Fase 4: Load Balancer (Core) [PENDIENTE]
### Hito 4.1: L4 Load Balancer
- [ ] Implementar proxy TCP/UDP basico
- [ ] Algoritmos de balanceo:
  - [ ] Round Robin
  - [ ] Least Connections
  - [ ] IP Hash
- [ ] Health checks de backends (TCP/HTTP)
- [ ] Configuracion de upstreams via archivo/API
- [ ] Estadisticas de trafico

### Hito 4.2: L7 Load Balancer (HTTP/HTTPS)
- [ ] Soporte HTTP/1.1 y HTTP/2
- [ ] SSL/TLS termination
- [ ] Routing basado en:
  - [ ] Host/ dominio
  - [ ] Path
  - [ ] Headers
- [ ] WebSocket proxy
- [ ] Rate limiting basico

### Hito 4.3: Alta Disponibilidad (Keepalived-style)
- [ ] Implementar VRRP o similar para failover
- [ ] Soporte VIP (Virtual IP) flotante
- [ ] Deteccion de nodo maestro/esclavo
- [ ] Migracion automatica de VIP
- [ ] Test: Matar nodo maestro y verificar failover < 5s

---

## Fase 5: Web de Test & Validacion [PENDIENTE]
### Hito 5.1: Aplicacion de Test
- [ ] Crear web sencilla (HTML+JS o SPA ligera)
- [ ] Mostrar informacion del servidor que responde (hostname, IP, timestamp)
- [ ] Contador de requests por instancia
- [ ] Health endpoint `/health`
- [ ] Dockerfile para desplegar multiples instancias

### Hito 5.2: Escenarios de Prueba
- [ ] Test 1: 2 backends, round robin, verificar distribucion
- [ ] Test 2: Matar un backend, verificar que deja de recibir trafico
- [ ] Test 3: Restaurar backend, verificar que vuelve a pool
- [ ] Test 4: Stress test con `ab` o `wrk` (1000 req/s)
- [ ] Test 5: Failover de VIP entre 2 nodos CupraFlow

---

## Fase 6: Extras & Produccion [PENDIENTE]
- [ ] Documentacion de instalacion y configuracion
- [ ] CLI interactivo (`cupraflow config` con wizard)
- [ ] Integracion con Let's Encrypt para SSL auto
- [ ] Metricas Prometheus / Grafana
- [ ] Logs estructurados (JSON)
- [ ] Sistema de plugins para extensibilidad

---

## Stack Tecnologico
### Actual (modo offline)
- **Runtime:** Rust 1.95+ (solo stdlib)
- **CLI:** Manual con `std::env::args()`
- **Config:** Archivos TOML/JSON parseados manualmente
- **Logging:** `println!` / `eprintln!`
- **Servicio Windows:** Pendiente (requiere `windows-service` crate)
- **Instalador:** Pendiente (requiere `cargo-wix`)

### Objetivo (cuando Cargo funcione)
- **Runtime:** Rust 1.75+ (binario nativo)
- **CLI:** `clap` v4 (derive macros)
- **Config:** `serde` + `toml`
- **Logging:** `tracing` + `tracing-subscriber`
- **Servicio Windows:** `windows-service` crate
- **Instalador:** `cargo-wix` para MSI
- **Updater:** `reqwest` + `tokio` para async HTTP, `semver` para versionado
- **LB:** `tokio` + `hyper` o implementacion manual con `tokio::net`
- **Web Test:** `tokio` + `axum` o `warp`
- **HA/VRRP:** Implementacion manual con sockets raw

## Archivos del proyecto
```
cupraflow/
├── .cargo/
│   └── config.toml          # Configuracion de Cargo (fix red)
├── .git/                     # Repo Git
├── config/
│   └── config.toml          # Configuracion del agente
├── docs/
│   ├── CHECKLIST.md         # Este archivo
│   └── CARGO_FIX.md         # Soluciones problema Cargo
├── scripts/
│   └── build.bat            # Script de build Windows
├── src/
│   └── main.rs              # Codigo fuente (solo stdlib)
├── target/
│   └── release/
│       └── cupraflow.exe    # Binario compilado
├── Cargo.toml               # Config actual (sin deps)
├── Cargo.toml.full          # Config completa (con deps)
└── README.md                # Pendiente crear
```

## Como resolver el problema de red
1. Ver `docs/CARGO_FIX.md` para opciones detalladas
2. Soluciones principales:
   - Usar `cargo vendor` en otra maquina y copiar
   - Configurar proxy/mirror alternativo
   - Resolver problema DNS de Windows
3. Cuando funcione, reemplazar `Cargo.toml` por `Cargo.toml.full`

## Pruebas realizadas
- [x] Compilacion exitosa en Windows: `cargo build --release`
- [x] Ejecutable generado: `target/release/cupraflow.exe`
- [x] Comando `version` funciona correctamente
- [x] Comando `help` funciona correctamente
- [x] CLI manual responde a todos los comandos definidos

## Notas
- Cada hito debe ser probado independientemente antes de pasar al siguiente
- Versionar con Git tags en cada hito completado
- Mantener CHANGELOG.md actualizado
- Compilar en release con `cargo build --release`
- El binario actual pesa aproximadamente ~200KB (solo stdlib)