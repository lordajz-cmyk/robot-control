# rise_sdvp: det som robot-control behöver

En kopia av de delar av [lordajz-cmyk/rise_sdvp](https://github.com/lordajz-cmyk/rise_sdvp)
som robotd bygger på, så att robot-control går att installera på en robot
**utan att klona rise_sdvp**. Kopierad från commit `1b55ca6` (2026-09-23),
alltså samma version som kör RobAnt.

| Mapp/fil | Vad | Används av |
|---|---|---|
| `Linux/Car_Client/` | Car_Client: länken mellan Pi:n och styrkortet (USB) som robotd pratar med på port 8300 | `install_car_client.sh` bygger den på Pi:n |
| `Embedded/RC_Controller/` | Styrkortets firmware (ChibiOS), med `CMD_GET_VESC_STATUS` och fixarna från 2026-09-22/23 | `../flash_styrkort.sh` |
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

Rättar man något i Car_Client eller firmware i rise_sdvp ska det kopieras hit
också (och tvärtom). Från robot-control-roten:

```bash
(cd ~/RControllStation/rise_sdvp && git archive HEAD Linux/Car_Client Embedded/RC_Controller) \
  | tar -x -C rise_sdvp --exclude='*.o' --exclude='Linux/Car_Client/Car_Client' \
      --exclude='Linux/Car_Client/logs' --exclude='Embedded/RC_Controller/precompiled'
git diff --stat rise_sdvp
```
