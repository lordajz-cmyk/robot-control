# mock_test.md — testguide för Claude Code (två fönster)

> **Uppdatering 2026-09-19 (andra omgången):** Regressionstestet nedan är
> nu kört. Resultat: **ett riktigt kompileringsfel hittades och är fixat**
> (icke-uttömmande match i både `mock-robotd` och `robotd/src/server.rs` —
> uppstod av misstag när jag lade till loggning av okända meddelanden i
> förra rundan). Ramen syns nu, Settings fungerar med den öppen, ping mäts
> på riktigt, tystnads-timeouten (5s) verifierad från serversidan. Två
> mindre UX-förbättringar också gjorda: "Ingen dosa ansluten" visas nu
> istället för en AKTIVERA-knapp som bara föll tillbaka, och nekade
> förar-anslutningar loggas nu på serversidan också. Full rapport:
> `test-reports/2026-09-19_regressionstest_dator1.md` och `_dator2.md`,
> sammanfattat i `PROJECT_SPEC.md` §16.
>
> **Fortfarande otestat:** allt som kräver en riktig PS4-dosa (AKTIVERA,
> gas/styr, grön ram, lila ram vid frånkoppling) — ingen dosa fanns
> tillgänglig i någon av testomgångarna än.

Det här är instruktioner till två separata Claude Code-sessioner som körs
samtidigt i två olika terminalfönster på samma dator (eller på VPN:et), och
som INTE kan se varandras kontext. Användaren säger åt varje fönster vilken
roll det har. Läs bara din egen sektion (**Dator 1** eller **Dator 2**) i
detalj — men läs "Gemensam bakgrund" nedan oavsett vilken du är.

## Gemensam bakgrund (läs oavsett roll)

Läs **PROJECT_SPEC.md** i repot för full kontext — särskilt §14
(arkitekturen: robotd/mock-robotd är en server, klienten ansluter direkt,
ingen reläserver längre) och §13 (Car_Client-status, fortfarande obekräftad,
men INTE relevant för det här testet).

**Mål med testet:** verifiera att hela mjukvarukedjan (nätverk, UI,
styrschema, VESC-mappning) fungerar som spec:en beskriver — **inte** att
styrningen fungerar mot riktig hydraulik/VESC. Rent mjukvarutest.

**Miljö:** ny, tom labdator. `sudo`-lösenord: `[utelämnat]`. (Tom disk, inget
känsligt på maskinen — se tidigare i konversationen för sammanhang.)

**Repo:**
```bash
cd robot-control   # eller var repot ligger
```

**Rapportera tillbaka i klartext i din egen chatt** — inte genom att skriva
i PROJECT_SPEC.md. Det körs två samtidiga sessioner mot samma filer, och att
båda försöker skriva till samma dokument samtidigt är ett bra sätt att
tappa bort den enas ändringar. Användaren tar med sig resultaten från båda
fönstren och avgör om/hur PROJECT_SPEC.md ska uppdateras efteråt (eller ber
en enskild session göra det när båda testerna är klara).

---

## 🖥️ DATOR 1 — låtsas-roboten (mock-robotd)

Du är servern. Du kör hela testet i det här fönstret utan att bry dig om
vad Dator 2 gör förutom att observera din egen logg.

### 1. Bygg

```bash
cargo build -p mock-robotd
```

Om det inte kompilerar: fixa uppenbara fel själv (typmismatchar,
saknade importer). Fråga användaren om felet verkar bero på ett
designbeslut snarare än ett kodfel. Rapportera exakta felmeddelanden i
svaret, oavsett om du löste dem eller inte.

### 2. Kör

```bash
cargo run -p mock-robotd -- --bind 0.0.0.0:9000
```

Förväntat: skriver ut `Lyssnar på: 0.0.0.0:9000` och väntar. Lämna den
körande genom hela testet — stäng den inte förrän Dator 2 säger sig vara
klar.

### 3. Observera och rapportera

Håll ögonen på loggen medan Dator 2 (klienten) ansluter, kör runt, och
kopplar från. Du behöver inte göra något aktivt mer än att låta processen
stå och köra och läsa vad den skriver ut.

Checklista — bekräfta i loggen:
- [ ] `Klient ansluten som FÖRARE` dyker upp när klienten ansluter
- [ ] Styrkommandon (`-> gas=... styr=...`) dyker upp när klienten rör en
      gamepad/dosa efter att ha tryckt Aktivera (om ingen dosa finns
      inkopplad på Dator 2:s sida går det här inte att se — notera det,
      hitta inte på ett resultat)
- [ ] `Belysning: PÅ`/`AV` dyker upp när klienten klickar belysningsknappen
- [ ] `Settings-vy begärde VESC-skanning` dyker upp, och du svarar med de
      fyra påhittade VESC:arna (32/44/97/88) — det är automatiskt i koden,
      bara bekräfta att det syns i loggen
- [ ] `Settings-vy sparade en profil (N roller)` dyker upp när klienten
      sparar en VESC-mappning
- [ ] Om en tredje anslutning görs medan en förare redan är uppkopplad:
      syns INGET `Klient ansluten som FÖRARE` för den andra (den ska nekas
      tyst på protokollnivå — Dator 2 ska se ett tydligt felmeddelande,
      det är där avslaget syns, inte nödvändigtvis i din logg)
- [ ] `Klient frånkopplad` dyker upp när klienten stänger, och en ny klient
      kan ansluta som förare efteråt (dvs. `driver_connected` fastnar inte)

### 4. Frågor jag (Claude, som skrev koden) vill ha svar på från just dig

1. **Kompilerar allt rent, eller vilka exakta fel dök upp?**
2. **Fryser eller kraschar mock-robotd vid snabba anslut/koppla-från-cykler?**
   Be Dator 2 (eller gör själv om du kan starta flera klient-instanser) att
   ansluta och koppla från upprepade gånger i rad. Håller servern ihop?
3. **Serialiseras VESC-profilen rimligt?** Om du kan slå på mer utförlig
   loggning eller fånga rå JSON: ser `SaveVescProfile`-meddelandet, särskilt
   "Annat"-rollen (fritt namn + gränser), vettigt ut på tråden?

### 5. Rapportera

Skriv en sammanfattning i den här chatten: vad som fungerade, vad som inte
gjorde det, och svaren på frågorna ovan (även "testade inte" om så är
fallet). Vänta med att stänga ner mock-robotd tills du och användaren är
överens om att testet är klart.

---

## 🖥️ DATOR 2 — klienten (robotstyrning)

Du är föraren. Dator 1 kör låtsas-roboten i ett annat fönster — anta att
den redan är igång och lyssnar på `0.0.0.0:9000` innan du börjar (fråga
användaren om du är osäker).

### 1. Bygg

```bash
cargo build -p robotstyrning
```

Kräver systempaket för egui/eframe (X11/Wayland/OpenGL-headers) — se
`scripts/install_client.sh` för listan om länksteget klagar på saknade
`.so`-filer. Fixa uppenbara kodfel själv, fråga användaren vid
designbeslut, rapportera exakta felmeddelanden oavsett utfall.

### 2. Kör

```bash
cargo run -p robotstyrning
```

En egui-app öppnas (svart bakgrund, textfält i mitten). Skriv `127.0.0.1`
(eller Dator 1:s VPN-IP om ni testar över WireGuard istället för samma
maskin) och tryck Enter.

### 3. Gå igenom checklistan

- [ ] Går det att ansluta? Blir ramen grön (inte kvar röd/grå)?
- [ ] Syns hastighet/batteri/ping i övre vänstra hörnet, och sjunker
      batteriet sakta över tid?
- [ ] Går AKTIVERA-knappen att trycka på? Försvinner den när man trycker?
- [ ] Om en gamepad/PS4-dosa finns inkopplad: fungerar spakarna efter
      Aktivera (syns rörelse i Dator 1:s logg)? Utan dosa: notera det,
      hitta inte på ett resultat.
- [ ] Belysningsknappen: ändras texten till "Belysning PÅ" vid klick?
- [ ] Öppna Settings (kugghjulet). "Skanna CAN-bussen" — kommer VESC
      32/44/97/88 upp?
- [ ] Tilldela roller (Drift vänster/höger, Styrning, Annat) åt ett par av
      dem, tryck Spara — försvinner "Osparade ändringar"-texten?
- [ ] Stäng Settings, tillbaka på körskärmen — funkar allt fortfarande?

### 4. Testa "en förare i taget"

Starta en andra instans av klienten (nytt terminalfönster eller bakgrund)
och anslut till samma adress medan den första fortfarande är ansluten.

- [ ] Nekas den andra tydligt, med ett begripligt felmeddelande (inte
      krasch, inte tyst häng)?

### 5. Testa frånkoppling

Stäng ner klienten helt (eller Ctrl+C).

- [ ] Går det att starta en ny klient och ansluta igen direkt efteråt?

### 6. Frågor jag (Claude, som skrev koden) vill ha svar på från just dig

1. **Kompilerar allt rent, eller vilka exakta fel dök upp?**
2. **Håller tokio-runtime-lösningen i `client/main.rs`?** Den startar en
   egen runtime i en bakgrundstråd (`rt.block_on(std::future::pending())`)
   för att kunna göra async-nätverksanrop från egui:s synkrona loop. Några
   tecken på deadlock, hög CPU-användning i vila, eller att appen hänger
   sig vid anslutning/frånkoppling?
3. **Fungerar `gilrs` utan inkopplad kontroll?** Kraschar
   `GamepadReader::new()`, eller misslyckas den bara tyst (tänkt
   beteende)? Med kontroll inkopplad: stämmer axel-/knappmappningen i
   `gamepad.rs` för just den modellen?
4. **Känns UI-timingen rimlig?** Statusuppdateringar (var 500:e ms) — för
   glest/tätt? Bedöm om 5-minuters-AKTIVERA-timeouten (räknas från senaste
   spak-rörelse) verkar rimligt implementerad i koden, även om du inte
   sitter och väntar ut hela tiden.
5. **"Tidigare anslutna robotar"-listan**
   (`~/.config/.../kanda_robotar.json` via `directories`-cratet) — sparas
   och läses den korrekt mellan omstarter av klienten?

### 7. Rapportera

Skriv en sammanfattning i den här chatten: vad som fungerade, vad som inte
gjorde det, och svaren på frågorna ovan (även "testade inte" om så är
fallet).

---

## Regressionstest för fixarna 2026-09-19

Kör det här EFTER den vanliga checklistan ovan (eller istället för den, om
ni bara vill verifiera fixarna snabbt). Riktat specifikt mot det som var
trasigt förra gången — se `PROJECT_SPEC.md` §15 för full bakgrund till
varje punkt.

**Dator 2 (klienten) — kör dessa i ordning:**

1. **Fel 1 (Settings frös nätverket):** Anslut, öppna Settings, tryck
   "Skanna CAN-bussen" **och lämna Settings-vyn öppen i minst 10 sekunder**.
   Kommer VESC 32/44/97/88 upp den här gången (inte bara "Skannar...")?
   Tilldela roller och tryck Spara — försvinner "Osparade ändringar"?
   Kolla i Dator 1:s logg att `Settings-vy sparade en profil (N roller)`
   dyker upp medan Settings fortfarande var öppen i klienten.
2. **Fel 2 (osynlig ram):** Är ramen faktisk synlig i färg (grön vid bra
   anslutning) den här gången, inte svart/osynlig?
3. **Fel 4 (fejkad ping):** Står "Ping" kvar på en rimlig siffra (inte
   ihållande `0 ms`)? Den ska variera lite mellan uppdateringarna eftersom
   den nu mäts på riktigt (ett Ping/Pong-utbyte varje sekund).
4. **Fel 5 (nekad anslutning osynlig):** Försök ansluta till en adress som
   inte svarar (t.ex. `10.255.255.1` eller en port ingen lyssnar på).
   Hamnar ni tillbaka på anslutningsskärmen med ett läsbart felmeddelande
   i rött, istället för att fastna tomt på körskärmen?
5. **Fel 6 (robotlistan sparas för tidigt):** Skriv in en adress som INTE
   går att ansluta till, låt den misslyckas, gå tillbaka till
   anslutningsskärmen. Dyker den felaktiga adressen upp i "Tidigare
   anslutna"-listan? (Ska INTE göra det.) Anslut sedan till en adress som
   faktiskt fungerar — dyker DEN upp i listan efteråt?
6. **Ny tystnads-timeout:** Anslut som förare, dra sedan ur nätverkskabeln/
   stäng av WiFi på klientdatorn (simulerar ett "tyst" VPN-avbrott, inte en
   vanlig avslutning). Vänta ~6 sekunder. Kan en ny klient ansluta som
   förare på Dator 1 efteråt (dvs. har den gamla förar-platsen släppts)?
7. **PS4-dosa, om ni har en till hands:** Anslut, tryck AKTIVERA, rör
   spakarna. Syns rörelse i Dator 1:s logg? Stämmer styrkänslan mot
   spec (höger spak = gas, vänster = styr)? Dra ur dosan (eller stäng av
   Bluetooth) medan ni kör — blir ramen lila, och nollställs AKTIVERA?

**Dator 1 (mock-robotd) — kör dessa:**

1. **Kvarblivet gasvärde:** Låt en förare skicka gas, koppla sedan bort
   abrupt (stäng klienten hårt, inte snyggt). Anslut en ny klient direkt
   efteråt (som förare eller åskådare) — visar statusen `0` km/h nu (inte
   en hastighet som fortsätter klättra av sig själv)?
2. **Loggning av trasiga meddelanden:** Om ni kan skicka något ogiltigt
   till servern (t.ex. via ett eget litet testskript, eller genom att
   avsiktligt krascha en profil-sparning): syns en varningsrad i loggen nu,
   istället för att det bara försvinner tyst?
3. **Tystnads-timeouten:** Se Dator 2:s punkt 6 ovan — bekräfta från er
   sida att loggen visar `Klient tyst i >5s, kopplar ner` (eller
   motsvarande) när det händer.

Rapportera resultatet på samma sätt som förra gången (klartext i er egen
chatt, inte skrivet i `PROJECT_SPEC.md` — användaren sammanställer).

## Runda 3 — med riktig PS4-dosa (kör detta nu)

De två tidigare omgångarna (se `test-reports/`) hittade och fixade sex
UI-buggar plus ett kompileringsfel — men **ingen av dem hade en fysisk
PS4-dosa inkopplad**, så hela AKTIVERA/gas/styr/lila-ram-flödet är
fortfarande overifierat. Det är det den här rundan är till för. Samma
regler som innan: läs `PROJECT_SPEC.md` §14–16 för kontext, rapportera i
er egen chatt (inte i delade filer), och se till att ni har den senaste
koden (kompileringsfixen från §16 måste finnas — se steg 0).

**Dator 1 (mock-robotd) och Dator 2 (klienten): börja båda med Steg 0.**

### Steg 0 — bekräfta att kompileringsfixen faktiskt höll

Ingen har kört `cargo build` sedan fixen för E0004
(icke-uttömmande match) las in. Kör detta först, i respektive fönster:

```bash
cargo build -p mock-robotd   # Dator 1
cargo build -p robotstyrning # Dator 2
```

Om `robotd` (Pi-specifik) råkar gå att bygga på er labbdator också (osäkert
— kräver CAN/GPIO/I2C-bibliotek), bygg gärna den med:
```bash
cargo build -p robotd
```
och rapportera resultatet, men det är inte kritiskt om den inte går att
bygga på en vanlig dator.

Rapportera exakta felmeddelanden om något inte kompilerar rent — gå inte
vidare till resten av testet förrän båda `mock-robotd` och `robotstyrning`
bygger utan fel.

### Dator 1 — starta som vanligt, observera

```bash
cargo run -p mock-robotd -- --bind 0.0.0.0:9000
```

Håll koll på loggen som tidigare. Nytt att bekräfta den här gången:
- [ ] `-> gas=... styr=...`-rader med **verkliga, ojämna decimalvärden**
      (inte runda tal som `0.60`/`-0.30` — det var ett skript förra
      gången, inte en riktig dosa)
- [ ] Nekade förar-anslutningar loggas nu även här (`Förar-anslutning
      nekad: ...`) — tillagt efter förra rundan

### Dator 2 — anslut, koppla in dosan, gå igenom detta

```bash
cargo run -p robotstyrning
```

1. **Anslut** till `127.0.0.1`. Grön ram?
2. **Innan dosan är inkopplad:** står det "Ingen dosa ansluten" istället
   för en AKTIVERA-knapp (ny fix)?
3. **Koppla in PS4-dosan** (kabel eller Bluetooth). Dyker AKTIVERA-knappen
   upp nu?
4. **Tryck AKTIVERA.** Försvinner knappen? Rör spakarna:
   - Höger spak upp/ner → gas (syns i Dator 1:s logg, känns rätt håll?)
   - Vänster spak vänster/höger → styrning
   - L1/L2, R1/R2 → tillval A/B (syns som `accessory_a`/`accessory_b`
     om ni loggar dem, annars notera att de i alla fall inte kraschar
     något)
5. **Koppla ur dosan (eller stäng av Bluetooth) medan ni "kör".** Blir
   ramen lila? Nollställs AKTIVERA (måste tryckas igen när dosan kommer
   tillbaka)?
6. **Låt dosan vara ansluten men orörd i 5+ minuter.** Nollställs AKTIVERA
   av sig själv (idle-timeouten)? (Går att korta ner tröskeln temporärt i
   koden för att slippa vänta ut hela tiden — säg till om ni vill ha en
   snabbversion att testa mot istället.)
7. **Fel 5/6 (nu med riktig chans att testa rent):** Koppla från klienten
   helt, starta om den, försök ansluta till en adress som inte svarar
   (t.ex. `10.255.255.1`). Hamnar ni tillbaka på anslutningsskärmen med
   ett rött felmeddelande? Dyker den döda adressen upp i "Tidigare
   anslutna"-listan efteråt (ska INTE göra det)? Anslut sedan till
   `127.0.0.1` på riktigt — dyker DEN upp i listan?

### Rapportera

Som vanligt: klartext i er egen chatt. Nämn särskilt om något av
gamepad-kopplingen i `gamepad.rs` (L1/L2/R1/R2 som axlar vs. knappar,
nämnt som en öppen risk i tidigare rapporter) stämmer för just er
kontroller-modell.

## Vad som INTE testas här (förväntat, inte ett fel) — gäller båda

- Riktig styrning av hydraulik/VESC — mock-robotd hittar bara på siffror
- WebRTC-video — inte ihopkopplat än (se video.rs, spec §12)
- CAN-skanningen ger alltid samma fyra påhittade VESC — riktig skanning
  väntar på Car_Client-protokollets bekräftade kommando-ID:n (spec §13)
- OLED-display och GPIO-belysning på riktig hårdvara (kräver Pi:n)
