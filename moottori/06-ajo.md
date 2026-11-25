[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 6. Ajo

## 6.1 Aika ja vuorot

Pelin aika on kokonaisluku, jonka yksikkö on mikroaskel. Kierros on 120 mikroaskelta, ja jokainen pelin aikaa kuluttava toiminto ajoitetaan tarkkaan mikroaskeleeseen. Luku sulkee moottorin ja RogueScriptin avoimet kysymykset nopeuden ja hinnan suhteesta, komennon oletushinnasta, `wait`-kutsun ja kentän `game.turn` yksiköistä, tapahtumien `turn` ja `wake` parametreista sekä ajoituksen tasatilanteista.

### 6.1.1 Mikroaskel ja kierros

Mikroaskel on pelin ajan pienin yksikkö, ja kierros on tasan 120 mikroaskelta. Kaikki pelin aikaa kuluttava ajoitetaan mikroaskeleeseen: vuorot, `wait`-odotukset, ajastimet ja taustakohtausten rivit.

Paikka ratkaisee yksikön: komennon hinta ja `wait` ovat mikroaskelia, ajastimet ja taustakohtaukset kierroksia.

| Yksikkö | Kirjoitusasu | Käyttäjät |
| --- | --- | --- |
| mikroaskel | paljas luku | `cost N`, `wait N` |
| kierros | paljas luku | `timer`, taustakohtauksen aikajana, datan `duration` |
| esitysaika | `ms`, `s` | etualan ja `realtime`-kohtaukset |

**Moottorin kentät.**

- `game.time` on kulunut aika mikroaskelina. Uusi peli alkaa arvosta 0.
- `game.turn` on `game.time / 120` alaspäin pyöristettynä. Kierros 0 alkaa hetkellä 0, kierros 1 hetkellä 120.
- Molemmat ovat vain luettavia, ja ne tallentuvat tilannekuvaan.

Kierroksen raja on mikroaskel, joka on jaollinen luvulla 120. Raja on se hetki, jolloin kierrokseen sidotut asiat tapahtuvat (luku [6.1.4](#614-mikroaskeleen-vaiheet)).

**Harva eteneminen.** Moottori ei käy läpi tyhjiä mikroaskelia, vaan hyppää suoraan seuraavaan hetkeen, jolle on ajoitettu jotain. Kierroksen raja on aina ajoitettu. Tulos on sama kuin jos jokainen mikroaskel käytäisiin läpi, koska tyhjässä mikroaskeleessa ei tapahdu mitään.

### 6.1.2 Toimijat ja ajoitin

Toimija on ilmentymä, joka saa vuoroja: pelaaja (viimeisimmän `control`-kutsun kohde, moninpelissä istuimen jäsen, luku [6.1.10](#6110-istuimet-ja-ryhmä)) tai kartalla oleva ilmentymä, jolla on moottorin komponentti `ai`. Moottori pitää herätysjonoa, jossa jokaisella toimijalla on enintään yksi herätys.

Herätys on pari (hetki, järjestysnumero). Hetki on mikroaskel, jolloin toimija saa vuoron tai sen keskeneräinen komento jatkuu. Järjestysnumero kasvaa yhdellä jokaisesta jonoon laitetusta herätyksestä koko pelin ajan, ja se tallentuu.

**Järjestys.** Moottori ottaa jonosta pienimmän hetken. Saman hetken herätykset ajetaan järjestysnumeron mukaan, eli siinä järjestyksessä kuin ne ajoitettiin. Pelaajalla ei ole etuoikeutta tasatilanteessa.

**Jonoon tulo.**

| Tilanne | Ensimmäinen herätys |
| --- | --- |
| toimija syntyy kartalle (legend, `spawn`, generaattori) | nyt + viive(oletushinta) |
| `control(e)`, kun `e`:llä ei ole herätystä | nyt |
| `control(e)`, kun `e` on jo jonossa | herätys säilyy |
| ilmentymä saa `ai`-komponentin (`add`, luku [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)) | nyt + viive(oletushinta) |
| `ai`-ilmentymä siirtyy kokoelmasta kartalle (`move`, luku [9.6](09-moottorin-funktiot-ja-tapahtumat.md#96-tapahtumat)) | nyt + viive(oletushinta) |

Aloituksessa `begin`-käsittelijän luoma pelaaja toimii siis hetkellä 0, ja kartan hirviöt toimivat ensimmäisen kerran hetkellä 120 normaalinopeudella.

**Jonosta poistuminen.** Toimijan herätys poistuu, kun ilmentymä tuhoutuu, siirtyy kokoelmaan tai menettää `ai`-komponentin eikä ole pelaaja. Kun `control` siirtyy toiselle, entinen pelaaja jää jonoon vain, jos sillä on `ai`.

**Pelaaja ja `ai`.** Pelaajan vuorolla moottori odottaa syötettä eikä kutsu sen `ai`-funktiota, vaikka komponentti olisi olemassa. Poikkeus on asetus `idle`, joka ajaa pelaajan tekoälyn valitseman komennon, kun pelaaja ei käytä vuoroaan (alla). Ruumiin vaihto (`control(golem)`) ei siksi poista golemin tekoälyä, joka palaa käyttöön, kun pelaaja vaihtaa takaisin.

**Kierrokset ilman pelaajan vuoroa.** Kun pelaajalla ei ole herätystä (lamaantuminen) tai pelaajaa ei ole (`player` on kuollut kahva tai `none`), peli ei pysähdy eikä pääty. Moottori kuljettaa kierroksia ja näyttää välikehykset.

- **Tahti.** Asetus `round` ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)) on yhden kierroksen kesto. Jos se on asetettu, kierrokset etenevät sillä tahdilla. Jos se puuttuu, jokainen kierros odottaa toimintoa `pass`.
- `pass` ajaa seuraavan kierroksen heti odottamatta tahtia (luku [8](08-syote.md)). Pelaajan omalla vuorolla se ei tee mitään, koska vuoron ohitus on pelin komento.
- **Paluu.** Kun pelaaja saa herätyksen tai `control()` antaa uuden pelaajan, moottori odottaa taas syötettä.
- **Aloitus ilman pelaajaa.** `begin` ilman `control`-kutsua on sallittu, ja peli alkaa silloin ilman pelaajaa.

**Odotusaika: `idle`.** Moninpelissä ajastin on istuinkohtainen (luku [6.1.10](#6110-istuimet-ja-ryhmä)). Asetus `idle` ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)) päättää vuoron, jota pelaaja ei käytä. Kun moottori odottaa pelaajan syötettä vuorolla ja aika kuluu ilman painallusta, moottori antaa pelaajan komennon itse.

- `idle 4s ai` kutsuu pelaajan `ai behavior` -funktiota ([RS 7.2](../roguescript/07-komennot-ja-tekoaly.md#72-tekoäly)) ja ajaa sen palauttaman komennon pelaajan komentona. Pelaajan säännöt pätevät: hinta 0 on sallittu, ja epäonnistunut komento ei kuluta aikaa, jolloin moottori odottaa taas syötettä.
- `idle 4s ai STD.WAIT` käyttää varakomentoa, jos pelaajalla ei ole `ai`-komponenttia. Ilman varakomentoa `idle` ei silloin tee mitään.
- `idle 4s STD.WAIT` ajaa aina nimetyn komennon. Lataus tarkistaa, että komento on olemassa ja parametriton.

Mikä tahansa painallus nollaa ajan. Ajastin ei laukea komennon parametrikyselyn (`from`, `aim`), toistolaskurin, valikon, dialogin eikä kohtauksen aikana. Laukeaminen on syöte kuten painallus, joten tila ja tallennukset pysyvät eheinä. Ilman riviä odotus on rajaton.

Logiikka voi vaihtaa pelaajan tekoälyn, joten samalla mekanismilla tehdään Ultima-tyylinen vuoron menetys, sekavuus, automaattinen tutkiminen tai katselutila.

### 6.1.3 Hinta ja nopeus

Komennon hinta on mikroaskelia normaalinopeudella 10, ja nopeus skaalaa sen viiveeksi. Oletushinta on 120 eli yksi kierros, joten tavallinen olento toimii kerran kierroksessa.

```latex
\text{viive} = \max\left(1, \left\lceil \frac{\text{hinta} \times 10}{\text{nopeus}} \right\rceil\right)
```

| Nopeus | Viive hinnalla 120 | Vuoroja kierroksessa |
| --- | --- | --- |
| 5 | 240 | 0,5 |
| 8 | 150 | 0,8 |
| 10 | 120 | 1 |
| 12 | 100 | 1,2 |
| 15 | 80 | 1,5 |
| 20 | 60 | 2 |
| 30 | 40 | 3 |

Luku 120 on valittu jaollisuuden takia: oletushinnan viive 1200 / nopeus on kokonaisluku esimerkiksi nopeuksilla 1–6, 8, 10, 12, 15, 16, 20, 24, 25, 30, 40 ja 60. Pyöristystä tarvitaan vain muilla nopeuksilla ja hinnoilla.

Nopeus luetaan moottorin komponentin `movement` kentästä `speed` sillä hetkellä, kun viive lasketaan. Jos komponentti tai kenttä puuttuu, nopeus on 10. Nopeus koskee kaikkia komentoja, ei vain liikettä.

**Hinta.** `cost N` komennon otsikossa on hinta, ja sen puute tarkoittaa hintaa 120. Hinta on vakio, joten tilanteesta riippuva hinta kirjoitetaan `wait`-kutsuna (luku [6.1.6](#616-monivaiheinen-komento)).

**Rajatapaukset.**

- Pyöristys on ylöspäin, jotta nopeuden kasvu ei koskaan lyhennä viivettä nollaan.
- `cost 0` on maksuton toiminto. Pelaaja saa uuden vuoron samalla hetkellä ilman uutta herätystä. Tekoälylle viive on aina vähintään 1, jotta silmukka ei jumita peliä.
- Nopeus 0 tai negatiivinen nopeus: toimija ei saa herätystä, eli se on lamaantunut. Kun nopeus muuttuu positiiviseksi, moottori ajoittaa sen hetkeen nyt + viive(oletushinta). Pelaajan lamaantuminen ei pysäytä peliä, vaan muut toimivat (luku [6.1.2](#612-toimijat-ja-ajoitin)).
- Nopeuden muutos ei siirrä jo ajoitettua herätystä. Uusi nopeus vaikuttaa seuraavaan viiveeseen.

### 6.1.4 Mikroaskeleen vaiheet

Jokainen mikroaskel ajetaan neljässä vaiheessa kiinteässä järjestyksessä. Vaiheet 1–3 ajetaan vain kierroksen rajalla, vaihe 4 jokaisessa mikroaskeleessa, jolla on herätyksiä.

1. **Kierros** (vain rajalla, ei hetkellä 0). `game.turn` kasvaa, ja moottori lähettää tapahtuman `turn`. Jakelu menee aktiivisten karttojen ilmentymille luontijärjestyksessä ja tavalliseen tapaan niiden `modifies`-kokoelmien jäsenille. Lopuksi yleiset käsittelijät saavat tapahtuman kerran, ja niissä subjekti on `game`.
2. **Ajastimet** (vain rajalla). Moottorin komponentin `timer` rivit, joiden aika on täynnä, lähettävät tapahtumansa. Järjestys on ilmentymien luontijärjestys ja ilmentymän sisällä rivijärjestys.
3. **Taustakohtaukset** (vain rajalla). Pelin aikaa käyttävien taustakohtausten rivit, joiden aika on tämä kierros, ajetaan kohtausten aloitusjärjestyksessä ([RS 9.9](../roguescript/09-kohtaukset-ja-aikajanat.md#99-taustakohtaukset)).
4. **Herätykset.** Tämän hetken herätykset ajetaan järjestysnumeron mukaan (luku [6.1.2](#612-toimijat-ja-ajoitin)). Herätyksiä ovat myös pelin ajan taustakohtausten tweenien askeleet ([RS 9.9](../roguescript/09-kohtaukset-ja-aikajanat.md#99-taustakohtaukset)). Herätys, joka ajoitetaan samaan hetkeen vaiheen aikana, esimerkiksi `control(e)`, ajetaan saman vaiheen lopussa.

Kun vaihe 4 on tyhjä, moottori siirtyy seuraavaan ajoitettuun hetkeen, joka on lähin herätys tai kierroksen raja.

**Etualan kohtaukset.** Jonoon laitettu etualan kohtaus (`start`) alkaa heti, kun käynnissä oleva toiminto päättyy (luku [6.1.7](#617-toiminto)), eikä vasta mikroaskeleen lopussa. Pelin aika pysähtyy kohtauksen ajaksi, ja mikroaskel jatkuu siitä, mihin se jäi.

**Tapahtumien parametrit.** `turn()` ei saa parametreja, koska kierros luetaan kentästä `game.turn`. `timer`-tapahtumien parametrit ovat ajastinrivin tapahtuman omat. Moottorin tapahtuma `wake` kuvataan luvussa [6.1.5](#615-vuoron-kulku).

### 6.1.5 Vuoron kulku

Vuoro on toimijan herätys, jolla ei ole keskeneräistä komentoa. Se etenee neljässä askeleessa, ja vasta komennon loppu ajoittaa seuraavan herätyksen.

1. **`wake`.** Moottori lähettää tapahtuman `wake` toimijalle, joka on sen subjekti ja ainoa parametri (`on wake():` komponentissa, `on wake(who):` yleisenä). Tila, joka vaikuttaa toimijan omaan tahtiin, kirjoitetaan tähän, ja `turn` jää kierroksen mittaisille vaikutuksille. Jos toimija tuhoutuu tai lakkaa olemasta toimija käsittelijöissä, vuoro päättyy ilman uutta herätystä.
2. **Valinta.** Pelaajan vuorolla moottori odottaa syötettä, ja pelin aika on pysähdyksissä; asetus `idle` voi päättää odotuksen (luku [6.1.2](#612-toimijat-ja-ajoitin)). Tekoälyn vuorolla moottori kutsuu `ai`-funktiota, joka palauttaa komennon. Paluuarvo `none` tarkoittaa lepoa, joka kuluttaa oletushinnan. Tekoäly voi kutsua toista tekoälyä ja palauttaa sen valinnan ([RS 7.2](../roguescript/07-komennot-ja-tekoaly.md#72-tekoäly)).
3. **Suoritus.** Komennon runko ajetaan. `wait` keskeyttää sen (luku [6.1.6](#616-monivaiheinen-komento)).
4. **Ajoitus.** Kun runko päättyy, moottori laskee viiveen hinnasta ja nopeudesta sillä hetkellä ja ajoittaa herätyksen hetkeen päättymishetki + viive.

**Epäonnistuminen.** Komento epäonnistuu, kun sen runko palauttaa `false`. Muuten se onnistuu, myös ilman `return`-lausetta. Jo tehdyt tilan muutokset jäävät voimaan, joten komento tarkistaa ehtonsa ennen muutoksia.

| Kuka | Onnistui | Epäonnistui |
| --- | --- | --- |
| pelaaja | herätys: loppu + viive(hinta) | ei aikaa; moottori odottaa uutta syötettä samalla hetkellä |
| tekoäly | herätys: loppu + viive(hinta) | menettää vuoronsa: herätys nyt + viive(oletushinta) ([RS 7.2](../roguescript/07-komennot-ja-tekoaly.md#72-tekoäly)) |

Pelaajan epäonnistunut komento ei siis kuluta aikaa: seinään kävely ei vie vuoroa. Jatkokomento: runko voi palauttaa komennon (return ATTACK(other)). Moottori ajaa sen samassa toiminnossa saman toimijan komentona kutsujan tilalla, ja hinta ja epäonnistuminen ovat palautetun komennon. Kutsujan hintaa ei peritä, mutta sen wait-odotukset ovat kuluneet. Ketju on sallittu, ja jokainen jatko kasvattaa kutsusyvyyttä, joten game.call\_depth pysäyttää kehän. Parametrit annetaan kutsussa, joten pelaajalta ei kysytä niitä. Jos komento ehti odottaa (`wait`) ennen epäonnistumista, odotettu aika on kulunut, eikä hintaa peritä.

### 6.1.6 Monivaiheinen komento

`wait N` komennossa keskeyttää rungon ja ajoittaa toimijalle jatkoherätyksen hetkeen nyt + viive(N). N skaalataan nopeudella samalla kaavalla kuin hinta, joten nopea kaivaja kaivaa nopeammin.

```roguescript
command DIG(dir: dir) cost 60:
    repeat 5:
        wait 120
        tile(actor.at + dir, FLOOR, layer: GROUND)
```

Normaalinopeudella DIG vie 5 × 120 + 60 = 660 mikroaskelta. Toimija ei saa välissä uusia vuoroja, mutta muut toimijat, ajastimet ja kierrokset etenevät normaalisti.

- **Jatko.** Jatkoherätys on tavallinen herätys jonossa (luku [6.1.2](#612-toimijat-ja-ajoitin)). Moottori lähettää toimijalle tapahtuman `wake` myös jatkossa, joten jokainen herätys on yksi `wake`. Sen jälkeen runko jatkuu `wait`-kutsun jälkeisestä kohdasta.
- **Keskeytyminen.** Jos toimija tuhoutuu tai menettää toimijan aseman odotuksen aikana, komento hylätään. Kesken jääneet muutokset jäävät voimaan, kuten epäonnistumisessa.
- **Pelaajan odotus.** Kun pelaajan komento odottaa, moottori ei kysy syötettä, vaan peli etenee jatkoherätykseen asti ja näyttää välikehykset (luku [7.2](07-esitys.md#72-kartan-piirtäminen)).
- **Muut odotukset.** `wait scene X()`, `wait confirm` ja `open` eivät kuluta pelin aikaa. Ne päättävät käynnissä olevan toiminnon kuten `wait N`, ja komento jää odottamaan samaan mikroaskeleeseen. Rajalla moottori ajaa ensin jonossa olevat etualan kohtaukset aloitusjärjestyksessä ja sitten kohtauksen, valikon tai paneelin, jota komento odottaa. Kun se päättyy, komento jatkuu uutena toimintona (luku [6.1.7](#617-toiminto)). Rajalla tehty `rollback` korvaa koko tilan, myös odottavan komennon.

**Missä odotus on sallittu.** `wait N`, `wait scene`, `wait confirm` ja `open` ovat komennossa sallittuja vain sen omassa rungossa, myös silmukoiden ja `if`-haarojen sisällä. Käsittelijässä, `fn`-funktiossa ja `query`-funktiossa ne ovat latausvirhe (luku [2.5](02-lataus.md#25-tarkistukset)). Odottava komento on siksi aina yksi kehys, komennon oma runko, eikä kutsupinoa tallenneta. Korutiinikohtauksissa odotukset ovat sallittuja, mutta ne eivät kuulu tähän, koska etualan kohtausta ei tallenneta (luku [5.2](05-aloitus-tallennus-ja-checkpointit.md#52-tallennus)).

**Suorituskohta.** Lataus numeroi kunkin komennon rungon silmukat (`repeat`, `for`, `while`) ja odotuskohdat (`wait N`, `wait scene`, `wait confirm`, `open` sekä sijoitukset `x = wait number(…)`, `x = wait text(…)` ja `x = open VALIKKO(…)`) lähdejärjestyksessä nollasta alkaen. Järjestys kulkee lauseittain, ja lauseen sisällä `if`-haarat, `else`, `match`-tapaukset ja silmukan runko ovat kirjoitusjärjestyksessä. Suorituskohta on rakenteellinen polku, jossa on kaksi osaa:

- **Odotuskohdan numero.** Se on staattinen: jokainen odotus on koodissa tasan yhdessä paikassa, joten numero kertoo lauseen sekä kaikki sitä ympäröivät silmukat ja `if`-haarat.
- **Silmukoiden tila** ulkoa sisään, yksi jokaiselle odotuskohtaa ympäröivälle silmukalle:

| Silmukka | Tila |
| --- | --- |
| `repeat N` | kierroksen numero nollasta |
| `for x in joukko` | silmukan alussa kiinnitetty joukko ja nykyinen indeksi; joukkoa ei lasketa uudelleen, ja tuhoutuneet jäsenet ohitetaan kuten ajossa |
| `while` | ei tilaa; ehto arvioidaan seuraavan kierroksen alussa tavalliseen tapaan |

`if`-haaraa ei tallenneta, koska odotuskohdan numero kertoo haaran, eikä ehtoa arvioida uudelleen.

**Paikalliset nimet.** Kaikki odotuskohdassa näkyvät paikalliset nimet tallennetaan arvoineen, myös silmukkamuuttujat, paikalliset listat ja pakan kopiot nostopinoineen ja poistopakkoineen. Kahva tallennetaan ilmentymän tunnisteena, ja kuollut kahva pysyy kuolleena lajeineen ja nimineen (luku [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)).

**Esimerkki.** Toimija tutkii kolmatta ruutua toista kertaa ja odottaa kohdassa `wait 40`:

```roguescript
command SEARCH():
    for spot in near(actor.at, 1):              // silmukka 0
        repeat 3:                               // silmukka 1
            wait 40                             // odotus 0
            if chance(20%):
                tile(spot, DOOR, layer: GROUND)
                wait confirm                    // odotus 1
                break
```

Tallennus sisältää tämän tiedon. Esitysmuoto on havainnollistus, ei tiedostomuoto (luku [5.2](05-aloitus-tallennus-ja-checkpointit.md#52-tallennus)).

```
command: SEARCH
body: 3f9a…                // rungon jäsennyspuun tiiviste
args: ()
at: odotus 0
loops: silmukka 0 (for): joukko [8 ruutua], indeksi 2
       silmukka 1 (repeat): kierros 1
locals: spot = (4, 5)
resume: 840 #57            // vain wait N
```

**Jatkaminen.** Moottori ei aja runkoa alusta. Se laskeutuu polkua pitkin: jokainen polun silmukka alkaa tallennetusta tilastaan, ja suoritus jatkuu odotuskohdan jälkeisestä lauseesta. `wait N` jatkuu jatkoherätyksessä, ja muut odotukset jatkuvat, kun kohtaus tai valikko päättyy. Sen jälkeen kaikki on tavallista ajoa: kierros loppuun, seuraavat kierrokset ja seuraavat jäsenet. Polku kuvataan kielen käsittein, joten se ei riipu tulkin toteutuksesta.

**Muuttunut runko.** Tiiviste lasketaan rungon jäsennyspuusta ilman lauseiden sijainteja, joten kommentit, tyhjät rivit ja välilyönnit eivät vaikuta siihen. Jos ladatun pelin komento puuttuu tai sen tiiviste ei vastaa tallennettua, odottava komento hylätään kuten keskeytymisessä: kesken jääneet muutokset jäävät voimaan, hintaa ei peritä, ja moottori kirjaa ajon varoituksen. `wait N` -odotuksen jatkoherätys säilyy ja on toimijan tavallinen vuoro. Muissa odotuksissa toimija saa vuoron samalla hetkellä.

### 6.1.7 Toiminto

Toiminto on ajon jakamaton yksikkö. Toimintojen välissä tila on aina eheä, joten etualan kohtaukset, tallennus, automaattinen tallennus, checkpoint ja rollback tapahtuvat vain siellä.

Toiminto sisältää kaikki tapahtumat, jotka se lähettää (`emit` on synkroninen, luku [6.2](#62-tapahtumat)).

| Toiminto | Alkaa | Päättyy |
| --- | --- | --- |
| kierros | vaihe 1 | kaikki `turn`-käsittelijät ajettu |
| ajastin | yksi `timer`-rivi laukeaa | sen tapahtuman käsittelijät ajettu |
| taustakohtauksen hetki | kohtauksen saman hetken rivit | rivit ajettu |
| herätys | `wake` | komento päättyy tai kohtaa `wait N`, `wait scene`, `wait confirm` tai `open` |
| tweenin askel | pelin ajan taustakohtauksen herätys | askeleen muutos ja sen tapahtumat ajettu |

**Toiminnon loppu.** Kutsut, jotka tehdään toiminnon lopussa, kerätään toiminnon aikana ja suoritetaan kiinteässä järjestyksessä:

1. **Siirrot.** `travel`-pyynnöt kutsujärjestyksessä tapahtumineen (`leave`, `depart`, `arrive`, luku [6.7](#67-siirtyminen-kartalta-toiselle)). Saman entiteetin useasta pyynnöstä ratkaisee viimeinen. Siirtojen käsittelijät kuuluvat samaan toimintoon, joten niiden `start`-, `checkpoint`- ja muut pyynnöt käsitellään tässä samassa lopussa.
   - **Istuimet.** `join`- ja `part`-tapahtumat istuinnumeron järjestyksessä siirtojen jälkeen (luku [8.2](08-syote.md#82-istuimet-ja-laitteet)). `part`-käsittelijöiden jälkeen istuimen jäsenet menettävät `seat`-komponentin.
2. **`rollback`.** Jos se on pyydetty, moottori lataa checkpointin, ja loput kohdat ohitetaan ladatun tilan mukaisesti: ladattu tila sisältää oman jononsa.
3. **`checkpoint`.** Tilannekuva otetaan siirtojen jälkeen, joten se näkee entiteetit kohdekartoillaan.
4. **`delete_save`.**
5. **`autosave` ja profiilin kirjoitus.** Myös `on arrive` ja `on checkpoint` -kohdat (luku [5.4](05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)).
6. **Etualan kohtaukset** jonojärjestyksessä, myös vaiheissa 1–5 jonoon laitetut. Kohtaus, valikko tai paneeli, jota komento odottaa, alkaa niiden jälkeen (luku [6.1.6](#616-monivaiheinen-komento)). Pelin aika on pysähdyksissä, ja kohtauksen jälkeen ajo jatkuu saman mikroaskeleen seuraavasta toiminnosta.

Järjestyksen ansiosta käsittelijä voi siirtää pelaajan ja käynnistää kohdekartalla pelattavan kohtauksen samassa kutsussa: `travel` ja `start` peräkkäin ([RS 4](../roguescript/04-tapahtumat-ja-kasittelijat.md) ja [9](../roguescript/09-kohtaukset-ja-aikajanat.md)).

- **Tallennus.** Pelaaja voi tallentaa vain toimintojen välissä, ei kuitenkaan etualan kohtauksen, dialogin tai `open`-valikon aikana (luku [5.2](05-aloitus-tallennus-ja-checkpointit.md#52-tallennus)). Jos pelaaja avaa pelin valikon pelaajan komennon odottaessa, moottori päättää käynnissä olevan toiminnon ennen valikkoa.

### 6.1.8 Aktiiviset kartat ja jäädytys

Aika kulkee vain aktiivisella kartalla. Moninpelissä aktiivisia ovat kaikki jäsenten kartat (luku [6.1.10](#6110-istuimet-ja-ryhmä)), ja atlaksessa pelaajan lehden koko lähialue (luku [6.10](#610-atlas)). Aktiivinen kartta on pelaajan kartta, ja jos pelaaja on kokoelmassa, sen ylimmän kantajan kartta. Jos pelaajaa ei ole, aktiivinen kartta on se, jolla pelaaja viimeksi oli, ja kamera jää sinne. Ilman pelaajaa alkanut peli käyttää aloituskarttaa (`game.start`). Muut kartat ovat jäädytettyjä: niiden toimijat eivät herää, ajastimet eivät etene eikä `turn` tavoita niiden ilmentymiä.

- **Kartan kello.** Jokaisella kartalla on oma kello, joka kulkee vain, kun kartta on aktiivinen: se on `game.time` vähennettynä ajalla, jonka kartta on ollut jäädytettynä. Kartan herätykset ovat sen kellon ajassa, joten jäädytetyn kartan herätykset odottavat sellaisinaan. Kello alkaa ensimmäisessä latauksessa arvosta game.time. Herätyksen hetki on aina sen kartan kellossa, jolla toimija on ajoitushetkellä.
- **Sulatus.** Kun kartta tulee taas aktiiviseksi, sen kello jatkaa siitä, mihin se pysähtyi. Järjestysnumerot säilyvät, joten kartan sisäinen järjestys ei muutu. Hirviö, jolta puuttui 30 mikroaskelta vuoroon pelaajan lähtiessä, saa vuoronsa 30 mikroaskelta pelaajan paluun jälkeen.
- **Ajastimet** laskevat vain aktiivisia kierroksia. Myrkky, jota on jäljellä 3 kierrosta, jatkaa samasta kohdasta paluun jälkeen.
- **Siirtyjä.** `travel` muuntaa entiteetin herätyksen kohdekartan kelloon niin, että jäljellä oleva odotus säilyy. Jäädytetylle kartalle siirtyvä olento ei siis myöhästy, kun kartta sulatetaan. Siirto tehdään toiminnon lopussa (luku [6.7](#67-siirtyminen-kartalta-toiselle)), joten komennon oma herätys on silloin jo jonossa ja muuntuu samalla tavalla. Siirtymä kuluttaa aikaa vain sen komennon hinnan verran, joka kutsui `travel`-funktiota.
- **Kokoelmissa** olevat ilmentymät eivät ole kartalla. Ne saavat `turn`- ja ajastintapahtumia vain kantajan `modifies`-kokoelman kautta, kun kantaja on aktiivisella kartalla.
- **Taustakohtaukset.** `scope: map` päättyy jo pelaajan lähtiessä ([RS 9.9](../roguescript/09-kohtaukset-ja-aikajanat.md#99-taustakohtaukset)). Yleinen taustakohtaus kulkee kierrosten mukana kartasta riippumatta.

### 6.1.9 Esimerkki

Aloituskartalla on peikko (nopeus 10) ja lepakko (nopeus 20). Ne syntyvät kartan latauksessa ennen `begin`-tapahtumaa, ja pelaaja (nopeus 10, tila `POISONED`) syntyy `begin`-käsittelijässä. Herätykset merkitään muodossa hetki #järjestysnumero.

| Hetki | Vaihe | Tapahtuu | Uusi herätys |
| --- | --- | --- | --- |
| 0 | lataus | peikko ja lepakko syntyvät | peikko 120 #1, lepakko 60 #2 |
| 0 | `begin` | `control(pelaaja)` | pelaaja 0 #3 |
| 0 | 4 | pelaaja: `wake`, syöte `MOVE` (hinta 120) | pelaaja 120 #4 |
| 60 | 4 | lepakko: `ATTACK` (hinta 120, viive 60) | lepakko 120 #5 |
| 120 | 1 | `turn`, `game.turn` = 1; `POISONED` vahingoittaa pelaajaa | – |
| 120 | 4 | peikko #1: `APPROACH` | peikko 240 #6 |
| 120 | 4 | pelaaja #4: syöte `DIG`, `wait 120` | jatko 240 #7 |
| 120 | 4 | lepakko #5: `ATTACK` | lepakko 180 #8 |
| 180 | 4 | lepakko #8 | lepakko 240 #9 |
| 240 | 1 | `turn`, `game.turn` = 2 | – |
| 240 | 4 | peikko #6 | peikko 360 #10 |
| 240 | 4 | pelaaja #7: `wake`, kaivaa, `wait 120` | jatko 360 #11 |
| 240 | 4 | lepakko #9 | lepakko 300 #12 |

Hetkellä 120 peikko toimii ennen pelaajaa, koska sen herätys ajoitettiin ensin (#1 < #4). Myrkky vaikuttaa ennen kenenkään vuoroa, koska vaihe 1 tulee ennen herätyksiä. Pelaajalta kysytään syötettä seuraavan kerran vasta, kun `DIG` on valmis.

### 6.1.10 Istuimet ja ryhmä

Paikallisessa moninpelissä samalla koneella pelaa 1–8 ihmistä yhteistyössä yhteisellä ruudulla ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)). Sama malli kattaa ryhmän, jonka hahmojen välillä yksi pelaaja vaihtaa. Ilman `seats`-kenttää peli on yhden istuimen peli, ja tämän luvun säännöt palautuvat edellisten lukujen yksinpelin sääntöihin. Perustelut ovat lisädokumentissa [RLML-moninpeli](../RLML-moninpeli.md).

- **Istuin** on yksi ihmispelaaja: numero 1…8 ja jäsenet. Istuimet kuuluvat pelin tilaan ja tallentuvat, laitteet eivät (luku [8.2](08-syote.md#82-istuimet-ja-laitteet)).
- **Jäsen** on entiteetti, jolla on moottorin komponentti `seat` (kenttä `n`). Istuimella voi olla monta jäsentä, jolloin ne ovat ryhmä.
- **Ohjattava** on istuimen jäsen, jonka vuorolla moottori odottaa istuimen syötettä. Istuimella on enintään yksi ohjattava.
- **Tyhjä istuin** on istuin ilman jäseniä, esimerkiksi kuolleen pelaajan. Se katsoo peliä ja voi saada hahmon logiikalta.
- **`player`** on ohjattava, jonka vuoro on käynnissä, ja vuorojen välissä viimeksi vuorossa ollut. Yhden istuimen pelissä merkitys on sama kuin edellisissä luvuissa.
- **`players`** on kaikkien istuinten elävät jäsenet kokoelmana istuinnumeron ja jäseneksi tulon järjestyksessä. Järjestys poikkeaa luontijärjestyksestä (luku [4](04-satunnaisuus-ja-toistettavuus.md)).

**Toimija.** Jäsen on toimija, vaikka sillä ei ole `ai`-komponenttia ja vaikka se ei ole ohjattava. Jäsen pysyy siis jonossa ohjauksen vaihtuessa, jolloin vaihtamalla ei saa ilmaisia vuoroja. Toimijuus päättyy, kun ilmentymä menettää `seat`-komponentin eikä sillä ole `ai`-komponenttia, esimerkiksi ruumiin vaihdossa tai `part`-tapahtuman jälkeen. Datan rivi `- seat N` pääsee jonoon syntyessään kuten `ai`-ilmentymä (luku [3.2](03-pelin-tila.md#32-ilmentymän-syntyminen)).

**Vuoron valinta.** Vaihe 2 (luku [6.1.5](#615-vuoron-kulku)) ratkaistaan `wake`-käsittelijöiden jälkeen, joten käsittelijä voi vaihtaa ohjauksen ennen valintaa. Valinta riippuu `game`-lohkon kentästä `party`:

| Toimija | `party switch` (oletus) | `party each` |
| --- | --- | --- |
| ohjattava jäsen | istuimen syöte | istuimen syöte |
| muu jäsen | `ai`, ilman sitä lepo oletushinnalla | tulee ohjattavaksi, istuimen syöte |
| muu toimija | `ai` | `ai` |

- Kun moottori alkaa odottaa syötettä, `player` asetetaan vuorossa olevaan jäseneen. Tasatilanteessa istuimilla ei ole etuoikeutta, vaan järjestysnumero ratkaisee.
- Lepo onnistuu aina eikä tarvitse komentoa.
- **Ohjaus siirtyy.** Säännöllä `party switch` ohjaus siirtyy, kun ohjattavalta puuttuu herätys ja saman istuimen muulla jäsenellä on herätys. Silloin ensimmäinen herätetty muu jäsen tulee vaiheessa 2 ohjattavaksi ja odottaa istuimen syötettä. Ilman sääntöä halvaantunut ohjattava pitäisi pelin kulussa ilman syötettä, eikä pelaaja pääsisi vaihtamaan.

**`control`.**

- **`control(e)`**: jos `e` on istuimen jäsen, siitä tulee sen istuimen ohjattava. Muuten `e` korvaa `player`-hahmon: `e` saa tämän istuimen, ja entinen menettää `seat`-komponentin, joten ruumiin vaihto toimii kuten luvussa [6.1.2](#612-toimijat-ja-ajoitin). Kun `player` on `none` tai kuollut kahva, istuin on se, jolla `player` viimeksi oli. Jos sellaista ei ole, kuten `begin`-käsittelijässä, istuin on 1.
- **`control(e, seat: n)`** tekee `e`:stä istuimen `n` jäsenen ja ohjattavan, ja entinen ohjattava jää jäseneksi. Istuimen on oltava liitetty, muuten kutsu on käyttöhetken virhe. `e` lähtee entiseltä istuimeltaan.
- **Jonoon tulo** on kuten luvussa [6.1.2](#612-toimijat-ja-ajoitin): herätys hetkeen nyt vain, jos `e`:llä ei ole herätystä.
- **Vaihto kesken vuoron.** Jos ohjattava `A` odottaa syötettä ja maksuton komento kutsuu `control(B)` saman istuimen jäsenelle, `A`:n vuoro jatkuu vaiheesta 2 muun jäsenen säännöllä eli tekoälylle tai levolle. `B` toimii omalla herätyksellään. Vaihto ei anna ylimääräisiä vuoroja eikä vie niitä. Maksuton komento antaa istuimelle uuden vuoron samalla hetkellä vain, jos `B`:llä on herätys nyt (luku [6.1.3](#613-hinta-ja-nopeus)).
- `remove(e, seat)` irrottaa jäsenen istuimeltaan, ja `add(e, seat(n: N))` vaihtaa sen istuimelle N. Kun ohjattava lähtee istuimeltaan mistä tahansa syystä, istuimen seuraava elävä jäsen tulee ohjattavaksi, ja lähtijä on ohjattava uudella istuimella vain, jos sillä ei ole elävää ohjattavaa. `player` ei vaihdu istuinmuutoksessa: moottori asettaa sen, kun jäsen alkaa odottaa syötettä, ja syöte kelpaa vain istuimelta, jonka syötettä odotetaan. Entiteetti, jolla ei ole `seat`- eikä `ai`-komponenttia, ei ole toimija, joten `remove(player, seat)` päättää sen vuorot. Kenttä `seat.n` on vain luettava, joten istuin vaihtuu vain näillä kutsuilla ja `control`-kutsulla, ja istuintaulu ja komponentti pysyvät samoina. Kuollut jäsen ei kuulu joukkoihin `players` ja `members(n)`, eikä se ole ohjattava, mutta `seat_of` kertoo yhä sen istuimen, joten `after DIED` -käsittelijä tunnistaa jäsenen ([RS 10.2](../roguescript/10-aloitus-ja-checkpointit.md#102-checkpointit)).

**Säilytetty vuoro.** Kun pelaajan komento epäonnistuu tai on maksuton, vuoro jatkuu samalla hetkellä alkuperäisellä järjestysnumerolla (luku [6.1.5](#615-vuoron-kulku)), eikä saman hetken muu toimija pääse väliin. Jos jäsen on yhä ohjattava, moottori kysyy samalta istuimelta uudelleen. Jos maksuton komento vaihtoi ohjauksen (`SWITCH`), vuoro jatkuu muun jäsenen säännöllä eli tekoälylle tai levolle, ja uusi ohjattava toimii omalla herätyksellään.

**Kierrokset ilman pelaajan vuoroa** (luku [6.1.2](#612-toimijat-ja-ajoitin)) pätevät, kun yhdelläkään istuimella ei ole jäsentä, jolla on herätys. `pass` toimii miltä tahansa istuimelta.

**`idle`** koskee jokaista istuimen vuoroa erikseen. Ajastin alkaa, kun moottori alkaa odottaa istuimen syötettä, ja vain vuorossa olevan istuimen painallus nollaa sen.

**Aktiiviset kartat.** Aktiivisia ovat kaikki kartat, joilla on jäseniä, ja ilman jäseniä luvun [6.1.8](#618-aktiiviset-kartat-ja-jäädytys) kartta. Kartta-alue näyttää `player`-hahmon kartan. Vakiokirjasto pitää ryhmän samalla kartalla, joten useampi aktiivinen kartta on poikkeus.

- **Kellot.** Aktiivisten karttojen kellot etenevät yhdessä, ja niiden herätykset ovat samassa jonossa. Kartan kello pysähtyy vasta, kun kartta jäädytetään.
- **Jäädytys.** Lähtökartta jäädytetään, kun sen viimeinen jäsen lähtee (luku [6.7](#67-siirtyminen-kartalta-toiselle)). `transient`-kartta hylätään samalla hetkellä. Siirto samalle `transient`-kartalle luo sen uudelleen vain, jos kartalla ei ole muita jäseniä. Muuten siirto on tavallinen siirto kartan sisällä. Saman toiminnon monta `travel`-kutsua samaan `transient`-karttaan luovat sen kerran.
- **Kartta-alueen kartta.** Aktiivinen kartta tarkoittaa `game`-subjektin `turn`-tapahtumalle, paneeleille, listavalikoille ja `naming`-kyselylle kartta-alueen karttaa (luku [6.2](#62-tapahtumat)).
- `scope: map` -taustakohtaus päättyy, kun kartan viimeinen jäsen lähtee.

## 6.2 Tapahtumat

- **`emit` on synkroninen.** Moottori ajaa kaikki käsittelijät ennen kuin `emit` palaa. Sama koskee moottorin omia tapahtumia: ne on käsitelty, kun ne lähettänyt funktio palaa.
- **Vaiheet:** `before` (voi muuttaa parametreja tai `cancel`), `on` ja `after`. Moottorin tapahtumilla on vain vaiheet `on` ja `after`, koska ne kertovat jo tapahtuneesta. `before` tai `cancel` moottorin tapahtumassa on latausvirhe. Toiminnon estäminen kuuluu komentoon, joka kutsuu moottorin funktiota.
- **Jakelu:** käsittelijät järjestetään kunkin vaiheen alussa yhteen listaan prioriteetin (oletus 0, suurin ensin) ja sitten latausjärjestyksen mukaan. Kun sama käsittelijä osuu useaan kohteeseen, subjekti tulee ennen `modifies`-jäseniä ja jäsenet luontijärjestyksessä ([RS 4](../roguescript/04-tapahtumat-ja-kasittelijat.md)). Kesken vaiheen lisätyn komponentin käsittelijät tulevat mukaan seuraavasta vaiheesta, ja ennen vuoroaan poistetun komponentin tai jäsenen käsittelijät ohitetaan. `when`-ehto arvioidaan ennen käsittelijää. Käsittelijä, joka jättää pois tapahtuman oletusarvoiset loppuparametrit, saa ne jakelussa mutta ei näe niitä ([RS 4](../roguescript/04-tapahtumat-ja-kasittelijat.md)).
- **Subjekti käsittelijässä.** Yleinen käsittelijä saa subjektin ensimmäisenä parametrinaan. Komponentin käsittelijä ei saa sitä parametrina, koska subjekti on `self` tai kokoelman jäsenellä `holder`; sen parametrit alkavat tapahtuman toisesta parametrista. Sääntö on sama moottorin ja pelin tapahtumille ([RS 3.1](../roguescript/03-komponentit.md#31-määrittely)).
- **Moottorin tapahtumat:** `begin`, `turn`, `wake`, `created`, `removed`, `enter`, `leave`, `arrive`, `depart` ja `hear`. Moottori lähettää ne itse. Parametrit, subjekti ja laukeamishetki ovat luvussa [9.6](09-moottorin-funktiot-ja-tapahtumat.md#96-tapahtumat). Törmäys ja vierekkäisyys ovat vakiokirjaston tapahtumia (luku [9.7](09-moottorin-funktiot-ja-tapahtumat.md#97-raja-vakiokirjastoon)).
  - `turn` lähetetään kierroksen rajalla aktiivisten karttojen ilmentymille ja kerran yleisille käsittelijöille, joissa subjekti on `game`. Sillä ei ole parametreja (luku [6.1.4](#614-mikroaskeleen-vaiheet)).
  - `wake` lähetetään toimijalle sen jokaisella herätyksellä. Toimija on subjekti ja ainoa parametri (luku [6.1.5](#615-vuoron-kulku)).
  - `enter` ja `leave` koskevat ruutua: entiteetti astuu ruutuun tai poistuu siitä. `enter` saa myös lähtöruudun (`prev`), jotta vakiokirjasto voi laskea, mikä muuttui.
  - `arrive` ja `depart` koskevat karttaa: entiteetti saapuu kartalle tai lähtee siltä (luku [6.7](#67-siirtyminen-kartalta-toiselle)), myös `travel`-kutsussa nykyiseen karttaan. Myös siirto kokoelmaan ja kokoelmasta kartalle on lähtö ja saapuminen (luku [9.6](09-moottorin-funktiot-ja-tapahtumat.md#96-tapahtumat)).
- **Kontekstinimet.** Moottori sitoo nimet `self`, `holder`, `actor`, `it`, `player`, `game`, `profile`, `map` ja `camera` kuhunkin suorituskontekstiin. `profile` on pelikerran profiilinäkymä (luku [5.6](05-aloitus-tallennus-ja-checkpointit.md#56-profiili)), ja se on sama kaikissa konteksteissa. Ruutujoukkofunktiot käyttävät kontekstin map-karttaa, ja arrive- ja depart-käsittelijässä se on tapahtuman kartta.
- **Kuollut subjekti.** `emit`, jonka subjekti on kuollut kahva (luku [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)), ei jaa tapahtumaa, vaan palaa heti ilman virhettä. Jos subjekti kuolee kesken jakelun, jakelu jatkuu loppuun, myös `after`-vaihe, ja loput käsittelijät näkevät kuolleen kahvan. Muissa parametreissa kuollut kahva ei estä jakelua.

**Kutsusyvyys.** Sisäkkäiset suoritukset kasvattavat yhteistä syvyyttä: `emit`, moottorin tapahtuma, jonka moottorin funktio lähettää käsittelijän sisällä (esimerkiksi `move` `enter`-käsittelijässä), sekä `fn`-, `query`- ja `ai`-kutsut. Raja on `game.call_depth` (oletus 256). Ylitys on käyttöhetken virhe, joka päättää koko toiminnon eikä vain sisintä kutsua (luku [2.6](02-lataus.md#26-virheiden-lajit)). Ilmoitus näyttää pinon ja tiivistää toiston, esimerkiksi `enter(goblin, A) → enter(goblin, B) → … ×128`. Kehää ei tunnisteta semantiikassa, koska toistuva kutsu voi olla oikea sääntö, kuten heijastettu vahinko.

**Kontekstin kartta.** Kontekstinimi `map` ja ruutujoukkofunktiot käyttävät tämän taulukon karttaa:

| Konteksti | `map` |
| --- | --- |
| komento, tekoäly, `wake` | toimijan kartta |
| komponentin käsittelijä | subjektin kartta; kokoelman jäsenellä ylimmän kantajan kartta |
| yleinen käsittelijä | subjektin kartta; `arrive` ja `depart`: tapahtuman kartta |
| `begin` | aloituskartta (`game.start`) |
| `game`-subjektin `turn` | aktiivinen kartta (luku [6.1.8](#618-aktiiviset-kartat-ja-jäädytys)) |
| generaattori | generoitava kartta |
| etualan kohtaus | pelaajan kartta kohtauksen alkaessa (luku [6.1.7](#617-toiminto)); ilman pelaajaa käynnistäjän `map` `start`-hetkellä; `wait scene` -alikohtaus perii kutsujansa kartan |
| taustakohtaus | käynnistäjän `map` `start`-hetkellä; `scope: map` -taustakohtauksella sen kartta |
| `fn`, `query` | kutsujan `map` |
| paneeli, listavalikko, `naming`-kysely | aktiivinen kartta |

Jos kartta on `none`, esimerkiksi kun kantaja ei ole millään kartalla, ruutujoukkofunktiot palauttavat tyhjän joukon ja moottori kirjaa ajon varoituksen.

**Ruudut kuuluvat kartalle.** `pos`- ja `area`-arvo kantaa karttansa sisäisesti, vaikka kirjoitusasu ei näytä sitä: `pos(5, 3)` ja ruutujoukkofunktiot tuottavat arvon kontekstin kartalle, `e.at` on `e`:n kartan ruutu ja `game.at` aloituskartan joukko. Eri karttojen ruudut eivät ole koskaan yhtä suuret. `move` tai `spawn` toisen kartan ruutuun ja joukko-operaatiot eri karttojen joukoille ovat käyttöhetken virheitä, koska karttojen välillä siirrytään `travel`-kutsulla. Komponentin kenttään tai kartan muuttujaan tallennettu alue säilyttää karttansa ([D 5.1](../dataformaatit/05-komponentit-ja-entiteetit.md#51-komponentit), [D 8.4](../dataformaatit/08-kartat-ja-prefabit.md#84-kartan-muuttujat-vars)).

**Entiteetin kartta.** `e.map` on kartta, jolla `e` on. Kokoelman jäsenellä se on ylimmän kantajan kartta, ja kuolleella kahvalla kartta, jolla se oli tuhoutuessaan. Kuolleen subjektin käsittelijässä myös kontekstin `map` on tämä kartta, joten `after DIED` -käsittelijä näkee uhrin kartan.

## 6.3 Komennot

- **Parametrien kysyminen.** Kun pelaaja valitsee komennon, moottori kysyy parametrit lähteen mukaan: `from` listavalikolla, `aim` kohdistimella ja `ask` suuntana tai syötekenttänä. `count` saa arvonsa toistolaskurista ilman kysymystä (luku [8](08-syote.md)). Tekoäly antaa arvot suoraan, ja kysymys ohitetaan.
- **Sidonta.** Näppäin osoittaa komennon nimeen. Sidottu komento (`MOVE.NORTH = MOVE(north)`) täyttää parametrit.
- **Hinta.** `cost N` on mikroaskelia normaalinopeudella, ja oletus on 120. Nopeus skaalaa sen viiveeksi (luku [6.1.3](#613-hinta-ja-nopeus)).
- **Epäonnistuminen.** Runko, joka palauttaa `false`, epäonnistuu. Pelaajalta se ei vie aikaa, ja tekoäly menettää vuoronsa (luku [6.1.5](#615-vuoron-kulku)).
- **Monivaiheinen komento.** `wait N` keskeyttää komennon, kuluttaa N mikroaskelta nopeudella skaalattuna ja jatkaa sen jälkeen. Odottava komento tallentuu rakenteellisena polkuna (luku [6.1.6](#616-monivaiheinen-komento)). Toistuva komento (`count`) on monivaiheinen komento: se toistaa runkoaan itse ja päättää itse, milloin toisto keskeytyy ([RS 7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)).

## 6.4 Kohtaukset

- **Jono.** `start` laittaa etualan kohtauksen jonoon, ja moottori aloittaa sen, kun käynnissä oleva toiminto (luku [6.1.7](#617-toiminto)) on päättynyt ja sen siirrot on tehty. Kohtauksen kartta ratkaistaan sen alkaessa (luku [6.2](#62-tapahtumat)).
- **Kello.** Moottori kuljettaa kohtauksen esitysaikaa, pysäyttää sen modaalisen `say`-kutsun ja `wait confirm` -kohdan ajaksi ja kuljettaa raitoja ja silmukoita yhteisellä kellolla.
- **Tweenit.** Kestollinen rivi etenee annetun ajan. Moottori tietää funktion lajista (luku [9.2](09-moottorin-funktiot-ja-tapahtumat.md#92-funktion-kuvaus)), mitkä kutsut muuttavat tilaa ja mitkä ovat esitystä. Esitystä muuttava kutsu (`fade`, `pan` ja muut) liukuu arvosta toiseen. Tilaa muuttava kutsu jaetaan askeliksi, jotka ovat tavallisia tilan muutoksia tapahtumineen ([RS 9.1](../roguescript/09-kohtaukset-ja-aikajanat.md#91-rivi)): `move` kulkee rivin alussa kerran lasketun reitin ruutu kerrallaan, joukon tilen vaihto etenee ruutu kerrallaan, ja yhden ruudun `tile`, `spawn` ja `destroy` muuttuvat rivin alussa ja häivytetään. Pelin ajan taustakohtauksessa askeleet ovat kohtauksen herätyksiä herätysjonossa (luku [6.1](#61-aika-ja-vuorot)). Kesto kutsulle, jolla ei ole esitystä, on latausvirhe.
- **Luonnos.** Etualan kohtauksen tilan muutokset menevät luonnokseen, joka näkyy ruudulla. Moottori kirjaa luonnoksen maailmaan, kun kohtaus päättyy ([RS 9.8](../roguescript/09-kohtaukset-ja-aikajanat.md#98-luonnos-ohitus-ja-alusta-aloitus)).
- **Pelaajan toiminnot:** `skip` (ohitussäännöt, sitten kirjaus), `restart` (hylkää luonnoksen) ja `rewind` (hiljainen restart ja kelaus toistamalla tehdyt valinnat).
- **Taustakohtaukset.** `background` kulkee pelin ajassa kierroksina, ja sen rivit ajetaan kierroksen rajalla (luku [6.1.4](#614-mikroaskeleen-vaiheet)). Muutokset kirjataan heti. `background realtime` kulkee esitysajassa myös syötettä odottaessa eikä saa muuttaa tilaa. Moottori päättää kohtauksen `scope: map`- ja `bind`-ehtojen mukaan ja palauttaa kahvan `stop`-kutsua varten. `bind`-kohteen poistuminen tarkoittaa sen tuhoa (`removed`, luku [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)).

## 6.5 Generointi

Moottori tarjoaa generaattoreille primitiivit (`carve`, `tile`, `region`, `footprint`, `stamp`, `external`, `scatter`) sekä reitinhaun (`path`). `region` ja `footprint` ovat kyselyjä, joten ne käyvät myös generaattorin ulkopuolella. Generointialgoritmit (`STD.SCATTER_TILES`, `STD.SMOOTH`, `STD.ROOMS`, `STD.CONNECT`, `STD.PLACE`) ovat vakiokirjastoa ([STD 10](../vakiokirjasto/10-generointi.md)). Satunnaisuus tulee pelin virrasta. Generoidun kartan argumentit moottori laskee latauksessa vakiolausekkeina ([RS 8.2](../roguescript/08-generointi.md#82-generoitu-kartta)). Funktioiden parametrit ovat luvussa [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot).

**Pelin tila generaattorissa.** Generaattori ajetaan, kun kartta luodaan, ja se näkee sillä hetkellä `game`-tilan ja profiilinäkymän ([RS 8.2](../roguescript/08-generointi.md#82-generoitu-kartta)). `transient`-kartta luodaan uudelleen jokaisella saapumisella, joten sama kartan nimi voi tuottaa syvyyden mukaan eri tason (`game.DEPTH.now`).

**Generaattorin oikeudet.** Generaattori kirjoittaa generoitavaan karttaan: ruutuihin, nimiöihin, muuttujiin ja kartalle syntyviin entiteetteihin. Se saa synnyttää entiteettejä kartalle (`spawn`, `scatter`) ja kirjoittaa niiden kenttiä, koska ne kuuluvat karttaan. `game`-tila, profiili ja muiden karttojen entiteetit ovat sille vain luettavia, ja kirjoitus niihin on käyttöhetken virhe. Generaattorin `spawn` toiselle kartalle (`map:`) ja `travel` ovat samoin virheitä.

## 6.6 Paikkahaku ja ruutujoukot

Moottori toteuttaa datan haun `at` / `near` / `from` ja RogueScriptin ruutujoukkofunktiot samalla mallilla.

- **Metriikka** on Chebyshev.
- **Kerrokset.** Ruutu vastaa hakujoukkoa, jos minkä tahansa kerroksen tile on jäsen, myös `pass`-kerroksen ([D 7.4](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#74-hakeva-ja-tuottava-paikka)).
- **Tyhjä tulos** on datassa käyttöhetken virhe ja RogueScriptissä tyhjä joukko. Yhden ruudun tarvitseva funktio epäonnistuu silloin (luku [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)). Poikkeus on `travel`, jonka puuttuva kohdenimiö on käyttöhetken virhe (luku [6.7](#67-siirtyminen-kartalta-toiselle)).

## 6.7 Siirtyminen kartalta toiselle

`travel(entiteetti, KARTTA, at: NIMIÖ, align: bool = false)` siirtää entiteetin kohdekartan nimiöön ([RS 6.4](../roguescript/06-ruudut-ja-paikkahaku.md#64-siirtyminen-kartalta-toiselle), [D 8.5](../dataformaatit/08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)).

- **Saapumisruutu.** Oletuksena moottori arpoo saapumisruudun kohderyhmästä kutsujan virrasta. `align: true` valitsee kohderyhmästä ruudun, jolla on sama indeksi kuin siirtyjän lähtöruudulla omassa nimiöryhmässään. Jos kohderyhmässä on vähemmän ruutuja, valitaan viimeinen. Jos lähtöruudulla ei ole nimiötä, ruutu arvotaan. Puuttuva `at` vie nimiöön `START`.
- **Puuttuva nimiö** kohdekartassa on virhe. Piirretylle kartalle lataus tarkistaa sen, kun kartta ja nimiö ovat kutsussa vakioita. Generoidulle kartalle se on käyttöhetken virhe.
- **Tapahtumat.** Moottori siirtää entiteetin ja lähettää sitten `leave(who, p)` ja `depart(who, lähtökartta, p)` sekä `arrive(who, kohdekartta, saapumisruutu)`. Saapumisruudussa ei lähetetä tapahtumaa `enter`, joten pelaaja ei pomppaa heti takaisin. Järjestys: kohdekartta ladataan ja nimiö tarkistetaan travel-kutsun hetkellä, jolloin kohdekartan created-tapahtumat lähetetään. Siirto ja tapahtumat leave, depart ja arrive tehdään toiminnon lopussa ensimmäisenä (luku [6.1.7](#617-toiminto)), joten käsittelijät eivät näe siirtoa kesken jakelun, ja kutsun jälkeiset rivit näkevät entiteetin vielä lähtökartalla. Jos sama toiminto siirtää saman entiteetin useasti, viimeinen kutsu ratkaisee. Saman toiminnon etualan kohtaukset alkavat siirtojen jälkeen. Aktiivinen kartta vaihtuu mikroaskeleen lopussa: lähtökartta ajaa saman hetken loput herätykset ennen jäädytystä, ja kohdekartta jatkaa kellostaan.
- **Aika.** Siirtyjän herätys muunnetaan kohdekartan kelloon, ja jäljellä oleva odotus säilyy. Kun pelaaja siirtyy, lähtökartta jäädytetään ja kohdekartta sulatetaan (luku [6.1.8](#618-aktiiviset-kartat-ja-jäädytys)). Moninpelissä lähtökartta jäädytetään vasta, kun sen viimeinen jäsen lähtee (luku [6.1.10](#6110-istuimet-ja-ryhmä)).
- **Pysyvyys.** Kohdekartta ladataan, kun sille saavutaan ensimmäisen kerran, ja se säilyy sen jälkeen: paluu löytää kartan sellaisena kuin se jäi, ja tallennus sisältää kaikki ladatut kartat. Kullakin kartan nimellä on yksi ilmentymä. `transient`-kartta ([D 8.1](../dataformaatit/08-kartat-ja-prefabit.md#81-karttatiedosto), [RS 8.2](../roguescript/08-generointi.md#82-generoitu-kartta)) hylätään, kun pelaaja lähtee siltä, ja se ladataan tai generoidaan uudelleen seuraavalla saapumisella. Sen entiteetit poistuvat ilman tapahtumia, ja niiden kahvat kuolevat.

**Sama kartta.** `travel` siirtyjän nykyiseen karttaan on aina sallittu, ja sen tapahtumat ovat samat kuin muussakin siirrossa: `leave`, `depart` ja `arrive`, joten automaattinen tallennus `on arrive` toteutuu.

- **`transient`-kartta** hylätään ja luodaan uudelleen toiminnon lopussa ennen saapumista, kuten pelaajan lähtiessä. Moninpelissä näin käy vain, jos kartalla ei ole muita jäseniä (luku [6.1.10](#6110-istuimet-ja-ryhmä)). Siirtyjä ja sen kokoelmat säilyvät, mutta kartan muut entiteetit poistuvat ilman tapahtumia. Uusi kartta saa uuden kellon arvosta `game.time`, ja generaattori näkee sillä hetkellä voimassa olevan `game`-tilan (luku [6.5](#65-generointi)). Näin yksi kartta `LEVEL` riittää koko luolastolle.
- **Pysyvä kartta** säilyttää tilansa ja kellonsa, ja siirtyjä vain siirtyy nimiöön. Ero `move`-kutsuun on tapahtumissa: `move` lähettää `leave` ja `enter`, `travel` tapahtumat `leave`, `depart` ja `arrive`.
- **Muut toimijat.** Pelaajan siirtyessä samalle kartalle aktiivinen kartta ei vaihdu, joten jäädytystä ja sulatusta ei tehdä. `transient`-kartan uusi ilmentymä on aktiivinen heti saapumisesta.

**Synnytys toiselle kartalle.** `spawn(x, map: KARTTA, at: NIMIÖ)` synnyttää kohdekartan nimiön ruutuun. Kohdekartta ladataan tarvittaessa kuten `travel`-kutsussa, ja sen `created`-tapahtumat lähetetään latauksessa. Synnytys tehdään heti eikä toiminnon lopussa, ja syntynyt toimija saa herätyksen kohdekartan kellossa (luku [6.1.8](#618-aktiiviset-kartat-ja-jäädytys)). `transient`-kartalle synnytetty katoaa kartan mukana. Puuttuva nimiö on käyttöhetken virhe kuten `travel`-kutsussa.

## 6.8 Äänet ja kuuleminen

Pelimaailman ääni on pelin tilaa, jonka olennot kuulevat. Se on eri asia kuin ääniefekti, joka on esitystä. Äänilajit ilmoitetaan datassa osiossa `sounds` ([D 12.3](../dataformaatit/12-aanten-ilmoitukset.md#123-pelimaailman-äänet)).

- **Kertaääni.** `noise(p: pos, kind: name, source: entity = none, rounds: number = 1)` kirjaa äänilajin `kind` ruutuun `p`. Kertaäänilaji kuuluu vain syntyhetkellä, ja sille annettu `rounds` on käyttöhetken virhe. Moottori ei päätä, mikä pitää ääntä: vakiokirjaston liikekomento kutsuu `noise`-funktiota, joten paikallaan oleva ei kuulu.
- **Jatkuva ääni.** `continuous`-lajin `noise` soi `rounds` kierrosta kartan kellossa. Entiteetti, jolla on komponentti `sound`, on jatkuvan äänen lähde niin kauan kuin se on olemassa ja komponentti on paikallaan.
- **Leviäminen.** Ääni leviää ruudusta toiseen kahdeksaan suuntaan, ja jokainen askel vähentää tasoa yhdellä. Tile tai entiteetti, jonka esto sisältää arvon `sound`, pysäyttää äänen. Kuultu taso on lähteen taso miinus etäisyys plus kuulijan `hearing`-komponentin `acuity`. Taso 0 tai alle ei kuulu.
- **Peitto.** Kuulija kuulee herätysvälinsä äänet: edellisen herätyksen jälkeen syntyneet kertaäänet ja välin aikana soineet jatkuvat äänet. Välin kovimmin kuultu ääni peittää äänet, jotka kuuluvat sitä vähintään `game.masking` tasoa hiljempaa (oletus 3). Vieressä huutava (taso 8) peittää kahden ruudun päästä kuuluvat askeleet (taso 1), ja vieressä soiva soittorasia peittää askeleet koko ajan. Kuulija kuulee myös omat äänensä, ja ne peittävät muita: oma askel (taso 3 omassa ruudussa) peittää kaukaiset askeleet. Kuunteleva pysyy siksi paikallaan tai hiipii (vakiokirjasto, `SNEAK`).
- **Arvo ja kyselyt.** Kuultu ääni on arvo `sound` (RogueScript, luku [2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)), jolla on kentät `.kind`, `.at` (lähteen ruutu), `.source` (entiteetti tai `none`) ja `.level` (kuultu taso). `heard(a: entity) -> collection sound` palauttaa a:n herätysvälin kuuluvat äänet kovimmasta alkaen, ja `hears(a: entity, b: entity) -> bool` on tosi, jos jonkin niistä lähde on b. Ilman `hearing`-komponenttia tulos on tyhjä.
- **Tapahtuma.** Kun kuulija herää, moottori lähettää ennen sen vuoroa tapahtuman `hear(who, s)` jokaisesta uudesta kuuluvasta äänestä kovimmasta alkaen. Omasta äänestä (`s.source == who`) tapahtumaa ei tule, vaikka ääni peittää. Jatkuvasta äänestä tapahtuma tulee vain, kun se alkaa kuulua. Vakiokirjasto kirjoittaa pelaajalle lajin kuvauksen `<LAJI>.DESC` lokiin, kun lähde ei näy.
- **Ääniefekti.** Kun pelaaja kuulee äänen, moottori soittaa lajin `effect`-äänen kuullun tason mukaisella voimakkuudella. Moninpelissä ääni soi kerran, kun yksikin jäsen kuulee sen, ja voimakkuus on kovin kuultu taso. Jatkuva ääni soi silmukkana niin kauan kuin se kuuluu.
- **Jäädytys.** Jäädytetyn kartan äänet eivät kuulu, ja niiden kestot odottavat kartan kellon mukana.

## 6.9 Kiertävä kartta

Otsikon määre `wrap` ([D 8.1](../dataformaatit/08-kartat-ja-prefabit.md#81-karttatiedosto)) tekee kartasta kiertävän molempien akselien suhteen: oikean reunan takana on vasen reuna ja alareunan takana yläreuna.

- **Koordinaatit.** `pos(x, y)` ja siirtymät lasketaan modulo kartan koko, joten `actor.at + east` oikeassa reunassa on vasemman reunan ruutu. Ruudun `x` ja `y` ovat aina välillä 0…koko − 1.
- **Etäisyys ja joukot.** `distance`, `near`, `from`, `rect`, `line`, `ray`, `path` ja `adjacent` käyttävät lyhintä kiertävää etäisyyttä. `line` ja `ray` kulkevat lyhyempää reittiä, ja tasatilanteessa positiiviseen suuntaan.
- **Näkö, valo ja ääni** leviävät reunan yli samoin säännöin.
- **Reunat.** `edge` palauttaa tyhjän joukon, joten reunanimiöitä ei tarvita.
- **Esitys.** Kamera piirtää kartan reunan yli (luku [7.1](07-esitys.md#71-asettelu)).

## 6.10 Atlas

Atlaksen lehdet ([D 8.12](../dataformaatit/08-kartat-ja-prefabit.md#812-atlas-karttalehdet)) ovat tavallisia karttoja, mutta niiden reunat liittyvät naapurilehtiin. Moottori siirtää kulkijan reunan yli, joten lehtien väliin ei tarvita nimiöitä eikä siirtymäkäsittelijöitä.

- **Reunan ylitys.** Kun liike veisi entiteetin lehden reunan yli ja suunnassa on lehti atlaksen ruudukossa, kohderuutu on naapurilehden vastaava ruutu: itään ylittävä liike lehdeltä B10 ruudusta (L−1, y) vie lehdelle C10 ruutuun (0, y). Vinoliike kulman yli vie vinossa olevalle lehdelle. Ruudukon ulkoreuna on tavallinen kartan reuna.
- **Esto.** Liikekomento tarkistaa kohderuudun eston naapurilehdeltä samoin kuin lehden sisällä (luku [6.3](#63-komennot)); naapurilehti on lähialueessa valmiiksi ladattuna. Törmäys lähettää `STD.BUMP`-tapahtuman kuten muulloinkin.
- **Siirto.** Ylitys on siirto kartalta toiselle (luku [6.7](#67-siirtyminen-kartalta-toiselle)): se tehdään toiminnon lopussa, ja se lähettää tapahtumat `leave`, `depart` ja `arrive`. Jäädytystä ei tehdä lehti kerrallaan vaan lähialueittain (alla), ja moninpelissä jäsenet voivat olla eri lehdillä.
- **Puuttuva lehti** luodaan atlaksen `fill`-tilestä, kun se tulee ensimmäisen kerran lähialueeseen, ja se säilyy kuten muutkin kartat. Jos `fill` estää liikkeen tai puuttuu, ylitys estyy.
- **Lähialue.** Lehden lähialue on lehti ja sen naapurit, enintään 3 × 3 lehteä. Haut ja aistit ulottuvat yhden lehtirajan yli: `near`, `from`, `rect`, `line`, `ray`, `path`, `adjacent` ja etäisyys sekä näkö, valo ja ääni lasketaan lähialueella atlaksen koordinaatein, ja tulos voi sisältää naapurilehtien ruutuja. Jokainen ruutu kantaa lehtensä kuten muutkin ruudut ([RS 2](../roguescript/02-yleinen-syntaksi.md)). Lähialueen ulkopuolelle osuvat ruudut putoavat pois kuten kartan reunan yli. `tiles()`, `at(HAKU)`, `label` ja `edge` koskevat kontekstin lehteä, ja datan paikkahaku ([D 8.8](../dataformaatit/08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)) pysyy lehdellä.
- **Säteet.** Koska haku ulottuu vain yhden lehtirajan yli, lehden leveyden ja korkeuden on oltava vähintään pelin suurin säde. Lataus varoittaa, jos jonkin entiteetin `sight`-, `light`- tai `hearing`-säde on suurempi kuin lehden pienempi mitta.
- **Aktiiviset lehdet.** Pelaajan lehden koko lähialue on aktiivinen (luku [6.1.8](#618-aktiiviset-kartat-ja-jäädytys)): kaikki sen lehdet kulkevat samalla kellolla, ja niiden toimijat heräävät. Naapurilehden hirviö, joka näkee pelaajan, jahtaa sitä siis rajan yli, eikä raja ole pakotie. Kun pelaaja ylittää rajan, lähialue siirtyy: pois jäävät lehdet jäädytetään ja uudet ladataan tai sulatetaan. Moninpelissä aktiivisia ovat jäsenten lähialueiden yhdiste.
- **Liike ja reitti.** `move` ja reitin seuraaminen naapurilehden ruutuun on reunan ylitys yllä olevin säännöin. `path` voi kulkea lehtirajan yli lähialueella.
- **Esitys.** Kamera näyttää lähialueen naapurilehdet kartta-alueen reunassa, joten lehtiraja ei näy (luku [7.1](07-esitys.md#71-asettelu)).

---

← [5. Aloitus, tallennus ja checkpointit](05-aloitus-tallennus-ja-checkpointit.md) · [Sisällys](README.md) · [7. Esitys](07-esitys.md) →
