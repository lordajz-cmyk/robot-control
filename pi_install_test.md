# pi_install_test.md — installera & testa robotd på riktig hårdvara (Drängen)

Det här är instruktioner till dig, en Claude Code-session som körs i en
terminal SSH:ad in mot den **riktiga** Raspberry Pi:n på Drängen — inte
mock-robotd, inte en labbdator. Roboten är redan i drift med Car_Client +
RControlStation, och användaren har **bokat** den (ingen annan kör den
under det här testfönstret), men den är fysiskt riktig utrustning. Läs det
här helt innan du kör något.

## Läs först

- `PROJECT_SPEC.md` §13 (Car_Client-arkitekturen, VESC-kommando-ID:n
  fortfarande obekräftade), §14 (WireGuard, ingen egen relä/parkoppling),
  §18 (startfördröjning), §19 (bekräftade fakta: `/dev/vehicle`, port 8300,
  `car_client.service`/`car_rtk.service`, flaggorna Car_Client startas med)
- `reference/rise_sdvp/README.md` — vad de tre referensskripten gör

## Övergripande mål, i prioritetsordning

1. **Installera `robotd` säkert utan att störa den pågående driften.**
   Detta är i sig riskfritt (se nedan) — gör det först, oavsett resten.
2. **Om möjligt: lös §13.** Den enskilt mest värdefulla utkomsten av hela
   den här sessionen vore en `tcpdump`-fångst av EN riktig VESC-styrsignal
   (inte bara GPS som förra fångsten), tagen medan användaren (eller någon
   de bokat in) skickar ett litet, kontrollerat styrkommando genom
   RControlStation. Föreslå det här till användaren, men tvinga inte fram
   det om de inte har möjlighet/lust just nu.
3. **Testa `robotd` manuellt, kort, bara i ett stillastående ögonblick**
   (se säkerhetsregler nedan) — bekräfta att den startar, ansluter (eller
   loggar sina backoff-försök korrekt) mot Car_Client, och inte kraschar.

## Säkerhetsregler — läs innan du kör något interaktivt

- **Rör INGET på Pi:n som redan är kopplat till Car_Client, udev,
  `car_client.service` eller `car_rtk.service`.** Vårt installationsskript
  gör det inte av sig själv, men om du felsöker manuellt: gå inte in och
  ändra/starta om de tjänsterna utan att fråga användaren först — roboten
  är i drift.
- **Aktivera INTE `robotd.service`** (`systemctl enable`) i det här
  passet. Skriptet installerar den avstängd med flit (§13 är olöst — vi
  vill inte att den startar automatiskt vid nästa boot av en riktig,
  fungerande robot innan styrningen är verifierad). Lämna den så.
- **Kör `robotd` manuellt (i förgrunden, `robotd` utan systemd) bara när
  användaren bekräftat att roboten står stilla och ingen kör den just nu**
  — även om den är "bokad" i stort, dubbelkolla i stunden innan du startar
  processen. Se §-avsnittet ovan om osäkerheten kring samtidiga TCP-klienter
  mot Car_Client (port 8300) — okänd risk, inte "kommer krascha" men
  otestad, så var försiktig och kör kort.
- **Skicka aldrig något styrkommando från `robotd` till Car_Client i det
  här passet.** Det finns ingen kod som gör det ännu (§13), men om du
  skriver något eget test-script: gör det INTE. Bara lyssna/observera.
- Om något känns fel eller oväntat: stanna, rapportera till användaren,
  fråga innan du fortsätter. Det här är inte en labbdator att experimentera
  fritt på.

## Steg 1 — läge, innan du rör något

```bash
# Vad kör redan?
systemctl status car_client.service car_rtk.service 2>&1 | head -40
screen -list
ls -la /dev/vehicle /dev/car 2>&1
ip link show wg0 2>&1
cat /sys/class/net/wg0/operstate 2>&1
```

Rapportera vad du ser innan du går vidare. Om `car_client.service`/
`car_rtk.service` inte finns alls (roboten kör Car_Client på något annat
sätt än vad referensskripten beskriver): stanna och fråga användaren,
gissa inte dig fram.

## Steg 2 — hämta/uppdatera vårt repo på Pi:n

```bash
cd ~/robot-control 2>/dev/null && git pull || echo "Repot finns inte här än — fråga användaren hur du bäst får det hit (git clone-URL, eller be dem köra scripts/deploy_to_pi.sh från sin egen dator)."
```

## Steg 3 — kör vårt install_pi.sh

```bash
cd ~/robot-control   # eller var repot hamnade
```

Innan du kör: du behöver **robotens faktiska WireGuard-IP** för
`BIND_ADDR`. Fråga användaren om du inte redan ser det i `ip addr show wg0`
från Steg 1. Robot-ID: fråga vad de vill kalla den här specifika
installationen (troligen "drangen", men kolla — kom ihåg att det finns
flera robotar/maskiner i deras fork: Drängen, RobAnt, Macbot/Mactrac).

```bash
ROBOT_ID=<fråga användaren> BIND_ADDR=<wg0-IP>:9000 bash scripts/install_pi.sh
```

Rapportera **hela utskriften**, särskilt efterinstallationskontrollen i
slutet (WireGuard/I2C/USB/`car_client.service`/`car_rtk.service`-status).
Om något kompileringsfel dyker upp: det är nytt för oss (ingen har byggt
`robotd` på riktig Pi-hårdvara än, bara försökt på labbdatorer där
Pi-specifika bibliotek saknades) — läs felet noga, fixa uppenbara saker,
men fråga användaren om något verkar bero på ett designbeslut snarare än
ett rent kodfel.

## Steg 4 — manuellt, kort test (bara i det bekräftat stillastående ögonblicket)

```bash
robotd
```

Kör i förgrunden (inte som tjänst), titta på loggen i ~30-60 sekunder:

- [ ] Startar den utan att krascha?
- [ ] Försöker den ansluta till Car_Client (loggrader om försök/backoff,
      se §18)? Lyckas den, eller loggar den väntade
      "kunde inte ansluta än"-rader?
- [ ] OLED-/GPIO-relaterade fel (väntat om ingen skärm/relä är inkopplat
      än — notera men inget att oroa sig över)?
- [ ] Några oväntade kraschar eller paniker?

Avsluta med `Ctrl+C` när du sett tillräckligt. **Lämna INGET körande i
bakgrunden** när du är klar med testet (varken `robotd` manuellt eller
någon systemd-tjänst du kan ha startat för att felsöka).

## Steg 5 — föreslå §13-fångsten (fråga, tvinga inte)

Om användaren har tid och lust just nu:

```bash
sudo apt install -y tcpdump   # om det saknas
sudo tcpdump -i any -w vesc_capture.pcap tcp port 8300
```

Be dem (eller gör själv om du har tillgång) ansluta med RControlStation
som vanligt och skicka ETT litet, kontrollerat styrkommando (litet
spakutslag, kort). Stoppa tcpdump, kör:

```bash
hexdump -C vesc_capture.pcap | head -300
```

Det här är den data vi saknat sen §13 skrevs — riktig VESC-styrning genom
protokollet, inte bara GPS. Analysera precis som tidigare NMEA-fångster
(ramningen — start/längd/CRC/slut — är redan bekräftad, det som saknas är
kommando-ID:t för just styrning/telemetri).

## Rapportera

Skriv en sammanfattning i en ny fil, `test-reports/<datum>_pi_live_install.md`
(samma stil som tidigare testrapporter i den mappen) — inte direkt i
`PROJECT_SPEC.md`, användaren tar med sig resultatet och avgör om/hur
spec-filen ska uppdateras efteråt. Inkludera:
- Exakt vad som redan kördes på Pi:n innan du började (Steg 1)
- Hela install-utskriften, särskilt efterinstallationskontrollen
- Resultatet av det korta manuella `robotd`-testet
- Om §13-fångsten gjordes: den rå hexdumpen, eller åtminstone en tydlig
  sammanfattning av vad som sågs
- Allt du är osäker på eller som avvek från vad den här filen beskriver
