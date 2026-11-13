#!/usr/bin/env bash
# CupraFlow — Installer for Linux
# Usage: curl -fsSL https://install.cupraflow.dev | sudo bash -s -- --token <TOKEN> [--version vX.Y.Z]
#
# CupraFlow owns SecuryBlack's Caddy (sb-caddy) on this server. It needs
# Docker; the proxy itself is installed later from the panel (proxy_install).
set -euo pipefail

SB_AGENT_LABEL="cupraflow"
LIB_URL="https://raw.githubusercontent.com/securyblack/sb-agent-core/master/scripts/install-lib.sh"
LIB_TMP="$(mktemp)"
curl -fsSL "$LIB_URL" -o "$LIB_TMP" || { echo "ERROR: could not fetch install-lib.sh from sb-agent-core" >&2; exit 1; }
# shellcheck source=/dev/null
source "$LIB_TMP"
rm -f "$LIB_TMP"

REPO="securyblack/cupra-flow"
BIN_NAME="cupraflow"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/cupraflow"
CONFIG_FILE="${CONFIG_DIR}/config.toml"

TOKEN=""
VERSION=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --token)   TOKEN="$2";   shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    *) sb_die "Unknown argument: $1" ;;
  esac
done

sb_require_root
sb_require_cmds curl tar systemctl
command -v docker &>/dev/null || sb_warn "Docker not found. CupraFlow runs Caddy in a Docker container."

TARGET="$(sb_detect_arch_linux)"
[[ -z "$VERSION" ]] && VERSION="$(sb_fetch_latest_version "$REPO")"
sb_info "Installing CupraFlow ${VERSION} (${TARGET})"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
# The .sha256 names the asset, so the download keeps that name.
ASSET_NAME="${BIN_NAME}-${TARGET}.tar.gz"
ASSET_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"
sb_download_and_verify "$ASSET_URL" "$TMP_DIR/$ASSET_NAME"
sb_install_binary "$TMP_DIR/$ASSET_NAME" "$BIN_NAME" "$INSTALL_DIR"

mkdir -p "$CONFIG_DIR"
chmod 700 "$CONFIG_DIR"
# Keep an existing token on reinstall; 0.1.x configs have none and get rewritten.
if [[ -z "$TOKEN" && -f "$CONFIG_FILE" ]]; then
  TOKEN="$(sed -n 's/^token *= *"\(.*\)"$/\1/p' "$CONFIG_FILE" | head -n1)"
fi
if [[ -z "$TOKEN" ]]; then
  read -rsp "  Auth token: " TOKEN </dev/tty
  echo ""
fi
[[ -z "$TOKEN" ]] && sb_die "Token cannot be empty"

cat > "$CONFIG_FILE" <<EOF
# CupraFlow configuration
# Do not share this file — it contains your auth token.
token = "${TOKEN}"
EOF
chmod 600 "$CONFIG_FILE"

sb_write_systemd_unit "cupraflow" "CupraFlow traffic agent (SecuryBlack)" "${INSTALL_DIR}/${BIN_NAME}" "$CONFIG_DIR"
sb_enable_start_service "cupraflow"
# enable --now does not restart a running service: on reinstall it would keep the old binary.
systemctl restart cupraflow

sb_success "CupraFlow ${VERSION} installed and running"
sb_info "Status: cupraflow status   Logs: journalctl -fu cupraflow"
