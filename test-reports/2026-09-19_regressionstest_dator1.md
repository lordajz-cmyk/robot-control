# Regressionstest (andra testomgången) — Dator 1 (mock-robotd)

(mock-robotd) — 2026-09-19, andra testomgången

> **Skrivet av: Dator 1** (Claude Code-sessionen som körde `mock-robotd`,
> alltså servern). Dator 2:s rapport står ovanför. Allt kördes mot
> `127.0.0.1:9000`, inte över VPN.
>
> **Vad som kommer varifrån.** Dator 2:s riktiga klientsession (användaren
> klickade i GUI:t) ger raderna *Belysning*, *VESC-skanning*, *profil med
> 4 roller* och *abrupt frånkoppling* i loggen. Dator 2 körde ingen andra
> förare och ingen dosa. Allt annat i den här rapporten (avslag, cykler,
> gasrest, trasiga meddelanden, tystnads-timeout, rå JSON) kommer från ett
> litet testskript som Dator 1 skrev själv mot samma server, **inte** från
> den riktiga klienten. Skriptet är ett stdlib-Python-script i sessionens
> scratchpad och ligger inte i repot. Det som skriptet visar gäller alltså
> serverns beteende på protokollnivå, inte klientens.

### Kodändring gjord av Dator 1

**`mock-robotd/src/main.rs` kompilerade inte.** Exakt fel:

```
error[E0004]: non-exhaustive patterns: `Ok(ClientMessage::Connect { .. })`,
`Ok(ClientMessage::VideoSignal(_))` and `Ok(ClientMessage::RequestViewCode)`
not covered
   --> mock-robotd/src/main.rs:141:31
```

Orsak: när det tysta `_ => {}` togs bort (loggning av trasiga meddelanden,
spec §15) hamnade tre varianter av `ClientMessage` utan arm. **Fix:** en ny
arm som loggar och ignorerar dem
(`Meddelande som mock-robotd inte hanterar, ignoreras: ...`). Bygger rent
efter det, `cargo build -p mock-robotd` utan varningar. Ändringen är
verifierad genom att loggraderna syntes när skriptet skickade
`VideoSignal`, `RequestViewCode` och ett andra `Connect`.

**⚠️ Samma fel finns med största sannolikhet i `robotd/src/server.rs`
(rad ~214–256).** Dator 1 kunde inte kompilera `robotd` här (kräver Pi-
bibliotek) och har därför **inte ändrat den filen**, men matchen där
saknar arm för `ClientMessage::Connect` och `ClientMessage::VideoSignal`
(`RequestViewCode` hanteras redan). Det är exakt samma sorts
"icke-uttömmande match" och ger samma E0004 så fort någon bygger
`robotd`. Åtgärd: lägg till en arm som loggar dem med `tracing::warn!`,
likt den som lades till i mocken. **Bör göras innan `robotd` byggs på Pi:n.**

### Checklistan (§3)

| Punkt | Resultat |
|---|---|
| `Klient ansluten som FÖRARE` | ✅ Syns (riktig klient) |
| Styrkommandon `-> gas=... styr=...` från riktig klient | ❌ **Inte sett.** Dator 2 hade ingen dosa och AKTIVERA nollställs utan dosa (se Dator 2:s rapport), så ingen gas skickades. De 12 `-> gas=+1.00`-rader som finns i loggen kommer från Dator 1:s eget skript |
| `Belysning: PÅ`/`AV` | ✅ Syns, tre PÅ/AV-par (riktig klient) |
| `Settings-vy begärde VESC-skanning` | ✅ Syns. Svaret med 32/44/97/88 verifierat på tråden av skriptet (se fråga 3) |
| `Settings-vy sparade en profil (N roller)` | ✅ Syns, `(4 roller)` (riktig klient) |
| Tredje anslutning nekas tyst | ✅ Verifierat med skriptet: ingen `Klient ansluten`-rad för den nekade, avslaget syns som `ConnectDenied { reason: "roboten körs redan av någon annan" }` på tråden. **Notera:** avslaget loggas inte alls på serversidan, så man ser det bara hos klienten |
| `Klient frånkopplad` + ny förare kan ansluta efteråt | ✅ Verifierat med skriptet, se A, B, F och G nedan. Riktiga klienten testade bara själva frånkopplingen (`Connection reset without closing handshake` → `Klient frånkopplad`) |

### Regressionstestet (Dator 1:s punkter)

| # | Punkt | Resultat |
|---|---|---|
| 1 | Kvarblivet gasvärde | ✅ Föraren körde gas 1,0 (hastighet steg till 4,5 km/h), stängdes abrupt (TCP stängd utan closing-handshake). En ny åskådare fick direkt `speed_kmh` = `0.0, 0.0, 0.0, 0.0` över fyra statusuppdateringar. Fixet håller |
| 2 | Loggning av trasiga meddelanden | ✅ Se D nedan. Alla trasiga meddelanden ger nu en loggrad |
| 3 | Tystnads-timeout | ✅ Loggen visar `Klient tyst i >5s, kopplar ner.` Servern stängde en tyst anslutning efter **5,0 s** och en ny förare kunde ansluta direkt efteråt. **Begränsning:** testat med en socket som anslöt och sedan tystnade, inte med riktigt uppdragen kabel/avstängt WiFi. Serverkoden är densamma oavsett hur tystnaden uppstår |

### Skriptets egna testfall (serverns beteende)

| | Test | Resultat |
|---|---|---|
| A | Förare 1 in, förare 2 in, åskådare in, förare 1 ut, förare 3 in | Förare 2 fick `ConnectDenied` med rätt `reason`. Åskådaren släpptes in. Förare 3 fick `ConnectedAsDriver` efter att 1 stängt |
| B | 500 rena + 300 abrupta anslut/koppla-från i rad | **800/800 lyckades** som förare, inga avvikelser, 0,5 s totalt |
| G | 100 samtidiga förar-anslutningar | **Exakt 1 accepterad, 99 nekade.** Ny förare kunde ansluta efteråt (`driver_connected` fastnade inte) |
| D | Trasiga meddelanden | Se nedan |
| E | Rå JSON, skanning och ping | Se fråga 3 nedan |

**D — trasiga meddelanden.** Exakta loggrader:

```
Kunde inte tolka meddelande från klient: expected value at line 1 column 1
Kunde inte tolka meddelande från klient: unknown variant `HittaPå`, expected one of `Connect`, `Control`, `ScanVescBus`, `SaveVescProfile`, `VideoSignal`, `RequestViewCode`, `Ping` at line 1 column 15
Kunde inte tolka meddelande från klient: expected value at line 1 column 79     <- profil med NaN
```

Profilen med `NaN` gav som väntat **inget svar** (`VescProfileSaved` uteblev),
men nu syns orsaken i loggen. Förra gången var det helt tyst. En giltig profil direkt efter gav `VescProfileSaved`, så
servern tar sig igenom felet och fortsätter.

### Svar på frågorna (§4)

1. **Kompilerar allt rent?** Nej, första bygget föll på E0004 enligt ovan.
   Efter en rad kod byggde det rent, utan varningar.
2. **Fryser eller kraschar vid snabba cykler?** Nej. 800 cykler
   (500 rena + 300 abrupta) på 0,5 s och 100 samtidiga anslutningar utan
   ett enda avvikande resultat. Processen mätt före/efter alla tester
   (~1 100 anslutningar): **trådar 5 → 5, filbeskrivare 10 → 10**. RSS
   6,8 MB → 7,8 MB. Ökningen på ~1 MB är liten och stämmer med normal
   allokatorkvarhållning, men en läcka på några hundra byte per anslutning
   kan inte uteslutas av så här kort körning. De 300 abrupta cyklerna
   loggar `websocket-fel: ... Connection reset without closing handshake`
   (förväntat, inte ett fel).
3. **Serialiseras VESC-profilen rimligt?** Ja. Det som testades var att
   servern tar emot exakt den JSON som serde genererar, inklusive en
   `Annat`-roll med citattecken, svenska tecken och emoji i namnet
   (`Lastarm "åäö" 🚜`, `min -0.5`, `max 1.0`) och svarar `VescProfileSaved`.
   Formen på tråden är enum-varianter som objekt:
   ```json
   {"SaveVescProfile":{"roller":{"32":"DriftVanster","44":{"Annat":{"namn":"…","input":"l1_l2","min":-0.5,"max":1.0}}}}}
   ```
   `roller` har alltså strängnycklar (`HashMap<u8,_>`), som spec §15 redan
   noterat. Svaret på `ScanVescBus` på tråden:
   `{"VescBusResult":[{"can_id":32,"responding":true},{"can_id":44,...},{"can_id":97,...},{"can_id":88,...}]}`.
   `Ping {sent_ms: 12345}` gav `Pong {sent_ms: 12345}`, alltså oförändrat
   eko som spec §15 säger. **Obs:** detta är *skriptets* JSON. Den som
   *klienten* faktiskt skickar för "Annat"-rollen har Dator 1 inte sett,
   eftersom mocken inte loggar rå JSON. Att `input` bara är en sträng
   (`"l1_l2"`) och inte validerad mot en lista är känt (spec §15).

### Bekräftar / kompletterar Dator 2:s "ej testat"

- **"Att en ny klient kan ansluta efter frånkoppling"** — bekräftat från
  serversidan (A, B, F, G ovan): förarplatsen släpps både vid ren stängning,
  vid abrupt stängning och vid tystnads-timeout. Det gäller mocken. Att
  *klienten* klarar att starta om och ansluta har Dator 1 inte kunnat testa.
- **"Tystnads-timeout"** — bekräftad på serversidan (5,0 s). Klientsidans
  Ping varje sekund gör att en levande klient aldrig triggar den.
- **Dator 2:s punkt 3 (ping visar 0–1 ms)** — mocken ekar tillbaka `sent_ms`
  direkt, så det är väntat att det blir ~0 ms på localhost.

### Kvarstår (kunde Dator 1 inte testa)

- Styrkommandon från en riktig klient med dosa (`-> gas=...` från en
  verklig `GamepadReader`, inte från skriptet).
- Rå JSON från *klientens* egen `SaveVescProfile` för "Annat"-rollen.
- Tystnad orsakad av riktigt nätavbrott (kabel/WiFi), och allt över en
  riktig WireGuard-tunnel.
- `robotd/src/server.rs` är **inte byggd eller testad**, se varningen ovan.

### Förslag (ingen ändring gjord)

- Logga nekade förar-anslutningar på serversidan
  (`Avvisade andra förare: ...`). Idag syns avslaget bara hos klienten,
  vilket gör det svårt att felsöka från servern.
- Överväg att låta klienten visa "Ingen dosa ansluten" i stället för en
  AKTIVERA-knapp som faller tillbaka (Dator 2:s öppna beslut).
