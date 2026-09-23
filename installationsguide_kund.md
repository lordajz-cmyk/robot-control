# Kom igång med Robotstyrning 🤖

Välkommen! Den här guiden hjälper dig att få igång programmet **Robotstyrning** på din
dator, så att du kan köra roboten med en handkontroll.

**Roboten är redan färdig.** Datorn och styrkortet inuti den är inställda av oss.
Du behöver bara göra i ordning **din egen dator**. Det tar ungefär en halvtimme,
och det mesta av tiden väntar du bara.

---

> ### 📝 Fylls i av oss innan vi skickar guiden
>
> | | |
> |---|---|
> | Robotens adress | `192.168.200.___` |
> | Filen för nätverket (WireGuard) | `__________.conf` |
> | Så här fick du programmet | ☐ USB-minne ☐ nedladdning ☐ vi skickade det till din dator |
> | Hjälp om något krånglar | Namn: __________ Telefon: __________ |

---

## Innan du börjar ✅

Bocka av att du har det här:

- ☐ En dator med **Ubuntu** (version 22.04 eller nyare) och internet.
- ☐ Ditt **lösenord** till datorn. Datorn frågar efter det några gånger.
- ☐ Mappen **robot-control** från oss (programmet).
- ☐ **Nätverksfilen** från oss (slutar på `.conf`). Den gör att din dator hittar roboten.
- ☐ En **PS4-handkontroll** och en USB-sladd till den.

---

## Steg 1: Öppna terminalen 🖥️

Terminalen är ett fönster där man skriver kommandon till datorn. Den ser svart eller
lila ut och är inte farlig.

1. Håll ner **Ctrl** och **Alt** och tryck på **T**.
2. Ett fönster öppnas. Där står det något i stil med `anna@dator:~$`.

**Så klistrar du in i terminalen:** i den här guiden finns grå rutor med text. Markera
texten, kopiera med **Ctrl + C** och klistra in i terminalen med
**Ctrl + Shift + V** (obs: *Shift* också!). Tryck sedan **Enter**.

> 💡 När datorn frågar efter lösenord syns **ingenting** när du skriver, inte ens
> stjärnor. Det är normalt. Skriv lösenordet och tryck Enter.

---

## Steg 2: Lägg programmet på rätt plats 📁

Mappen **robot-control** ska ligga i din **hemmapp**. Hemmappen är den som heter som
ditt användarnamn, och den öppnas när du klickar på **Filer** (Hem).

- **Fick du programmet på USB-minne eller som nedladdning?** Kopiera mappen
  `robot-control` till hemmappen. Är det en zip-fil: högerklicka → **Packa upp här**
  och flytta sedan mappen till hemmappen.
- **Skickade vi det direkt till din dator?** Då ligger det redan rätt.

Kontrollera att det blev rätt. Klistra in i terminalen:
```
ls ~/robot-control
```
✅ **Rätt:** du ser en lista med bland annat `client`, `robotd` och `scripts`.
❌ **Fel:** står det `Det finns ingen sådan fil eller katalog` ligger mappen på fel plats.

---

## Steg 3: Installera programmet ⚙️

Klistra in de här två raderna, en i taget, och tryck Enter efter varje:
```
cd ~/robot-control
```
```
bash scripts/install_client.sh
```

Nu händer det här:
1. Datorn frågar efter **ditt lösenord**. Skriv det och tryck Enter.
2. Det rullar förbi massor av text. Det är datorn som hämtar och bygger programmet.
3. **Det tar 10–20 minuter.** Gå och ta en kaffe ☕. Stäng inte fönstret.

✅ **Klart** när det står `=== Klart ===` längst ner.

---

## Steg 4: Koppla datorn till robotens nätverk 🔐

Roboten och din dator pratar med varandra genom ett eget säkert nätverk som heter
**WireGuard**. Nätverksfilen från oss innehåller allt som behövs.

1. Lägg nätverksfilen i mappen **Hämtningar**. (Heter mappen **Downloads** hos dig,
   byt `Hämtningar` mot `Downloads` i raderna nedan.)
2. Klistra in de här raderna, en i taget. Byt `DITTFILNAMN` mot filens namn från rutan
   överst, till exempel `kund1.conf`:

```
sudo apt install -y wireguard resolvconf
```
```
sudo cp ~/Hämtningar/DITTFILNAMN /etc/wireguard/wg0.conf
```
```
sudo systemctl enable --now wg-quick@wg0
```

3. Testa att din dator når roboten. **Roboten måste vara påslagen.** Byt `___` mot
   siffrorna i robotens adress:
```
ping -c 3 192.168.200.___
```
✅ **Rätt:** tre rader som slutar med `ms`, till exempel `tid=35 ms`.
❌ **Fel:** står det `100% packet loss`? Vänta två minuter (roboten kanske inte har startat
klart) och försök igen. Hjälper inte det, ring oss.

> Det här steget görs bara **en gång**. Efter det kopplar datorn upp sig av sig själv
> varje gång den startar.

---

## Steg 5: Koppla in handkontrollen 🎮

**Enklast:** sätt i USB-sladden mellan handkontrollen och datorn. Klart!

**Utan sladd (Bluetooth):**
1. Öppna **Inställningar → Bluetooth** på datorn.
2. Håll ner **PS**-knappen och **Share**-knappen samtidigt på handkontrollen tills
   ljuset börjar blinka snabbt.
3. Klicka på **Wireless Controller** i listan på datorn.

---

## Steg 6: Starta Robotstyrning 🚀

Öppna en **ny** terminal (Ctrl + Alt + T) och skriv:
```
Robotstyrning
```
Programmet finns också i programmenyn. Sök på **Robotstyrning**.

1. En svart ruta med ett textfält visas.
2. Skriv **robotens adress** (se rutan överst), till exempel `192.168.200.12`, och
   tryck **Enter**.
3. Nu ser du kamerabilden från roboten. 🎉

Nästa gång räcker det med steg 6. Adressen finns sparad under **Tidigare anslutna**,
så du kan klicka på den.

---

## Så kör du roboten 🕹️

### Första gången: gör så här
- Ställ roboten så att hjulen **inte når marken** (på pallbockar eller liknande), eller
  se till att det är tomt och gott om plats runt den.
- Sätt **Max** lågt (uppe till höger), till exempel `0.20`.

### Skärmen
| Var | Vad |
|---|---|
| **Uppe till vänster** | Grön text: fart, batteri, hur många motorer som svarar, temperatur, ping. |
| **Uppe till höger** | 🔒 Lås · 💡 Belysning · 📷 CAM/LOS · **Max** · ⚙ Inställningar |
| **Mitten** | Den gröna knappen **AKTIVERA**. |
| **Ramen runt fönstret** | 🟢 grön = bra kontakt · 🟡 gul = dålig kontakt · 🔴 röd = ingen kontakt (roboten stannar) · 🟣 lila = handkontrollen saknas |

### Köra
1. Tryck på **AKTIVERA**. Står det en text under knappen berättar den varför det inte går
   än, till exempel att kamerabilden saknas.
2. **Vänster spak upp/ner** = kör framåt/bakåt.
3. **Höger spak åt sidan** = svänger.
4. Släpp spakarna, så stannar roboten.

### Stanna och låsa 🛑
- **Släpp spakarna**, så stannar roboten.
- Tryck **🔒 Lås** uppe till höger, eller **Esc** på tangentbordet. Då låses körningen och
  AKTIVERA måste tryckas igen.
- Om nätverket försvinner stannar roboten **av sig själv** inom en halv sekund.
- Om handkontrollen tappar kontakten låses körningen av sig själv.
- Står du still i 5 minuter låses körningen av sig själv.

### Bra att veta
- **Max** bestämmer hur fort roboten får köra. Lågt värde = lugnt. Programmet kommer ihåg
  det du ställer in.
- **CAM** betyder att du kör med kameran. **LOS** betyder att du ser roboten med egna ögon.
  I LOS-läget kan du köra även om kameran inte fungerar.

---

## Om något krånglar 🔧

| Det här händer | Gör så här |
|---|---|
| `Robotstyrning: kommandot finns inte` | Stäng terminalen och öppna en ny. Hjälper inte det: gör om steg 3. |
| "Anslutningen bröts" eller inget händer när du ansluter | Är roboten påslagen? Gör testet i steg 4.3 (`ping`). |
| "Car_Client upptagen" | Någon annan är ansluten till roboten med ett annat program. Vänta eller ring oss. |
| "Ingen dosa ansluten" | Sätt i USB-sladden till handkontrollen, eller tryck på PS-knappen. |
| AKTIVERA går inte att trycka | Läs texten under knappen. Saknas kameran: tryck på **📷 CAM** så att det står **LOS**. |
| **VESC: 0/3 svarar** | Motorstyrningen har inte ström. Kontrollera batteriet och huvudbrytaren. |
| Roboten går trögt | Höj **Max** lite i taget. |
| Något annat | Ring oss. Ta gärna en bild på skärmen (tryck **Print Screen**). |

---

**Lycka till och kör försiktigt! 🚜**
