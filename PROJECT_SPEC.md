# Robot Control System — Projektspec

Återskapad spec (ursprungligen sammanställd i en tidigare session) för en 4-hjulsdriven,
400kg, batteridriven robot med hydraulisk drivning, som ska få ett nytt, robust och enkelt
styrsystem för fjärrstyrning "från var man än i världen befinner sig".

## 1. Hårdvara (befintlig, ändras inte)

- **Chassi:** 400kg robot, 4-hjulsdriven, batteridriven (4× 12V litium i serie, ~48V)
- **Drivning:** Två stora elmotorer driver var sin hydraulpump (seriekopplade per sida,
  höger/vänster) → hydraulmotorer i varje hjul. Ren varvtalsstyrning, öppen loop
  (ingen flödesåterkoppling).
- **Styrning:** En mindre elmotor driver en mindre hydraulpump → hydraulcylinder som
  vrider hjulen. Vinkelgivare finns för feedback.
- **Motorstyrning:** Alla tre motorer körs via VESC (VESC Labs), kopplade med CAN H/L.
- **"CarController"-kort:** Egenbyggt/specialbeställt kort, innehåller en u-blox
  ZED-F9P RTK-GPS (GNSS-antenn på U7-port), CAN H/L-plint (4-polig kontakt),
  7–60V strömingång, SWD-pinnar (SWCLK/SWDIO/NRST) för programmering av kortets
  egen mikrokontroller. Sannolikt släktskap med Benjamin Vedders **rise_sdvp**-projekt
  (github.com/vedderb/rise_sdvp) — nuvarande system kör troligen `Car_Client` på Pi:n
  och `RControlStation` på klientdatorn därifrån.
- **Raspberry Pi:** 4 eller 5 (beroende på tillgänglighet), ansluten till CarController-
  kortet med **2× USB (Micro-USB)**-kablar — en för u-blox GPS (eget USB-protokoll),
  en trolig CAN-brygga via kortets mikrokontroller (protokoll okänt tills vidare —
  utreds med kommandon nedan).
- **5G-modem:** Inbyggd router med eget WiFi-nät på roboten. Okänt om publik IP eller
  NAT (troligen NAT).
- **Nödstopp:** Fysisk, sitter på plint på VESC. Bryter bara strömmen till
  drivmotorerna (inte Pi/CarController). Hydrauliken tvärstannar direkt vid strömbrott.
- **Kamera:** USB-kamera (enkel, framåtriktad, fast montering).
- **Belysning:** Planeras, styrs via relä kopplat till Raspberry Pi GPIO (CarController-
  kortets pinnar är upptagna av vinkelgivare/hastighetsgivare).
  - GPIO17 → optoisolerat 1-kanals reläkort (5V-styrt, klarar 12–48V lastsida) →
    belysning. Håller lampornas strömkrets helt separerad från Pi:ns logik.
- **Display (ny):** Liten OLED (t.ex. 0.96" SSD1306, I2C) monteras inne i den
  IP56-klassade lådan, bakom lock. Syns bara när man öppnar lådan (inte åtkomlig
  utifrån) — INTE en stor, ständigt synlig skärm. Visar: robotnamn +
  parkopplingskod vid uppstart, samt löpande status: internet upp/ner
  (WireGuard-gränssnittet `wg0`), om Car_Client-länken kör, om förväntad
  USB-hårdvara syns, batteri (platshållare tills §13 är löst, visas som
  "–"), senaste felmeddelande. Uppdateras av `robotd` var 2:a sekund.
- **Motsvarande tidigare robotbygge:** har haft lastarm + tilt, styrt via extra
  VESC — mönster att kunna återanvända (VESC roll = "Annat" + fri knapp-/axel-mappning).

## 2. Vad som är fel med nuvarande system

Nuvarande program: `Car_Client` (Pi) + `RControlStation` (Qt, Ubuntu-dator), startas
manuellt i terminal (`RControlStation`). Inte i sig instabilt i CAN/körlogik, men:

- **Fult** — omodernt gränssnitt
- **Avancerat** — för många funktioner/inställningar för vad som faktiskt behövs
- **Jobbigt att få igång** — ingen enkel "starta och kör"-känsla
- Byggt för direkt TCP/UDP mellan dator och robot — funkar bra på samma nät, men blir
  skakigt över mobilnät pga NAT. Det är sannolikt kärnan i den upplevda opålitligheten
  vid fjärrstyrning.

**Beslut:** Nya systemet är INTE en ny version av RControlStation. Helt nytt,
minimalistiskt program. CAN/protokollnivån mot CarController-kortet kan eventuellt
återanvändas (rise_sdvp är GPL3, inga hinder för eget bruk) om det visar sig vara
samma protokoll — men GUI:t och nätverkslagret byggs om helt.

## 3. Systemarkitektur (beslutad)

**Tre delar:**

1. **`robotd`** (Raspberry Pi, Rust) — kör-tjänst:
   - Pratar CAN mot VESC:arna (SocketCAN om möjligt, annars serieprotokoll via
     CarController-kortet — avgörs av hårdvarutest, se §6)
   - Läser GPS, vinkelgivare, hastighetsgivare
   - Lokal säkerhets-watchdog, oberoende av nätverket: giltig styrsignal måste komma
     inom **150 ms** → annars mjuk nedbromsning; **400 ms** tystnad → hård nollställning.
     Detta är kärnskyddet mot 5G-avbrott.
   - Mjuka acceleration/retardationsramper på VESC-kommandon (hydrauliken ska inte
     få ryckiga steg)
   - Ansluter **ut** till reläservern (löser NAT-problemet), egen tråd, separat från
     den snabba styrloopen — nätverket kan aldrig störa körkänslan
   - Styr belysningsrelä via GPIO
   - Driver OLED-displayen (namn/parkopplingskod vid start, löpande status)
   - VESC-rollmappning (se §5) sparas som profil lokalt på Pi:n
   - Enkel körlogg: vem körde, hur länge, varningar/fel (medvetet inte för detaljerad)

2. **Reläserver** (VPS, liten, EU/Sverige) — mötesplats:
   - Robot ansluter ut, klient ansluter ut — löser NAT helt utan portöppning
   - All säker logik (parkoppling, en förare i taget, temporära titta-koder) i egen
     modul, oberoende av nätverkskoden
   - Flera robotar kan dela samma relä-server

3. **Klientprogram** (Ubuntu-dator, Rust):
   - Svart anslutningsskärm → textfält i mitten → namn eller IP till roboten
   - Efter anslutning: helskärm, kamerabild i botten, **AKTIVERA**-knapp ovanpå bilden
   - Statusinfo i hörnet: hastighet, signalstyrka, ping, batteri (uppskattas via
     VESC `input_voltage`), temperatur
   - Går att minimera/flytta som vanligt fönster också (inte tvingad helskärm)
   - Kommer ihåg tidigare anslutna robotar (klickbar lista)
   - Språk: **svenska**

## 4. Säkerhet & parkoppling (beslutad)

- **Ingen databas/lösenord.** Varje robot får ett unikt kryptografiskt nyckelpar
  (Ed25519, likt SSH/WireGuard) vid installation.
- Parkoppling: engångskod visas på robotens OLED-display (eller via `robotctl pair`
  över SSH) → skrivs in på klientdatorn en gång → efter det räcker robotnamnet för
  att ansluta automatiskt från den datorn.
- Flera klientdatorer kan parkopplas mot samma robot (t.ex. en kollega), men:
  - **En förare i taget.** Försöker en andra parkopplad dator ta över medan någon
    redan kör → **nekas helt** ("roboten körs redan av någon annan"). Inget automatiskt
    övertagande.
  - **Titta-läge:** flera kan ansluta samtidigt och se kamera + status utan att styra.
    Tillfällig, lättare att dela "titta-kod" räcker för det (ger aldrig kontroll).
- All trafik reläas via reläservern, alltid samma väg (inte lokalt WiFi ens när
  roboten är nära) — för enkelhet och förutsägbarhet.

## 5. VESC-mappning (Settings-vy)

- Skannar CAN-bussen, listar alla svarande VESC-ID live
- Varje ID tilldelas en roll: **Drift vänster**, **Drift höger**, **Styrning**, eller
  **Annat** (fritt namn)
- "Annat" kan bindas till valfri kontrollindata (knapp/axel/upp-ner-knappar, t.ex.
  L1/L2 för lastarm, R1/R2 för tilt) + min/max-gränser
- Sparas som profil på Pi:n, bytbar — samma mjukvara kan köra flera robotar med
  olika utrustning (t.ex. tidigare bygge med lastare+tilt)

## 6. Kontrollschema (PS4-dosa, beslutat)

- **Vänster spak (upp/ner):** gas/broms
- **Höger spak (vänster/höger):** styrning
  (Ändrat 2026-09-23 till samma som i RControlStation, som föraren är van vid.)
- **L1/L2:** lastarm upp/ner (om monterad)
- **R1/R2:** tilt (om monterad)
- Tillvalsfunktioner (lastarm/tilt) är **inte** låsta bakom AKTIVERA-knappen — går
  att använda direkt.
- **Ingen dödmansgrepp-knapp.** Föraren litar på egen uppmärksamhet.
- **AKTIVERA-knapp** krävs innan gas/styr fungerar. Återställs (måste tryckas igen):
  - efter ~5 minuters stillastående
  - vid dosa-frånkoppling (Bluetooth)
  - **INTE** efter ett kort nätverkstapp som läker ut — då återgår körning automatiskt
    när anslutningen blir grön igen
- Innan AKTIVERA går att trycka: appen kontrollerar att väsentliga system är OK.
  - **Blockerande:** kamerabild saknas (bara i CAM-läge, se nedan), eller
    drift-/styr-VESC svarar inte på bussen (gäller ALLTID, oavsett läge)
  - **Bara varnande (blockerar inte):** GPS, temperatur, övrigt — visas t.ex. som "–"

### Körläge: CAM / LOS (tillagt 2026-09-19)

En enkel knapptryckning på körskärmen (som belysningsknappen — inget
bekräftelsesteg, eftersom AKTIVERA ändå måste tryckas separat efteråt):

- **CAM** (standard, alltid vid varje ny anslutning — minns inte senast
  valda läget) — kräver kamerabild för att AKTIVERA ska gå att trycka.
- **LOS** ("line of sight") — föraren ser roboten direkt, kamerakravet
  släpps. VESC-kravet gäller fortfarande — det handlar om att roboten
  mekaniskt går att styra, inte om föraren ser den.
- **Låst medan AKTIVERAD** — går bara att byta läge innan/efter körning,
  inte mitt i.
- **Viktigt:** läget styr BARA aktiverings-kraven, inte vad som visas på
  skärmen. Kamerabilden (eller en framtida VR-ström) renderas oavsett
  läge — tänkt scenario: köra med VR-glasögon, ta av dem och köra LOS,
  men fortfarande kunna streama/spela in från kameran till en skärm.
  Video-renderingen ska alltså aldrig kopplas till körläget.
- Tydlig lila "LOS-läge"-markör ovanpå kameraytan när aktivt, så det
  aldrig är oklart vilket läge man kör i.

## 7. Status, varningar, larm (beslutat)

- **Ram runt hela programfönstret**, färgkodad efter anslutningskvalitet:
  - **Grön** — bra anslutning
  - **Gul** — dålig anslutning
  - **Röd** — anslutning tappad → roboten stannar (slutar skicka gas/styr-kommandon,
    speglar Pi:ns watchdog-trösklar 150/400ms)
  - **Lila** — separat färg specifikt för **dosa (PS4) frånkopplad**, med egen
    varningstext, så det inte blandas ihop med nätverksproblem
- **Korta ljudsignaler** vid färgbyten (inga larmsirener, inga ihållande larm)
- VESC-överhettning: **varnar tydligt**, sänker INTE effekt automatiskt — föraren
  väljer själv att sakta ner/stanna
- Belysning: on/off via **on-screen-knapp** (inte dosan — dosans knappar är redan
  fullbokade och belysning kräver inte snabb reaktion)

## 8. Nätverk

- Robot initierar utgående anslutning till reläservern (löser NAT, oavsett var i
  världen klienten befinner sig)
- Klient ansluter till samma reläserver
- Målkänsla: **<100ms** upplevd fördröjning på styrkommandon
- Kamera: WebRTC (låg latens, hanterar dåligt/varierande mobilnät bättre än RTSP)
- Balanserad videokvalitet: varken maximal skärpa eller maximal flyt på bekostnad av
  det andra — ~720p, adaptiv bitrate, justeras vidare live
- Alltid samma anslutningsväg via reläet, aldrig lokal genväg även vid närhet till
  roboten

## 9. Loggning

Medvetet enkel, inte för detaljerad:
- Vem körde
- Hur länge
- Varningar och felmeddelanden

Ingen rutt-/GPS-historik eller mer detaljerad telemetrilogg för tillfället.

## 10. Sekundärt / senare (inte nu)

- **Karta/rutt-körning (framtida, inte nu):** Om det någon gång blir
  aktuellt att kunna se sin egen position på en karta och klicka ut en
  bana att köra ("kör hit"), ska det byggas som ett **helt nytt system**
  från grunden — INGET återanvänds från det gamla RControlStation/
  rise_sdvp-baserade systemet (10-15 år gammalt, för krångligt/jobbigt
  att hålla på med, precis den erfarenheten som var hela anledningen till
  att bygga om styrsystemet från grunden). Samma filosofi som resten av
  det här projektet: enklare, mer tillförlitligt, mer genomtänkt — inte
  en återupplivning av det gamla. Inget av det här är planerat eller
  påbörjat än.
- **GPS-parsning (2026-09-19):** `StatusUpdate.gps_fix` finns som fält i
  protokollet och visas på OLED-displayen, men är just nu bara en
  platshållare (hårdkodad `false`, ingen kod läser faktisk NMEA-data från
  Car_Client). Medvetet beslut att INTE bygga NMEA-parsningen förrän
  någon funktion faktiskt behöver den (t.ex. ett framtida kart-/rutt-system,
  se ovan) — annars bara kod att underhålla för data ingenting
  konsumerar. Värt att notera: ramningen på port 8300 är redan
  byte-verifierad (§14) och vi har redan sett riktiga `$GNGGA`-meningar i
  en fångst, så det här skulle vara ett relativt lätt jobb att plocka upp
  igen den dagen det faktiskt behövs.
- Eventuell CAN-hatt (PiCAN2/Waveshare, MCP2515) som reservlösning om CarController-
  kortets USB-CAN-brygga visar sig opålitlig/odokumenterad. GPS-modulen (ZED-F9P)
  behålls oavsett — RTK-kapabel, redan mycket bra.

## 11. Öppna frågor / att bekräfta med hårdvara

**Status:** Innan riktig hårdvara fanns tillgänglig fick vi hypotetiska/simulerade
terminalsvar (genererade av ett annat AI-verktyg, inte avlästa från er faktiska
Pi). De är **inte** verifierad data och byggs inte in som fakta i koden, men ett
element i dem är värt att notera: VID/PID `0483:5740` som gissades för
`/dev/car` är STMicroelectronics riktiga USB-VID för deras "Virtual COM
Port"-klass — en rimlig gissning givet att CarController-kortet har SWD-pinnar
(STM32-programmering), men fortfarande obekräftad. Kommandona nedan måste
fortfarande köras mot verklig hårdvara.

Kommandon att köra på Raspberry Pi:n (kopplad till CarController-kortet, inga
VESC:ar/hydraulik behöver vara inkopplade för detta test):

```bash
ps aux | grep -i car
ls -la /dev/car
udevadm info -a -n /dev/car | grep -E "idVendor|idProduct|serial"
```

Om `/dev/car` **inte** finns, kör den bredare listan istället:

```bash
lsusb
lsusb -v | grep -A5 "CarController\|u-blox"
dmesg | tail -60
ip link show
which candump || echo "can-utils ej installerat"
```

Om `can0` dyker upp i `ip link show`:

```bash
sudo apt install can-utils   # om det saknas
sudo ip link set can0 up type can bitrate 500000
candump can0
```

Svaret avgör:
- Om `/dev/car` finns + en process som heter `Car_Client` syns → det är med stor
  sannolikhet exakt rise_sdvp-protokollet, känt och dokumenterat, inget gissningsarbete.
- Annars: `can0` som dyker upp automatiskt → standard SocketCAN/gs_usb, enkelt.
- Annars: seriell enhet (`/dev/ttyACM0`/`/dev/ttyUSB0`) → eget/proprietärt protokoll,
  kräver mer utredning (ev. kontakt med den som byggde nuvarande system, eller
  reservplan med CAN-hatt).

**Om det blir seriell-brygga via `/dev/car`:** `robotd/src/serial_bridge.rs`
implementerar paket-inramningen (start/längd/payload/CRC16/slut) som Benjamin
Vedder återanvänder i sina seriella protokoll (VESC UART, BLDC-Tool). Jag har
INTE kunnat läsa den faktiska källkoden för rise_sdvp:s
Controller↔Car_Client-protokoll (GitHub blockerade automatiserad åtkomst till
filträdet), så det är en rimlig men obekräftad gissning på ramningsnivå — och
kommando-ID:na för själva CAN-vidarebefordran är medvetet INTE gissade, utan en
öppen TODO. Bekräftas antingen mot verklig byte-trafik (hexdump av `/dev/car`)
imorgon, eller mot källkoden om vi lyckas läsa den på annat sätt (t.ex. `git
clone` lokalt hos er, eller att ni klistrar in filens innehåll här).

## 13. Arkitekturbeslut: bygg ovanpå Car_Client (bekräftat på hårdvara 2026-09-15)

Riktiga terminalsvar från Pi:n bekräftade:
- `/dev/car` → `ttyACM0`
- VID/PID `0483:5740` = STM32 Virtual COM Port (matchade gissningen)
- **`Car_Client` (rise_sdvp) kör redan** i en screen-session (`car`), startad
  med `./Car_Client -p /dev/car --useudp --logusb --usetcp`

Slutsats: länken mellan Pi och CarController-kortet är redan bevisat
fungerande — det ni upplevt som opålitligt är RControlStation/nätverksdelen,
inte den seriella kortlänken.

**Beslut:** `robotd` tar INTE över `/dev/car` direkt (väg B). Istället
ansluter `robotd` till det redan körande `Car_Client` via dess lokala
TCP-port (`car_client_link.rs`), samma sätt som `RControlStation` redan gör
idag. Vi river inte upp något som redan fungerar — vårt nya lager
(watchdog, ramper, reläserver, nytt UI) läggs ovanpå.

Kvar att verifiera: exakt TCP-port och de faktiska kommando-ID:na för att
läsa VESC-telemetri / skicka styrning genom Car_Client. Förslag: fånga
riktig trafik med `tcpdump`/Wireshark medan `RControlStation` kör som vanligt
och ansluter till `Car_Client`:

```bash
sudo tcpdump -i any -w car_client_capture.pcap tcp
# kör RControlStation som vanligt en stund, avsluta sen tcpdump
```

Skicka en `hexdump -C`/`xxd` av några paket ur den fångsten, så matchar jag
dem mot paket-inramningen i `serial_bridge.rs` och fyller i resten av
`car_client_link.rs` med verifierade kommando-ID:n istället för TODO:er.

### Uppdatering 2026-09-16 — port bekräftad, två öppna frågor kvar

Loggutskrift från en manuell körning av `Car_Client` bekräftade:
- **TCP-port: `8300`** (rättat i `car_client_link.rs`/`config.rs`, var tidigare
  gissat till 65102)
- Paketen har fält `Id`, `mCarId`, `cmd` — två riktiga kommando-ID:n sågs:
  `cmd 120` (engångs, `Id 255/mCarId 255`, troligen broadcast/init) och
  `cmd 221` (upprepas, kopplat till NMEA/GPS-reconnect)

**Två öppna saker att reda ut:**
1. Den manuella körningen använde `-p /dev/vehicle`, medan
   screen-sessionen (`car`) som redan kördes använde `-p /dev/car`. Loggen
   visade "Address in use" — dvs **två `Car_Client`-instanser kan ha kört
   samtidigt** och tävlat om samma seriella länk. Bör redas ut: vilken
   enhet (`/dev/car` eller `/dev/vehicle`) är den som faktiskt används i
   normal drift, och kör bara EN `Car_Client`-instans åt gången.
2. Den råa datan som syntes (`\x00x`) matchade inte paket-inramnings-
   antagandet i `serial_bridge.rs` (som förväntar startbyte `0x02`/`0x03`).
   Kan bero på att det bara var ett kort/ofullständigt utdrag — en riktig
   `tcpdump`-fångst på port 8300 skulle avgöra om ramnings-antagandet
   behöver justeras.

### Uppdatering 2026-09-16 (2) — ramning + CRC bekräftade byte-exakt

En riktig `tcpdump`-fångst (`tcp port 8300`) från roboten gav verklig,
oredigerad TCP-trafik. Byte-för-byte-analys mot ett NMEA-positionspaket
bekräftade:

```
[0x02 start] [1 byte längd] [payload] [CRC16-CCITT poly 0x1021 init 0x0000] [0x03 slut]
```

CRC:et matchade exakt (payload → `0x9ae9`, verifierat med init `0x0000` —
det tidigare gissade `0xFFFF` gav fel resultat och är rättat i koden).
Ramningen i `serial_bridge.rs` är alltså **inte längre en gissning**.

Trafiken på port 8300 innehöll rena NMEA-meningar (`$GNGGA,...`) inbäddade
i payload, föregångna av två bytes (`04 3f`) som återkom identiskt i varje
sådant paket — troligen en typ-/kommandomarkör för "GPS-position", men
innebörden är inte bekräftad. Kvar: samma analys mot ett paket som
innehåller VESC-telemetri eller ett styrkommando, för att få fram de
kommando-ID:na också (samma metod fungerar — fånga mer trafik, särskilt
medan RControlStation skickar styrkommandon, och skicka en ny
`hexdump -C`/`tcpdump`-fångst).

Koden i detta repo är en **funktionell återskapning** av det som beskrevs och
byggdes i en tidigare session (telefon), utifrån den fullständiga specen ovan.
Den ursprungliga koden fanns bara i den sessionens tillfälliga arbetsyta och
kunde inte kopieras rakt av — det som ligger här är återuppbyggt från grunden för
att matcha samma arkitektur och beslut.

**Okompilerat/otestat än** (ingen nätverksåtkomst eller riktig hårdvara i den här
miljön): CAN-delen (väntar på hårdvarutestet i §11), WebRTC-videot (mest känsliga
delen — GStreamer-bindningar i Rust är versionskänsliga), robotd:s
WebSocket-server (`server.rs`, se §14 för arkitekturen den kör i).

**Rimligt tillförlitligt utan hårdvara:** ramp/watchdog-logiken i `robotd` (ren
matematik/tillståndsmaskin, enhetstestad).

Nästa steg: kör kommandona i §11 på riktig Pi + CarController, rapportera resultat,
så fylls CAN-lagret i korrekt utifrån verklig data.

## 14. Arkitekturomläggning: WireGuard istället för egen reläserver (2026-09-19)

**Bakgrund:** Vid genomgång av användarens befintliga fork
(`lordajz-cmyk/rise_sdvp`) visade det sig att en fungerande, redan
utprovad lösning för fjärråtkomst redan finns: **WireGuard VPN**, med
egna installationsskript (`wireguard.sh` för klienter/robot,
`wireguard_admin.sh` för servern) och ett fast IP-schema:

- VPN-server (hemma, bakom DuckDNS): `192.168.200.1`
- Klientdatorer/robotar: tilldelade fasta IP:n i `192.168.200.x`
  (exempel ur guiden: laptop `.3`, en robot-Pi `.4`, en Jetson `.7`, en
  annan robot-Pi `.8`)

Detta bekräftades även oberoende: IP-adresserna i den tidigare
`tcpdump`-fångsten (`192.168.200.99`, `192.168.200.10`) låg i samma
nätverk.

**Beslut:** Den tidigare planen (egen reläserver dit både robot och
klient ansluter ut, med Ed25519-nyckelpar och engångskoder för
parkoppling) **skrotas**. Den löste ett problem (NAT-genomgång +
kryptering + autentisering) som redan är löst, bättre, av WireGuard —
och som användaren redan litar på och har verktyg för.

**Ny modell:**
- `robotd` **lyssnar** direkt på sin WireGuard-IP (`bind_addr` i
  `/etc/robotd/config.json`, t.ex. `192.168.200.8:9000`), eller på
  `0.0.0.0` bakom en `ufw`-regel som bara släpper in trafik på `wg0`
  (rekommenderat i den befintliga `Sakerhetsanalys.md`).
- Klienten ansluter **direkt** till den IP:n — anslutningsskärmens
  textfält (namn/IP) blir nu bokstavligen bara det: robotens VPN-IP,
  eller ett namn som löses lokalt via klientens `/etc/hosts`.
- **Ingen egen kryptografisk parkoppling.** WireGuard är redan
  krypterat (ChaCha20-Poly1305) och redan autentiserat (bara
  registrerade publika nycklar/peers kommer in på nätverket alls).
  "Parkoppling" av en ny klientdator = lägga till den som WireGuard-peer
  med de befintliga skripten, inget nytt att bygga.
- **En förare i taget** hanteras lokalt av `robotd` självt (en enkel
  bool-flagga i `server.rs`), ingen central registry/server behövs.
- **Tillfälliga titta-koder** finns kvar som ett enkelt, icke-kryptografiskt
  lager ovanpå VPN:et (bara jämförelse av en textsträng) — rimligt givet
  att bara redan betrodda VPN-peers ens kan nå roboten över huvud taget.

**Konsekvenser för koden:**
- `relay-protocol` förenklad kraftigt: bort med `Hello`/`AuthChallenge`/
  `AuthResponse`/`PairingCode`/Ed25519-signering. Kvar: bara
  applikationsmeddelanden (`ClientMessage`/`RobotMessage`,
  styrkommandon, status, VESC-mappning).
- `relay-server`-cratet borttaget ur workspacet (byggs inte längre,
  ligger kvar i mappen som arkiv/referens — se dess egen kod om något
  därifrån skulle vara värt att återanvända senare).
- `robotd`: `relay_client.rs` och `identity.rs` borttagna, ersatta av
  `server.rs` (en axum WebSocket-server robotd själv driver).
- `client`: `net.rs` förenklad till en rak WebSocket-anslutning, ingen
  signeringsnyckel. `main.rs` startar nu en egen tokio-runtime (behövs
  för att kunna göra async nätverksanrop från egui:s synkrona loop).
- `mock-robotd`: omskriven till att vara en server (precis som riktiga
  `robotd`) istället för att ringa ut till ett relä.

**Relevant säkerhetsinfo hittad i samma genomgång (från
`Sakerhetsanalys.md` i användarens fork), värd att komma ihåg:**
- RC_Controller-kortet (STM32) har **redan en egen, oberoende
  watchdog** i `timeout.c`: om inget giltigt USB-paket kommer inom
  `heartbeat_maxtime` (exempel: 2 sekunder) stängs autopiloten av och en
  hård bromsström läggs på om farten är över 2 km/h. Det är alltså ett
  skyddslager UNDER vår egen `robotd`-watchdog (150/400ms), inte ett
  ersättande av den.
- STM32-chippet har dessutom en hårdvaru-watchdog (IWDG) som startar om
  kortet om firmware helt hänger sig.
- Rekommendation från samma dokument, värd att genomföra: en trådlös
  nödstopps-sändare utöver den fysiska nödstoppen, samt en `ufw`-regel
  på Pi:n som bara tillåter trafik på `wg0` (blockerar `eth0`/`wlan0`
  förutom SSH).
- De tre robotarna i användarens fork heter **Drängen**, **RobAnt** och
  **Macbot/Mactrac** (viss namnvariation i olika dokument). Hårdvaran i
  den här konversationen bekräftades vara **Drängen**.

### Öppen fråga till nästa gång

`robotd`:s port (`bind_addr` i config.json) behöver stämma överens med
vilken port klienten försöker ansluta till (`robot_port` i
`client/src/main.rs`, just nu hårdkodat `9000`). Fungerar bra som
standard, men blir en sak att komma ihåg om den någonsin ändras på ena
sidan men inte den andra.

## 15. Mock-test genomfört, riktiga buggar hittade och fixade (2026-09-19)

Två samtidiga Claude Code-sessioner (Dator 1 = mock-robotd/servern, Dator 2
= klienten) körde `mock_test.md` på en ny labdator, mot varandra på
`127.0.0.1` (inte över VPN). Fullständiga rapporter: `testresultat_dator1.md`
och `testrapport_dator2.md` (från den sessionen — inte nödvändigtvis kvar i
det här repot, se dess README/historik).

**Kompileringsfel hittade och fixade** (verkliga, inte gissningar):
- `client/Cargo.toml` saknade `futures-util` och `tracing`, som `net.rs`
  använder direkt — lades till.
- `mock-robotd/Cargo.toml` saknade `serde` (bara `serde_json` fanns) —
  lades till, plus två oanvända importer bortstädade.

**Verifierat att fungera, byggt utifrån headless-tester mot de riktiga
`relay-protocol`-typerna:**
- En-förare-i-taget, åskådarläge, avslag med korrekt `reason`.
- 500 rena + 300 abrupta anslut/koppla-från-cykler, och 100 samtidiga
  förar-anslutningar (exakt 1 accepterad) — inga läckor, minne/trådar/fd
  stabila.
- VESC-profilens serialisering round-trippar korrekt, inklusive fritt
  namn med specialtecken. (Notera för framtiden: `HashMap<u8,_>` blir
  strängnycklar i JSON och osorterad ordning — inget fel, bara bra att
  känna till om profilen någonsin diffas.)

**Riktiga buggar hittade i UI:t och fixade:**
1. **(Allvarligast)** Klienten slutade läsa nätverket så fort Settings-vyn
   var öppen — `poll_updates()` anropades bara från körskärmen.
   **Fixat:** flyttat till `App::update()`, körs nu oavsett skärm.
2. Den färgade ramen (grön/gul/röd/lila) syntes aldrig — kamerabildens
   `rect_filled` ritades ovanpå Frame-strecket. **Fixat:** ramen ritas nu
   sist, på ett eget lager (`egui::Order::Foreground`) ovanpå allt annat.
3. Statusintervallet (500 ms) var kortare än gul-tröskeln (300 ms), så
   ramen skulle flimra grön/gul konstant även på perfekt anslutning.
   **Fixat:** trösklarna breddade (gul vid >700 ms, röd vid >2000 ms).
4. Ping var hårdkodat till 0 (`Pong(0)` skickades vid varje meddelande,
   oavsett faktisk fördröjning). **Fixat:** ett riktigt `Ping`/`Pong`-par
   lades till i protokollet (`relay-protocol`), klienten mäter nu en äkta
   tur-och-retur-tid var sekund. Robotd/mock-robotd ekar tillbaka `sent_ms`
   oförändrat.
5. Nekad/avbruten anslutning visade ingenting i UI:t — fastnade på
   körskärmen utan text. **Fixat:** hanteras nu centralt i `update()`,
   växlar tillbaka till anslutningsskärmen och visar orsaken.
6. Robotnamnet sparades i "tidigare anslutna"-listan redan vid
   anslutningsförsöket, innan man visste om det lyckades — felstavningar
   hamnade i listan. **Fixat:** sparas nu först vid bekräftad lyckad
   anslutning.

**Buggar hittade i mock-robotd och fixade:**
- Kvarblivet gasvärde: `latest_throttle` nollställdes aldrig när föraren
  kopplade från, så en ny åskådare kunde se hastigheten fortsätta klättra
  utan att någon körde. **Fixat.**
- Trasiga/okända meddelanden ignorerades helt tyst (`_ => {}`), utan svar
  och utan loggrad — gjorde t.ex. en profil med `NaN` omöjlig att
  felsöka (gav aldrig `VescProfileSaved`, Settings stod kvar på "Osparade
  ändringar"). Samma mönster fanns i `robotd/src/server.rs`. **Fixat i
  båda** — okända/trasiga meddelanden loggas nu (`tracing::warn!`/`println!`).

**Ny robusthet tillagd, utöver de rapporterade felen:**
- En tystnads-timeout (5 sekunder utan giltigt meddelande) i både
  `robotd/src/server.rs` och `mock-robotd` — löser risken båda
  testomgångarna oberoende av varandra flaggade: att `driver_connected`
  annars kan fastna om VPN-tunneln dör tyst utan en riktig TCP-close.

**Inte åtgärdat, medvetet (bedömdes som lägre prioritet eller kräver mer
underlag):**
- Ingen validering av VESC-profilen i mock-robotd (`min > max`, okänd
  `input`-sträng, tom profil sparas alla) — rimligt för en mock, men
  riktiga `robotd` bör ta ställning till det när CAN-kopplingen är klar.
- Gamepad/PS4-dosa kunde inte testas i något av fönstren (ingen fysisk
  dosa inkopplad) — kvarstår som ett okänt tills vidare, inklusive risken
  att `LeftTrigger2`/`RightTrigger2` i `gamepad.rs` kan komma som axlar
  snarare än knappar på vissa drivrutiner.
- Test över riktig WireGuard-tunnel — bara `127.0.0.1` testat hittills.

**Nästa steg:** kör om `mock_test.md` efter de här fixarna (särskilt punkt
1 och 2 ovan var blockerande för att kunna testa Settings-flödet och
ramfärgen alls), och testa med en riktig PS4-dosa inkopplad denna gång.

## 16. Regressionstest, omgång 2 — en riktig kompileringsbugg hittad (2026-09-19)

Samma två Claude Code-sessioner körde det riktade regressionstestet från
§15. Fullständiga rapporter: `test-reports/2026-09-19_regressionstest_dator1.md`
och `_dator2.md`.

**Allvarligast: ett riktigt kompileringsfel jag själv införde i §15.**
När den tysta `_ => {}`-matchen togs bort (för att logga okända/trasiga
meddelanden istället för att svälja dem tyst) blev matchen över
`ClientMessage` icke-uttömmande — `Connect`, `VideoSignal` och
`RequestViewCode` saknade en egen arm. Gav `error[E0004]:
non-exhaustive patterns` i både `mock-robotd/src/main.rs` **och**
`robotd/src/server.rs` (samma bugg på båda ställena, Dator 1 kunde bara
bygga och bekräfta den i mock-robotd — `robotd` kräver Pi-specifika
bibliotek som inte fanns på labbdatorn — men flaggade korrekt att samma
matchning saknade samma armar i `server.rs` också). **Fixat i båda**: en
`Ok(other) => { logga och ignorera }`-arm tillagd före `Err`-armen.

**Verifierat att fungera efter fixarna från §15:**
- Settings-vyn läser nätverket och fungerar med vyn öppen (skanning +
  sparning gick igenom, bekräftat i loggen medan Settings var öppen hos
  klienten).
- Ramen är synlig i färg (om än bara lila kunde bekräftas, se nedan).
- Ping mäts på riktigt (varierade mellan 0–1 ms på `127.0.0.1`, som
  förväntat för en lokal loopback — meningsfullt värde kräver test över
  en riktig länk).
- Tystnads-timeouten (5s) bekräftad från serversidan: en tyst anslutning
  kopplades ner efter exakt ~5,0s, och en ny förare kunde ansluta direkt
  efteråt.
- Kvarblivet gasvärde-fixen i mock-robotd höll (ny åskådare såg `0.0`
  km/h, inte en klättrande spökhastighet).
- Trasiga meddelanden loggas nu korrekt med läsbar orsak (testat: ogiltig
  JSON, okänd variant, profil med `NaN`).
- Stresstester (800 anslut/koppla-från-cykler, 100 samtidiga
  förar-anslutningar) — inga läckor, `driver_connected` fastnar aldrig.

**Inte verifierat (ingen PS4-dosa fanns i någon av testomgångarna än):**
Fel 5 (nekad anslutning → tillbaka till anslutningsskärmen), Fel 6
(robotlistan sparas först vid lyckad anslutning — testmiljön hade redan
en gammal post från innan fixarna, så testet var inte rent), gas/styr
från en riktig dosa, grön ram (bara lila setts, eftersom ingen dosa =
`gamepad_connected: false` = lila ram enligt spec §7).

**Två UX/observability-förbättringar gjorda utifrån testarnas förslag
(inte buggar, men bra fångade):**
- **"Ingen dosa ansluten"** visas nu istället för en AKTIVERA-knapp som
  gick att trycka men föll tillbaka direkt (kändes trasigt för
  användaren i test, är egentligen avsett beteende enligt spec §6).
- **Nekade förar-anslutningar loggas nu på serversidan** också (`robotd`
  och `mock-robotd`), inte bara hos klienten som fick avslaget — enklare
  att felsöka från robot-sidan.

**Noterat men inte åtgärdat:**
- CPU i vila (klient, debugbygge): 11–17% av en kärna, pga.
  `ctx.request_repaint_after(33ms)` som ritar om ~30 ggr/s hela tiden
  oavsett om något ändrats. Sannolikt betydligt lägre i release-bygge,
  men inte uppmätt. Kan optimeras senare (rita om bara vid faktisk
  förändring) om det visar sig vara ett problem i praktiken.
- Ping-loopens `Ping`/`Pong` gör att en "levande" klient aldrig triggar
  tystnads-timeouten av misstag — bekräftat som avsett samspel mellan de
  två mekanismerna.

**Nästa steg:** samma som innan — riktig hårdvara (Pi + CarController)
för §11/§13, och gärna en riktig PS4-dosa nästa gång mock-testet körs för
att täppa till de sista luckorna (Fel 5/6, gas/styr, grön ram).

## 17. Runda 3 — riktig PS4-dosa, två nya buggar hittade och fixade (2026-09-20)

Samma tvådator-upplägg, nu med en fysisk PS4-dosa (Bluetooth) på Dator 2.
Fullständiga rapporter: `test-reports/2026-09-20_runda3_dator1.md` och
`_dator2.md`.

**Bugg 1 (allvarlig): `robotd` kompilerar inte alls.** `display.rs` gav 6
fel — `ssd1306 0.8` förväntar sig embedded-hal 0.2:s
`blocking::i2c::Write`, men `linux-embedded-hal 0.4` implementerar bara
embedded-hal **1.0**:s `I2c`-trait. En ren versionskrock i
`robotd/Cargo.toml`, inte ett kodfel i sig. **Fixat:** `ssd1306` uppgraderad
till `0.10`, som liksom `linux-embedded-hal 0.4` bygger på embedded-hal 1.0
rakt igenom. Fortfarande inte kompilerings-verifierat (ingen i test-
omgångarna hade en miljö som kunde bygga `robotd` fullt ut — display-delen
kräver I2C-relaterade headers utöver CAN/GPIO).

**Bugg 2: ingen dödzon på spakarna, med en oväntat allvarlig bieffekt.**
Riktig hårdvara visade att spakarna aldrig går exakt till 0.0 (brus på
±0.12–0.16 även i neutralläge). Två konsekvenser:
- Skulle ge krypning på riktig hydraulik i "vila"
- **Allvarligare:** 5-minuters-idle-timeouten i klienten räknar bara
  |värde| > 0.02 som aktivitet — bruset ensamt låg alltid över den
  tröskeln, så timeouten hade sannolikt **aldrig löst ut** i praktiken,
  trots att koden i övrigt är korrekt skriven.

**Fixat:** en dödzon (`STICK_DEADZONE = 0.12`) i `client/src/gamepad.rs`,
med linjär omskalning så fullt utslag fortfarande går att nå strax utanför
dödzonen. Enhetstestad (tre tester: brus nollas, fullt utslag bevaras,
värden strax utanför dödzonen ligger nära noll — inte plötsligt fullt
utslag).

**Bekräftat fungerande:**
- L1/R1 är digitala knappar, L2/R2 är analoga knappvärden (0.0–1.0) på den
  testade PS4-dosan — formeln `l1 - l2`/`r1 - r2` i `gamepad.rs` stämmer
  för just den modellen. (Kvarstående, mindre risk: andra
  dosor/drivrutiner kan exponera L2/R2 som rena axlar istället — bara den
  här modellen är verifierad.)
- 497 respektive 329 riktiga styrkommandon från fysisk dosa, mjuka,
  ojämna värden i hela -1.00..+1.00 — inga hack, inga krascher.
- Dosa bort/tillbaka: lila ram + "Ingen dosa ansluten" vid frånkoppling,
  grön ram + AKTIVERA-knapp (nollställd `activated`) när den kom tillbaka.
- Settings-skanning/sparning, en-förare-i-taget, frånkoppling, tystnads-
  timeout — alla bekräftade igen (delvis syntetiskt av Dator 1, för att
  hålla Dator 2:s logg ren).
- Ingen deadlock/hängning i tokio-runtime-lösningen, inte heller vid
  Bluetooth-dosans bort/tillbaka.

**Inte testat än:**
- 5-minuters idle-timeouten utvärtad i praktiken (operatören hade inte
  tid) — koden är läst och bedöms korrekt, men outnyttjad live. Bör testas
  om nu när dödzonen är fixad, eftersom det var just dödzon-bristen som
  troligen hindrade den från att fungera alls.
- Fel 5/6 (död adress, "Tidigare anslutna"-listan) med en RIKTIGT ny
  adress — testmiljön hade redan en gammal post från tidigare körningar.
- Andra klient-instans som nekas, snabba anslut/koppla-cykler — med
  riktig klient (gjordes syntetiskt istället).
- Ping/CPU över en riktig länk (WireGuard), release-bygge.

**Mindre observation, inte åtgärdad:** `mock-robotd` loggar inte
`accessory_a`/`accessory_b`, så L1/L2/R1/R2-värden inte går att verifiera
från serverloggen (bara att de inte kraschar något). Låg prioritet — bara
en loggrad att lägga till i mocken den dagen någon vill verifiera det
närmare.

## 18. Genomgång inför SD-kortsflashning (2026-09-20)

Innan `robotd` installeras på ett SD-kort för första gången på riktig
hårdvara, en sista genomgång av allt runt omkring:

**Riktig bugg hittad och fixad: tyst dataförlust i configen.**
`RobotConfig::load_or_default` gjorde `serde_json::from_str(&s)
.unwrap_or_default()` — om filen fanns men INTE gick att tolka (t.ex. för
att ett fält saknades, som hände här: `install_pi.sh` skapade en
`config.json` utan det senare tillagda `usb_device_path`-fältet) kastades
**hela filen**, tyst, och robotd föll tillbaka på fabriksinställningar
(fel robotnamn, fel bind-adress, tom VESC-profil) utan en enda
loggrad om varför. Fixat i två steg:
1. `RobotConfig` har nu `#[serde(default)]` på structen — saknade fält
   fylls från `Default` istället för att hela tolkningen misslyckas.
   Framtida nya fält kan alltså läggas till utan att gamla config-filer
   på redan flashade SD-kort går sönder.
2. Om filen ändå inte går att tolka (t.ex. trasig JSON-syntax) loggas nu
   ett tydligt `tracing::error!` om att inställningarna kastades, istället
   för tyst tystnad.

**`install_pi.sh` uppdaterat:**
- Genererad `config.json` inkluderar nu `usb_device_path` (saknades sen
  fältet lades till i en senare omgång än skriptet skrevs).
- Stöder nu `git pull`-arbetsflödet explicit i header-kommentaren (klona
  en gång, kör `git pull` + skriptet vid varje uppdatering — skriptet
  bryr sig inte om hur filerna kom dit, bara att det körs från
  projektroten).
- Ny **efterinstallationskontroll** i slutet av skriptet: kollar direkt i
  terminalen om WireGuard (`wg0`) är uppe, om `/dev/i2c-1` finns, om den
  konfigurerade USB-enhetssökvägen finns, och om en `car`-screen-session
  kör — samma kontroller OLED-displayen gör löpande, men synliga direkt
  vid installationen istället för att behöva öppna lådan.

**Paketgenomgång:** gick igenom alla beroenden i `robotd`/`robotctl`
(`Cargo.toml`) mot `install_pi.sh`s apt-lista. Inget av de aktuella
beroendena (tokio, axum, socketcan, rppal, linux-embedded-hal, ssd1306,
embedded-graphics, clap) kräver något systempaket utöver det som redan
installeras — alla är antingen rena Rust-crates eller pratar direkt med
kärnan via ioctl/syscalls. `libssl-dev`/`libudev-dev` i listan är i
praktiken kvarlevor sen tidigare (ingen nuvarande direkt-dependency
kräver dem), men lämnade kvar som säkerhetsmarginal snarare än borttagna
i onödan.

**Inget annat bedöms blockera en första installation.** Det som
återstår (§13:s VESC-kommando-ID:n, GPIO-relämodulens polaritet, OLED:ens
I2C-adress/upplösning) är sådant som behöver verifieras MOT den riktiga
hårdvaran efter installationen, inte något som hindrar att installera
och starta `robotd` för första gången.

### Startfördröjning vid strömsättning (2026-09-20)

Känd risk från er erfarenhet med RControlStation: programmet på Pi:n
hoppade igång innan resten av systemet var redo (5G-modemet tar ~90s att
koppla upp sig, CarController-kortet ~60s att starta), vilket gav problem
tidigare och krävde manuell strömcykling för att lösa. Löst i två lager:

1. **`robotd.service`** har nu `ExecStartPre=/bin/sleep 90` — robotd
   startar inte förrän 90 sekunder efter att systemd skulle kört den
   (dvs. efter boot). Gäller vid varje omstart av tjänsten, inte bara vid
   boot (rimligt — ett `Restart=on-failure`-återförsök direkt efter en
   krasch ska inte heller hamra på Car_Client innan den hunnit starta om).
2. **Anslutningen till Car_Client i `main.rs`** försöker nu upp till 10
   gånger med 3 sekunders mellanrum (~27s extra marginal) istället för
   att ge upp efter ett enda misslyckat försök. Om Car_Client av någon
   anledning tar längre tid en enskild dag än de 90 fasta sekunderna,
   hittar robotd den ändå istället för att permanent markera
   `car_client_ok: false` för resten av körningen.

Om Car_Client fortfarande inte svarar efter alla försök fortsätter
`robotd` köra ändå (belysning, OLED, nätverket mot klienten fungerar
oberoende av CAN-länken) — det är bara CAN-relaterad funktionalitet som
då saknas, inte hela tjänsten som kraschar.

## 19. Verifierat mot de riktiga installationsskripten (2026-09-20)

Användaren delade de faktiska filerna `install_pi.sh`, `install_allt.sh`
och `start_car.sh` från `lordajz-cmyk/rise_sdvp` (tidigare hade jag bara
läst READMEns beskrivning av dem). Flera saker gick från antaget/gissat
till bekräftat:

**Bekräftat, matchar redan vår kod exakt:**
- USB-enhetssökväg: `/dev/vehicle` (vår `usb_device_path`-default)
- Car_Client-flaggor: `-p /dev/vehicle --useudp --logusb --usetcp
  --tcprtcmserver 8200 --tcpubxserver 8210 --setid 4` — identiskt med vad
  vi redan antog utifrån tidigare loggutskrifter
- udev: VID/PID `0483:5740` → symlink `car`/`vehicle` (STM32, bekräftar
  tidigare analys). GPS-modulen (`1546:01a8`/`01a9`) → `ublox`/`rtk`,
  separat enhet.

**Nytt, bekräftat:**
- Car_Client körs via en riktig systemd-tjänst, **`car_client.service`**
  (inte bara en manuellt startad screen-session) — som i sin tur startar
  ett skript (`start_car.sh`) som startar en `screen`-session vid namn
  `car`. Tre lager: systemd → skript → screen → Car_Client-binären.
- En tredje tjänst, **`car_rtk.service`**, kör RTKLIB:s `str2str` och
  hämtar RTK-korrektioner från **Swepos** (kräver ett Swepos-konto/
  användarnamn+lösenord) till en lokal TCP-port (1234) som u-blox-modulen
  läser. Inget `robotd` behöver bry sig om, men RTK-precision (cm-nivå)
  är alltså beroende av det kontot — utan det blir det bara vanlig
  GNSS-noggrannhet (~1-2m).

**⚠️→✅ Motsägelse hittad och åtgärdad:** `start_car.sh` (både den
fristående filen och den `install_pi.sh` genererar i steg 5) hade
`sleep 10`, INTE de 90 sekunder användaren beskrev muntligt som lösningen
på det ursprungliga "startar för snabbt"-problemet med RControlStation.
Användaren gav uttryckligt tillstånd att ändra i referenskopiorna
("Du får ändra va du vill. det är mitt repo.") — **rättat i
`reference/rise_sdvp/start_car.sh` och `install_pi.sh`** till `sleep 90`.
Inte automatiskt tillämpat på den andra forken på GitHub (det kräver en
egen commit där) — se den mappens README för hur man för över fixen dit,
eller till en redan körande maskin.

**Uppdaterat i vårt install_pi.sh utifrån detta:**
- `robotd.service` har nu `After=car_client.service` för rätt
  startordning (påverkar inte om tjänsten saknas — systemd ignorerar
  tyst ett `After` mot en okänd unit).
- Efterinstallationskontrollen kollar nu `systemctl is-active
  car_client.service` och `car_rtk.service` (de bekräftade riktiga
  namnen) istället för att bara leta efter en screen-session vid namn
  `car` (fallback kvar för den som kört Car_Client manuellt utan
  tjänsten).
- Felmeddelandet i `robotd` om Car_Client-anslutningen misslyckas
  refererar nu till `car_client.service` istället för bara
  screen-sessionen.
