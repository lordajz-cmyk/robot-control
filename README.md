# Robot Control — återskapat projekt

Se **PROJECT_SPEC.md** för hela bakgrunden, alla beslut och vad som är kvar att
bekräfta mot hårdvaran. Den filen är det viktigaste dokumentet i det här repot
— läs särskilt **§14** för arkitekturen (WireGuard-baserad, se nedan) innan du
läser äldre avsnitt som pratar om en reläserver (skrotad, se §14).

## Struktur

- `relay-protocol/` — delade meddelandetyper (styrkommandon, status,
  VESC-mappning). Inga beroenden på nätverk/kryptografi längre sedan
  arkitekturomläggningen till WireGuard (PROJECT_SPEC.md §14).
- `robotd/` — huvudtjänsten som körs på Raspberry Pi:n:
  - lyssnar direkt på robotens WireGuard-IP (`server.rs`) — klienten
    ansluter rakt in, inget relä
  - ansluter till det redan körande `Car_Client` (se PROJECT_SPEC.md §13)
  - watchdog, mjuka ramper, GPIO-belysning (`gpio_lighting.rs`),
    OLED-status i den stängda lådan (`display.rs` — internet, om
    Car_Client-länken kör, USB-kontakt, batteri/senaste fel; bara synlig
    när man öppnar locket, inte en stor synlig skärm)
- `robotctl/` — litet CLI-verktyg på Pi:n. Sedan WireGuard-omläggningen är
  parkoppling = att bli WireGuard-peer (era egna `wireguard.sh`-skript),
  inte något `robotctl` behöver göra — se filens docstring.
- `mock-robotd/` — en låtsas-robot (samma protokoll, påhittad telemetri)
  för att testa hela kedjan utan hårdvara. Är, precis som riktiga
  `robotd`, en server man ansluter till.
- `client/` (paketnamn `robotstyrning`) — Ubuntu-appen. Svart
  anslutningsskärm (robotens VPN-IP eller namn) → helskärm med kamera →
  AKTIVERA-knapp → färgad ram.
- `relay-server/` — **arkiv, byggs inte längre.** Den gamla, mer
  komplicerade reläserver-lösningen (Ed25519-parkoppling m.m.) som byttes
  ut mot WireGuard. Ligger kvar bara som referens.

## Installation med skript

- `scripts/install_pi.sh` — körs PÅ Raspberry Pi:n: installerar systempaket,
  Rust, bygger `robotd`/`robotctl`, lägger dem i `/usr/local/bin`, skapar
  `/etc/robotd/config.json` (med `bind_addr` — sätt den till robotens
  WireGuard-IP + port), installerar (men aktiverar inte) en systemd-tjänst.
- `scripts/deploy_to_pi.sh <användare@pi-adress>` — körs på ER dator:
  kopierar projektet till Pi:n via rsync/scp och kör `install_pi.sh` där
  över SSH i ett steg.
- `scripts/install_client.sh` — körs på styrdatorn (Ubuntu): installerar
  systempaket, Rust, bygger klienten + mock-robotd, lägger dem i
  `~/.local/bin`, skapar en skrivbordsgenväg för klienten.

Alla tre är skrivna nu, innan riktig hårdvara finns tillgänglig — de är
väntade att fungera men otestade i praktiken. Första gången ni kör dem,
kör dem manuellt och läs igenom vad som skrivs ut, snarare än att lita
blint på att allt går perfekt första gången.

## Testköra hela systemet utan hårdvara

Eftersom ingen robot finns färdigbyggd än går det att testa hela
programvarukedjan — anslutningsskärm, AKTIVERA-flödet, färgad ram,
Settings-vyns VESC-skanning — på en vanlig Ubuntu-dator, ingen Pi eller
CAN-buss behövs, och sedan WireGuard-omläggningen räcker det med **två**
terminaler istället för tre:

```bash
# Terminal 1: låtsas-roboten (hittar på telemetri, pratar riktigt protokoll)
cargo run -p mock-robotd -- --bind 0.0.0.0:9000

# Terminal 2: klienten
cargo run -p robotstyrning
# skriv "127.0.0.1" på anslutningsskärmen
```

Vill man testa över den riktiga WireGuard-tunneln istället: kör
`mock-robotd` (eller riktiga `robotd`) på en maskin som är uppe på VPN:et,
och skriv dess `192.168.200.x`-adress på anslutningsskärmen istället för
`127.0.0.1`.

Det här testar INTE om styrningen faktiskt fungerar mot er hydraulik/VESC —
bara att nätverket och gränssnittet i övrigt hänger ihop som tänkt. Bra sätt
att hitta buggar (som gamepad-buggen 2026-09-16, se PROJECT_SPEC.md) utan
att behöva ha hårdvaran uppkopplad.

## Bygga

```bash
cargo build --workspace
```

`robotd` kräver `libsocketcan`/CAN-headers samt GPIO/I2C-bibliotek — bygg den
delen direkt på Raspberry Pi:n (eller korskompilera). `client` kräver
GStreamer-utvecklingspaket för videot när `video.rs` kopplas ihop på riktigt.
(`relay-server` är inte med i workspacet längre, se §14 i PROJECT_SPEC.md.)

## Vad som är verkligt kod vs. skelett

- **Klar logik, testad:** `robotd/src/watchdog.rs`, `robotd/src/ramp.rs`,
  `robotd/src/vesc_can.rs` (parsing/frame-bygge), `robotd/src/serial_bridge.rs`
  (paket-inramning + CRC16, verifierad byte-exakt mot riktig trafik, se
  PROJECT_SPEC.md §13).
- **Ihopkopplat end-to-end, väntar på verifierade CAN-kommando-ID:n:**
  `robotd/src/server.rs` ↔ `client/src/net.rs` (direktanslutning över
  WireGuard, se §14) — själva nätverket/UI:t fungerar, men `robotd` svarar
  ännu med tom VESC-lista och skickar ingen riktig StatusUpdate, eftersom
  Car_Client-protokollets kommando-ID:n inte är bekräftade än (§13).
- **Väntar på hårdvara/nät för att kopplas ihop:** CAN-detektion
  (`can_iface.rs` — väg B, se §11/§13), WebRTC-video (`client/src/video.rs`
  — mest oprövade delen).
- **Strukturellt klart, väntar på hårdvara för att verifieras:**
  `robotd/src/gpio_lighting.rs` (GPIO17 → relä, aktiv-hög/låg okänt än),
  `robotd/src/display.rs` (SSD1306 OLED, 128x64 antaget, I2C-adress ej
  bekräftad).

## Nästa steg

Kör kommandona i PROJECT_SPEC.md §11 på Pi:n (kopplad till CarController-
kortet, inga VESC:ar/hydraulik behöver vara inkopplade) för att bekräfta
Car_Client-protokollets kommando-ID:n — det är den sista stora
osäkerheten. Klistra in resultatet i chatten, eller peka en Claude
Code-session hit med den här filen + PROJECT_SPEC.md.
