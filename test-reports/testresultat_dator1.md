# Testresultat — Dator 1 (mock-robotd, låtsas-roboten)

Datum: 2026-09-19. Roll enligt `mock_test.md`: servern (`mock-robotd --bind 0.0.0.0:9000`).
Skriven i en egen fil (inte i `PROJECT_SPEC.md`) enligt `mock_test.md`, så att de två
sessionerna inte skriver över varandra. Om/hur `PROJECT_SPEC.md` ska uppdateras avgör
användaren.

**Sammanfattning:** Allt i checklistan är bekräftat i loggen utom gas/styr från en riktig
gamepad, som inte kunde ses. Servern höll för 800+ anslut/koppla-från-cykler utan läckor.
Ett kompileringsfel (saknad `serde`-dependency) fixades. Två små brister i mocken hittades
(kvarblivet gasvärde, tyst ignorerade trasiga meddelanden), inget som stoppar testet.

---

## 1. Bygg

- Maskinen var tom: `cargo` fanns inte. Installerade Rust via rustup (cargo 1.98.1).
  Inga `apt`-paket behövdes (`gcc` fanns), `sudo` användes inte.
- **Första bygget misslyckades** med ett verkligt fel:
  ```
  error[E0433]: cannot find module or crate `serde` in this scope
     --> mock-robotd/src/main.rs:176:18
  async fn send<T: serde::Serialize>(
  ```
  Orsak: `main.rs` använder `serde::Serialize` direkt, men `serde` saknades i
  `mock-robotd/Cargo.toml` (bara `serde_json` fanns, och den re-exporterar inte `serde`).
- **Fix (kodfel, inte designbeslut):**
  - `mock-robotd/Cargo.toml`: la till `serde = "1"`.
  - `mock-robotd/src/main.rs`: tog bort två oanvända importer som gav varningar
    (`std::time::Instant`, `tokio::sync::mpsc`).
- Efter fixen: bygget rent, inga fel eller varningar.
- Notering: Dator 2 bygger klienten i samma `target/`-katalog i samma repo. Det ger
  "Blocking waiting for file lock on build directory" om båda kör cargo samtidigt.
  Ofarligt, men att köra den färdiga binären direkt (`target/debug/mock-robotd`) undviker det.

## 2. Körning

Startade med `RUST_LOG=debug`. Förväntad utskrift bekräftad: `Lyssnar på: 0.0.0.0:9000`.
Servern kördes obruten under hela testet (PID 14429).

## 3. Checklista mot loggen

| Punkt | Resultat |
|---|---|
| `Klient ansluten som FÖRARE` vid anslutning | ✅ Sågs |
| Styrkommandon `-> gas=… styr=…` | ⚠️ Se not nedan |
| `Belysning: PÅ` / `AV` | ✅ Sågs (2 gånger vardera under Dator 2:s körning) |
| `Settings-vy begärde VESC-skanning` + svar 32/44/97/88 | ✅ Sågs (2 gånger). Svaret verifierat på tråden, se §5 |
| `Settings-vy sparade en profil (N roller)` | ✅ `(4 roller)` sågs |
| Andra förare nekas tyst, ingen FÖRARE-rad för den | ✅ Verifierat med egen testklient, se §4 |
| `Klient frånkopplad` + ny förare kan ansluta efteråt | ✅ Flaggan fastnar inte, se §4 |

**Not om gas/styr:** En enda rad `-> gas=+0.60 styr=-0.30 aktiverad=true` sågs under
Dator 2:s körning. Jag vet inte om den kom från en riktig gamepad eller från en
testklient. De runda värdena (0.60 / -0.30) tyder mer på ett skript. **Gamepad-styrning
från en fysisk dosa är därför inte bekräftad av min sida.** Mina egna testkommandon
(§6) visar däremot att servern tar emot och loggar `Control`-meddelanden korrekt.

Dator 2:s del av loggen i siffror: 104 FÖRARE-anslutningar, 1 ÅSKÅDARE, 105
frånkopplingar (balanserat). Cirka 100 snabba anslut/koppla-från-cykler kördes alltså av
Dator 2 innan jag började.

## 4. En förare i taget (egen testklient)

Egen WebSocket-klient (`probe deny`), sekvens och verkligt svar:

```
A (förare)                : "ConnectedAsDriver"
B (2:a förare)            : {"ConnectDenied":{"reason":"roboten körs redan av någon annan"}}
B stängdes av servern efter nekandet: true
V (åskådare medan A kör)  : "ConnectedAsViewer"
B igen (A kör fortf.)     : {"ConnectDenied":{"reason":"roboten körs redan av någon annan"}}
C efter att A stängt      : "ConnectedAsDriver"
```

Serverloggen för samma körning hade exakt två `Klient ansluten som FÖRARE` (A och C),
ingen för de nekade B-anslutningarna, precis som spec:en/checklistan förväntar sig.
Avslaget syns alltså bara hos klienten, som får ett läsbart `reason`.
Åskådare släpps in medan en förare kör (avsett).

## 5. Frågor från mock_test.md §4

### Fråga 1 — Kompilerar allt rent?
Nej, inte första gången. Se §1: ett fel (`serde` saknas i `Cargo.toml`), åtgärdat. Därefter rent.

### Fråga 2 — Fryser eller kraschar mock-robotd vid snabba anslut/koppla-från-cykler?
**Nej. Håller.** Egen stresstest (`probe stress`) mot den körande servern:

| Test | Resultat |
|---|---|
| 500 sekventiella rena cykler (Connect → Close) | 500/500 accepterade som förare, 0 avvikande (343 ms totalt) |
| 300 abrupta avbrott (TCP släpps utan close-frame direkt efter Connect) | 300/300 accepterade, 0 avvikande |
| 100 samtidiga förar-anslutningar (kapplöpning) | **1 förare, 99 nekade, 0 övrigt** (exakt som förväntat) |
| Ny förare efter allt ovan | `ConnectedAsDriver` — `driver_connected` fastnade inte |

Serverloggen: 802 `ansluten som FÖRARE` och 802 `frånkopplad` för stresstesterna
(500 + 300 + 1 + 1), dvs. inget tappat och inget kvar.

Resurser före → efter (PID 14429):

| | Före | Efter |
|---|---|---|
| RSS | 7 040 kB | 7 676 kB |
| Trådar | 5 | 5 |
| Öppna fd | 10 | 10 |
| Etablerade anslutningar mot :9000 | 0 | 0 |
| CPU i vila | 0,2 % | 0,3 % |

Inga läckor (fd och trådar konstanta, den lilla RSS-ökningen är normal allokatorbeteende).

### Fråga 3 — Serialiseras VESC-profilen rimligt?
**Ja.** Rå JSON på tråden från `SaveVescProfile` (4 roller, "Annat" med citattecken,
åäö och ☃ i namnet, samma typer som klienten använder i `settings.rs::save`):

```json
{"SaveVescProfile":{"roller":{"97":"Styrning","88":{"Annat":{"namn":"Lyftarm \"åäö\" ☃","input":"l1_l2","min":-0.5,"max":1.0}},"32":"DriftVanster","44":"DriftHoger"}}}
```

172 byte. Round-trip (serialisera → deserialisera med samma typer) OK, "Annat"-rollen
återskapas exakt inklusive specialtecken. Servern svarade `"VescProfileSaved"`.

Saker att känna till om formatet (inga fel, men bra att veta för riktiga `robotd`):
- `HashMap<u8, _>` blir **JSON-nycklar som strängar** (`"32"`), och **ordningen är
  slumpmässig** mellan körningar. Räkna inte med stabil ordning om profilen någonsin
  jämförs textuellt eller diffas.
- Enum-varianter utan data blir bara en sträng (`"Styrning"`), varianten med data ett
  objekt (`{"Annat":{…}}`) — standard-serde, entydigt.
- `input` är en fri sträng (`"l1_l2"`/`"r1_r2"`) i protokollet, inte en enum. Servern
  accepterar vilken sträng som helst (testat: `"blabla"` sparades utan klagomål).

## 6. Ytterligare observationer (inte efterfrågade, men hittade under testet)

Alla testade med `probe` mot den körande servern.

1. **Kvarblivet gasvärde efter frånkoppling (mock-bugg).**
   `latest_throttle` är delat state och nollställs aldrig när föraren försvinner. Test:
   förare skickar gas 1,0 och kopplar bort abrupt → en ny åskådare ansluter → statusen
   visar `speed_kmh` som klättrar 4,5 → 7,65 → 9,86 → … → 14,1 km/h utan att någon kör.
   Påverkar bara mockens påhittade hastighet, men UI:t på Dator 2 kan visa en "spökhastighet"
   om man ansluter direkt efter en körning. Enkel fix: nollställ `latest_throttle` när
   föraren kopplar från (i `if is_driver { … }` längst ner i `handle_socket`).
   Riktiga `robotd` har watchdog/ramper som ska täcka motsvarande, men värt att kontrollera
   att den nollställer på frånkoppling och inte bara på tystnad.

2. **Trasiga meddelanden ignoreras tyst (inget svar, ingen loggrad).**
   `_ => {}` i match-satsen sväljer allt som inte går att tolka. Testat: skräptext, okänd
   variant och en profil där `min` är `null` gav **inget svar och ingen logg**. Servern
   överlevde allt och svarade korrekt på nästa meddelande.
   Praktisk följd: om klienten någon gång skickar en profil som inte går att avserialisera
   (t.ex. ett `NaN`-värde i min/max — `serde_json` skriver `NaN` som `null`, som sedan inte
   kan läsas tillbaka som `f32`) får den **aldrig `VescProfileSaved`**, och Settings-vyn blir
   stående med "Osparade ändringar" utan felmeddelande. Osannolikt i normal användning,
   men en `Err`-gren som loggar (`println!`/`tracing::warn!`) skulle göra sådant felsökbart.
   Samma `_ => {}`-mönster finns i riktiga `robotd/src/server.rs` (rad 225 och 230).

3. **Ingen validering av profilen i mocken.** `min > max` (5.0 / -5.0), okänd
   `input`-sträng och helt tom profil (0 roller) sparas alla med `VescProfileSaved`.
   För en mock är det OK, men riktiga `robotd` bör avgöra vad som ska nekas.

4. **Ingen timeout/ping i mocken.** En klient som försvinner utan att TCP stängs (t.ex.
   nätverkskabel ur) skulle hålla förar-flaggan tills TCP själv ger upp. Testade abrupta
   *stängningar* (fungerar bra, se §5 fråga 2), inte "tyst död"-anslutningar. Relevant för
   riktiga `robotd` (där "en förare i taget" annars kan blockera en ny förare efter ett
   nätavbrott), inte för mockens syfte.

## 7. Vad jag inte testade

- **Riktig gamepad/PS4-dosa** — ingen dosa fanns inkopplad, och jag såg inte klientens
  UI. Allt om Dator 2:s sida (ram, ping, AKTIVERA, gilrs, tokio-runtime, kända robotar)
  är Dator 2:s att rapportera.
- **Över WireGuard** — bara `127.0.0.1` på samma maskin.
- **Loggvolym/långkörning** — servern kördes ca 15 minuter, inte timmar.

## 8. Ändringar jag gjort i repot

- `mock-robotd/Cargo.toml` — `serde = "1"` tillagd.
- `mock-robotd/src/main.rs` — två oanvända importer borttagna.
- `testresultat_dator1.md` — den här filen.

Ingen annan repo-fil är ändrad. Testprogrammet (`probe`, ~150 rader Rust) ligger i sessionens
temporära arbetsyta, inte i repot; jag kan lägga in det i repot om ni vill kunna köra om
stresstesterna.

Mock-robotd stoppades (PID 14429) efter att användaren bekräftat att testet var klart och
alla frågor i `mock_test.md` §4 var besvarade här.
