# CupraFlow

Agente de gestion de red y balanceo de carga para Windows y Linux.

## Estado Actual

**Fase 1 - Hito 1.1: COMPLETADO**

Proyecto base creado y compilable. Modo offline (solo stdlib de Rust) debido a un bug temporal de Cargo en Windows.

## Requisitos

- Rust 1.75+ (instalado via rustup)
- Windows 10/11 o Windows Server 2019/2022 (fase actual)
- Linux (soporte planificado)

## Compilacion

```bash
cd cupraflow
cargo build --release
```

El ejecutable se generara en `target/release/cupraflow.exe`

## Uso

```bash
# Ver version
cupraflow version

# Ver ayuda
cupraflow help

# Comandos disponibles (simulados)
cupraflow install
cupraflow uninstall
cupraflow start
cupraflow stop
cupraflow status
```

## Roadmap

Ver `docs/CHECKLIST.md` para el roadmap completo por fases.

### Fases principales:
1. **Fase 1:** Instalador Windows + Servicio
2. **Fase 2:** Auto-update desde GitHub
3. **Fase 3:** Arquitectura modular
4. **Fase 4:** Load Balancer (Keepalived-style)
5. **Fase 5:** Web de test + Validacion
6. **Fase 6:** Produccion

## Problema conocido: Cargo en Windows

Si ves el error `getaddrinfo() thread failed to start`, ver `docs/CARGO_FIX.md`.

## Licencia

MIT