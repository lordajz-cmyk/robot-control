# Testrapport — Dator 2 (klienten `robotstyrning`)

Datum: 2026-09-19. Test enligt `mock_test.md`, mot mock-robotd på samma maskin
(`127.0.0.1:9000`, inte VPN). Dator 2 skrev bara den här filen och gjorde en
kodändring (se "Ändringar"). `PROJECT_SPEC.md` är orörd.

Metod: klienten byggdes och kördes på riktigt, och användaren klickade i GUI:t
(anslutning, belysning, Settings). Jag kunde inte själv styra eller se
GUI-fönstret (inga automationsverktyg, Wayland), så det som kräver visuell
kontroll bygger på användarens rapport eller på kodgranskning, och det anges
nedan. Nätverksdelen testades dessutom headless med ett litet Rust-program som
använder de riktiga `relay-protocol`-typerna (alltså exakt trådformat).
Programmet ligger i sessionens temporära scratchpad och är inte incheckat.

## Ändringar

- `client/Cargo.toml`: lade till `futures-util = "0.3"` och `tracing = "0.1"`.
  `client/src/net.rs` använder båda men de saknades (rest från omskrivningen i
  spec §14). Utan dem gick klienten inte att kompilera.
- Ingen annan kod ändrad. Fel 1–6 nedan är **inte** åtgärdade.
- Miljö: `cargo` låg inte i PATH och rustup saknade default-toolchain.
  `rustup default stable` kördes (toolchain 1.98.1 fanns redan installerad).

## Resultat mot checklistan

| Punkt | Resultat |
|---|---|
| Bygge | OK efter Cargo.toml-fixen. Kvarvarande varningar: `f32`-fallback `main.rs:208`, oanvänd kod i `video.rs`, `request_view_code`, `editing_other_name` |
| Ansluta, grön ram | Ansluter. Ramen syntes **inte** i färg (svart) — se fel 2 |
| Hastighet/batteri/ping uppe till vänster | Hastighet `0.0` och Ping `0 ms` sågs. Batteri och att det sjunker över tid: inte rapporterat, testades inte |
| AKTIVERA klickbar, försvinner | Klickbar, försvinner **inte** (ingen dosa) — se fel 3 |
| Gamepad | Testades inte, ingen dosa inkopplad (`/dev/input/js*` saknas) |
| Belysningsknapp | OK (verifierat av användaren) |
| Settings öppnas, Skanna | Vyn öppnas och Skanna går att trycka, men den **står kvar på "Skannar..."** och VESC 32/44/97/88 visas aldrig — se fel 1 |
| Tilldela roller, Spara | Testades inte i GUI (Skanna gav ingen lista). Spara-flödet testades headless, se nedan |
| Stäng Settings, tillbaka på körskärmen | Ej rapporterat |
| En förare i taget | OK på protokollnivå: förare #2 fick `ConnectDenied { reason: "roboten körs redan av någon annan" }`, en åskådare släpptes in. Men UI:t visar inget felmeddelande — se fel 5 |
| Frånkoppling och ny anslutning | OK: 50 snygga och 50 abrupta cykler utan nekning (se nedan) |

### Headless-test mot mock-robotd

- **Helflöde:** `Connect` → `ConnectedAsDriver`; `Control` (lights=true);
  `ScanVescBus` → `VescBusResult` med 32/44/97/88; `SaveVescProfile` →
  `VescProfileSaved`; ett andra `Control` med lights=false; snygg stängning.
- **Snabba cykler:** 50 anslut/koppla från med Close-frame: ok=50, nekade=0,
  fel=0, totalt 41 ms, långsammaste 3,2 ms. 50 med abrupt `drop` utan Close:
  ok=50, nekade=0, fel=0, totalt 26 ms.
- **Samtidighet:** 20 samtidiga förar-anslutningar gav exakt 1 förare och 19
  nekade. Efter att alla släppts kunde en ny förare ansluta direkt, så
  `driver_connected` fastnar inte.
- **Rå JSON på tråden** (svar på Dator 1:s fråga 3, ser vettig ut, även "Annat"
  och `åäö`):
  - `{"Control":{"throttle":0.6,"steering":-0.3,"accessory_a":0.0,"accessory_b":0.0,"lights":true,"activated":true,"timestamp_ms":1}}`
  - `"ScanVescBus"`
  - `{"SaveVescProfile":{"roller":{"44":"DriftHoger","97":"Styrning","32":"DriftVanster","88":{"Annat":{"namn":"Lastarm åäö","input":"l1_l2","min":-1.0,"max":1.0}}}}}`
- Testerna gav Dator 1 drygt 100 extra `Klient ansluten/frånkopplad`-rader i
  loggen (mina anslutningar, inget fel).

## Fel och brister i klienten

### 1. Klienten slutar läsa från nätverket när Settings är öppen — bekräftat

`poll_updates()` anropas bara på körskärmen (`client/src/main.rs:177`). I
`Screen::Settings`-grenen (`main.rs:98-104`) töms uppdateringskön aldrig.
Kön har plats för 32 meddelanden (`net.rs:76`). När den är full blockerar
`updates_tx.send().await` hela nätverkstasken (`net.rs:172-210`).

Belägg:
- Användaren: Skanna-knappen står kvar på "Skannar..." och ingen VESC dyker upp.
- `ss -tn`: klientens Recv-Q växte stadigt, ~600 byte per 2 s (9862 → 10451
  → 11040), dvs klienten läser inte längre från socketen.
- Skanningssvaret sätts i `NetLink::poll_updates` (`vesc_scan_result`), men
  `SettingsView::poll_net` läser bara det fältet — den kan aldrig få svaret
  så länge `poll_updates` inte körs.

Följder:
- Settings kan inte visa VESC-listan, och därmed går roll-/Spara-flödet inte
  att nå i GUI:t.
- Inga styrkommandon skickas heller medan Settings är öppen. På en riktig
  robot skulle watchdogen (150/400 ms) slå till.
- Serverns skrivbuffert fylls på. Med riktig robotd bör man kolla att
  `server.rs` inte blockerar sin egen loop på en långsam klient.

Föreslagen fix (inte gjord): anropa `self.net.poll_updates()` även i
Settings-grenen i `update()`, och fortsätt skicka `Control` medan Settings är
öppen.

### 2. Den färgade ramen är osynlig — sannolik orsak, ej visuellt bekräftad

Användaren såg en svart ram. `CentralPanel`-framen har stroke 6 px utan
marginal (`main.rs:207-209`). Sedan ritar `ui.painter().rect_filled(rect, …)`
(`main.rs:214`) kamerabildens bakgrund över hela panelen, ovanpå framen. Den
halva av strokens bredd som ligger innanför fönstret täcks, och den andra
halvan hamnar utanför fönstret. Förslag: rita ramen efter innehållet (eller
`inner_margin` ≥ ramtjocklek).

Följd: grön/gul/röd/lila-signalen (spec §7) når aldrig användaren.

### 3. Ingen dosa: lila ram och AKTIVERA som inte fastnar

Utan kontroll är `gamepad_connected = false` direkt på körskärmen. Då
nollställs `activated` varje bildruta (`main.rs:182-184`) och ramen ska bli
lila (`main.rs:82-84`). Väntat beteende enligt spec (§6/§7), men aktiverings-
flödet går alltså inte att testa utan riktig dosa. `gilrs::new()` kraschar
inte utan kontroll (verifierat: appen startar, tom stderr).

### 4. Statusintervall vs. gul-tröskel, och fejkad ping

Mocken skickar status var 500 ms (`mock-robotd/src/main.rs:105`), men
`link_quality` blir gul om det gått mer än 300 ms sedan senaste meddelande
(`net.rs:121`). Ramen skulle blinka grön/gul konstant, även på en perfekt
anslutning. Ping är hårdkodat till `Pong(0)` (`net.rs:184,198`), så "Ping"
visar alltid 0 ms. Klienten skickar ingen riktig ping. Förslag: höj gul-
tröskeln till ≳ 1,5 × statusintervall, eller skicka status/ping oftare, och
mät riktig round-trip.

### 5. Nekad anslutning visar inget i UI:t

`ConnectionState::Denied` och `deny_reason` (`net.rs:32`) läses inte av något
i `main.rs`/`settings.rs`. Andra instansen hamnar på körskärmen utan text.
Det finns ingen väg tillbaka till anslutningsskärmen efter nekad eller
avbruten anslutning (`Screen::Connect` sätts aldrig igen). Testets krav på
"tydligt felmeddelande" är därmed **inte uppfyllt i GUI:t**, även om skälet
skickas korrekt.

### 6. Robotlistan

Sparas redan vid anslutningsförsöket (`main.rs:153-156`), före lyckad
anslutning, så stavfel hamnar i listan. Ingen borttagning finns. Filen
`~/.config/robotstyrning/kanda_robotar.json` skrevs korrekt
(`["127.0.0.1"]`). Att den läses in och visas efter omstart av klienten är
verifierat i koden men inte i GUI:t.

## Svar på testfrågorna för Dator 2

1. **Kompilerar rent?** Nej först: 5 fel (E0432 ×1, E0433 ×3, E0599 ×1), alla
   från de två saknade beroendena `futures-util` och `tracing`. Fixat, därefter
   rent bygge med 6 varningar.
2. **Tokio-runtimen:** ingen deadlock i själva runtimen och appen startar/
   stängs normalt. CPU i vila 11–17 % i debugbygge på grund av 30 fps-loopen
   (`request_repaint_after(33 ms)`, `main.rs:106`). Däremot ger fel 1 ett
   praktiskt hängande nätverkslager i Settings.
3. **`gilrs` utan kontroll:** kraschar inte och ger inget fel. Med kontroll:
   testades inte. Obekräftad risk: `LeftTrigger2`/`RightTrigger2`
   (`gamepad.rs:47,51`) kan på vissa PS4-drivrutiner komma som axlar och inte
   som knappar.
4. **UI-timing:** 500 ms statusintervall är för glest mot 300 ms-tröskeln (fel 4).
   5-minuters-timeouten är rimligt implementerad (`main.rs:22,179`, räknas
   från senaste spakrörelse > 0,02) men kontrolleras bara på körskärmen, och
   gamepaden pollas inte i Settings. Väntades inte ut.
5. **Robotlistan:** sparar korrekt (fel 6). Inläsning efter omstart testades
   inte i GUI.

## Ej testat (ärligt)

- Gamepad/spakar/axel- och knappmappning (ingen dosa).
- AKTIVERA-flödets faktiska fastnande, 5-minuters-timeout, lila ram med
  riktig dosa.
- Grön/gul/röd ramfärg och deras byten, ljudsignaler (ingen finns i koden att
  granska).
- Batteriets nedräkning över tid i GUI:t.
- Rolltilldelning och Spara i GUI:t (blockerat av fel 1).
- Att robotlistan visas efter omstart.
- Över VPN/WireGuard (medvetet: bara samma maskin).
- Avbruten anslutning utan TCP-avslut: mocken har ingen timeout/keepalive, så
  `driver_connected` kan stå kvar tills TCP-timeouten löper ut om VPN:et dör
  tyst. Bör kollas i `robotd/src/server.rs`.
