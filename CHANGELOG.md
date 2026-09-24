# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
