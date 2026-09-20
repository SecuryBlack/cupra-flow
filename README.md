# CupraFlow

Agente de gestión de red, alta disponibilidad y conmutación por error (VIP Failover) escrito en Rust para Linux y Windows.

[![Website](https://img.shields.io/badge/Website-cupraflow.dev-F97316?style=flat-square)](https://cupraflow.dev)
[![Ecosystem](https://img.shields.io/badge/Ecosystem-SecuryBlack-33E1BF?style=flat-square)](https://securyblack.com)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

> **Parte del ecosistema SecuryBlack:**
> [OxiPulse (Métricas)](https://github.com/securyblack/oxi-pulse) · [FerroSentry (Seguridad)](https://github.com/securyblack/ferro-sentry) · **CupraFlow (Alta Disponibilidad)** · [CromoForge (GitOps)](https://github.com/securyblack/cromo-forge) · [TitanVault (Backups)](https://github.com/securyblack/titan-vault) · [SecuryBlack Cloud](https://securyblack.com)

---

## 🚀 Capacidades Principales

- **IP Flotante Virtual (VIP Failover):** Implementación moderna estilo Keepalived/VRRP para migración de IP en caliente (< 500 ms) sin caída de servicio.
- **Malla Cifrada WireGuard:** Interconexión punto a punto de nodos y clusters híbridos (bare-metal + cloud VPS) con ChaCha20-Poly1305.
- **Health-Checks Inteligentes L4/L7:** Sondeo activo de puertos TCP y endpoints HTTP antes de enrutar tráfico o desencadenar failovers.
- **Consola TUI Standalone:** Monitorización en vivo del estado del cluster (nodos MASTER/BACKUP, latencias y peers) con interfaz de terminal interactiva.
- **Integración SecuryBlack Cloud:** Telemetría de red, túnel gRPC y reporte de eventos en tiempo real.

---

## 📋 Uso y Comandos

```bash
# Lanzar la interfaz interactiva de terminal (TUI)
cupraflow tui

# Ver estado de interfaces y nodos
cupraflow status

# Ver versión
cupraflow version
```

---

## 🌐 Ecosistema Open Source de SecuryBlack

CupraFlow es el pilar de red y alta disponibilidad dentro de la suite de agentes modulares de SecuryBlack:

| Agente | Enfoque Principal | Web Oficial | Repositorio |
| :--- | :--- | :--- | :--- |
| **OxiPulse** | Telemetría, métricas OTLP y logs sin overhead | [oxipulse.dev](https://oxipulse.dev) | [securyblack/oxi-pulse](https://github.com/securyblack/oxi-pulse) |
| **FerroSentry** | EDR ligero, auditd, detección de fuerza bruta y firewall | [ferrosentry.dev](https://ferrosentry.dev) | [securyblack/ferro-sentry](https://github.com/securyblack/ferro-sentry) |
| **CupraFlow** | Alta disponibilidad, IP flotante VIP y balanceo de tráfico | [cupraflow.dev](https://cupraflow.dev) | [securyblack/cupra-flow](https://github.com/securyblack/cupra-flow) |
| **CromoForge** | Despliegues continuos, GitOps y gestión de contenedores | [cromoforge.dev](https://cromoforge.dev) | [securyblack/cromo-forge](https://github.com/securyblack/cromo-forge) |
| **TitanVault** | Copias de seguridad en streaming y recuperación ante desastres | [titanvault.dev](https://titanvault.dev) | [securyblack/titan-vault](https://github.com/securyblack/titan-vault) |

Todos los agentes pueden gestionarse de forma centralizada y visual conectándolos a [SecuryBlack Cloud](https://securyblack.com).

---

## License

CupraFlow is licensed under the [Apache License, Version 2.0](LICENSE).