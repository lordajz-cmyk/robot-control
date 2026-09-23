#!/usr/bin/env bash
# ny_robot.sh — körs på DATORN. Installerar robotd på en robots Pi och slår på
# den som tjänst: frågar efter namn, WireGuard-IP m.m., skickar över koden,
# kör scripts/install_pi.sh och scripts/uppdatera_robotd.sh --aktivera där.
#
#   bash scripts/ny_robot.sh
#
# Förutsättningar på Pi:n: Raspberry Pi OS, WireGuard uppsatt (robotens fasta
# 192.168.200.x-adress) och rise_sdvp-installationen (Car_Client) — robotd
# bygger ovanpå Car_Client. Går att köra om; befintlig config skrivs bara
# över om du svarar ja (den gamla sparas som .bak).
#
# Senare uppdateringar av robotd: bash scripts/skicka_robotd.sh (med PI=...).

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

GREEN='\033[1;32m'; YELLOW='\033[1;33m'; RED='\033[1;31m'; NC='\033[0m'

fraga() { # fraga "Text" standardvärde -> svar i $SVAR
  local text="$1" std="${2:-}"
  if [ -n "$std" ]; then
    read -r -p "$text [$std]: " SVAR
    SVAR="${SVAR:-$std}"
  else
    SVAR=""
    while [ -z "$SVAR" ]; do read -r -p "$text: " SVAR; done
  fi
}

echo -e "${GREEN}=== Ny robot: installera robotd ===${NC}"
echo

fraga "Robotens namn (t.ex. drangen, robant)"
NAMN="$(echo "$SVAR" | tr '[:upper:]' '[:lower:]' | tr -cd 'a-z0-9_-')"
[ -n "$NAMN" ] || { echo -e "${RED}Ogiltigt namn.${NC}"; exit 1; }

while true; do
  fraga "Robotens WireGuard-IP (192.168.200.x)"
  IP="$SVAR"
  [[ "$IP" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] && break
  echo -e "${RED}Det ser inte ut som en IP-adress, försök igen.${NC}"
done

fraga "Användarnamn på Pi:n" "$NAMN"
ANV="$SVAR"
fraga "Bil-ID i Car_Client (--setid)" "4"
CAR_ID="$SVAR"
fraga "VESC-ID på roboten, kommaseparerade" "28,36,76"
VESC_IDS="$(echo "$SVAR" | tr -d ' ')"
fraga "Max vid fullt spakutslag (som Max i RControlStation)" "0.45"
DRIVE_MAX="$SVAR"
fraga "Slå på I2C för OLED-skärmen? Kräver omstart av Pi:n senare (j/n)" "n"
I2C="$SVAR"

PI="${ANV}@${IP}"
echo
echo -e "${GREEN}--- Sammanfattning ---${NC}"
echo "  Robot:      $NAMN"
echo "  Pi:         $PI"
echo "  robotd:     lyssnar på ${IP}:9000 (video ${IP}:9001)"
echo "  Bil-ID:     $CAR_ID"
echo "  VESC:       $VESC_IDS"
echo "  Max:        $DRIVE_MAX"
echo "  I2C/OLED:   $I2C"
echo "  Startar automatiskt vid uppstart, tar Car_Client bara medan någon kör"
echo "  med robotstyrning (RControlStation fungerar som vanligt annars)."
echo
read -r -p "Skriv ja för att installera: " OK
[ "$OK" = "ja" ] || { echo "Avbrutet, inget gjort."; exit 0; }

echo
echo -e "${GREEN}--- Kontrollerar anslutningen till $PI ---${NC}"
if ! ssh -o BatchMode=yes -o ConnectTimeout=8 "$PI" true 2>/dev/null; then
  echo -e "${YELLOW}Kan inte logga in utan lösenord.${NC}"
  read -r -p "Kopiera din SSH-nyckel till Pi:n (slipper lösenord för SSH framöver)? [J/n] " K
  if [ "${K:-j}" != "n" ]; then
    ssh-copy-id "$PI"
  fi
  ssh -o ConnectTimeout=8 "$PI" true || { echo -e "${RED}Når inte $PI — är WireGuard uppe på båda?${NC}"; exit 1; }
fi

if ! ssh "$PI" 'systemctl is-active --quiet car_client.service'; then
  echo -e "${YELLOW}Varning: car_client.service kör inte på Pi:n. robotd installeras ändå,"
  echo -e "men kan inte köra roboten förrän rise_sdvp-installationen (Car_Client) är på plats.${NC}"
fi

SKRIV_OVER=0
if ssh "$PI" 'test -f /etc/robotd/config.json'; then
  echo -e "${YELLOW}Pi:n har redan en /etc/robotd/config.json:${NC}"
  ssh "$PI" 'cat /etc/robotd/config.json'
  echo
  read -r -p "Ersätt den med värdena ovan? Den gamla sparas som .bak. [j/N] " E
  [ "${E:-n}" = "j" ] && SKRIV_OVER=1
fi

echo -e "${GREEN}--- Kopierar projektet ---${NC}"
rsync -a --exclude target --exclude .git --exclude osm_tiles --exclude '*.png' ./ "${PI}:robot-control/"

SKIPPA_I2C=1
[ "$I2C" = "j" ] && SKIPPA_I2C=0

echo -e "${GREEN}--- Installerar på Pi:n (frågar efter sudo-lösenordet) ---${NC}"
ssh -t "$PI" "cd robot-control && \
  ROBOT_ID='$NAMN' BIND_ADDR='${IP}:9000' CAR_ID='$CAR_ID' VESC_IDS='$VESC_IDS' \
  DRIVE_MAX='$DRIVE_MAX' SKRIV_OVER_CONFIG='$SKRIV_OVER' SKIPPA_I2C='$SKIPPA_I2C' \
  bash scripts/install_pi.sh && bash scripts/uppdatera_robotd.sh --aktivera"

echo
if ! grep -qE "^[0-9.]+\s+.*\b${NAMN}\b" /etc/hosts; then
  read -r -p "Lägg till '$NAMN' i datorns /etc/hosts, så kan du skriva namnet i robotstyrning? [J/n] " H
  if [ "${H:-j}" != "n" ]; then
    echo "$IP $NAMN" | sudo tee -a /etc/hosts > /dev/null
    echo "Tillagt: $IP $NAMN"
  fi
fi

echo
echo -e "${GREEN}=== Klart: $NAMN ===${NC}"
echo "Anslut med robotstyrning till: $NAMN  (eller $IP)"
echo "Logg på roboten:    ssh $PI journalctl -u robotd -f"
echo "Uppdatera senare:   PI=$PI bash scripts/skicka_robotd.sh"
