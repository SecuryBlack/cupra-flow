#!/usr/bin/env bash
# CupraFlow — Uninstaller for Linux. Leaves sb-caddy and its routes running:
# CromoForge talks to Caddy directly again once CupraFlow's socket is gone.
set -euo pipefail
[[ $EUID -eq 0 ]] || { echo "Run as root" >&2; exit 1; }

systemctl disable --now cupraflow 2>/dev/null || true
rm -f /etc/systemd/system/cupraflow.service /usr/local/bin/cupraflow
# /run is tmpfs, but until the next reboot a stale socket would make CromoForge
# think CupraFlow is still the owner of the routes.
rm -f /run/sb-agent/cupraflow-cmd.sock /run/sb-agent/cupraflow-local-cmd.sock
systemctl daemon-reload
echo "CupraFlow removed. Config kept in /etc/cupraflow (delete it by hand if you want)."
