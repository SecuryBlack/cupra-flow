# CupraFlow

Rust agent under development for high availability and networking. The current code contains configuration, CLI and service scaffolding; the network/HA engine is not verified as operational. See the [workspace wiki](../sb-wiki/Producto/Estado%20actual.md).

## Use with SecuryBlack

SecuryBlack is a hosted panel for server metrics, security findings and alerts. This repository contains the native agent; the hosted panel is a separate part of the product.

CupraFlow remains under development. The demo introduces the broader SecuryBlack product; it does not demonstrate operational HA or networking support in this agent.

[Watch the 42-second product demo](https://securyblack.com/en?utm_source=github&utm_medium=referral&utm_campaign=agent_readme&utm_content=cupra-flow#how-it-works) · [Open the hosted panel](https://app.securyblack.com) · [Installation documentation](https://securyblack.com/en/docs/installation)

Follow the agent-specific installation and configuration instructions below. Use the hosted panel to obtain connection settings for your server.

[![Website](https://img.shields.io/badge/Website-cupraflow.dev-F97316?style=flat-square)](https://cupraflow.dev)
[![Ecosystem](https://img.shields.io/badge/Ecosystem-SecuryBlack-33E1BF?style=flat-square)](https://securyblack.com)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

> **Part of the SecuryBlack ecosystem:**
> [OxiPulse (Metrics)](https://github.com/securyblack/oxi-pulse) · [FerroSentry (Security)](https://github.com/securyblack/ferro-sentry) · **CupraFlow (Networking/HA development)** · [CromoForge (Docker/PostgreSQL)](https://github.com/securyblack/cromo-forge) · [TitanVault (Backups)](https://github.com/securyblack/titan-vault) · [SecuryBlack Cloud](https://securyblack.com)

---

## Planned capabilities (not verified as operational)

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
| **OxiPulse** | Server telemetry and OTLP metrics | [oxipulse.dev](https://oxipulse.dev) | [securyblack/oxi-pulse](https://github.com/securyblack/oxi-pulse) |
| **FerroSentry** | Lightweight EDR, auditd, brute-force mitigation & firewall | [ferrosentry.dev](https://ferrosentry.dev) | [securyblack/ferro-sentry](https://github.com/securyblack/ferro-sentry) |
| **CupraFlow** | Networking/HA development (not operationally verified) | [cupraflow.dev](https://cupraflow.dev) | [securyblack/cupra-flow](https://github.com/securyblack/cupra-flow) |
| **CromoForge** | Docker containers, logs & PostgreSQL management | [cromoforge.dev](https://cromoforge.dev) | [securyblack/cromo-forge](https://github.com/securyblack/cromo-forge) |
| **TitanVault** | Streaming backups (restore workflow not verified) | [titanvault.dev](https://titanvault.dev) | [securyblack/titan-vault](https://github.com/securyblack/titan-vault) |

All agents can be centrally managed with unified observability by connecting them to [SecuryBlack Cloud](https://securyblack.com).

---

## License

CupraFlow is licensed under the [Apache License, Version 2.0](LICENSE).
## Maintainer context (workspace)

English is the primary language for this repository's public documentation. Shared product decisions, commercial terms and the backlog live in the [workspace wiki](../sb-wiki/Índice.md), [current state](../sb-wiki/Producto/Estado%20actual.md) and [product log](../sb-wiki/Producto/Bitácora.md). These links require `sb-wiki` as a sibling checkout. Keep agent-specific usage and technical contracts here; update shared decisions in the wiki.
