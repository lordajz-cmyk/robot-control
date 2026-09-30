#!/usr/bin/env bash
# uppdatera.sh — EN uppdatering av allt, körs på DATORN. Inga frågor utom lösenord.
#
#   cd ~/robot-control && bash scripts/uppdatera.sh
#   PI=anvandare@192.168.200.x bash scripts/uppdatera.sh    annan robot (sparas till nästa gång)
#
# Ordning (datorn först: RControlStation måste känna till styrkortets nya
# firmwareversion innan kortet flashas, annars kopplar den ner direkt):
#   1. git pull (robot-control), och skriptet startar om sig självt med den nya versionen
#   2. Robotstyrning (scripts/install_client.sh)
#   3. RControlStation, om den finns: git pull i rise_sdvp och ombyggnad
#      (databasen med dosans inställningar behålls)
#   4. Robotens Pi: koden skickas över och scripts/uppdatera_pi.sh körs där:
#      Car_Client, robotd och styrkortets firmware (bara om versionen är ny)
#
# Stäng Robotstyrning och RControlStation innan. Robotens inställningar, WireGuard-
# nycklar och styrkortets inställningar rörs inte.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ROOT="$PWD"
GREEN='\033[1;32m'; YELLOW='\033[1;33m'; RED='\033[1;31m'; NC='\033[0m'
steg() { echo -e "\n${GREEN}##### $* #####${NC}"; }
KONF="$HOME/.config/robot-control"
RESULTAT=()

if [ "${1:-}" != "--efter-pull" ]; then
  if pgrep -x RControlStation >/dev/null || pgrep -x robotstyrning >/dev/null; then
    echo -e "${RED}Stäng Robotstyrning och RControlStation först, och kör sedan samma kommando igen.${NC}"
    exit 1
  fi
  steg "1/4 Hämtar senaste versionen (git pull)"
  if ! git pull --ff-only; then
    echo -e "${RED}git pull gick inte. Har filer i ~/robot-control ändrats för hand?${NC}"
    echo "Visa: git status    Ångra ändringarna: git checkout -- .    Kör sedan igen."
    exit 1
  fi
  exec bash scripts/uppdatera.sh --efter-pull
fi

# --- 2. Robotstyrning --------------------------------------------------------------
steg "2/4 Robotstyrning"
if bash scripts/install_client.sh; then
  RESULTAT+=("Robotstyrning: uppdaterad")
else
  RESULTAT+=("Robotstyrning: FEL, se ovan")
fi

# --- 3. RControlStation --------------------------------------------------------------
steg "3/4 RControlStation"
RCS_REPO=""
# 1. Där programmet faktiskt ligger: install_dator.sh bygger i den klonade mappen
#    (var den än ligger) och länkar /usr/local/bin/RControlStation dit.
for l in "$(command -v RControlStation 2>/dev/null)" /usr/local/bin/RControlStation "$HOME/.local/bin/RControlStation"; do
  [ -n "$l" ] && [ -e "$l" ] || continue
  d="$(readlink -f "$l")"
  while [ -n "$d" ] && [ "$d" != "/" ]; do
    if [ -d "$d/Linux/RControlStation" ] && [ -d "$d/.git" ]; then RCS_REPO="$d"; break 2; fi
    d="$(dirname "$d")"
  done
done
# 2. Vanliga platser, och annars en rise_sdvp-klon någonstans i hemmappen.
if [ -z "$RCS_REPO" ]; then
  for c in "$HOME/rise_sdvp" "$HOME/RControllStation/rise_sdvp" "$HOME/RControlStation/rise_sdvp" \
           $(find "$HOME" -maxdepth 6 -type d -path "*/Linux/RControlStation" -not -path "*/.*" 2>/dev/null | sed 's|/Linux/RControlStation$||'); do
    if [ -d "$c/Linux/RControlStation" ] && [ -d "$c/.git" ]; then RCS_REPO="$c"; break; fi
  done
fi
if [ -z "$RCS_REPO" ]; then
  if command -v RControlStation >/dev/null 2>&1; then
    echo -e "${YELLOW}RControlStation finns ($(readlink -f "$(command -v RControlStation)")), men inte i en git-klon av rise_sdvp — kan inte uppdatera den.${NC}"
    echo "Installera om enligt steg 8 i guiden (git clone … ~/rise_sdvp, sudo bash install_dator.sh)."
    RESULTAT+=("RControlStation: FEL, hittar ingen git-klon att uppdatera")
  else
    echo "RControlStation är inte installerad (steg 8 i guiden) — hoppar över."
    RESULTAT+=("RControlStation: inte installerad")
  fi
elif ! git -C "$RCS_REPO" pull --ff-only; then
  echo -e "${RED}git pull i $RCS_REPO gick inte (ändrade filer?).${NC}"
  RESULTAT+=("RControlStation: FEL vid git pull")
else
  STATION="$RCS_REPO/Linux/RControlStation"
  BYGG="$STATION/build/cmake_linux"
  # Bygg vidare i den befintliga byggmappen: databasen (dosa-bindningar) bredvid
  # programmet ligger kvar. Finns ingen byggmapp: fullständigt bygge.
  if [ -f "$BYGG/CMakeCache.txt" ]; then
    LOGG="$(mktemp /tmp/rcontrolstation_bygge.XXXXXX.log)"
    cmake --build "$BYGG" -j"$(nproc)" > "$LOGG" 2>&1
    OK=$?
    grep -E "error|Built target RControlStation$" "$LOGG" | tail -5
    [ "$OK" -eq 0 ] && rm -f "$LOGG" || echo "Hela byggloggen: $LOGG"
  else
    (cd "$STATION" && ./build_cmake_linux.sh release)
    OK=$?
  fi
  if [ "$OK" -eq 0 ] && [ -x "$BYGG/build/lin/RControlStation" ]; then
    RESULTAT+=("RControlStation: uppdaterad")
  else
    RESULTAT+=("RControlStation: FEL vid bygget, se ovan")
  fi
fi

# --- 4. Robotens Pi och styrkortet ----------------------------------------------------
steg "4/4 Roboten (Pi:n och styrkortet)"
PI="${PI:-$(cat "$KONF/pi" 2>/dev/null || echo robant@192.168.200.10)}"
echo "Robot: $PI"
# En och samma ssh-anslutning för allt, så att lösenordet bara behövs en gång.
SOCK="$(mktemp -u /tmp/uppdatera_ssh.XXXXXX)"
# accept-new: första gången godkänns robotens nyckel utan yes/no-fråga (en ändrad
# nyckel stoppas fortfarande).
SSH=(ssh -o ConnectTimeout=15 -o StrictHostKeyChecking=accept-new -o ControlMaster=auto -o ControlPath="$SOCK" -o ControlPersist=600)
if ! "${SSH[@]}" -o ServerAliveInterval=15 "$PI" true; then
  echo -e "${RED}Når inte roboten ($PI). Är den påslagen och WireGuard uppe?${NC}"
  RESULTAT+=("Roboten: FEL, gick inte att nå")
else
  mkdir -p "$KONF" && echo "$PI" > "$KONF/pi"
  echo "--- Skickar koden till roboten ---"
  # Byggda filer skickas aldrig: en Car_Client eller firmware byggd på den här
  # datorn fungerar inte på Pi:n.
  if rsync -a -e "${SSH[*]}" \
       --exclude target --exclude .git --exclude osm_tiles --exclude '*.png' --exclude graphify-out \
       --exclude 'rise_sdvp/Linux/Car_Client/Car_Client' --exclude '*.o' --exclude 'moc_*' \
       --exclude 'qrc_*' --exclude 'ui_*.h' --exclude '.qmake.stash' \
       --exclude 'rise_sdvp/Linux/Car_Client/Makefile' \
       --exclude 'rise_sdvp/Embedded/RC_Controller/build' --exclude 'rise_sdvp/Embedded/RC_Controller/.dep' \
       ./ "$PI:robot-control/"; then
    if "${SSH[@]}" -t "$PI" "cd robot-control && bash scripts/uppdatera_pi.sh"; then
      RESULTAT+=("Roboten: Car_Client, robotd och styrkortet uppdaterade")
    else
      RESULTAT+=("Roboten: FEL, se ovan")
    fi
  else
    RESULTAT+=("Roboten: FEL när koden skulle skickas")
  fi
  "${SSH[@]}" -O exit "$PI" 2>/dev/null
fi

steg "Sammanfattning"
ALLT_OK=1
for r in "${RESULTAT[@]}"; do
  if [[ "$r" == *FEL* ]]; then echo -e "${RED}  $r${NC}"; ALLT_OK=0; else echo -e "  $r"; fi
done
if [ "$ALLT_OK" -eq 1 ]; then
  echo -e "\n${GREEN}Allt är uppdaterat. Starta Robotstyrning eller RControlStation som vanligt.${NC}"
else
  echo -e "\n${YELLOW}Något blev inte klart. Kör samma kommando igen; det som redan är klart går fort.${NC}"
  exit 1
fi
