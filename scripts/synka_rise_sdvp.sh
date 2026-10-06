#!/usr/bin/env bash
# synka_rise_sdvp.sh — håller robot-controls kopia av rise_sdvp (rise_sdvp/) i fas med
# maprosystemsab/rise_sdvp master, som är den enda källan för firmware och Car_Client.
#
#   bash scripts/synka_rise_sdvp.sh            hämta master och uppdatera kopian
#   bash scripts/synka_rise_sdvp.sh --kolla    visa bara skillnaderna, ändra inget (kod 1 = olika)
#   bash scripts/synka_rise_sdvp.sh --kalla DIR  använd en lokal rise_sdvp-klon i stället för GitHub
#
# Vad som synkas:
#   Embedded/RC_Controller  hela källkoden, utan byggskräp, host_test/ och precompiled/
#   Linux/Car_Client        hela källkoden, utan den byggda Car_Client-binären
#   Linux/PI, Linux/tools   bara filerna som redan finns i kopian (den är ett urval)
# README.md, LICENSE och install_car_client.sh hör till robot-control och rörs inte.
#
# Efteråt: bygg firmwaren (./flash_styrkort.sh --maskin robant --bara-bygg) och checka in.

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

KOLLA=0
KALLA=""
while [ $# -gt 0 ]; do
  case "$1" in
    --kolla) KOLLA=1; shift ;;
    --kalla) KALLA="$2"; shift 2 ;;
    -h|--hjalp|--help) sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "Okänt argument: $1" >&2; exit 2 ;;
  esac
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [ -z "$KALLA" ]; then
  echo "Hämtar maprosystemsab/rise_sdvp master ..."
  git clone -q --depth 1 https://github.com/maprosystemsab/rise_sdvp.git "$TMP/rise_sdvp"
  KALLA="$TMP/rise_sdvp"
fi
[ -d "$KALLA/Embedded/RC_Controller" ] || { echo "Hittar inte rise_sdvp i $KALLA" >&2; exit 2; }
echo "Källa: $KALLA ($(git -C "$KALLA" log -1 --format='%h %s' 2>/dev/null | cut -c1-70))"

# Bara spårade filer från källan, så att lokalt byggskräp i en klon inte följer med.
EXPORT="$TMP/export"
mkdir -p "$EXPORT"
git -C "$KALLA" archive HEAD Embedded/RC_Controller Linux/Car_Client Linux/PI Linux/tools | tar -x -C "$EXPORT"

SKRAP=(--exclude '*.o' --exclude '*.bak*' --exclude '*.backup' --exclude 'fw_*.bin'
       --exclude 'host_test/' --exclude 'precompiled/' --exclude 'build/' --exclude '.dep/'
       --exclude '.qmake.stash' --exclude '__pycache__/' --exclude '* (copy)*')
RSYNC=(rsync -rl --checksum "${SKRAP[@]}")   # bara innehåll, inte tidsstämplar
[ "$KOLLA" -eq 1 ] && RSYNC+=(--dry-run --itemize-changes) || RSYNC+=(--itemize-changes)

AND=$(mktemp -p "$TMP")
"${RSYNC[@]}" --delete "$EXPORT/Embedded/RC_Controller/" rise_sdvp/Embedded/RC_Controller/ >> "$AND"
"${RSYNC[@]}" --delete --exclude 'Car_Client' "$EXPORT/Linux/Car_Client/" rise_sdvp/Linux/Car_Client/ >> "$AND"
for d in Linux/PI Linux/tools; do
  "${RSYNC[@]}" --existing "$EXPORT/$d/" "rise_sdvp/$d/" >> "$AND"
done

ANDRADE=$(grep -v '^\.d\|^cd' "$AND" | grep -c . || true)
if [ "$ANDRADE" -eq 0 ]; then
  echo "Kopian är i fas med källan."
  exit 0
fi
grep -v '^\.d\|^cd' "$AND" | sed 's/^/  /'
if [ "$KOLLA" -eq 1 ]; then
  echo "$ANDRADE filer skiljer (inget ändrat). Kör utan --kolla för att synka."
  exit 1
fi
echo "$ANDRADE filer uppdaterade. Bygg firmwaren och checka in:"
echo "  ./flash_styrkort.sh --maskin robant --bara-bygg && git add rise_sdvp && git commit"
