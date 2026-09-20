# CupraFlow

High-availability, network routing, and VIP failover agent written in pure Rust for Linux and Windows servers.

[![Website](https://img.shields.io/badge/Website-cupraflow.dev-F97316?style=flat-square)](https://cupraflow.dev)
[![Ecosystem](https://img.shields.io/badge/Ecosystem-SecuryBlack-33E1BF?style=flat-square)](https://securyblack.com)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

> **Part of the SecuryBlack ecosystem:**
> [OxiPulse (Metrics)](https://github.com/securyblack/oxi-pulse) · [FerroSentry (Security)](https://github.com/securyblack/ferro-sentry) · **CupraFlow (High Availability)** · [CromoForge (GitOps)](https://github.com/securyblack/cromo-forge) · [TitanVault (Backups)](https://github.com/securyblack/titan-vault) · [SecuryBlack Cloud](https://securyblack.com)

---

## 🚀 Core Capabilities

- **Floating Virtual IP (VIP Failover):** Modern Keepalived / VRRP v2/v3 implementation for seamless sub-second (< 500 ms) IP migration with zero downtime.
- **Encrypted WireGuard Mesh:** Point-to-point encrypted overlay network interconnecting hybrid bare-metal and multi-cloud servers with ChaCha20-Poly1305.
- **Intelligent L4 / L7 Health Checks:** Continuous active probing of TCP sockets and HTTP/gRPC endpoints before routing traffic or initiating failovers.
- **Standalone Interactive TUI:** Real-time terminal cockpit visualizing cluster status (MASTER / BACKUP state, VRRP heartbeats, and WireGuard peer latency).
- **SecuryBlack Cloud Integration:** Network telemetry streaming, gRPC secure tunnel, and instant failover event reporting.

---

## 📦 Quickstart & Installation

### Linux — One-line Install
```bash
curl -fsSL https://install.cupraflow.dev | sudo bash
```

### Windows — PowerShell (Administrator)
```powershell
irm https://install.cupraflow.dev | iex
```

---

## 📋 CLI & TUI Usage

```bash
# Launch interactive terminal UI
cupraflow tui

# Check network interfaces and cluster state
cupraflow status

# View version info
cupraflow version
```

---

## 🌐 SecuryBlack Open Source Ecosystem

CupraFlow is the networking and high-availability pillar of the SecuryBlack modular agent suite:

| Agent | Core Focus | Official Website | Repository |
| :--- | :--- | :--- | :--- |
| **OxiPulse** | Telemetry, OTLP metrics, and zero-overhead vital signs | [oxipulse.dev](https://oxipulse.dev) | [securyblack/oxi-pulse](https://github.com/securyblack/oxi-pulse) |
| **FerroSentry** | Lightweight EDR, auditd, brute-force mitigation & firewall | [ferrosentry.dev](https://ferrosentry.dev) | [securyblack/ferro-sentry](https://github.com/securyblack/ferro-sentry) |
| **CupraFlow** | High availability, floating VIP failover & traffic balancing | [cupraflow.dev](https://cupraflow.dev) | [securyblack/cupra-flow](https://github.com/securyblack/cupra-flow) |
| **CromoForge** | Continuous delivery, GitOps & container management | [cromoforge.dev](https://cromoforge.dev) | [securyblack/cromo-forge](https://github.com/securyblack/cromo-forge) |
| **TitanVault** | Zero-disk streaming backups & disaster recovery | [titanvault.dev](https://titanvault.dev) | [securyblack/titan-vault](https://github.com/securyblack/titan-vault) |

All agents can be centrally managed with unified observability by connecting them to [SecuryBlack Cloud](https://securyblack.com).

---

## License

CupraFlow is licensed under the [Apache License, Version 2.0](LICENSE).