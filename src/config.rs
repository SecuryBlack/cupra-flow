//! Config de CupraFlow (`/etc/cupraflow/config.toml`): el token del
//! servidor, para verificar la firma de las órdenes, y si Caddy va detrás de
//! otro proxy. Los campos de 0.1.x (`server`, `loadbalancer`, `vip`,
//! `wireguard`…) se ignoran.

use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// Token de servidor SecuryBlack (el mismo que usan los demás agentes).
    #[serde(default)]
    pub token: Option<String>,
    /// `ip:puerto` donde escucha Caddy en HTTP plano cuando otro proxy
    /// (Traefik) ocupa el 80 y el 443 y le pasa el tráfico. Sin
    /// certificados: el TLS lo pone el de delante. Hay que fijarlo antes de
    /// `proxy_install`.
    #[serde(default)]
    pub behind_proxy: Option<std::net::SocketAddr>,
}
