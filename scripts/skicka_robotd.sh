#!/usr/bin/env bash
# skicka_robotd.sh — körs på DATORN. Kopierar projektet till Pi:n och kör
# scripts/uppdatera_robotd.sh där (bygg, installera, starta om robotd.service).
#
#   bash scripts/skicka_robotd.sh                       RobAnt, bara uppdatera
#   bash scripts/skicka_robotd.sh --aktivera            RobAnt, och slå på autostart
#   bash scripts/skicka_robotd.sh --stang-av            RobAnt, stäng av tjänsten
#   PI=robant@192.168.200.10 bash scripts/skicka_robotd.sh ...   annan robot
#
# Frågar efter Pi:ns sudo-lösenord (en gång).

set -euo pipefail

PI="${PI:-robant@192.168.200.10}"
cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "--- Kopierar till ${PI}:robot-control ---"
rsync -a --exclude target --exclude .git --exclude osm_tiles --exclude '*.png' ./ "${PI}:robot-control/"

ssh -t "$PI" "cd robot-control && bash scripts/uppdatera_robotd.sh ${1:-}"
