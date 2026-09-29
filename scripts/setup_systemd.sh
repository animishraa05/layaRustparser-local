#!/usr/bin/env bash
# ==============================================================================
# scripts/setup_systemd.sh
# One-Click Systemd Service Installer for ULPF on AWS EC2 Ubuntu
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSTEMD_DIR="$SCRIPT_DIR/systemd"

echo "=========================================================="
echo "  ULPF Systemd Services Installer"
echo "=========================================================="

if [ "$EUID" -ne 0 ]; then
    echo "ERROR: Please run this script with sudo: sudo ./scripts/setup_systemd.sh"
    exit 1
fi

echo "==> Copying service definition files to /etc/systemd/system/..."
cp "$SYSTEMD_DIR/ulpf-backend.service" /etc/systemd/system/
cp "$SYSTEMD_DIR/ulpf-frontend.service" /etc/systemd/system/
cp "$SYSTEMD_DIR/ulpf-ingest.service" /etc/systemd/system/

echo "==> Reloading systemd daemon..."
systemctl daemon-reload

echo "==> Enabling services to start on boot..."
systemctl enable ulpf-backend.service
systemctl enable ulpf-frontend.service
systemctl enable ulpf-ingest.service

echo "==> Starting services now..."
systemctl restart ulpf-backend.service
systemctl restart ulpf-frontend.service
systemctl restart ulpf-ingest.service

echo "=========================================================="
echo "  ULPF Services Installed and Activated Successfully! ✓"
echo "  Check status:"
echo "    sudo systemctl status ulpf-backend"
echo "    sudo systemctl status ulpf-frontend"
echo "    sudo systemctl status ulpf-ingest"
echo "=========================================================="
