# framtida_inkop.md — hinderdetektering (kamera-baserad "nödbroms")

Det här är **inte** en del av grundprogrammet (`robotd`/klienten som redan är
byggt) — det är research inför ett eventuellt framtida tillval: ett extra,
oberoende säkerhetslager som kan bromsa Drängen om något (särskilt en
människa) dyker upp rakt framför, på liknande sätt som en bils autobromsning
(AEB). Drängen förblir manuellt fjärrstyrd — det här är ett skyddsnät under
föraren, inte autonom navigering.

Se PROJECT_SPEC.md för bakgrunden till huvudprojektet. Den här filen är
fristående och påverkar inget av den befintliga koden förrän/om ni bestämmer
er för att bygga vidare på den.

## Gemensamt för båda alternativen

- **Riktning:** framåtriktad täckning (i körriktningen), inte runt hela
  roboten, i det första steget.
- **Beteende:** ska bromsa mjukt (samma princip som den befintliga
  watchdogen, inte tvärnita), med en tydlig indikator i klienten om VARFÖR
  det bromsade — inte bara en generisk röd ram.
- **Rekommenderat komplement oavsett budget:** en enkel, "dum"
  avståndssensor som sista skyddslager, oberoende av AI-detekteringen:
  - **VL53L1X (Time-of-Flight)** — några tior kronor, kopplas på Pi:ns I2C.
    Ger ett riktigt avstånd i cm, ingen gissning, ingen AI. Om
    kameradetekteringen missar något (dålig belysning, ovanlig vinkel) är
    det här sista utväg.

## Integration mot befintlig kod (gäller båda alternativen)

Tänkt som **ytterligare ett oberoende lager** i `robotd`s huvudloop, i
samma stil som watchdogen redan tvingar ramp-målet mot noll vid tappad
nätverksförbindelse. Ett separat, litet program (Python, se nedan) skulle
skicka enkla meddelanden ("person upptäckt, 2.3 m, riktning fram") till
`robotd` över en lokal socket. Om den processen inte körs eller inte är
inkopplad: inget meddelande kommer, inget händer — basprogrammet är alltså
inte beroende av att det här någonsin byggs.

---

## Alternativ A — Budgetvänligt (~800–1 200 kr)

**Krav:** Raspberry Pi **5** specifikt (se anledning nedan).

| Komponent | Ungefärligt pris | Kommentar |
|---|---|---|
| Raspberry Pi AI Kit (Hailo-8L, M.2 HAT+) | ~700–800 kr | Officiell Raspberry Pi-produkt, 13 TOPS. Kopplas in via Pi 5:ans PCIe-kontakt (M.2 HAT+) — fungerar INTE på samma sätt på Pi 4, som saknar den kontakten. |
| Befintlig USB-kamera | 0 kr | Kan återanvändas som den är för en första version |
| VL53L1X (ToF-sensor) | ~50–100 kr | Se "Gemensamt" ovan |

**Om Google Coral:** nämns ofta i äldre guider som budgetalternativ, men är i
praktiken övergivet av Google sedan 2023–2024 (ingen ny mjukvara, oklar
framtid). Hailo-8L via Raspberry Pis egen AI Kit är billigare per prestanda
och har officiellt, aktivt supporterad mjukvara — rekommenderas istället.

**Begränsningar att vara medveten om:**
- Kräver Pi 5 (se ovan) — om er nuvarande Pi är en 4:a behöver den bytas för
  det här spåret specifikt.
- Avstånd till upptäckt objekt uppskattas grovt utifrån hur stort det ser ut
  i bilden, ingen riktig djupmätning från kameran (VL53L1X-sensorn ovan
  täcker upp den svagheten för den smala kon den täcker rakt fram).
- Ingen inbyggd väderklassning på kameran — om den redan sitter skyddad är
  det inget nytt problem, annars behövs egen kapsling.

---

## Alternativ B — Premium (~5 000–9 000 kr)

**Kärnkomponent: Luxonis OAK 4-serien** (OAK 4 D eller OAK 4 Pro, gärna med
"W" för bred bildvinkel 120°).

En enda enhet som gör allt: stereokamera (riktig djupmätning, ingen
gissning) **+** kraftfull inbyggd AI-processor (RVC4-chip, upp till 48
TOPS) som kör objektdetekteringen på kameran själv. Er Pi belastas i
praktiken inte alls — den tar bara emot färdiga resultat.

| Komponent | Ungefärligt pris | Kommentar |
|---|---|---|
| OAK 4 D/Pro (helst PoE-variant) | ~5 000–8 000 kr | Kontrollera IP-klassning vid köp — föregångaren fanns i en IP67-variant (OAK-D-POE), säkerställ att en lika tålig OAK 4-variant väljs för utomhusbruk |
| VL53L1X (ToF-sensor) | ~50–100 kr | Samma som ovan, oberoende sista kontroll |
| DC-DC-omvandlare (om ej PoE) | ~100–200 kr | Kameran vill oftast ha 5V eller 12V |
| Monteringsfäste (laserskuret/3D-printat) | Egen tillverkning | Fram på chassit, i linje med körriktningen |

**Fördelar jämfört med Alternativ A:**
- Riktig djupmätning (stereo), inte en gissning från bildstorlek
- Nästan ingen belastning på Pi:n — kameran gör jobbet själv
- Funkar med Pi 4 ELLER 5 (kopplas via USB/PoE, inget beroende av Pi 5:ans
  M.2-kontakt)
- Kraftigt mer AI-kapacitet om ni i framtiden vill lägga till fler
  detekteringstyper (fordon, djur, etc.)

---

## Mjukvara (gäller båda, i grova drag)

- Luxonis (OAK) använder deras eget **DepthAI**-SDK, Python-baserat.
- Hailo-8L (budgetalternativet) använder **HailoRT**, med officiellt
  paketerad integration för Raspberry Pi OS.
- I båda fallen: en liten fristående Python-process kör detekteringen och
  skickar enkla resultat vidare till `robotd` (se "Integration" ovan) —
  ingen av dem kräver att huvudprogrammet skrivs om i grunden.
- Färdiga, redan tränade modeller (t.ex. YOLO-nano/tiny, eller MobileNet-SSD)
  räcker för grundfallet "är det en människa" — ingen egen datainsamling
  eller träning behövs för att komma igång.

---

## Öppna frågor att ta ställning till senare (inte nu)

1. Ska det bara varna föraren, eller alltid bromsa automatiskt?
2. Ska föraren kunna medvetet köra förbi en varning (t.ex. vid backning nära
   en vägg man vet är ofarlig)?
3. Vilken Pi-version blir det (styr om Alternativ A är praktiskt möjligt)?
4. Väderklassning/kapsling — hur exponerad sitter kameran i praktiken på
   den färdiga roboten?
