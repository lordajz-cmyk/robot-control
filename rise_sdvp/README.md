# rise_sdvp: det som robot-control behöver

En kopia av de delar av [maprosystemsab/rise_sdvp](https://github.com/maprosystemsab/rise_sdvp)
som robotd bygger på, så att robot-control går att installera på en robot
**utan att klona rise_sdvp**. **rise_sdvp master är källan**; den här kopian
synkas därifrån med `scripts/synka_rise_sdvp.sh` (senast 2026-10-06, master `8cd01b2`).

| Mapp/fil | Vad | Används av |
|---|---|---|
| `Linux/Car_Client/` | Car_Client: länken mellan Pi:n och styrkortet (USB) som robotd pratar med på port 8300 | `install_car_client.sh` bygger den på Pi:n |
| `Embedded/RC_Controller/` | Styrkortets firmware (ChibiOS) för alla maskiner (`make robant/mactrac/drangen`, `BOARD=mp101` för Upwis MP101-kort) | `../flash_styrkort.sh` |
| `Linux/PI/udev/` | udev-regler: `/dev/vehicle` (styrkortet), `/dev/ublox` (GPS), ST-Link | `install_car_client.sh` |
| `install_car_client.sh` | Grundsystemet på Pi:n: paket, udev, Swepos-RTK (`car_rtk.service`), bygger Car_Client och slår på `car_client.service` | `../scripts/ny_robot.sh` eller för hand |
| `LICENSE` | GPLv3, gäller koden i den här mappen | |

Inte med: RControlStation (ersätts av robotstyrning), webbservern, gamla
färdigbyggda `.bin` för andra maskiner (en `.bin` raderar styrkortets
inställningar när den flashas, se `../flash_styrkort.sh`).

## Ändringar mot originalet

`install_car_client.sh` är rise_sdvp:s `install_pi.sh` med tre ändringar: den
hittar udev-reglerna i den här mappen, Car_Client:s `--setid` går att sätta med
`CAR_ID` (standard 4), och en rubrik som säger var den kommer ifrån.

## Hålla i synk

Ändringar i firmware och Car_Client görs i **rise_sdvp** (Pull Request till master),
aldrig bara här. Synka sedan kopian från robot-control-roten:

```bash
bash scripts/synka_rise_sdvp.sh --kolla   # visar skillnader mot GitHub, ändrar inget
bash scripts/synka_rise_sdvp.sh           # hämtar master och uppdaterar kopian
./flash_styrkort.sh --maskin robant --bara-bygg && git add rise_sdvp && git commit
```

Skriptet hämtar bara spårade filer, hoppar över byggskräp, `host_test/`, `precompiled/` och
den byggda `Car_Client`-binären. `Linux/PI` och `Linux/tools` är ett urval: bara filer som
redan finns här uppdateras.
