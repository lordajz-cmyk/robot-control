#!/usr/bin/env bash
# deploy_to_pi.sh — kopierar robot-control-projektet till Raspberry Pi:n och
# kör install_pi.sh där, över SSH. Körs på ER egen dator (inte på Pi:n).
#
# Användning:
#   bash scripts/deploy_to_pi.sh pi@raspberrypi.local
#   bash scripts/deploy_to_pi.sh robant@192.168.200.10
#
# Kräver att ni redan kan SSH:a till Pi:n (samma sätt ni loggat in innan för
# att köra lsusb/dmesg-kommandona).

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Användning: $0 <användare@pi-adress>" >&2
  echo "Exempel:    $0 pi@raspberrypi.local" >&2
  exit 1
fi

PI_HOST="$1"
REMOTE_DIR="robot-control"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

if [ ! -f "Cargo.toml" ]; then
  echo "Hittar inte Cargo.toml — kör det här från robot-control-projektets rot." >&2
  exit 1
fi

echo "--- Kopierar projektet till ${PI_HOST}:${REMOTE_DIR} ---"
if command -v rsync >/dev/null 2>&1; then
  rsync -av --exclude target --exclude .git ./ "${PI_HOST}:${REMOTE_DIR}/"
else
  echo "rsync hittades inte, använder scp istället (långsammare, ingen exclude-filtrering)."
  ssh "${PI_HOST}" "mkdir -p ${REMOTE_DIR}"
  scp -r ./* "${PI_HOST}:${REMOTE_DIR}/"
fi

echo "--- Kör install_pi.sh på Pi:n ---"
ssh -t "${PI_HOST}" "cd ${REMOTE_DIR} && bash scripts/install_pi.sh"

echo "--- Klart. Anslut och testa: ---"
echo "  ssh ${PI_HOST}"
echo "  robotd"
