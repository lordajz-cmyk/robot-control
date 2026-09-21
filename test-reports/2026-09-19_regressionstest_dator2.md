# Regressionstest (andra testomgången) — Dator 2 (klienten)

## Rapport från DATOR 2 (klienten) — 2026-09-19, andra testomgången

> **Skrivet av: Dator 2** (Claude Code-sessionen som körde klienten
> `robotstyrning`). Dator 1 (mock-robotd) skriver sin egen rapport
> separat. Testet kördes mot mock-robotd på `127.0.0.1:9000`, inte över
> VPN. Inga kodändringar gjordes av Dator 2 den här omgången.

**Metod och begränsningar.** Klienten byggdes och startades av Dator 2.
Användaren klickade sedan igenom checklistan för hand i klientfönstret;
Dator 2 hade inga verktyg för att styra GUI:t (Wayland, inget xdotool)
och lät bli att skicka syntetisk input medan användaren körde. Dator 2
observerade genom skärmdumpar av fönstret, CPU-mätning och Dator 1:s
loggfil. Det som står som "syntes i loggen" är alltså verifierat på
serversidan, medan det som kräver att se GUI:t bygger på användarens
rapport eller på kodgranskning, och det anges.

### Resultat

| Punkt | Resultat |
|---|---|
| Bygge (`cargo build -p robotstyrning`) | OK, inga fel. 6 varningar om oanvänd kod (`video.rs`, `editing_other_name` m.fl.) |
| Ansluta | OK. Loggen: `Klient ansluten som FÖRARE` |
| Ram | Synlig (Fel 2 fixat), men **lila**, inte grön — se "AKTIVERA/dosa" nedan |
| Hastighet/batteri/ping | Visas. Batteri 86 % → 85 % på ca 40 s (mocken drar 0,01 % per 500 ms) |
| Belysning | OK. `Belysning: PÅ`/`AV` syns i loggen (tre gånger) |
| Settings: skanna + spara | OK enligt loggen (`begärde VESC-skanning`, `sparade en profil (4 roller)`). Att VESC-listan syntes och att "Osparade ändringar" försvann har Dator 2 inte sett själv |
| AKTIVERA | **Fungerar inte utan dosa**, se nedan |
| Frånkoppling | Loggen: `websocket-fel: Connection reset without closing handshake` → `Klient frånkopplad`, förarplatsen släpptes. Att en ny klient sedan kan ansluta testades **inte** |

### AKTIVERA/dosa — det enda som inte fungerade

Användarens rapport: "allt verkar funka utom aktivera-knappen".
Orsaken är kod, inte ett nytt fel: `client/src/main.rs` (i
`draw_driving_screen`, `if !self.gamepad_connected { self.activated =
false; }`) nollställer `activated` varje bildruta så länge ingen dosa är
inkopplad. Knappen går att trycka men faller tillbaka direkt, så den ser
trasig ut. Samma orsak gör att ramen alltid är lila utan dosa
(`border_color`), så **grön/gul/röd ram går inte att verifiera på en
maskin utan dosa**. Det var "fel 3" i förra rapporten. Att
AKTIVERA är låst utan dosa följer specen (§6, återställs vid
dosa-frånkoppling); det som saknas är att UI:t säger *varför*.
**Öppet beslut för användaren:** lämna som det är tills en riktig dosa
finns, eller stäng av knappen och visa "Ingen dosa ansluten" när dosan
saknas.

### Regressionstestet (Dator 2:s punkter)

1. **Fel 1 (Settings frös nätverket):** skanning och sparning gick igenom
   medan Settings var öppen (loggen). Fixet håller.
2. **Fel 2 (osynlig ram):** ramen syns nu (lila utan dosa). Grön ej sedd.
3. **Fel 4 (fejkad ping):** ping visade 0 ms och senare 1 ms, alltså mäts
   den på riktigt. På localhost blir den ändå 0–1 ms eftersom den visas i
   hela millisekunder, så punkten "inte ihållande 0 ms" går inte att
   bedöma här, bara över en riktig länk.
4. **Fel 5 (nekad anslutning osynlig):** ej testat.
5. **Fel 6 (robotlistan sparas för tidigt):** ej testat. OBS:
   `~/.config/robotstyrning/kanda_robotar.json` innehöll redan
   `["127.0.0.1"]` från en körning kl. 14:25 (före fixarna), så den
   filen kan inte i sig visa att sparningen fungerar; testa med en
   *ny* fungerande adress, t.ex. `127.0.0.2`.
6. **Tystnads-timeout (6 s):** ej testat.
7. **PS4-dosa:** ej testat. Ingen dosa fanns, och `/dev/uinput` är
   root-only så en virtuell dosa gick inte att skapa utan att ändra
   systemrättigheter — det gjordes inte.

### Svar på frågorna i §6

1. **Kompilerar allt rent?** Ja, inga fel, bara varningar.
2. **Tokio-runtime-lösningen:** inga tecken på deadlock eller hängning vid
   anslutning/frånkoppling. I vila drog klienten ca 13 % av en kärna
   (debugbygge, 17 trådar, ~98 MB RSS). Orsaken är
   `ctx.request_repaint_after(33 ms)` i `update()` som ritar om ~30 ggr/s
   hela tiden; troligen mycket lägre i release, men **inte mätt**.
3. **gilrs utan kontroll:** kraschar inte, ger bara en tom dosa
   (`connected: false`) → lila ram. Axel-/knappmappningen är **ej
   testad** (ingen dosa).
4. **UI-timing:** 500 ms statusintervall mot gul tröskel 700 ms /röd 2000
   ms ser rimligt ut, och ingen flimring sågs. 5-minuters-timeouten
   (`last_activity.elapsed() > ACTIVATE_IDLE_TIMEOUT`) är rätt kodad men
   utväntades inte.
5. **"Tidigare anslutna":** koden läser filen vid start och sparar först
   vid bekräftad lyckad anslutning. Omstart och sparning av ny adress är
   **inte testade** (se punkt 5 ovan).

### Kvarstår efter den här omgången

- Riktig PS4-dosa: AKTIVERA, spakar, grön ram, lila ram vid frånkoppling.
- Fel 5, Fel 6, tystnads-timeout och "ny klient kan ansluta efter
  frånkoppling".
- Test över riktig WireGuard-tunnel (ping-värdet är då meningsfullt).
