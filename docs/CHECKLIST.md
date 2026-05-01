# CupraFlow - Checklist de Desarrollo

## NOTA IMPORTANTE
**Estado actual:** Cargo funciona correctamente. El bug de libcurl/Cargo en Windows fue resuelto.
**Stack activo:** clap, serde, toml, tracing, anyhow.

---

## Fase 1: Fundamentos & Instalador Windows
### Hito 1.1: Estructura base del proyecto [COMPLETADO]
- [x] Crear estructura de carpetas (`/src`, `/config`, `/web`, `/scripts`, `/docs`)
- [x] Elegir stack tecnologico: **Rust 1.75+**
- [x] Crear `Cargo.toml` con metadatos del proyecto
- [x] Crear archivo de configuracion base (`config/config.toml`)
- [x] Implementar sistema de logging con `tracing` + `tracing-subscriber`
- [x] Crear CLI con `clap` derive macros: `install`, `uninstall`, `start`, `stop`, `status`, `version`, `check`, `help`
- [x] Implementar parseo de configuracion con `serde` + `toml`
- [x] Compilar exitosamente en release (`cargo build --release`)
- [x] Probar ejecutable: `cupraflow.exe version` funciona
- [x] Inicializar repositorio Git
- [x] Crear `.gitignore` para Rust
- [x] Subir a GitHub (ver `docs/GITHUB_SETUP.md`)

### Hito 1.2: Servicio Windows [COMPLETADO]
- [x] Implementar wrapper como servicio Windows (usando crate `windows-service`)
- [x] Crear script de instalacion del servicio (`cupraflow install`)
- [x] Crear script de desinstalacion (`cupraflow uninstall`)
- [x] Manejar arranque automatico del servicio (AutoStart / OnDemand / Disabled)
- [x] Test: Instalar en Windows limpio y verificar que el servicio aparece en `services.msc` / `sc query`

**Notas:**
- El binario detecta automaticamente si fue lanzado por el SCM (modo servicio) o por consola.
- En modo servicio, los logs se escriben a `C:\ProgramData\CupraFlow\cupraflow.log`.
- El modo consola maneja `install`, `uninstall`, `start`, `stop`, `status`.
- Resuelto: inferencia de tipos con `windows-service 0.7` + `clap 4.6` requiere anotaciones explícitas (`None::<&OsStr>`).

### Hito 1.3: Instalador y Distribucion [COMPLETADO]
- [x] Elegir estrategia: script PowerShell + ZIP (estilo oxi-pulse)
- [x] Crear `scripts/install.ps1` (instalacion silenciosa y automatica)
- [x] Crear `.github/workflows/release.yml` (build + release automatico)
- [x] Incluir config por defecto en el paquete ZIP
- [x] Script maneja: descarga, checksum, registro de servicio, start, restart-on-failure
- [ ] Test: Instalar en VM Windows limpia via `irm ... | iex`
- [ ] Test: Desinstalar completamente sin residuos

---

## Fase 2: Sistema de Actualizacion (GitHub) [EN PROGRESO]
### Hito 2.1: Releases en GitHub
- [x] Repositorio GitHub configurado
- [x] GitHub Actions para builds automaticos (`.github/workflows/release.yml`)
- [x] Empaquetado ZIP con binario + config
- [x] Checksums SHA256 generados automaticamente
- [ ] Crear primera release (`v0.1.0`) y validar artefactos
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
### Actual
- **Runtime:** Rust 1.95+ (binario nativo)
- **CLI:** `clap` v4 (derive macros)
- **Config:** `serde` + `toml`
- **Logging:** `tracing` + `tracing-subscriber`
- **Servicio Windows:** `windows-service` crate (v0.7)
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
│   └── config.toml          # Configuracion de Cargo
├── .git/                     # Repo Git
├── config/
│   └── config.toml          # Configuracion del agente
├── docs/
│   ├── CHECKLIST.md         # Este archivo
│   └── CARGO_FIX.md         # Soluciones problema Cargo
├── scripts/
│   └── build.bat            # Script de build Windows
├── src/
│   ├── main.rs              # Punto de entrada
│   ├── cli.rs               # Definicion CLI con clap
│   └── config.rs            # Estructuras de config con serde
├── target/
│   └── release/
│       └── cupraflow.exe    # Binario compilado
├── Cargo.toml               # Config actual
├── Cargo.toml.full          # Config completa de referencia
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
- [x] Comando `check` carga y valida config.toml correctamente
- [x] CLI con clap responde a todos los comandos definidos
- [x] Logging con tracing funciona (formatos: pretty, compact, json)
- [x] Configuracion por defecto funciona si falta config.toml
- [x] Servicio Windows: codigo compila con `windows-service` v0.7
- [x] Comandos `install`, `uninstall`, `start`, `stop` implementados (requieren ejecucion como Administrador)
- [x] Modo servicio detecta SCM vs consola automaticamente (error 1063)
- [x] Servicio instala correctamente (AutoStart, descripcion, nombre)
- [x] Servicio inicia y detiene correctamente via SCM
- [x] Logs de servicio se escriben a `C:\ProgramData\CupraFlow\cupraflow.log.YYYY-MM-DD`
- [x] Servicio desinstala limpiamente sin residuos
- [x] Script `scripts/install.ps1` adaptado de oxi-pulse
- [x] Workflow `.github/workflows/release.yml` creado
- [x] Build local genera ZIP + SHA256 (`scripts/build.bat`)
- [x] Config se busca en multiples rutas: exe dir, exe dir/config, ProgramData

## Notas
- Cada hito debe ser probado independientemente antes de pasar al siguiente
- Versionar con Git tags en cada hito completado
- Mantener CHANGELOG.md actualizado
- Compilar en release con `cargo build --release`
- El binario release pesa aproximadamente ~3.5MB (con tokio + windows-service + tracing)