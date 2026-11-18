# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.2] - 2026-11-18

### Added

- **Update from the panel** (`update_now`). The agent's «Update» button in SecuryBlack now works for CupraFlow: it runs the same signed-release check as the daily one, right away, and restarts into the new version. Only accepted through Nexus (signed), not on the local socket.

## [0.7.1] - 2026-11-18

### Fixed

- **Traffic metrics were empty since routes are wrapped in a `subroute`.** Caddy only measures the top-level handlers of each route (whatever runs inside a `subroute` is never measured), so a wrapped route is counted on its `subroute`. CupraFlow skipped `subroute` to avoid double counting and sent nothing for those domains. It now counts `subroute` as well; `vars` is still skipped.

## [0.7.0] - 2026-11-16

### Added

- **Behind another proxy** (`behind_proxy = "ip:port"` in `config.toml`). For servers where another proxy such as Traefik already owns ports 80 and 443: Caddy is installed listening in plain HTTP on that address, without opening 80 or 443 and without certificates, and the front proxy terminates TLS. Peer fallback still works (it goes out through the peer's 443); ACME challenge forwarding and per-route TLS policies are skipped. Set it before installing Caddy.

## [0.6.0] - 2026-11-15

### Added

- **Traffic per domain** for the panel. CupraFlow turns on Caddy's per-host metrics (`apps.http.metrics.per_host`, only for configured hosts), reads them every minute and sends what each domain did in that minute (requests, the latency histogram and responses by status class) over OTLP to the local Nexus agent, like OxiPulse does. The first reading after a start is only the baseline, and a Caddy restart is handled. Only the final handler of each request counts, so routes with paths or allowed IPs are not counted twice.

## [0.5.0] - 2026-11-15

### Added

- **`route_list`**: a read-only order for the panel's «Tráfico» section. It returns Caddy's state and image, every SecuryBlack route as it is live in Caddy (hosts, paths, upstreams, protocol, redirects, allowed IPs, TLS issuer and peers), and every certificate Caddy stores, with its issuer and expiry date. Certificates with no route left (old tests) show up too, so they can be cleaned up.

## [0.4.0] - 2026-11-15

### Added

- **Route options** for SecuryBlack's own services. All are optional, and a route without them produces exactly the same Caddy config as 0.3.
  - `paths`: match only these path prefixes on the hosts, without stripping them. Routes with paths go before host-only routes, so they win.
  - `allow_ips`: IPs or CIDR ranges allowed in, checked against the connecting socket's IP, never `X-Forwarded-For`. Everyone else gets a 403 and never reaches the app. Cannot be combined with `peers`.
  - `redirect_to`: a route with no upstream that answers 301 to another host, keeping the path and query.
  - `tls: "internal"`: certificates from Caddy's internal CA instead of ACME, for names behind Cloudflare in "Full" mode or names that are not public. Cannot be combined with `peers`, whose forwarding checks the peer's certificate against the domain.
  - `upstream_protocol: "h2c"`: HTTP/2 without TLS to the app (gRPC), flushed immediately and with no time limit on long streams.
- **Several routes per app**: a route id can be `<app>.<name>`. `route_remove` of `<app>` also removes its `<app>.<name>` routes. Ids ending in `-peers`, `-acme` or `-tls` are refused, since those name each route's extra objects.

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
