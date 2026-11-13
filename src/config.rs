//! Config de CupraFlow (`/etc/cupraflow/config.toml`): solo el token del
//! servidor, para verificar la firma de las órdenes. Los campos de 0.1.x
//! (`server`, `loadbalancer`, `vip`, `wireguard`…) se ignoran.

use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// Token de servidor SecuryBlack (el mismo que usan los demás agentes).
    #[serde(default)]
    pub token: Option<String>,
}
