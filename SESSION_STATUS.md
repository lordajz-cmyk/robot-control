# Var vi slutade — 2026-09-23, eftermiddag: VÅRT EGET PROGRAM KÖR ROBOTEN 🎉

Läs den här först. Installation: se **installationsguide.md**.

## Läget
Klient (robotstyrning) → WireGuard → robotd → Car_Client → styrkort → VESC fungerar på
upphissade RobAnt, verifierat av användaren ("DET FUNKAR!", max 0,45 känns som RControlStation).
- **robotd kör som tjänst** på RobAnt (`robotd.service` enabled/active, /usr/local/bin/robotd,
  installerad 13:20 med `scripts/skicka_robotd.sh --aktivera`).
- **robotd tar Car_Client bara medan en klient är ansluten** och släpper den efteråt —
  bekräftat i journalen 13:28:15 (tog) / 13:29:00 (släppte). RControlStation fungerar alltså
  som vanligt när ingen kör med robotstyrning. Car_Client tar bara EN TCP-klient
  (`tcpserversimple.cpp:139`), därför är det så.
- Status i klienten (ljusgrön text på mörk botten): fart, batteri V/%, VESC x/y svarar, temp,
  fel. Kommer från `CMD_GET_STATE` + `CMD_GET_VESC_STATUS` var 500 ms.
- AKTIVERA kräver på riktigt att alla `known_vesc_ids` svarar (+ kamera i CAM-läge) och
  visar orsaken under knappen.
- Max-reglage i klienten (0,05–1,00, sparas i ~/.config/robotstyrning/max.json), skickas som
  `ControlCommand.max_output`; robotd begränsar till `drive.max_cap` (standard 1,0).
- Spakar som RControlStation: vänster upp/ner = gas, höger sidled = styrning.
- **Styrkortets aktuatorer ställs in från robotstyrning** (⚙ → Styrkortets aktuatorer, ersätter
  Confcommon Read/Write). robotd läser MAIN_CONFIG (78), byter bara aktuatorbytena, skriver (77)
  och kontrolläser (`robotd/src/board_config.rs`). **Verifierat på RobAnt 13:53:** läsning och
  skrivning OK, kontrolläst. Styrkortet nu: antal 4; 28/36 Fart, 76 Styrning, rad 4 = VESC 0
  Nödstopp (tidigare VESC 8; ingen sådan VESC finns, ofarlig rest — kan sättas till antal 3).
- 🔒 Lås-knapp (och Esc) i klienten slår av AKTIVERA.
- **Uppstart verifierad 2026-09-23 ~15:35, även efter strömavbrott (huvudströmbrytaren):**
  WireGuard, Car_Client, RTK och robotd startar av sig själva, VESC 3/3, AKTIVERA OK.
  RControlStation fungerar också efteråt (bekräftat av användaren) — de delar Car_Client som tänkt.
  Två fel hittades och rättades på vägen:
  1. robotd startade inte vid uppstart: `After=car_client.service` + car_client:s
     `After=multi-user.target` gav en ordningscirkel, systemd strök robotds start.
     Rättat i robotd.service (uppdatera_robotd.sh, install_pi.sh).
  2. Styrkortet svarade inte efter omstart: Car_Client öppnade `/dev/vehicle` en gång till
     som RTCM-serieport (`inputRtcm = true` i main.cpp) och läste bort styrkortets svar,
     slumpmässigt beroende på uppstarten. Standard nu `false` (som i Vedders original), i
     rise_sdvp (commit ffcab51, pushad 2026-09-24 via SSH-adressen; origin är https),
     robot-control/rise_sdvp och på RobAnt (ombyggd 15:29).

## Hur robotd styr
`CMD_RC_CONTROL_ADV` (125): `[bil-ID][125][aktivitet][värde*1e4 i32 BE]`, aktivitet 10 = fart,
11 = styrning; styrkortet väljer VESC via sina aktuatorer (Confcommon). Omsändning var 100 ms
vid körning, 0 direkt vid stopp, 0 var 1 s i vila. Watchdog 150/400 ms, ramper, AKTIVERA krävs.

## Repot är fristående (2026-09-23)
- `rise_sdvp/`: Car_Client, styrkortets firmware (RC_Controller) och udev-regler kopierade från
  rise_sdvp commit 1b55ca6 + `install_car_client.sh` (rise_sdvp:s install_pi.sh anpassad).
  Provbyggt: firmware `make robant` och Car_Client (qmake6) OK. Håll i synk, se rise_sdvp/README.md.
- `wireguard/`: wireguard.sh, wireguard_admin.sh och guiderna.
- `reference/rise_sdvp` borttagen (inaktuella kopior).
- `scripts/ny_robot.sh` (ny robot, frågar namn/IP/…), `skicka_robotd.sh` (uppdatera),
  `uppdatera_robotd.sh` (körs på Pi:n). `ny_robot.sh` är INTE körd mot en riktig ny Pi än.

## Rutiner
- Starta robotd för hand bara med `ssh -t` (annars blir den kvar). Nu när tjänsten kör: stoppa
  tjänsten först, annars krockar port 9000.
- sudo på Pi:n kräver lösenord → användaren kör sådant i en egen terminal (inte `!`).
- Claude Code blockerar SSH-skrivningar mot Pi:n; läsning (journalctl, cat, ss) går.

## Installationsskripten för kunder (2026-09-24)
Kunden kör två skript: `robot-control/scripts/install_client.sh` (Robotstyrning) och
`rise_sdvp/install_dator.sh` (RControlStation, `sudo`). Båda testade från ren klon från GitHub:
bygger utan fel. Genvägar: `Robotstyrning`/`robotstyrning` (~/.local/bin) och `RControlStation`
(/usr/local/bin). Se `installationsguide_kund.md` (steg 1–8).
- **Provinstallerat från noll i tomma behållare (Podman), som kundguiden steg 2, 4 och 8:
  Ubuntu 22.04 OCH 24.04 — allt OK**, inga paket saknades, genvägarna fungerar i ny terminal.
  Testskriptet: användare `kund` med sudo, `git clone` från GitHub, `install_client.sh`,
  `sudo bash install_dator.sh`. Kör igen före utskick: `bash scripts/test_kundinstallation.sh [24.04]`. (Kräver `--network host`;
  inte testat: fönster/dosa, eftersom behållaren saknar skärm.)
- **Rättat i RControlStation (rise_sdvp, pushat):** en NY databas fick kontroll-id 1–6, så dosan
  skickade aktivitet 4/5 i stället för 10/11 och inget rörde sig. Nu fasta id 7–12 och
  standardbindningar 5→10 (vänster upp/ner = Speed), 8→11 (höger sidled = Steering). Påverkar
  bara nya databaser; testat med första start utan databas. `data.db` ligger (med rätta) inte i
  git — den innehåller användarens fält/rutter.
- RControlStation sparar dosa-inställningar efter varje rad vid inläsning och raderar tillfälligt
  de andra ("Cleared controller …" i loggen). Slutresultatet blir rätt; gammalt beteende, orört.

## RControlStation hos kunder — beslut 2026-09-24
- RControlStation hämtar gårdar, fält, rutter och maskiner från **webbservern på VPN-servern**
  (`http://192.168.200.1:8080`, koden i `rise_sdvp/web/`), inte från lokala `data.db`. Kunder
  ser därför samma exempel som användaren direkt när VPN:et är uppe. Ingen exempeldatabas behövs.
- **Användarens beslut:** att det fungerar direkt går före sekretess. Säkerhetsfrågorna nedan är
  kända och **medvetet lämnade som de är** — ta inte upp dem igen om inte användaren gör det:
  NTRIP-lösenord (Swepos m.fl.) följer med i `/all_farms`, alla kunder ser samma data och kan
  ändra (t.ex. `/remove_machine`), och WireGuard-klienter når troligen hela 192.168.200.0/24.
- **Före utskick:** lägg in kundens robot (namn + IP) i maskinlistan på servern, så finns den i
  RControlStation. Fyll i robotens namn i rutan i `installationsguide_kund.md`.
- Max i RControlStation är nu 0,35 som standard (var 0,15), pushat i rise_sdvp.
- **Kartan visar rätt direkt vid anslutning (rise_sdvp ed0b30d, verifierat på RobAnt 2026-09-24):**
  vid varje anslutning sätts nollpunkten (karta + styrkort) på robotens första GPS-position från
  rtkrcv (även SPP), kartan centreras ~100 m och Follow för bilen slås på. Tidigare låg styrkortets
  nollpunkt i Uppsala och roboten ritades mil fel (östled ~dubbelt) om man inte valde gård.
  Gårdsval gäller till nästa anslutning. Kartan startar på senaste position (QSettings
  RControlStation/karta). Användarens val: nollpunkt vid VARJE anslutning (sparade rutter kan
  hamna några m fel om roboten startar på annat ställe — välj gården först för sparade rutter).
- Kundguiden steg 8 beskriver anslutningen: markera roboten i listan → knappen längst till
  vänster (Connect to selected machine); annars IP i textrutan + knappen "Text".

## GPS/RTK-kedjan på RobAnt (undersökt 2026-09-24, inget ändrat)
Kedja: u-blox (`/dev/ublox`) → Car_Client → TCP 8210 (UBX) → rtkrcv (`screen rtklib`,
`Linux/PI/rtkrcv_arm/rover_ublox.conf`) + Swepos via `car_rtk.service` (str2str → TCP 1234)
→ NMEA på TCP 2948 → RControlStation ("Lösning"). robotd/robotstyrning använder inte GPS.
- **"Ingen GPS-data" inomhus efter strömavbrott = kallstart, inget fel.** u-bloxen glömmer tid
  och banor utan ström; RAWX hade 0 mätningar och GPS-vecka 0. Inne blev det SPP först efter
  en natt. Ute: RTK direkt.
- **Mätt ute 2026-09-24:** 27 satelliter (GPS 11, GLONASS 8, BeiDou 5, Galileo 3), 43 mätningar,
  C/N0 medel 36 (15–49) dB-Hz, 5 Hz. rtkrcv: RTK FLOAT/FIX växlande, **bara 7 satelliter
  används**, HDOP 1,0, korrektionsålder 1 s.
- **Idé till senare (stabilare FIX nära hus/träd):** (1) Swepos-strömmen saknar BeiDou —
  `str2str -msg 1005,1074,1084,1094,1230` har inte 1124, så BeiDou kan inte användas för RTK;
  lägg till 1124 om Swepos-mountpointen har det. (2) Se över elevations-/SNR-mask m.m. i
  `rover_ublox.conf` som sållar bort de flesta satelliterna.
- Så lyssnar man utan att störa: `timeout 4 bash -c "exec 3<>/dev/tcp/127.0.0.1/8210; cat <&3"`
  (UBX, tål flera lyssnare) och samma mot 2948 (NMEA från rtkrcv). **Inte** mot 8300
  (Car_Client tar bara en klient där).

## Kvar
- Firmware skriver "Activity %d -> %d actuator(s)" per kommando (commands.c) — tysta.
- `gps_fix` i status är alltid false (ingen NMEA-tolkning, medvetet, se PROJECT_SPEC §10).
- Settings-vyn: när den visas skickas inga styrkommandon (draw_driving_screen körs inte) →
  watchdog nollar. Ofarligt men värt att veta.
- `ny_robot.sh` första riktiga körning på en ny Pi — läs utskriften noga.
- Inget pushat till GitHub (robot-control) — bara lokala commits.

---

# Var vi slutade — 2026-09-23, förmiddag: ROBOTEN KÖR! 🎉

Läs den här först nästa gång. (Gårdagens anteckningar ligger kvar längre ner.)

## Läget
Dosan styr RobAnt fullt ut: fram, bak, höger och vänster (verifierat av användaren,
roboten upphissad). Statusdata, ping, batteri (52,6 V) och GPS med RTK fungerar.

## Vad som var fel och vad som fixades 2026-09-23
1. **Datorn frös när RControlStation öppnades.** Orsak: varje TCP-fel öppnade en ny
   modal QMessageBox → ~2000 felrutor på 2 min när Car_Client startades om.
   Fix: en icke-modal felruta i taget (`mainwindow.cpp`, `static QPointer<QMessageBox>`).
2. **Styrkortets CAN nådde aldrig ut** (därför svarade VESC-skanningen aldrig och hjulen
   stod still). Orsak: kortet matades bara via ST-Link/USB — strömplinten (GND / 7–60 V,
   TPS54561) var tom, så CAN-transceivern TJA1051 saknade 5 V. Diagnostik visade
   TEC→240, bus-off, LEC=5. **Fix (hårdvara):** kortet kopplat till 48 V-plinten.
   OBS: ingen bygel på EN_BAT_VIN (J6), den stänger av regulatorn.
3. **Styrning bara åt ett håll** — kablage på VESC 76, fixat av användaren.
4. **Varje flashning raderade styrkortets inställningar** (aktuatorer m.m.): .bin täckte
   EEPROM-emuleringen 0x08004000–0x0800BFFF. Fix: `flash_styrkort.sh` flashar ELF.
   (Samma fel finns i Gunnars `upload_fw*`.)
5. **Dosa-filtret (Gemini)** skickade aldrig 0 vid släpp och inget när spaken hölls
   stilla. Fix: dödzon + alltid 0, omsändning per action var 100 ms, nödstopp (0 till
   allt) om dosan försvinner. `JSconnected()` läckte SDL-handtag — fixat.
6. **Firmware:** servotråden (`servo_vesc.c`) bytte global `vesc_id` utan lås — låst nu.
   Diagnostikrad "VESC out … CAN TEC … status: duty/rpm/I" max 2 ggr/s per VESC.
7. **Car_Client:** återansluter u-bloxen när den får nytt tty-namn (efter flash/USB-omräkning).
8. **RControlStation:** styrkortets printf loggas till stdout ("FW printf"); segfault i
   SDL vid avslut fixad.

Allt är byggt, kopierat till Pi:n, flashat och Car_Client omstartad (10:32).
Säkerhetskopior: `*.bak_20260923*` bredvid varje ändrad fil.

## Kvar / att tänka på
- Inget är incheckat i git (varken rise_sdvp eller robot-control) — fråga användaren.
- Dosa-bindningar: vänster spak båda axlar → Speed, höger båda → Steering. Bättre:
  bara vänster upp/ned = fart, höger vänster/höger = styrning.
- Robot-ID: webbservern 192.168.200.1:8080 ger nu ID 4. Geminis "id 0 = jokertecken"
  finns kvar i firmware/Car_Client/RControlStation — städa vid flera robotar.
- Diagnostikraden i `motor_set_vesc_value` kan tas bort när allt är stabilt.
- ~~Kartan: bilen ~8 m från RTK-punkterna~~ **LÖST 2026-09-23:** Gemini nollade kartans
  ENU-referens på första RTK-punkten vid Connect, medan styrkortet har egen ref
  (60.0631485, 18.0789337, h=0). Nu hämtar kartan bilens ref vid Connect och var 10:e s
  (`mainwindow.cpp`, `enuSyncTimer`). Bekräftat av användaren: "MYCKET BÄTTRE".
  OBS: "Sol:"-texten vid bilen visar Pi:ns rtkrcv-lösning, inte styrkortets GPS.
  Gemini skriver dessutom om GPS-inställningar på kortet vid varje Connect
  (`carinterface.cpp`, `configurationReceived`) — granska vid tillfälle.
- Claude Code:s säkerhetsfilter stoppar ssh/scp-skrivningar mot Pi:n; användaren kör
  `! ...`-kommandon (eller lägger till en behörighetsregel).

---

# Var vi slutade — 2026-09-22, kväll (efter att användaren gått hem, Claude jobbade vidare själv)

Läs den här först nästa gång.

## Kvällens kontext
Gemini löste tidigare ikväll en total datorfrysning (svart skärm) som uppstod när
RControlStation öppnades — orsak/metod okänd för Claude, fråga användaren om det blir
relevant. Efter det testade användaren VESC-styrningen och rapporterade tre problem,
som Claude fick fria händer att lösa ("gör vad du vill, bara det fungerar imorgon"):
1. `Write` i RControlStation gav alltid `Write timeout on serial port...` och krävde
   omstart av allt efteråt.
2. `screen -r car` på Pi:n "gick bananas" (floodade) så fort dosan rördes.
3. Ingen poll data (batteri/vinkel/sensorvärden frysta, ingen ping/GPS), och dosan gav
   utslag men styrde inget.

## Vad Claude gjorde och fixade (utan användaren närvarande)

### 1. Write-timeout — LÖST och verifierat på riktiga styrkortet
- Orsak: `conf_general_store_main_config()` i firmware höll `chSysLock()` (allt-avbrott-av)
  runt HELA sparloopen för `MAIN_CONFIG` (100+ EEPROM-variabler). USB frös i flera hundra
  ms i ett svep → `Car_Client`s hårda 200ms-skrivtimeout (`serialport.cpp`) slog till varje
  gång.
- Fix: låset tas nu per variabel istället för runt hela loopen (`conf_general.c`).
  Extra marginal: Car_Client:s timeout höjd 200→800ms (`serialport.cpp`).
- **Byggt, flashat (verifierat OK av OpenOCD) och driftsatt.** `car_client.service`
  omstartad, port 8300 lyssnar.

### 2. Screen-flooding — LÖST och verifierat på riktiga styrkortet
- Orsak: `commands_printf("Activity: %d", ...)` i `commands.c` (CMD_RC_CONTROL_ADV) skrev
  ut en rad för VARJE mottaget spak-kommando.
- Fix: raden borttagen/utkommenterad. Flashat tillsammans med fix #1 (samma binary).

### 3. flash_styrkort.sh kan nu köras helt utan interaktion
- Ny flagga `--ja` (eller env `RC_FLASH_JA=1`) hoppar över FLASHA-bekräftelsen,
  verktygsinstallations-frågan och det interaktiva skrivbordstestet. Standardläget
  (utan flaggan) kräver fortfarande FLASHA — ingen ändring i säkerhet för manuell körning.

### 4. Poll data / frysta värden — TROLIGEN INTE en regression, men EN RIKTIG BUGG HITTADES OCH FIXADES ÄNDÅ
- Claude verifierade med en kort, säker `strace` (ingen skada, ingen risk för roboten) att
  RControlStation FAKTISKT tar emot ~970 byte statuspaket kontinuerligt över nätverket —
  dataflödet till/från Car_Client och styrkortet var friskt. Kodgranskning av
  `stateReceived`→`setStateData` hittade ingen spärr som skulle blockera UI-uppdatering.
  **Slutsats: den ursprungliga observationen ("frysta värden") var sannolikt korrekt i
  stunden men troligen inte orsakad av kvällens fixar** — trafiken flödar nu.
- **Men en helt separat, allvarlig bugg hittades under utredningen:**
  `database.cpp` letade efter `data.db` i **aktuell katalog FÖRST** (`"data.db"`, relativ
  sökväg). SQLite skapar tyst en ny TOM databasfil om ingen finns — så om man startar
  `RControlStation` från fel katalog (t.ex. hemkatalogen, vilket händer om man bara skriver
  `RControlStation` i en ny terminal — exakt vad Claude själv rådde till tidigare ikväll!)
  används en helt annan/tom databas i tysthet, UTAN felmeddelande.
  - Användarens nya dosa-bindningar ikväll (axel 1→Styrning, 6→Fart, 7→Styrning) hade
    hamnat ENBART i `/home/mapro/data.db`, inte i projektets databas.
  - **Fixat:** koden kollar nu `QFile::exists()` innan den öppnar en kandidat, och
    ordningen är omvänd — binärens egen mapp (deterministisk, oberoende av terminal-CWD)
    provas FÖRE den nakna `"data.db"` (som nu är sist, bara för utveckling).
  - **Data räddad:** dosa-bindningarna från `/home/mapro/data.db` är nu ihopslagna
    (INSERT OR REPLACE, ingen radering) i alla tre projekt-databaserna
    (`RControllStation/data.db`, `rise_sdvp/data.db`, `rise_sdvp/web/data.db`).
    Backuper av alla tre finns som `*.bak_20260922` bredvid originalen.
  - En kopia av den sammanslagna databasen ligger nu även bredvid den kompilerade
    binären (`.../build/cmake_linux/build/lin/data.db`) som en robust fallback.
  - RControlStation byggd om med fixen och **omstartad** (gammal instans dödad, ny körs,
    PID syns i `ps aux`). Startloggen bekräftar: `Database opened from:
    ".../build/lin/data.db"` — rätt databas, inte en CWD-slump.
- **Kvar att verifiera imorgon:** klicka "Connect" i den redan körande RControlStation
  och se om poll data/batteri/GPS uppdaterar live nu. Koppling/dataflöde var friskt vid
  kvällens test, så det bör fungera — men ej verifierat med riktig anslutning efteråt.

### 5. "Kan inte köra" (dosan gör inget) — EJ SLUTGILTIGT LÖST, men utrett
- Databasen har vettiga bindningar: axel 5+6 (vänster spak) → "Speed Control" (id 10),
  axel 7+8 (höger spak) + knapp 1 → "Steering Control" (id 11). Detta ser rimligt ut.
- Två knappbindningar (controller 2 "Left Button Down"→4, controller 4 "Right Button
  Down"→3) är **skräp/inaktuella** — de pekar på action-ID:n som inte längre finns i
  `controls`-tabellen (nuvarande ID:n är 7–12). Ofarligt (result blir NULL/inget), men
  städa bort om ni vill ha en ren config.
- **Det som INTE kunde verifieras:** matchar VESC-motorernas `activity`-fält (satt via
  Confcommon-fliken, sparat på styrkortets EEPROM via "Write") med samma ID:n (10=Speed,
  11=Steering)? Detta ligger bara på styrkortet, inte i sqlite-databasen, och kräver en
  levande "Read" i RControlStation för att se — gjordes inte ikväll (RControlStation var
  fortfarande anslutet och Claude ville inte krocka med den aktiva sessionen eller styra
  roboten utan användaren närvarande).
  **Imorgon:** öppna Confcommon-fliken (motorernas inställningar), klicka "Read", och
  kontrollera att den VESC-motor som ska köra framåt/bak har action = "Speed Control" och
  den som styr har action = "Steering Control" (samma namn som dosans bindningar ovan).

## Säkerhet — vad som INTE gjordes ikväll
Claude skickade INGA styrkommandon till roboten och testade INTE dosan mot riktiga
motorer, eftersom användaren gick hem och ingen fanns kvar för att bekräfta att det var
säkert i stunden. All verifiering ikväll var läsning/observation (strace, loggar,
databaskoll) — inget som kunde få roboten att röra sig.

## Git-status
Fortfarande inget committat/pushat sedan `c257246`. Nu ÄNNU mer arbete i arbetsträdet
(på laptopen: `database.cpp` databasfix; på Pi:n: `commands.c`, `conf_general.c`,
`serialport.cpp`, `flash_styrkort.sh`). Committa/pusha bara på uttrycklig begäran.

## Kvar att göra (i ordning)
1. Klicka "Connect" i den redan körande, redan korrekt konfigurerade RControlStation.
   Verifiera poll data/batteri/GPS uppdaterar live.
2. Läs (Read) styrkortets config i Confcommon-fliken, kontrollera VESC-motorernas
   `action`-fält matchar dosans bindningar (Speed Control / Steering Control).
3. Om allt stämmer men roboten ändå inte rör sig vid spakrörelse: nästa steg är att titta
   på om `main_config.vehicle.disable_motor` är satt, eller om `motor_set_vesc_value()`
   (i `commands.c`/`motor_control.c`) har någon annan spärr — ej undersökt ikväll.
4. Städa bort de två skräp-knappbindningarna (id 2→4, id 4→3) i `controllers`-tabellen
   om ni vill.
5. Fortsätt med VESC-CAN-skanningen (se `[[robot-vesc-can-skanning]]`-minnet) — detta är
   helt orört ikväll, separat problem.
6. Committa + pusha allt (på begäran).
