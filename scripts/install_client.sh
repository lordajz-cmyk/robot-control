#!/usr/bin/env bash
# install_client.sh — installerar robotstyrning (klienten), relay-server och
# mock-robotd på en Ubuntu-dator. Körs PÅ den dator ni ska styra roboten
# från (eller på vilken Ubuntu-dator som helst ni vill testa/leka på).
#
#   bash scripts/install_client.sh
#
# Vad den gör:
#   1. Installerar systempaket klienten (egui/eframe) och GStreamer-video
#      (för senare, ännu inte ihopkopplat) behöver
#   2. Installerar Rust via rustup om det saknas
#   3. Bygger klienten (robotstyrning), relay-server och mock-robotd
#      (release-läge)
#   4. Lägger binärerna i ~/.local/bin (ingen sudo/root behövs för det här
#      skriptet, till skillnad från install_pi.sh)
#   5. Skapar en skrivbordsgenväg för klienten
#
# Efter installation, se README.md ("Testköra hela systemet utan hårdvara")
# för hur man kör klienten mot antingen mock-robotd (ingen hårdvara behövs)
# eller en riktig robot.

set -euo pipefail

echo "=== robot-control: installation på styrdatorn (Ubuntu) ==="

if [ ! -f "Cargo.toml" ] || [ ! -d "client" ]; then
  echo "Kör det här skriptet från roten av robot-control-projektet (där Cargo.toml ligger)." >&2
  exit 1
fi

echo "--- Installerar systempaket ---"
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  libx11-dev \
  libxkbcommon-dev \
  libxkbcommon-x11-0 \
  libwayland-dev \
  libgl1-mesa-dev \
  libxcb-render0-dev \
  libxcb-shape0-dev \
  libxcb-xfixes0-dev \
  libudev-dev \
  gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad \
  libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev \
  curl \
  git

echo "--- Kontrollerar Rust ---"
if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust hittades inte, installerar via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
else
  echo "Rust finns redan ($(cargo --version))."
fi

echo "--- Bygger klienten och mock-robotd (release) ---"
cargo build --release -p robotstyrning -p mock-robotd

echo "--- Installerar binärer till ~/.local/bin ---"
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/robotstyrning "$HOME/.local/bin/robotstyrning"
install -m 755 target/release/mock-robotd "$HOME/.local/bin/mock-robotd"

if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
  echo ""
  echo "OBS: ~/.local/bin ligger inte i din PATH än. Lägg till i din ~/.bashrc:"
  echo '  export PATH="$HOME/.local/bin:$PATH"'
fi

echo "--- Skapar skrivbordsgenväg ---"
DESKTOP_DIR="$HOME/.local/share/applications"
mkdir -p "$DESKTOP_DIR"
cat > "$DESKTOP_DIR/robotstyrning.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Robotstyrning
Comment=Styrprogram för roboten
Exec=$HOME/.local/bin/robotstyrning
Terminal=false
Categories=Utility;
EOF

cat <<'EOF'

=== Klart ===

Installerat:
  robotstyrning       (klienten — kör: robotstyrning)
  mock-robotd         (låtsas-roboten, för lokal test utan hårdvara)

Snabbtest utan någon hårdvara alls (två terminaler):
  Terminal 1: mock-robotd --bind 0.0.0.0:9000
  Terminal 2: robotstyrning
  (skriv "127.0.0.1" på anslutningsskärmen)

Över er faktiska WireGuard-tunnel: skriv robotens VPN-IP istället
(t.ex. "192.168.200.8") när robotd/mock-robotd kör där.

Se README.md i projektet för fullständiga instruktioner.
EOF
