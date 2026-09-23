# Var vi slutade — 2026-09-23, eftermiddag: VÅRT EGET PROGRAM KÖR ROBOTEN! 🎉

Klient (robotstyrning) → WireGuard → robotd → Car_Client → styrkort → VESC.
Verifierat av användaren på upphissade RobAnt: skanning visar 28/36/76 som
svarande, dosan kör fram/bak/styr med max 0,45 (samma känsla som RControlStation).

## Hur det fungerar
- robotd skickar samma kommando som RControlStation: `CMD_RC_CONTROL_ADV` (125),
  aktivitet 10 = fart, 11 = styrning, värde = spak × max (config `drive`).
  Omsändning var 100 ms vid körning, 0 direkt vid stopp, 0 var 1 s i vila.
- **Car_Client tar bara EN TCP-klient** (`tcpserversimple.cpp:139`). robotd och
  RControlStation kan alltså inte vara anslutna samtidigt — stäng den ena först.
- Spakar som i RControlStation: vänster upp/ner = gas, höger sidled = styrning.
- robotd loggar "Körning: …" en gång per sekund och watchdog-övergångar.
- `/etc/robotd/config.json` på Pi:n har `"drive": {"speed_max": 0.45, "steering_max": 0.45}`
  (ändrat för hand med sudo; standard i koden är nu också 0.45).

## Så testar man (robotd.service är fortfarande AV)
1. Stäng RControlStation (Car_Client tar bara en klient).
2. Robotd-terminalen: `ssh -t robant@192.168.200.10 'timeout -s INT 600 ~/robot-control/target/release/robotd'`
   (`-t` krävs, annars blir robotd kvar på Pi:n när man trycker Ctrl+C).
3. `~/Hämtningar/robot-control/target/release/robotstyrning`, anslut 192.168.200.10, AKTIVERA.
Uppdatera Pi:n: rsync + `cargo build --release -p robotd` (se historiken), ingen install_pi.sh
(den slår på I2C — fråga först).

## Kvar
- Status till klienten (batteri, fart, VESC-svar) — robotd skickar ingen StatusUpdate än.
  AKTIVERA:s VESC-krav är fortfarande en platshållare (`can_activate`, vesc_ok = true).
- Reglage för max i klienten, så man slipper ändra config.json + starta om robotd.
- Firmware skriver fortfarande "Activity %d -> %d actuator(s)" per kommando (commands.c).
- Aktivera robotd.service först när allt ovan är på plats (fråga användaren).
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
