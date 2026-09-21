# claude.md — minnesfil (vad vi har gjort)

Varje avsnitt är märkt med vilken session som skrev det. Skriv bara i ditt
eget avsnitt (Dator 1 / Dator 2), så tappar ingen den andras ändringar.
Bakgrund: `PROJECT_SPEC.md` (särskilt §14), testguide: `mock_test.md`.

---

## [Dator 2] Klienttest 2026-09-19 (robotstyrning mot mock-robotd)

**Roll:** Dator 2 = klienten/föraren enligt `mock_test.md`. Testet kördes mot
mock-robotd på samma maskin (`127.0.0.1:9000`, inte VPN).

**Fullständig rapport:** `testrapport_dator2.md` (checklista, felnummer 1–6
med radnummer, svar på testfrågorna, vad som inte testades).

### Vad jag gjorde
- Läste alla `.md`-filer i repot.
- Fixade byggfel: lade till `futures-util = "0.3"` och `tracing = "0.1"` i
  `client/Cargo.toml` (`net.rs` använde dem, men de saknades). **Enda
  kodändringen.**
- Miljö: `cargo` låg inte i PATH (`export PATH="$HOME/.cargo/bin:$PATH"`),
  och rustup saknade default, så `rustup default stable` kördes.
- Startade klienten. Användaren klickade i GUI:t (anslutning, belysning,
  Settings). Jag kunde inte själv styra GUI:t (Wayland, inga automationsverktyg).
- Körde ett headless Rust-test mot mock-robotd med de riktiga
  `relay-protocol`-typerna: helflöde, "en förare i taget", 50+50 snabba
  anslut/koppla från-cykler, 20 samtidiga förare. Allt gick igenom. Programmet
  låg i sessionens scratchpad och är inte sparat i repot.

### Resultat i korthet
- Nätverk och protokoll fungerar: en förare i taget nekas korrekt, åskådare
  släpps in, `driver_connected` fastnar inte, VESC-skanning och profilsparande
  svarar rätt.
- Belysningsknappen fungerar.
- **Fel 1 (mest allvarligt):** klienten slutar läsa från nätverket när
  Settings är öppen (`poll_updates()` anropas bara på körskärmen,
  `main.rs:177`), så skanningen står kvar på "Skannar..." och inga
  styrkommandon skickas. Bekräftat av användaren och av `ss` (Recv-Q växte).
- Fel 2: den färgade ramen är osynlig (`rect_filled` `main.rs:214` täcker
  den). Fel 3: ingen dosa, så AKTIVERA fastnar inte och ramen borde vara lila.
  Fel 4: statusintervall 500 ms mot gul-tröskel 300 ms, och ping är fejkad
  (`Pong(0)`). Fel 5: nekad anslutning visar inget i UI:t. Fel 6: robotlistan
  sparas redan vid försök.

### Tillstånd nu
- Fel 1–6 är **dokumenterade men INTE åtgärdade** (användaren bad bara om
  dokumentation).
- Inget körs längre: klient och mock-robotd är avslutade, port 9000 är fri.
- Kvar på maskinen: `Cargo.lock` och `target/` (från bygget),
  `~/.config/robotstyrning/kanda_robotar.json` (`["127.0.0.1"]`), och
  `rustup default stable`.
- `PROJECT_SPEC.md` är orörd. Användaren avgör om och hur den uppdateras
  efter att båda sessionernas resultat är sammanställda.

### Nästa steg (förslag)
1. Fixa fel 1: anropa `self.net.poll_updates()` i `Screen::Settings`-grenen i
   `client/src/main.rs`, och skicka `Control` även medan Settings är öppen.
2. Fixa fel 2 (rita ramen efter innehållet) och fel 4 (tröskel/ping), och visa
   `deny_reason` i UI:t (fel 5).
3. Testa igen med en riktig PS4-dosa: AKTIVERA, lila ram, axel- och
   knappmappning (`gamepad.rs`; trigger-knapparna L2/R2 kan vara axlar).
4. Kolla att `robotd/src/server.rs` har timeout/keepalive, annars kan
   `driver_connected` fastna om VPN:et dör tyst.
5. Sammanställ med Dator 1:s resultat och uppdatera `PROJECT_SPEC.md`.

---

## [Dator 1] Servertest 2026-09-19 (mock-robotd, låtsas-roboten)

**Roll:** Dator 1 = servern enligt `mock_test.md`. Körde `mock-robotd --bind
0.0.0.0:9000` på samma maskin som Dator 2 (`127.0.0.1`, inte VPN) och läste
loggen medan klienten testades.

**Fullständig rapport:** `testresultat_dator1.md` (checklista mot loggen, svar på
alla tre testfrågor, tabeller från stresstesterna, rå JSON, vad som inte testades).

### Vad jag gjorde
- Läste alla `.md`-filer i repot.
- Installerade Rust via rustup (cargo saknades helt på maskinen). Inga
  `apt`-paket behövdes, `sudo` användes inte.
- Fixade byggfel i `mock-robotd`: `serde` användes direkt i `main.rs:176` men
  saknades i `mock-robotd/Cargo.toml`. Lade till `serde = "1"` och tog bort två
  oanvända importer (`Instant`, `mpsc`). **Enda kodändringarna.**
- Körde servern med `RUST_LOG=debug` i bakgrunden (loggen i scratchpad).
  Körde binären direkt i stället för `cargo run`, eftersom Dator 2:s
  `cargo build` samtidigt höll byggkatalogens lås.
- Skrev ett eget testprogram (`probe`, Rust + tokio-tungstenite mot de riktiga
  `relay-protocol`-typerna) för egna tester av servern. Det ligger i sessionens
  scratchpad och är inte sparat i repot.
- Stoppade servern först efter användarens besked om att testet var klart och att
  alla frågor var dokumenterade.

### Resultat i korthet
- Alla checklistepunkter bekräftade i loggen, utom gas/styr från en riktig
  gamepad. Den enda `-> gas=`-raden från Dator 2:s körning såg ut att komma från
  ett skript (0.60/-0.30), så jag har inte bekräftat dosan.
- En förare i taget: andra föraren nekas med `roboten körs redan av någon annan`,
  ingen loggrad för den nekade, åskådare släpps in, ny förare kan ansluta efteråt.
- Fråga 2, snabba cykler: 500 rena cykler, 300 abrupta avbrott och 100
  samtidiga förare (exakt 1 accepterad, 99 nekade). Inga läckor: trådar 5 och
  öppna fd 10 före och efter, RSS 7,0 till 7,7 MB.
- Fråga 3, profilen: `SaveVescProfile` serialiseras vettigt och klarar
  round-trip inklusive "Annat" med åäö, citattecken och ☃. Nycklarna blir
  strängar (`"32"`) och ordningen är slumpmässig.
- Servern svarade på varje VESC-skanning och profilsparning i loggen. Det
  stämmer med Dator 2:s **Fel 1**: att skanningen hänger på "Skannar..." beror
  då på klienten (läser inte nätverket i Settings), inte på mock-robotd.

### Brister jag hittade (INTE åtgärdade)
1. `latest_throttle` nollställs aldrig när föraren kopplar från (mock). En ny
   åskådare ser hastigheten klättra mot ~14 km/h utan förare. Fix: nollställ i
   `if is_driver { … }` längst ner i `handle_socket`.
2. Trasiga meddelanden ignoreras tyst (`_ => {}`), utan svar och utan loggrad.
   Profil med `NaN` i min/max (serialiseras som `null`) ger aldrig
   `VescProfileSaved`, så Settings blir stående på "Osparade ändringar". Samma
   mönster finns i `robotd/src/server.rs` (rad 225 och 230).
3. Mocken validerar inte profilen (`min > max`, okänd `input`-sträng och tom
   profil sparas alla).
4. Ingen timeout/ping i mocken. Abrupta stängningar funkar, men "tyst död"
   (kabel ur) testades inte och kan hålla förar-flaggan.

### Tillstånd nu
- Inget körs längre: mock-robotd är stoppad, port 9000 är fri.
- Kvar på maskinen: Rust i `~/.cargo` och `~/.rustup`, `target/` i repot, samt
  den udda tomma mappen `{robotd` i repot (skräp från ett `mkdir` med
  klammerexpansion som inte expanderades, kan tas bort).
- `PROJECT_SPEC.md` är orörd.

### Nästa steg (förslag)
1. Fixa bristerna 1 och 2 i mocken (små ändringar), och kolla samma punkter i
   `robotd/src/server.rs`.
2. Kontrollera att `robotd` har timeout/keepalive så att `driver_connected`
   inte fastnar vid tyst nätavbrott (samma punkt som Dator 2:s steg 4).
3. Kör om testerna efter Dator 2:s klientfixar (särskilt Fel 1) och testa med
   en riktig PS4-dosa.
4. Sammanställ Dator 1 och Dator 2 och uppdatera `PROJECT_SPEC.md` (användaren
   avgör).
