#!/usr/bin/env bash
# =============================================================================
# CupraFlow — Installer for Linux
# SecuryBlack High-Availability & Network VIP Failover Agent
# =============================================================================

set -euo pipefail

SB_AGENT_LABEL="cupraflow"
REPO="securyblack/cupra-flow"
BIN_NAME="cupraflow"
SERVICE_NAME="cupraflow"
SERVICE_DESC="CupraFlow High-Availability & Network VIP Failover Agent (SecuryBlack)"

# Download shared library from sb-agent-core
LIB_URL="https://raw.githubusercontent.com/securyblack/sb-agent-core/master/scripts/install-lib.sh"
LIB_TMP="$(mktemp)"
curl -fsSL "$LIB_URL" -o "$LIB_TMP" || { echo "ERROR: could not fetch install-lib.sh from sb-agent-core" >&2; exit 1; }
# shellcheck source=/dev/null
source "$LIB_TMP"
rm -f "$LIB_TMP"

sb_require_root
sb_require_cmds curl tar systemctl

TARGET="$(sb_detect_arch_linux)"
sb_info "Detected target architecture: ${TARGET}"

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
    VERSION="$(sb_fetch_latest_version "$REPO")"
fi
sb_info "Installing CupraFlow version: ${VERSION}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

ASSET_URL="https://github.com/${REPO}/releases/download/${VERSION}/${BIN_NAME}-${TARGET}.tar.gz"
sb_download_and_verify "$ASSET_URL" "$TMP_DIR/asset.tar.gz"
sb_install_binary "$TMP_DIR/asset.tar.gz" "$BIN_NAME" "/usr/local/bin"

# Write default configuration if not present
CONFIG_DIR="/etc/cupraflow"
mkdir -p "$CONFIG_DIR"
if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
    sb_info "Creating default configuration in $CONFIG_DIR/config.toml..."
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
[server]
port = 8080
bind_address = "0.0.0.0"

[logging]
level = "info"
format = "pretty"

[service]
name = "cupraflow"
description = "CupraFlow High-Availability Agent"

[vip]
enabled = false
interface = "eth0"
virtual_ip = "192.168.1.100/24"
vrid = 51
priority = 100
state = "BACKUP"

[wireguard]
enabled = false
interface = "wg0"
listen_port = 51820
EOF
    chmod 600 "$CONFIG_DIR/config.toml"
fi

# Install and start systemd service
sb_write_systemd_unit "$SERVICE_NAME" "$SERVICE_DESC" "/usr/local/bin/$BIN_NAME"
sb_enable_start_service "$SERVICE_NAME"

sb_success "CupraFlow has been successfully installed and started!"
sb_info "To view live cluster state: cupraflow tui"
sb_info "To check status: cupraflow status"
