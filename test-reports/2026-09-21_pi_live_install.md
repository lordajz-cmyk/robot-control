# 2026-09-21 — Live-installation av robotd på riktig Pi (RobAnt)

Genomförd av Claude Code över SSH (`robant@192.168.200.10`, över wg0) enligt
`pi_install_test.md`. Roboten var upphissad och stillastående, bekräftat av
användaren. Pi:n startades **inte** om vid något tillfälle (pågående
programmering — användaren bad uttryckligen om det).

## Steg 1 — läget innan något rördes

- Raspberry Pi, Debian 13 (trixie), kernel 6.18.39+rpt-rpi-v8, aarch64. 7,6 GiB RAM, 209 GB ledigt.
- `car_client.service`: enabled + active (screen-session `car`). Startas med:
  `./Car_Client -p /dev/vehicle --useudp --logusb --usetcp --tcprtcmserver 8200 --tcpubxserver 8210 --setid 4`
- `rtklib`-screen kör `rtkrcv` (start_ublox). `car_rtk.service` rapporterades active av post-install-kontrollen.
- `/dev/vehicle` och `/dev/car` → `ttyACM1`.
- `wg0` uppe, IP `192.168.200.10/24`. (`operstate` visas som `unknown` — normalt för WireGuard.)
- `eth0` 192.168.1.239, `wlan0` nere.
- `lsusb`: u-blox GNSS, STM32 Virtual COM Port (0483:5740, CarController), ST-LINK/V2
  (används för att strömsätta kortet, enligt användaren), Logitech C922 webbkamera.
- Verktyg som fanns: git, gcc, v4l2-ctl, tcpdump, rsync, raspi-config. Rust saknades.
- Användaren `robant` är i grupperna sudo, dialout, gpio, i2c, video m.fl.
- I2C är **avstängt**: `/dev/i2c-1` saknas, `dtparam=i2c_arm=on` är bortkommenterad i `/boot/firmware/config.txt`.

## Steg 2–3 — installation

- Repot kopierades med rsync till `~/robot-control` (exkl. `target`, `.git` och skräpkatalogen `{robotd`).
- Kördes med `ROBOT_ID=robant BIND_ADDR=192.168.200.10:9000 bash scripts/install_pi.sh`
  (ROBOT_ID valdes av Claude utifrån användarnamnet — inte bekräftat av användaren; ändras i `/etc/robotd/config.json`).
- Systempaket: nyinstallerade `can-utils`, `libcap-dev`, `libssl-dev`, `libudev-dev`;
  uppgraderade `curl`, `libcap2`, `libcap2-bin`, `libcurl3t64-gnutls`, `libcurl4t64`.
- Rust installerades via rustup i `~/.cargo` (användarnivå).
- `cargo build --release -p robotd -p robotctl`: **lyckades på 5 min 32 s**, en enda varning (`unused import: response::IntoResponse`). Inga kompileringsfel.
- Binärer i `/usr/local/bin/robotd` och `/usr/local/bin/robotctl`.
- `/etc/robotd/config.json` skapades (robot_id=robant, bind_addr=192.168.200.10:9000, car_client_addr=127.0.0.1:8300, usb_device_path=/dev/vehicle).
- `robotd.service` installerad men **inte aktiverad** (`disabled`), som avsett.
- Slutresultat: `INSTALL_EXIT=0`. Hela utskriften finns kvar i `~/install.log` på Pi:n.

Efterinstallationskontrollen:

| Kontroll | Resultat |
|---|---|
| wg0 | `[SAKNAS]` — **falskt larm**: skriptet testar `operstate = up`, men WireGuard rapporterar alltid `unknown`. wg0 är i själva verket uppe. |
| /dev/i2c-1 | `[SAKNAS]` — se avvikelse 1 |
| /dev/vehicle | `[OK]` |
| car_client.service | `[OK]` |
| car_rtk.service | `[OK]` |

## Steg 4 — manuellt robotd-test (45 s, förgrunden, som användaren `robant`)

Kört med `RUST_LOG=info timeout -s INT 45 robotd`. Loggen:

```
robotd startar som 'robant'
Ansluten till Car_Client på 127.0.0.1:8300
Ansluten till Car_Client-länken (försök 1/10).
Belysningsrelä redo på GPIO17.
WARN Ingen OLED-display tillgänglig (kunde inte öppna I2C-bussen /dev/i2c-1: No such file or directory) — fortsätter utan.
robotd lyssnar på 192.168.200.10:9000
```

- [x] Startade utan krasch, inga paniker under 45 s.
- [x] Anslöt till Car_Client på första försöket (port 8300) — inga backoff-rader behövdes.
- [x] OLED-fel väntat och hanterat (fortsätter utan skärm).
- [x] GPIO17 (belysningsrelä) initierades utan fel. Reläet är inte inkopplat så vitt känt — ingen effekt observerad.
- [x] Efteråt: ingen `robotd`-process kvar, port 9000 ej lyssnande, Car_Client fortsatt lyssnande på 8300, `car_client.service`/`car_rtk.service` active, `robotd.service` disabled.
- Inga styrkommandon skickades. Ingen kod för det finns ännu (§13).
- Observerat men ej testat: att robotd och Car_Client hade samtidig TCP-anslutning på 8300 utan synligt problem under 45 s (Car_Client-loggen kontrollerades inte i detalj, bara att tjänsten förblev active).

## Steg 4b — klienttest mot riktiga robotd (användarens dator → Pi över wg0)

`robotd` kördes ca 10 min (startad med `timeout -s INT 600`, stoppades manuellt efteråt), klienten
`robotstyrning` byggdes lokalt (release) och startades. Rapporterat av användaren:

- [x] Anslutning till `192.168.200.10` fungerar.
- [x] Dosan på → gult läge + AKTIVERA-knapp. Dosan av → lila läge + "ingen dosa ansluten". Dosan på igen → gult + AKTIVERA-knapp åter.
- [ ] Kamerabild: **fungerade inte vid det första klienttestet — koden fanns inte.** `client/src/video.rs` var en stubb
      (`handle_signal` TODO, `latest_frame` alltid `None`) och `robotd` hade ingen kameraströmkod; `VideoSignal` i
      `server.rs` loggas/ignoreras. **Byggdes senare samma dag, se "Steg 6 — videoström" nedan.**
- robotd-loggen: enda händelsen efter start var `Settings-vy begärde VESC-skanning — ingen riktig CAN-koppling än.`
  Inga fel eller paniker.

Kamerahårdvaran i sig fungerar: Logitech C922 syns som `/dev/video0` (YUYV 640x480 @30 fps m.fl.). En stillbild
togs med `ffmpeg -f v4l2 … -frames:v 1` och visade en rimlig utomhusbild (himmel, träd, byggnader).
GStreamer-plugins (base/good/bad/libav) finns på Pi:n; `gst-inspect-1.0`/`gst-launch-1.0` saknas (paketet
`gstreamer1.0-tools` är inte installerat). Ping över wg0 mätte ca 200–350 ms RTT.

## Steg 5 — §13-fångst (tcpdump av riktigt styrkommando)

**Gjord.** `sudo tcpdump -n -i any -w vesc_capture.pcap "port 8300 or port 8200 or port 8210"`,
180 s, 9 833 paket, 0 tappade. Filen finns i `test-reports/2026-09-21_car_client_capture.pcap`
(original kvar på Pi:n i `~/vesc_capture.pcap`). Analys med `2026-09-21_analyze_pcap.py`,
utskrift i `2026-09-21_car_client_capture_analys.txt`.

Trafiken på 8300 gick mellan Pi:n och 192.168.200.99 (antas vara användarens dator med RControlStation — inte verifierat), **enbart TCP**
(inget UDP på 8300). Alla 10 496 ramar har giltig CRC16-CCITT (poly 0x1021, init 0) — ramningen
`0x02 len payload crc 0x03` är därmed bekräftad även för riktig trafik i båda riktningarna, 0 resync-byte.

Payloadstruktur: `[bil-ID][kommando][data...]`. Bil-ID = `0x04` överallt (matchar `--setid 4`).

| cmd | antal | riktning | innehåll |
|---|---|---|---|
| `0x78` (120) | 4 477 + 4 478 | klient→Pi: 2 B (`04 78`), Pi→klient: 189 B | tillståndspollning + tillståndssvar (~25 Hz) |
| `0x59` (89) | 597 | klient→Pi, 2 B (`04 59`) | periodisk ping/poll, innebörd okänd |
| `0x3f` (63) | 894 | Pi→klient, 84 B | vidarebefordrad NMEA (`$GNGGA…`) |
| `0x00` (0) | 25 | Pi→klient, 13 B | text `Activity: 0` |
| **`0x7d` (125)** | **25** | **klient→Pi, 7 B** | **troligen styrkommando** (se nedan) |

`0x7d`-ramarna: `04 7d 00 <int32 big-endian, signerad>`. Kom i två skurar, vid t≈161–163 s och t≈173 s
efter fångststart, 25 ramar totalt. Observerade värden, i ordning:

- Skur 1 (t≈161–163 s): −6, 1500, 6, −6, 524, 1500, 1465, −6, −888, −1500, −1300, −6
- Skur 2 (t≈173 s): 1039, 2529, 1314, 4451, 1235, 4804, 4843, 4804, 1274, 921, 3784, 20, −20

Tolkningen är **inte bekräftad**. Observationer: värdena är signerade och passerar 0; ±1500 verkar vara
ett tak i skur 1 men skur 2 når 4843, så de kan höra till en annan axel/skala. Mellanbyten efter
kommandot (`00`) är alltid 0 — kan vara ett läges-/index-fält eller den höga byten av ett 40-bitarsvärde.
För att avgöra vilken axel som är vilken (styrning/gas/hydraulik) och vad skalan är krävs en logg över
vad som gjordes i RControlStation under fångsten (se Öppna frågor).

Notera även: specen (§13) nämnde `cmd 221`; det syns **inte** i den här fångsten, som i stället visar
`cmd 63` för NMEA. Ingen av de tidigare noterade kommandonamnen kunde verifieras mot källkod —
`reference/` innehåller bara skript, inte rise_sdvp:s `datatypes.h`.

## Steg 6 — videoström (byggd och testad 2026-09-21)

**Resultat:** Kamerabilden syns i klienten, "inte märkbar fördröjning" (användarens bedömning, ej mätt), och
dosan-av → programmet avaktiveras som avsett. `robotd` levererade **15,0 bilder/s vid 904–909 kbit/s under 9+
minuter**, utan varningar, fel eller återanslutningar; ffmpeg ~27 % av en kärna, Car_Client/RTK opåverkade.

**Bakgrund (mätt samma dag):** wg0-länken Pi→dator gav 1–5 Mbit/s med stora svängningar (ping 200–350 ms).
Kamerans MJPEG i 720p är 15,7 Mbit/s och gav ~2 bilder/s i en första demo; omkodad MJPEG i 800x448 gick att se
i webbläsare vid ~1 Mbit/s. Pi 4B har hårdvaru-H.264 (`h264_v4l2m2m`), som ger 720p/576p-video vid ~0,9 Mbit/s.

**Vad som byggdes**
- `relay-protocol/src/video.rs` (delat): trådformat (`RVD1` + 13 B header + H.264 access unit), `AuSplitter`
  (delar Annex-B-ström på AUD), `contains_idr`. 7 enhetstester (bl.a. delning oavsett chunkstorlek 1–64 byte).
- `robotd/src/video.rs`: TCP-server på `bind_addr`-värden, port `video.port` (9001). Kameran (ffmpeg) körs BARA
  medan någon tittar (3 s karens). Bounded kö per klient; efter → släpp bilder, hoppa till nästa nyckelbild
  (≤1 s). Begränsad TCP-sändbuffert (128 kB) så en långsam länk inte lagrar sekunder av gammal video. ffmpeg
  startas om med backoff om den dör eller hänger (5 s utan bild). Ny `video`-sektion i `config.json`
  (alla fält har standardvärden; befintliga config.json fungerar oförändrade).
- `client/src/video.rs`: egen OS-tråd, återanslutning med backoff, avkodning med `openh264` (0.9.8, byggs från
  källkod av cargo — kräver INTE nasm eller apt-paket), trasig bildruta → ny avkodare. `main.rs`: bild ritas
  proportionellt, "BILDEN FRUSEN"-varning om ingen ny bild på 1,5 s, videostatus i statusytan.
- **Beteendeändring:** `can_activate()` — `camera_ok` var `true` (TODO), är nu `video.is_live()`. AKTIVERA i
  CAM-läge kräver alltså en levande bild; LOS-läge är oförändrat.
- `scripts/install_pi.sh`: lade till `ffmpeg` och `v4l-utils` i apt-listan.
- Den nya `robotd` är installerad i `/usr/local/bin/robotd` (tjänsten fortfarande `disabled`).

**Kända begränsningar / att göra**
1. **Mock-robotd har ingen video.** Klienten mot mock visar "Ingen bild ännu" och i CAM-läge går AKTIVERA inte att
   trycka (LOS fungerar). Påverkar testflödena i `mock_test.md`. Åtgärd: låt mock skicka en syntetisk H.264-ström
   (openh264-craten har en kodare) — ej gjort.
2. **Ingen adaptiv bitrate.** Fast `bitrate_kbps` (standard 900). Blir länken sämre än så hoppar bilden till nästa
   nyckelbild (upp till 1 s frysning) istället för att sänka kvaliteten. Sänk `video.bitrate_kbps` om det syns.
3. **Videoporten är inte kopplad till titta-koder** (spec §4): alla på VPN:et kan titta. Samma förtroende som en
   förar-anslutning, men åskådarspärren gäller inte video.
4. Nyckelbild ~1 gång/s och SPS/PPS på varje bild (litet överskott, ~30 B/bild) — kan trimmas.
5. Ingen ljudström, ingen inspelning, inget VR-spår (nämns i klientens kommentarer) — inte påbörjat.
6. Latensen är inte mätt objektivt (bara upplevd). Kan mätas genom att filma en klocka bredvid kameran.
7. `client/src/main.rs:456` har en (befintlig) `f32`-varning och `net.rs`/`settings.rs` två andra — orörda.

## Öppna frågor efter fångsten (kräver svar från användaren)

1. ~~Vad gjordes exakt i RControlStation under fångsten?~~ **Besvarad av användaren:** ingen avsiktlig körning.
   Användaren programmerade in knappval för handkontrollen (dosan) i RControlStation, alltså konfigurerade/
   provade dosans knappar och spakar, utan VESC inkopplade. `0x7d`-ramarna (25 st, två skurar) är därför
   sannolikt **dosans spak-/knappvärden** som RControlStation skickade medan hen provade dem, inte ett
   verifierat styrkommando till en VESC. Tolkningen av de fyra värdena (±1500-skalan, ev. 40-bitarsfält)
   förblir obekräftad. **Ny kontrollerad fångst behövs med VESC inkopplade** (förslag i chatten 2026-09-21:
   tomgång → gas fram → stopp → back → styr vänster → styr höger, med tydliga pauser emellan).
2. ~~Är 192.168.200.99 verkligen den dator RControlStation kördes från?~~ **Besvarad: ja, det är användarens dator.**
   (Användaren nämnde också att hen samtidigt testar att lägga in fler funktioner i RControlStation, så
   `0x7d`-tolkningen bör verifieras mot vad som faktiskt skickades under fångsten.)
3. Vad är `cmd 0x59` (2-byte-ping)? Kan behövas som keepalive i `robotd`.

## Avvikelser och osäkerheter

1. **I2C-steget hoppades över med flit.** `raspi-config nonint do_i2c 0` skulle ha ändrat
   `/boot/firmware/config.txt` på en robot i drift, och `/dev/i2c-1` uppstår först efter omstart
   (som inte fick göras). Konsekvens: OLED-skärmen fungerar inte förrän I2C aktiveras och Pi:n
   startats om vid lämpligt tillfälle. Gjordes via en tillfällig `sudo`-shim som senare togs bort.
2. **Skriptet ger falskt larm på wg0** (`operstate` är `unknown` för WireGuard). Bör ändras till
   `ip link show wg0` / `wg show wg0` i `scripts/install_pi.sh`.
3. **ROBOT_ID = `robant`** — gissning, ej bekräftad.
4. Task-beskrivningen i `pi_install_test.md` utgår från Drängen; den här maskinen har användaren
   `robant` och `--setid 4` i Car_Client-flaggorna. Bekräfta att spec/§19 ska gälla även här.
5. `install_pi.sh` uppgraderade `curl` m.fl. som beroenden av `libssl-dev`/`libcap-dev` (5 paket),
   inte bara nyinstallation — ofarligt men värt att veta på en robot i drift.
6. Ett lösenord användes tillfälligt via ett askpass-skript i `~/.shim` på Pi:n; det är borttaget.
   SSH-nyckel är **inte** utlagd på Pi:n — inloggning skedde med lösenord.
7. Skräpkatalogen `{robotd` i repot (rester av misslyckad brace-expansion) bör tas bort.
