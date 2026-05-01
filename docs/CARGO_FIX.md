# Solución al problema de Cargo en Windows

## Problema
Cargo falla con `getaddrinfo() thread failed to start` al intentar descargar dependencias desde crates.io.

## Causa probable
Bug conocido de libcurl/Cargo en Windows con resolución DNS (IPv6/IPv4).

## Soluciones

### Opción 1: Configurar Cargo para usar protocolo git (RECOMENDADA)
Crear archivo `.cargo/config.toml` en la raíz del proyecto:

```toml
[net]
git-fetch-with-cli = true
retry = 3
```

Luego ejecutar:
```bash
cargo build --release
```

### Opción 2: Forzar IPv4
```powershell
$env:CARGO_HTTP_TIMEOUT = "60"
cargo build --release
```

### Opción 3: Descargar manualmente (offline)
Si tienes otra máquina con internet:
1. En la máquina con internet, ejecutar: `cargo vendor`
2. Copiar la carpeta `vendor/` a tu proyecto
3. Crear `.cargo/config.toml`:
```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```

### Opción 4: Usar mirror alternativo
```toml
[registries.crates-io]
index = "sparse+https://index.crates.io/"
```

### Opción 5: Sin dependencias externas (fallback)
Si nada funciona, podemos reescribir `main.rs` sin dependencias externas usando solo la stdlib de Rust:
- CLI manual con `std::env::args()` en lugar de `clap`
- Sockets estándar en lugar de `tokio`
- Serde/toml se pueden omitir inicialmente

## Prueba rápida
Ejecuta en PowerShell:
```powershell
# Verificar que la red funciona
curl https://crates.io

# Verificar resolución DNS
nslookup crates.io
nslookup index.crates.io

# Probar con timeout mayor
$env:CARGO_NET_RETRY = "5"
$env:CARGO_HTTP_TIMEOUT = "120"
cargo build --release
```