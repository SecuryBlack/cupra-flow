# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-11-14

### Added
- **Peers** (contract in `sb-wiki/Tecnica/CupraFlow v2 - contrato.md`): `route_set` takes `peers`, the app's other servers. With peers, the route checks the local container (active check on `health.path`, plus passive), and when it is down the request goes over HTTPS to the first healthy peer, with the domain as SNI and Host. Forwarded requests carry `X-Sb-Peer: 1` and are only served locally, so there are no loops. A peer's `X-Forwarded-For` is trusted only from the peers' IPs.
- **Certificates with peers**: each server gets its own certificate by HTTP-01. A challenge this server does not know is forwarded to the peers on port 80; TLS-ALPN is off for those domains.

- **Only what is safe to repeat goes to a peer**: a request that never reached the local container (503, no healthy upstream) is forwarded with any method and its body intact; after a failed attempt (502/504) only GET and HEAD are forwarded, so a POST or DELETE the app may have processed is never run twice.

### Fixed
- **Caddy admin API**: a new connection per request, since every config change restarts the admin endpoint and dropped the reused connection.

Routes without `peers` produce exactly the same Caddy config as 0.2.

## [0.2.2] - 2026-11-13

### Changed
- No functional changes. Release to check that 0.2.1, with the new updater (`self_update` 1.3 in `sb-agent-core` 0.4.3), updates itself while running as a service.

## [0.2.1] - 2026-11-13

### Security
- **Dependencies**: `sb-agent-core` 0.4.3, which moves `self_update` to 1.3 and `zip` to 9.5 (GHSA-m665-v7pm-7grj, GHSA-q7v9-x93c-xqv3, GHSA-8hxf-qr8p-62x4), and `rustls` 0.23.46. Releases are still verified with zipsign before installing.

### Fixed
- **Installer**: the checksum check looks for the asset's own name, and a reinstall restarts the service so the new binary runs.

## [0.2.0] - 2026-11-13

### Added
- **Proxy**: CupraFlow owns SecuryBlack's Caddy (`sb-caddy`), moved from CromoForge 0.7.2 with the same names, so an existing Caddy and its routes are adopted as they are.
- **Orders**: `proxy_install`, `proxy_status`, `route_set` and `route_remove`, through Nexus (signed) and through a local socket for CromoForge (`/run/sb-agent/cupraflow-local-cmd.sock`).
- **Linux**: real systemd service, installer with `--token`, `scripts/uninstall.sh`, Linux release builds and daily self-update.

### Changed
- **Dependencies**: `sb-agent-core` 0.4.2 (signed orders).
- **Config**: `/etc/cupraflow/config.toml` only holds `token`; old sections are ignored.

### Removed
- The `clap` CLI (`install`, `start`, `stop`, `check`…) and the load-balancer config that was only printed. `--version`, `status` and `top` remain.

## [0.1.3] - 2026-08-23

### Changed
- **Dependencies**: Consume `sb-agent-core` from crates.io registry.

### Fixed
- **Installer**: Add TLS 1.2 enforcement, execution policy bypass, and static CRT for Windows Server 2019 compatibility.

## [0.1.2] - 2026-08-23

### Changed
- **Architecture**: Complete retrofit onto shared `sb-agent-core` runtime.
- **CLI**: Add `cupraflow top` and extend `cupraflow status` to query socket snapshot via TUI.

## [0.1.1] - 2026-07-21

### Added
- **Config**: Auto-write current version into `config.toml` upon agent startup if missing or outdated.

## [0.1.0] - 2026-05-01

### Added
- **Initial Release**: High-performance reverse proxy and load balancer agent in Rust.
- **Windows**: Windows service support and automated PowerShell installer.

[Unreleased]: https://github.com/SecuryBlack/cupra-flow/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/SecuryBlack/cupra-flow/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/SecuryBlack/cupra-flow/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/SecuryBlack/cupra-flow/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/SecuryBlack/cupra-flow/releases/tag/v0.1.0
