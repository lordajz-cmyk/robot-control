#!/bin/bash

# ==============================================================================
# 🚀 install_allt.sh: Helautomatisk installation för hela robot-systemet
# ==============================================================================
# Detta skript är en samlings-installerare som kör både Pi-konfigurationen
# (install_pi.sh) och styrkortsflashningen (flash_styrkort.sh).
# ==============================================================================

GREEN='\e[32m'
RED='\e[31m'
YELLOW='\e[33m'
BLUE='\e[34m'
BOLD='\e[1m'
NC='\e[0m' # No Color

# Säkerställ att skriptet körs som root (sudo) eftersom install_pi.sh behöver det
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}${BOLD}Fel:${NC} Detta skript måste köras med sudo! Kör: ${BOLD}sudo ./install_allt.sh${NC}"
  exit 1
fi

REAL_USER=${SUDO_USER:-$USER}
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# ------------------------------------------------------------------------------
# 📥 Kontrollera och hämta källkod (git clone)
# ------------------------------------------------------------------------------
if [ -d "$DIR/Linux/Car_Client" ] && [ -d "$DIR/Embedded/RC_Controller" ]; then
  # Skriptet körs inifrån repot
  REPO_ROOT="$DIR"
elif [ -d "$DIR/rise_sdvp/Linux/Car_Client" ] && [ -d "$DIR/rise_sdvp/Embedded/RC_Controller" ]; then
  # Skriptet körs utanför repot (standard för rcontrollstation-mappen)
  REPO_ROOT="$DIR/rise_sdvp"
elif [ -d "$DIR/rise_sdvp/rise_sdvp/Linux/Car_Client" ] && [ -d "$DIR/rise_sdvp/rise_sdvp/Embedded/RC_Controller" ]; then
  # Skriptet körs utanför repot med dubbla nästlade rise_sdvp-mappar
  REPO_ROOT="$DIR/rise_sdvp/rise_sdvp"
else
  # Källkoden saknas – vi försöker klona den!
  echo -e "${YELLOW}Källkodsmappen 'rise_sdvp' saknas på: $DIR${NC}"
  echo -e "Klónar källkoden från GitHub (https://github.com/lordajz-cmyk/rise_sdvp.git)..."
  
  if [ "$EUID" -eq 0 ]; then
    sudo -u "$REAL_USER" git clone --recursive https://github.com/lordajz-cmyk/rise_sdvp.git "$DIR/rise_sdvp"
  else
    git clone --recursive https://github.com/lordajz-cmyk/rise_sdvp.git "$DIR/rise_sdvp"
  fi

  if [ $? -eq 0 ] && [ -d "$DIR/rise_sdvp/Linux/Car_Client" ]; then
    echo -e "${GREEN}✅ Källkoden klonades framgångsrikt!${NC}\n"
    REPO_ROOT="$DIR/rise_sdvp"
  else
    echo -e "${RED}❌ Misslyckades med att hämta källkoden. Kontrollera din internetanslutning eller länk.${NC}"
    exit 1
  fi
fi

echo -e "${BLUE}${BOLD}======================================================================${NC}"
echo -e "${BLUE}${BOLD}   🛰️  HEL-INSTALLATION: RASPBERRY PI + FLASHNING AV STYRKORT  🛰️${NC}"
echo -e "${BLUE}${BOLD}======================================================================${NC}"
echo -e "Detta skript kommer att köra båda delarna av installationen sekventiellt.\n"

# Sök efter install_pi.sh och flash_styrkort.sh
if [ -f "$DIR/install_pi.sh" ]; then
  INSTALL_PI_PATH="$DIR/install_pi.sh"
elif [ -f "$REPO_ROOT/install_pi.sh" ]; then
  INSTALL_PI_PATH="$REPO_ROOT/install_pi.sh"
else
  INSTALL_PI_PATH="$REPO_ROOT/install_pi.sh" # Fallback
fi

if [ -f "$DIR/flash_styrkort.sh" ]; then
  FLASH_STYRKORT_PATH="$DIR/flash_styrkort.sh"
elif [ -f "$REPO_ROOT/flash_styrkort.sh" ]; then
  FLASH_STYRKORT_PATH="$REPO_ROOT/flash_styrkort.sh"
else
  FLASH_STYRKORT_PATH="$REPO_ROOT/flash_styrkort.sh" # Fallback
fi

# Gör underskripten körbara utifall de inte är det
chmod +x "$INSTALL_PI_PATH"
chmod +x "$FLASH_STYRKORT_PATH"

# ------------------------------------------------------------------------------
# Del 1: Konfigurera Raspberry Pi
# ------------------------------------------------------------------------------
echo -e "${YELLOW}${BOLD}--- DEL 1: Konfigurerar Raspberry Pi 4 ---${NC}"
"$INSTALL_PI_PATH"

if [ $? -ne 0 ]; then
  echo -e "\n${RED}❌ Fel inträffade under konfigurationen av Raspberry Pi. Avbryter.${NC}"
  exit 1
fi

# ------------------------------------------------------------------------------
# Del 2: Kompilera och flasha styrkortet (STM32)
# ------------------------------------------------------------------------------
echo -e "\n${YELLOW}${BOLD}--- DEL 2: Kompilerar och flashar Carcontroller-kortet ---${NC}"
# Vi kör flash_styrkort.sh som den vanliga användaren (men skriptet kommer använda sudo för OpenOCD vid behov)
sudo -u "$REAL_USER" "$FLASH_STYRKORT_PATH"

if [ $? -ne 0 ]; then
  echo -e "\n${RED}❌ Fel inträffade under flashningen av styrsystemskortet.${NC}"
  exit 1
fi

echo -e "\n${GREEN}${BOLD}======================================================================${NC}"
echo -e "${GREEN}${BOLD}🎉 ALLT ÄR NU HELT KLART! HELA SYSTEMET ÄR UPPE OCH SNURRAR! 🎉${NC}"
echo -e "${GREEN}${BOLD}======================================================================${NC}"
echo -e "Både din Raspberry Pi 4 och ditt Carcontroller-kort (STM32) är färdigkonfigurerade."
echo -e "Koppla in båda USB-kablarna mellan korten, placera roboten utomhus och kör!"
echo -e ""
echo -e "💡 Användbara kommandon:"
echo -e "  - Se styrsystemets konsol (live):      ${BOLD}screen -r car${NC}"
echo -e "  - Kontrollera Swepos-status:           ${BOLD}sudo systemctl status car_rtk.service${NC}"
echo -e "  - Se Swepos-dataström live:            ${BOLD}journalctl -u car_rtk -f${NC}"
echo -e "======================================================================"
