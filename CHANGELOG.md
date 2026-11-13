# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
