# Referenskopior från lordajz-cmyk/rise_sdvp

Det HÄR repot (`robot-control`) bygger `robotd` OVANPÅ Car_Client, ersätter
den inte (se PROJECT_SPEC.md §13). Filerna i den här mappen är kopior av
installationsskripten från den ANDRA, separata forken
(`lordajz-cmyk/rise_sdvp`) som faktiskt bygger/startar Car_Client, sparade
här som referens/dokumentation.

**OBS:** `start_car.sh` och `install_pi.sh` är **rättade** här (2026-09-20)
— originalen hade `sleep 10`, ändrat till `sleep 90` (se PROJECT_SPEC.md
§19 för bakgrunden: 10 sekunder matchade inte de 90 som faktiskt löste
startproblemen med RControlStation tidigare). Om du vill använda fixen på
riktigt:

- **På en maskin som redan kört gamla `install_pi.sh`:** redigera
  `~/start_car.sh` direkt, byt `sleep 10` mot `sleep 90`, kör
  `sudo systemctl restart car_client.service` (eller vänta till nästa
  omstart).
- **För en ny installation:** kopiera den här mappens `install_pi.sh`
  (med fixen) till din `rise_sdvp`-mapp innan du kör den, ELLER kör
  originalet och gör samma redigering av `~/start_car.sh` efteråt.
- Vill du hellre committa fixen permanent i `lordajz-cmyk/rise_sdvp`: det
  är en enda rad att ändra i själva repot, på GitHub eller lokalt.

De körs inte av något i det här repot (`robot-control`) — bara sparade
här så framtida sessioner inte behöver fråga om samma filer igen.

Se PROJECT_SPEC.md §19 för vad som i övrigt bekräftades genom att läsa
dessa (portar, device-sökvägar, tjänstenamn).

- `install_pi.sh` — bygger Car_Client, sätter upp udev, Swepos RTK,
  autostart (`car_client.service`, `car_rtk.service`)
- `install_allt.sh` — kör ovanstående + flashar CarController-kortets
  firmware, i ett kommando (`sudo ./install_allt.sh`)
- `start_car.sh` — skriptet `car_client.service` faktiskt startar; skapar
  screen-sessionen `car` som kör själva Car_Client-binären
