# RLML-moninpeli

6.11.2026 · RLML 0.3:n lisädokumentti

Dokumentti kuvaa paikallisen moninpelin: 1–N pelaajaa samalla koneella, yhteistyö yhteisellä ruudulla. Sama malli kattaa ryhmän, jonka hahmojen välillä yksi pelaaja vaihtaa. Luvut 1–8 ovat speksi ja luku 9 toteutussuunnitelma. Kun speksi on viety päädokumentteihin, tämä dokumentti jää suunnitelmaksi ja päätösten perusteluksi.

**Tarkistettu 6.11.2026.** Dokumentti on verrattu päädokumentteihin, ja seuraavat kohdat on korjattu:

- `TAKE_EXIT`in vahvistus on dialogi, ei `wait confirm`.
- Uutta tekoälyä `COMPANION` ei ole, vaan seuraaja on STD:n `FOLLOWER`.
- STD ei sido näppäimiä.
- Jäsenet eivät suutu toisilleen ystävällisestä tulesta.
- Ohjaus siirtyy, kun ohjattavalla ei ole herätystä.
- Aktiiviset ja väliaikaiset kartat on määritelty useammalle jäsenelle.
- Kamera ei sido nimeen.
- Uusi kysely `in_view`.
- Vuoron merkki on asettelun määre.

Vaihe 0:n luettelo (luku 9) on täydennetty.

**Tila 7.11.2026.** Vaihe 0 on tehty: speksi on viety päädokumentteihin (D, RS, M 6.1.10 ja 8.2, STD) ja `lib/std`:hen. Vaiheet 1 ja 2 ovat ytimessä:

- istuintaulu, vuoron valinta, ohjauksen siirto ja `control(e, seat:)`
- kyselyt ja yhteinen näkö
- syöte istuimittain (`Runner::input`, `awaiting`)
- latauksen tarkistukset

Toteuttamatta ovat:

- useampi aktiivinen kartta (ryhmä jakaa `player`-hahmon kartan)
- kesken pelin liittyminen ja poistuminen sekä tapahtumat `join` ja `part` ajossa. Kun `part` kytketään, käsittelijät ajetaan ennen kuin `State::part_seat` irrottaa jäsenet istuimelta. Muuten `PART_PLAYER` saisi tyhjän `members(seat)`-joukon. Samalla poistuvien jäsenten herätykset, `player` ja odotettu istuin on päivitettävä.
- liittymisruutu (näppäimistöosat, peliohjaimet ja reititys toimivat päätteessä, ja laitteet jaetaan istuimille järjestyksessä, M 8.2)
- `follow(camera)` logiikasta (kameran ryhmäseuranta ja vuoron merkki toimivat päätteessä)
- vaiheet 3–8

Viittaukset: D = RLML-dataformaatit, RS = RogueScript, M = RLML-moottorin vastuut, STD = RLML-vakiokirjasto.

## 1. Tavoite ja rajaus

- **Yhteistyö yhteisellä ruudulla.** Pelaajat jakavat kartan, näön, muistin ja lokin.
- **Vuoropohjainen.** Aika ja vuorot kulkevat kuten M 6.1. Pelaajat eivät pelaa samanaikaisesti, vaan jokainen hahmo saa vuoronsa ajoittimesta, ja moottori odottaa sen pelaajan syötettä, jonka hahmo on vuorossa.
- **Yksinpeli ei muutu.** Ilman `seats`-kenttää peli on yhden istuimen peli, ja kaikki säännöt palautuvat nykyisiksi. Nykyiset pelit ja testipelit toimivat sellaisenaan.
- **Rajattu pois:** kilpailu ja salainen tieto (hot seat, istuinkohtainen muisti), jaettu ruutu ja verkkopeli. Malli ei sulje niitä pois (luku 10).

## 2. Käsitteet

- **Istuin** on yksi ihmispelaaja: numero 1…N, syötelaite ja jäsenet. Istuimet ovat pelin tilaa ja tallentuvat, laitteet eivät.
- **Jäsen** on hahmo, jolla on moottorin komponentti `seat` (kenttä `n: number`). Istuimella voi olla monta jäsentä, jolloin ne ovat ryhmä.
- **Ohjattava** on istuimen jäsen, jonka vuorolla moottori odottaa istuimen syötettä. Istuimella on enintään yksi ohjattava.
- **Tyhjä istuin** on istuin ilman jäseniä, esimerkiksi kuolleen pelaajan. Se katsoo peliä ja voi saada hahmon logiikalta.
- **`player`** on ohjattava, jonka vuoro on käynnissä, ja vuorojen välissä viimeksi vuorossa ollut. Yhden istuimen pelissä merkitys on sama kuin nyt.
- **`players`** (uusi moottorin nimi) on kaikkien istuinten elävät jäsenet kokoelmana, järjestyksessä istuinnumero ja sitten jäseneksi tulon järjestys.

## 3. Data

### 3.1 `game`-lohkon kentät

```
game KNIGHTS
- seats 1..4
- party switch
- leash on
```

- **`seats a..b`** on liitettävien istuinten vähimmäis- ja enimmäismäärä. Oletus on `1..1`. Moottorin raja on 8, ja `a` on vähintään 1. Muut arvot ovat latausvirheitä.
- **`party switch | each`** kertoo, miten istuimen muut kuin ohjattavat jäsenet toimivat (luku 5.2):
  - `switch` (oletus): vain ohjattava odottaa syötettä, ja muut jäsenet toimivat tekoälyllään. Pelaaja vaihtaa ohjattavaa napilla (Lost Vikings, Ultima).
  - `each`: jokainen jäsen odottaa istuimen syötettä omalla vuorollaan (taktiikkapelit).
- **`leash on | off`** kytkee STD:n talutushihnan (luku 7.3). Oletus on `on`, kun `seats`-yläraja on yli 1, ja muuten `off`.
- **Lukitus.** Istuinmäärä lukitaan uuden pelin liittymisruudussa ja tallentuu pelikerran mukana kuten autosaven paikkamäärä (M 5.4). Liittyminen ja poistuminen kesken pelin pysyvät välin `a..b` sisällä.

### 3.2 Entiteetin istuin

Jäsenyys on moottorin komponentti: `- seat 1` datassa tai `control(e, seat: 1)` logiikassa (luku 5.3). Yleensä logiikka antaa sen, ja datan rivi on testejä ja valmiita ryhmiä varten. Istuinnumero yli `seats`-ylärajan on latausvirhe.

### 3.3 Näppäimet ja laitteet

Näppäimistön voi jakaa usealle istuimelle näppäimistöosilla. Osa on näppäimistön sidontojen joukko, ei istuinnumero, koska istuinten numerot riippuvat liittymisjärjestyksestä.

```
keys GAME
- MOVE.NORTH k up
- SWITCH tab

keys GAME keyboard 2
- MOVE.NORTH w
- SWITCH q
- join e
```

- **Osa 1** on osio ilman `keyboard`-määrettä, eli nykyiset sidonnat. `keyboard N` on lisäosa.
- **Ristiriita.** Sama näppäin kahdessa näppäimistöosassa samassa kontekstissa on latausvirhe, koska moottori ei tietäisi, kenen painallus se on.
- **Peliohjaimet** käyttävät samaa `pad`-osiota (D 11.3). Laite erottaa istuimet, joten ohjainkohtaisia osioita ei tarvita.
- **Hiiri** kuuluu näppäimistöosan 1 istuimelle. Jos osaa 1 ei ole liitetty, hover toimii kaikille, mutta hiiren toiminnot ovat sitomattomia.
- **`join`** on uusi moottorin toiminto. Moottori antaa sille oletussidonnat vain kontekstissa `JOIN` (luku 4.1): `enter` näppäimistöllä ja `start` peliohjaimella. Pelin peruskontekstissa peli sitoo `join`in itse jokaiseen osaan, josta liittyminen kesken pelin on mahdollista. Lisäosa ilman `join`-sidontaa saa latausvaroituksen.
- **Muut kontekstit.** Konteksti, jolla ei ole `keyboard N` -osioita, käyttää osan 1 sidontoja kaikille näppäimistön istuimille. Tämä koskee esimerkiksi moottorin oletuksia kontekstissa `MENU`. Näppäimistö on silloin yksi laite, ja reititys (luku 6.5) rajaa ohjaajaksi avaajan istuimen. Lisäosan voi antaa mille tahansa kontekstille (`keys MENU keyboard 2`).
- **Tunniste.** Osio tunnistetaan kontekstin nimestä ja osan numerosta, joten `keys GAME` ja `keys GAME keyboard 2` ovat eri lohkoja yhdistämisessä ja korvaamisessa. `modal` ja `passthrough` ovat kontekstin ominaisuuksia, ja ne annetaan osassa 1. Lisäosan eri arvo on latausvirhe.
- Pelaajan `overrides/keys.rlml` paikkaa myös `keyboard N` -osiot.

## 4. Istuimet ajossa

### 4.1 Uusi peli ja liittymisruutu

Järjestys on moottorin pelivalitsin, pelin alkuvalikko, `new_game`, liittymisruutu ja laskettu aloitus. Liittymisruutu näytetään vain, kun `seats`-yläraja on yli 1.

- **Konteksti `JOIN`.** Laite, joka valitsi `new_game`, on istuin 1. Muut laitteet liittyvät toiminnolla `join` ja saavat seuraavan vapaan numeron. `back` liitetyltä laitteelta poistaa sen istuimen, ja `confirm` miltä tahansa liitetyltä laitteelta aloittaa pelin, kun istuimia on vähintään `a`.
- **Sama näppäin.** Oletuksena `enter` on kontekstissa sekä `join` että `confirm`. Liittämättömältä laitteelta se on `join` ja liitetyltä `confirm`, joten tämä ei ole ristiriita.
- **Alkuvalikot.** `new_game`, joka ohittaa alkuvalikot (M 8), ei ohita liittymisruutua.
- **Toistettavuus.** Laskettu aloitustila riippuu siemenestä, datasta, profiilista ja liitettyjen istuinten määrästä (M 5.1). Liitetyt istuimet ovat tiedossa ennen `begin`-tapahtumaa, ja `seats()` palauttaa ne.

### 4.2 Liittyminen kesken pelin

- `join` pelin peruskontekstissa vapaalta laitteelta liittää uuden istuimen, kun istuimia on alle ylärajan. Etualan kohtauksen, dialogin tai valikon aikana painallus hylätään eikä jonoudu (luku 6.5). Pelaaja painaa uudelleen, kun ne ovat päättyneet.
- Liittyminen on oma toimintonsa kuten valikon toiminto (M 6.1.6). Moottori antaa istuimelle vapaan numeron ja lähettää toiminnon lopussa moottorin tapahtuman `join(seat)`, jonka subjekti on `game`. Käsittelijä ei kirjoita subjektia parametriksi, kuten `turn()`-käsittelijäkään. Liittyminen kirjataan syötteeksi toistoon ja tallennukseen.
- **Lopun työt.** `join` ja `part` ajetaan toiminnon lopun töinä (M 6.1.7) heti siirtojen jälkeen ja ennen rollbackia. Saman toiminnon tapahtumat ajetaan istuinnumeron mukaisessa järjestyksessä.
- STD:n `JOIN_PLAYER` antaa istuimelle hahmon (luku 7.2). Jos mikään käsittelijä ei anna, istuin jää tyhjäksi.

### 4.3 Poistuminen

- Moottorin toiminto `leave_seat` on taukovalikossa vain, kun istuimia on yli `a`. Viimeinen istuin poistuu `quit`-toiminnolla.
- Moottori lähettää toiminnon lopussa tapahtuman `part(seat)`. Käsittelijöiden jälkeen istuimen jäsenet menettävät `seat`-komponentin, ja istuin ja laite vapautuvat. Muiden istuinten numerot eivät muutu, ja vapautunut numero annetaan seuraavalle liittyjälle.
- STD:n `PART_PLAYER` antaa jäsenille seuraajan tekoälyn, joten hahmot jäävät ryhmään (luku 7.2).

### 4.4 Tallennus, lataus ja toisto

- **Tallennukseen** kuuluvat istuinten numerot, jäsenyydet, ohjattavat ja `party`-tila. Laitteet eivät kuulu.
- **Latauksessa** liittymisruutu kysyy laitteen jokaiselle tallennetulle istuimelle numerojärjestyksessä. Istuin, jolle ei tule laitetta (`skip` tai `confirm` ennen sitä), poistuu kuten `part` ennen ensimmäistä syötettä. Uusia istuimia voi liittää ylärajaan asti.
- **Automaattinen tallennus.** `on input every N` (D 3.4) laskee syötteen odotukset kaikilta istuimilta yhteen. `on arrive` tallentaa kerran toiminnossa, jossa yksi tai useampi jäsen saapuu kartalle.
- **Jäsen kartalla.** Säännöt, jotka puhuvat pelaajasta kartalla, koskevat mitä tahansa jäsentä. `nosave` (M 5.2) estää tallennuksen, kun yksikin jäsen on `nosave`-kartalla.
- **`slots 1`.** Pelikerran oma tallennus lopettaa pelin kaikilta istuimilta (M 5.4).
- **Rollback.** Checkpoint sisältää istuintaulun. `rollback()` palauttaa jäsenyydet ja ohjattavat checkpointin mukaisiksi. Istuin, joka on liitetty checkpointin jälkeen, säilyy liitettynä mutta jää tyhjäksi. Laitteet eivät muutu.
- **Toisto.** Syötelokin jokainen rivi kertoo istuimen. Syöteloki on uusi kohta M 4:ään: rivi on (istuin, toiminto), ja `join` ja `leave_seat` ovat rivejä muiden joukossa.

## 5. Vuorot

Luku muuttaa M 6.1.2:ta ja M 6.1.5:tä.

### 5.1 Toimija

Toimija on istuimen jäsen tai kartalla oleva ilmentymä, jolla on `ai`.

- **Jäsen pysyy jonossa**, vaikka se ei ole ohjattava ja vaikka sillä ei ole `ai`-komponenttia. Tämä korvaa säännön "entinen pelaaja jää jonoon vain, jos sillä on `ai`", ja estää vaihtamalla saatavat ilmaiset vuorot: herätys säilyy vaihdon yli.
- **Toimijuus päättyy**, kun ilmentymä menettää `seat`-komponentin eikä sillä ole `ai`-komponenttia, esimerkiksi ruumiinvaihdossa tai `part`-tapahtuman jälkeen.
- **Datan jäsen.** Datan rivi `- seat N` tekee entiteetistä toimijan. Luonnin vaiheessa 8 (M 3) se pääsee jonoon ilman `ai`-komponenttiakin.

### 5.2 Vuoron valinta

Vaihe 2 (M 6.1.5) ratkaistaan vasta `wake`-käsittelijöiden jälkeen, joten käsittelijä voi vaihtaa ohjauksen ennen valintaa.

| Toimija | `party switch` | `party each` |
| --- | --- | --- |
| ohjattava jäsen | istuimen syöte | istuimen syöte |
| muu jäsen | `ai`, ilman sitä lepo (oletushinta) | tulee ohjattavaksi, istuimen syöte |
| muu toimija | `ai` | `ai` |

Kun moottori alkaa odottaa syötettä, `player` asetetaan vuorossa olevaan jäseneen. Tasatilanteessa istuimilla ei ole etuoikeutta, vaan järjestysnumero ratkaisee kuten ennen.

M 6.1.5:n epäonnistumistaulu saa rivin "muu jäsen ilman `ai`": lepo onnistuu aina ja maksaa oletushinnan.

**Ohjaus siirtyy.** Säännöllä `party switch` ohjaus siirtyy, kun ohjattavalta puuttuu herätys, esimerkiksi halvaantuneelta, ja saman istuimen muulla jäsenellä on herätys. Silloin ensimmäinen herätetty muu jäsen tulee vaiheessa 2 ohjattavaksi ja odottaa istuimen syötettä. Ilman sääntöä moottori voisi kulkea loputtomasti kysymättä syötettä, eikä pelaaja pääsisi edes käyttämään komentoa `SWITCH`.

### 5.3 `control`

- **`control(e)`**: jos `e` on istuimen jäsen, siitä tulee sen istuimen ohjattava. Muuten `e` korvaa nykyisen `player`-hahmon: `e` saa tämän istuimen, ja entinen menettää `seat`-komponentin. Ruumiinvaihto toimii siis kuten ennen. Kun `player` on `none` tai kuollut kahva, istuin on se, jolla `player` viimeksi oli. Jos sellaista ei ole, kuten `begin`-käsittelijässä, istuin on 1.
- **`control(e, seat: n)`** tekee `e`:stä istuimen `n` jäsenen ja ohjattavan. Entinen ohjattava jää jäseneksi. Istuimen on oltava liitetty, muuten kutsu on käyttöhetken virhe.
- **Jonoon tulo** on kuten M 6.1.2: `control` antaa herätyksen hetkeen nyt vain, jos `e`:llä ei ole herätystä.
- **Vaihto kesken vuoron.** Jos ohjattava `A` odottaa syötettä ja maksuton komento kutsuu `control(B)` saman istuimen jäsenelle, `A`:n vuoro jatkuu vaiheesta 2 muun jäsenen säännöllä, eli tekoälylle tai levolle. `B` toimii omalla herätyksellään. Vaihto ei anna ylimääräisiä vuoroja eikä vie niitä.
- **Maksuton vaihto** (M 6.1.3). Hinta 0 antaa pelaajalle uuden vuoron samalla hetkellä. Pelaaja tarkoittaa tässä istuinta, joten `SWITCH` jatkaa saman istuimen vuoroa uudella ohjattavalla vain, jos tällä on herätys nyt. Muuten istuimen vuoro päättyy, ja edellinen ohjattava toimii vaiheen 2 säännöllä.
- **Uudet funktiot.** `seats() -> list` palauttaa liitetyt istuinnumerot, `seat_of(e) -> number` istuimen tai `none`, `members(n) -> collection` istuimen jäsenet ja `active(n) -> entity` ohjattavan tai `none`. Kaikki ovat kyselyjä eivätkä muuta tilaa.
- **Järjestys.** `players` ja `members(n)` poikkeavat M 4:n säännöstä, jonka mukaan joukot ovat luontijärjestyksessä. Ne ovat istuinnumeron ja jäseneksi tulon järjestyksessä.

### 5.4 Kierrokset ilman pelaajan vuoroa

M 6.1.2:n kierrokset ilman pelaajan vuoroa pätevät, kun yhdelläkään istuimella ei ole jäsentä, jolla on herätys. `pass` toimii miltä tahansa istuimelta.

### 5.5 `idle`

`idle`-asetus koskee jokaista istuimen vuoroa erikseen. Ajastin alkaa, kun moottori alkaa odottaa istuimen syötettä, ja vain vuorossa olevan istuimen painallus nollaa sen. Hidas pelaaja ei siis pysäytä muita, jos asetus on päällä.

### 5.6 Aktiiviset kartat

Aktiivisia ovat kaikki kartat, joilla on jäseniä (M 6.1.8 yleistettynä). Kartta-alue näyttää `player`-hahmon kartan. STD pitää yhteistyöryhmän samalla kartalla (luku 7.4), joten useampi aktiivinen kartta on poikkeus, joka syntyy lähinnä pelin omasta logiikasta. Tämä vastaa M 11:n avoimeen kysymykseen useista aktiivisista kartoista.

- **Kellot.** Aktiivisten karttojen kellot etenevät yhdessä, ja herätykset ovat samassa jonossa. Kartan kello pysähtyy, kun kartta jäädytetään. Jäädytetyn kartan kello on jäljessä, joten kartan oma kello (M 6.1.8) säilyy, mutta useampi aktiivinen kartta etenee samaa tahtia.
- **Jäädytys** (M 6.7). Lähtökartta jäädytetään vasta, kun sen viimeinen jäsen lähtee. Kartta, jolle saapuu jäsen, sulatetaan, jos se oli jäädytetty.
- **Väliaikainen kartta.** Kartta hylätään, kun sen viimeinen jäsen lähtee. Siirto samalle väliaikaiselle kartalle luo sen uudelleen vain, jos kartalla ei ole muita jäseniä. Muuten siirto on tavallinen siirto kartan sisällä, eikä muita entiteettejä poisteta. Saman toiminnon useat `travel`-kutsut samaan väliaikaiseen karttaan luovat sen kerran.
- **Kartta-alueen kartta.** Kun M 6.2 puhuu aktiivisesta kartasta, se tarkoittaa `game`-subjektin `turn`-tapahtumalle, paneeleille ja listavalikoille kartta-alueen karttaa (M 6.2).

## 6. Esitys ja syöte

Luku muuttaa M 7.1, 7.2, 8 ja 8.1.

### 6.1 Yhteinen näkö

- **Yhteinen näkökenttä** on kaikkien `players`-jäsenten näkökenttien unioni kartta-alueen kartalla. Sitä käyttävät välikehykset, `fog`, muisti, hover, `look` ja klikkausmatkan reitti ("näkee tai muistaa").
- Jäsen ilman `sight`-komponenttia näkee koko kartan kuten nyt.
- Kun jäseniä ei ole, näkyvissä pysyy viimeinen yhteinen näkökenttä.
- **`sees(x, e)`** hyväksyy kokoelman: tulos on tosi, jos yksikin kokoelman jäsen näkee. Tyhjä kokoelma ei näe mitään.
- **`in_view(x) -> bool`** on uusi kysely. Se kertoo, onko ruutu tai entiteetti yhteisessä näkökentässä, tai viimeisessä yhteisessä näkökentässä, kun jäseniä ei ole. Se korvaa nykyisen säännön, jonka mukaan kuollut pelaaja näkee viimeisen näkökenttänsä (M 7.2), joten viimeisenkin jäsenen kuolema näkyy lokissa.
- **Muut aktiiviset kartat.** Jäsenet, jotka eivät ole kartta-alueen kartalla, päivittävät oman karttansa muistia omalla näkökentällään.

### 6.2 Muisti

Ennallaan: yksi muisti kartaa kohti (M 7.2). Kaikki pelaajat näkevät, mitä kuka tahansa on tutkinut.

### 6.3 Kamera

- **Oletuskohde on ryhmä.** Kamera seuraa kartta-alueen kartan jäsenten rajaavaa suorakulmiota legendin tavalla (`follow center` tai `follow margin N`). Jos suorakulmio ei mahdu kartta-alueeseen, kamera seuraa `player`-hahmoa, ja kun suorakulmio taas mahtuu, kamera palaa ryhmään. Yhdellä jäsenellä tulos on sama kuin nyt. Kiertävällä kartalla (M 6.9) suorakulmio lasketaan lyhimmän välin mukaan.
- **`follow(camera)`** ilman kohdetta palauttaa oletuksen. Se korvaa M 7.1:n säännön, jonka mukaan `follow(camera, player)` palauttaa oletuksen. `follow(camera, e)` seuraa yhtä kahvaa kuten ennen, myös silloin, kun `e` on `player`. Kahva ei vaihdu vuoron mukana.
- Kun jäseniä ei ole, kamera jää paikalleen kuten nyt.
- **Uudet kyselyt** `view_width()` ja `view_height()` palauttavat kartta-alueen koon ruutuina sellaisena kuin asettelun legendi sen kirjoittaa (`size.width N`, `size.height N` tai `size LxK` soluina). Koko tulee datasta eikä ruudun koosta, joten peli kulkee samoin kaikilla ikkunakoilla ja ilman päätettä (luku [4](04-satunnaisuus-ja-toistettavuus.md)). Akselilla, jolle kokoa ei ole kirjoitettu, tulos on aktiivisen kartan koko. STD:n hihna käyttää niitä, joten hihna rajoittaa vain, kun peli antaa kartta-alueelle koon.

### 6.4 Vuoron merkki

Kun `players`-jäseniä on useampi ja moottori odottaa syötettä, moottori korostaa `player`-hahmon ruudun. Korostus on asettelun `map`-alueen määre `turn TYYLI` (D 11.1), esimerkiksi `- m map turn reverse`, eikä `panels`-osion rivi, koska `panels` on avainnettu alueen mukaan (D 9.3). Ilman määrettä moottori käyttää käänteisiä värejä. Statuspaneeli voi näyttää vuorossa olevan istuimen kyselyllä `seat_of(player)`.

### 6.5 Syötteen reititys

- **Vuorolla** vain vuorossa olevan istuimen laite ohjaa, myös parametrikyselyt (`from`, `aim`, `ask`), toistolaskuri ja `look`. `pass` toimii miltä tahansa istuimelta kierroksilla ilman pelaajan vuoroa.
- **Valikkoa, dialogia ja kohtausta** ohjaa avaajan istuin. Jos avaaja ei ole istuin, esimerkiksi tekoälyn vuorolla käynnistynyt kohtaus, mikä tahansa istuin ohjaa. Valikko, joka avataan sidonnasta ja ajaa pelaajan komennon (M 7.5), ajaa sen avaajan istuimen ohjattavalle. `more`-sivutusta (M 7.4) kuittaa mikä tahansa istuin.
- **Kohtauksen kartta** on avaajan kartta, ja jos avaaja ei ole istuin, kartta-alueen kartta (RS 9).
- **`ALWAYS`** (taukovalikko) toimii kaikilta laitteilta, ja taukovalikkoa ohjaa sen avannut laite. Kun taukovalikko avataan toisen istuimen syötteen odotuksessa, moottori päättää käynnissä olevan toiminnon ennen valikkoa kuten nyt (M 6.1.7).
- **Näppäinkehotteet.** `{^TOIMINTO}` (M 7.3) näyttää osan 1 ensimmäisen näppäimen, sillä teksti on yhteinen.
- **`join`** toimii vapailta laitteilta.
- **Muiden istuinten painallukset** hylätään eivätkä jonoudu.
- **Hiiri.** Hover toimii aina. Hiiren toiminnot (`travel_to`, valinnat) toimivat vain hiiren istuimen vuorolla.
- **Klikkausmatka** (M 8.1) etenee vain matkaajan omilla vuoroilla, ja muiden istuinten vuorot kulkevat välissä. Vain matkaajan istuimen painallus pysäyttää sen. Uusi toimija yhteisessä näkökentässä pysäyttää sen kuten ennen. Muiden istuinten odotukset lasketaan `on input every N` -tallennukseen.

### 6.6 Ääni

Moottori soittaa lajin `effect`-äänen (M 6.8) kerran, kun yksikin jäsen kuulee äänen. Voimakkuus on kovin jäsenten kuulema taso.

## 7. Vakiokirjasto

### 7.1 Aloitus

`SPAWN_PLAYER` luo jokaiselle liitetylle istuimelle hahmon `game.PLAYER.class` ja kutsuu `control(e, seat: n)`. Istuin 1 saa ruudun aloituskohdasta `game.at` kuten nyt. Seuraavat saavat vapaan ruudun aloituskohdasta tai sen ympäriltä kahden ruudun säteeltä, joten yhden ruudun `START` riittää. Istuinkohtaiset hahmoluokat ja hahmonluontikohtaukset tekee peli korvaamalla käsittelijän.

```roguescript
query OPEN(a: area, who: entity) -> area:
    return a where entities(in: it).count == 0 and not blocks(it, move, for: who)

fn SPAWN_MEMBER(n: number, at: area) -> entity:
    if game has PLAYER:
        e = spawn(game.PLAYER.class, at: at).first
    else:
        e = spawn(DEFAULT.PLAYER, at: at)
    if e != none:
        control(e, seat: n)
    return e

on begin() as SPAWN_PLAYER:
    for n in seats():
        if n == seats().first:
            SPAWN_MEMBER(n, game.at)
        else:
            SPAWN_MEMBER(n, OPEN(near(game.at, 0..2), none))
```

### 7.2 Liittyminen ja poistuminen

- **`JOIN_PLAYER`** (`on join`) luo hahmon vapaaseen ruutuun `player`-hahmon ympäriltä kahden ruudun säteeltä ja antaa sen istuimelle. Jos elävää jäsentä ei ole tai vapaata ruutua ei löydy, istuin jää tyhjäksi.
- **`PART_PLAYER`** (`on part`) antaa poistuvan istuimen jäsenille seuraajan tekoälyn `FOLLOWER` ja komponentin `FOLLOW` (STD 6.1). Uutta tekoälyä ei tarvita.
- **Johtaja.** `LEADER_OF` palauttaa `FOLLOW.target`-kentän tai, jos se puuttuu, lähimmän jäsenen (`nearest(players, e)`). Nykyinen varajohtaja `player` vaihtuisi joka vuoro.

```roguescript
on join(seat) as JOIN_PLAYER:
    if player == none or player.at == none:
        return
    SPAWN_MEMBER(seat, OPEN(near(player, 1..2), none))

on part(seat) as PART_PLAYER:
    for e in members(seat):
        add(e, ai(behavior: FOLLOWER))
        if e has no FOLLOW:
            add(e, FOLLOW())

query LEADER_OF(e: entity) -> entity:
    if e has FOLLOW and e.FOLLOW.target != none:
        return e.FOLLOW.target
    return nearest(players, e)
```

### 7.3 Talutushihna

Kun `leash` on päällä, `CAN_STEP` epää ohjattavan askeleen, jos saman kartan jäsenten rajaava suorakulmio kasvaisi suuremmaksi kuin `view_width() - 2` × `view_height() - 2`.

- Epäonnistunut askel ei vie pelaajalta aikaa (M 6.1.5).
- **Vain ohjattava.** Hihna koskee vain istuimen ohjattavaa (`active(seat_of(who)) == who`). Tekoälyn ohjaamat jäsenet, seuraajat ja hirviöt kulkevat vapaasti. Näin STD 5:n lupaus pitää: tekoäly ei suunnittele askelta, jota `STEP` ei hyväksy. Hihna ei vedä ketään, se vain estää loittonemisen.
- **Vain levittävä askel.** Hihna epää vain askeleen, joka levittää suorakulmion rajan yli. Jo hajonnut ryhmä, esimerkiksi skriptin siirron jälkeen, voi aina palata yhteen.
- Kiertävällä kartalla (M 6.9) suorakulmio lasketaan lyhimmän välin mukaan.
- **Paikanvaihto.** `STEP` vaihtaa paikkaa ystävän kanssa, jolla on tekoäly tai joka on jäsen (`seat_of(other) != none`), eikä vain `player`-hahmon kanssa. Vaihto ei kasvata ryhmän suorakulmiota, koska `CAN_STEP` on jo tarkistanut askeltajan ruudun.

### 7.4 Uloskäynti

Kun saman kartan jäseniä on useampi, `TAKE_EXIT` toimii ryhmänä:

- Jos kaikki saman kartan jäsenet ovat uloskäynnin ruudussa tai sen vieressä, koko ryhmä siirtyy.
- Muuten komento epäonnistuu ilman ajan kulumista ja käynnistää etualan dialogin `EXIT.LEAVE_OTHERS`. Dialogi kysyy vahvistuksen valinnalla (RS 9.7). `YES` siirtää koko ryhmän, ja `NO` palaa. Vahvistettu siirto ei maksa komennon hintaa, koska dialogi alkaa vasta toiminnon lopussa. `wait confirm` ei käy tähän, koska se vain kuittaa eikä anna valintaa (RS 9.5).
- **Seuraajat.** Seuraajat, joilla on `FOLLOW`, joiden johtaja on jäsen ja jotka ovat uloskäynnin vieressä, siirtyvät ryhmän mukana.
- **Saapuminen.** `travel` arpoo jokaiselle ruudun nimiöstä (M 6.7). `GROUP_ARRIVAL` siirtää saapuneen jäsenen tai seuraajan vapaaseen ruutuun kahden ruudun säteellä, jos saapumisruudussa on jo estävä entiteetti.

```roguescript
fn TRAVEL_GROUP(from: entity, exit: entity):
    group = players where it.map == from.map
    tag = entities(in: near(exit.at, 1)) where it has FOLLOW and seat_of(it) == none and LEADER_OF(it) in group
    for e in group:
        travel(e, exit.EXIT.to, at: exit.EXIT.at)
    for e in tag:
        travel(e, exit.EXIT.to, at: exit.EXIT.at)

scene EXIT.LEAVE_OTHERS(from: entity, exit: entity):
    choice:
        YES:
            TRAVEL_GROUP(from, exit)
        NO:
            return

command TAKE_EXIT() cost 120:
    exit = (entities(in: actor.at) where it has EXIT).first
    if exit == none:
        return false
    if seat_of(actor) == none:
        travel(actor, exit.EXIT.to, at: exit.EXIT.at)
        return
    away = players where it.map == actor.map and distance(it, exit) > 1
    if away.count > 0:
        start(EXIT.LEAVE_OTHERS(actor, exit))
        return false
    TRAVEL_GROUP(actor, exit)

on arrive(who, map, p) as GROUP_ARRIVAL when seat_of(who) != none or who has FOLLOW:
    if (entities(in: p) where it != who and blocks(it, move, tiles: false)).count == 0:
        return
    spot = OPEN(near(p, 1..2), who)
    if spot.count > 0:
        move(who, to: spot)
```

### 7.5 Viestit ja havainnot

- **`WITNESSED(e)`** on tosi, kun `e` on jäsen tai `in_view(e)` (luku 6.1). Viimeisenkin jäsenen kuolema kirjautuu siksi lokiin, vaikka `DEATH` tuhoaa sen ennen `DEATH_MESSAGE`-käsittelijää.
- **Nimetyt viestit.** Kun jäseniä on useampi, "sinä"-viestit vaihtuvat nimettyihin. `HIT_MESSAGE` ja `DEATH_MESSAGE` käyttävät jäsenillekin nykyisiä avaimia `MESSAGE.COMBAT.HIT` (`TARGET`, `AMOUNT`) ja `MESSAGE.COMBAT.DIED` (`VICTIM`). Esineille tulevat uudet avaimet `MESSAGE.LOOT.ALLY_PICKUP` ja `MESSAGE.LOOT.ALLY_DROP` parametreilla `WHO` ja `ITEM`. Ehto on `players.count > 1`, joten yksinpelin viestit pysyvät ennallaan. `PICKUP_MESSAGE` ja `DROP_MESSAGE` koskevat kaikkia jäseniä, eivät vain `player`-hahmoa.
- **Nimi.** Oletuspelaajan nimi on "sinä" (`STD.DEFAULT.PLAYER.NAME`). Moninpelin nimet ovat avoin kysymys (STD 14). Peli antaa omille hahmoluokilleen nimet.
- **Kuulo.** `HEAR_MESSAGE` kirjaa äänen kerran ajanhetkessä koko ryhmälle, ei kerran jokaiselle jäsenelle. Ehto `who == player` muuttuu muotoon `seat_of(who) != none`, ja viimeisen kirjauksen hetki tallentuu kenttään `game.LISTENER`. `LISTENER` lisätään siis `game`-subjektille, ei kuulijalle.

### 7.6 Kuolema

`DEATH` tuhoaa jäsenen kuten ennen, ja istuin jää tyhjäksi. Elvytys on pelin logiikkaa: `control(spawn(…), seat: n)`. Kun kaikki istuimet ovat tyhjiä, maailma jatkaa kulkuaan (M 6.1.2). RS 10:n esimerkit, jotka tunnistavat pelaajan kuoleman ehdolla `victim == player`, käyttävät ehtoa `seat_of(victim) != none`. Pelin loppu kaikkien istuinten tyhjentyessä on ehto `players.count == 0`.

### 7.7 Ryhmän vaihto

- **`SWITCH()`**, hinta 0: saman istuimen seuraava elävä jäsen tulee ohjattavaksi. Komento epäonnistuu, jos muita jäseniä ei ole. **`SWITCH.PREV`** vaihtaa edelliseen.
- **Sidonnat.** STD ei sido näppäimiä, joten peli sitoo `SWITCH`-komennot omaan peruskontekstiinsa. Mini-esimerkki käyttää `tab` ja `shift+tab`. Ne eivät ole ristiriidassa, koska `tab` on kontekstien `SCENE`, `AIM` ja `LOOK` sidonta, ja ne ovat modaalisia kontekstejä.
- **Vihamielisyys.** Jäsenet kuuluvat ryhmään `GROUP.PLAYER`, jos niiden luokalla on `FACTION GROUP.PLAYER` kuten oletuspelaajalla.
- **Ystävällinen tuli.** Kaukoaseen harhalaukaus voi osua jäseneen (STD 5.1), mutta jäsenet eivät suutu toisiinsa. `PROVOKE` palaa heti, kun uhri ja hyökkääjä ovat molemmat jäseniä. Ilman tätä yksi harhalaukaus tekisi jäsenistä vihollisia toisilleen, koska `GRUDGE` laskee suhteen arvoon `PROVOKED` ja `ENEMIES` on molemminpuolinen. Vahinko tulee perille.

```roguescript
fn SWITCH_BY(step: number) -> bool:
    n = seat_of(actor)
    if n == none:
        return false
    group = members(n)
    if group.count < 2:
        return false
    i = 0
    here = 0
    for e in group:
        if e == actor:
            here = i
        i = i + 1
    control(group[(here + step) % group.count])
    return true

command SWITCH() cost 0:
    return SWITCH_BY(1)

command SWITCH.PREV() cost 0:
    return SWITCH_BY(-1)
```

## 8. Lataustarkistukset

- `seats` välin ulkopuolella (alle 1, yli 8 tai `a > b`) on latausvirhe.
- `party` tai `leash` muulla arvolla on latausvirhe.
- Sama näppäin kahdessa näppäimistöosassa samassa kontekstissa on latausvirhe.
- `modal` tai `passthrough` lisäosassa, joka poikkeaa osasta 1, on latausvirhe.
- `keyboard N` -osa ilman `join`-sidontaa pelin peruskontekstissa saa varoituksen.
- `seat`-komponentti, jonka numero on yli `seats`-ylärajan, on latausvirhe.
- `keyboard N` tai `leash on` yhden istuimen pelissä saa varoituksen. `party` sallitaan ilman varoitusta, koska ryhmä on hyödyllinen myös yhdelle istuimelle.
- **Lukitus.** `seats` lukitaan pelikerralle (luku 3.1). `party` ja `leash` luetaan datasta joka latauksessa kuten muut `game`-kentät, ja pelaajan `overrides` voi muuttaa niitä. `seats` ei muutu overridella kesken pelikerran.

## 9. Toteutussuunnitelma

Lähtökohta on suunnitelman mukainen: moottorin ydin on moottoririippumaton Rust-kirjasto, ja ensimmäinen käyttöliittymä on pääte. Tärkein linjaus on rakentaa istuimet ytimeen alusta asti, vaikka ensimmäiset pelit ovat yhden istuimen pelejä. Yksinpeli on silloin istuin 1, eikä mitään tarvitse jälkiasentaa.

Vaiheet 0–2 tehdään nyt. Vaiheet 3–8 odottavat tallennusta, syötelokia, klikkausmatkaa, hiirtä, peliohjaimia ja MCP-palvelinta, joita toteutuksessa ei vielä ole (6.11.2026).

### Vaihe 0: speksi päädokumentteihin

- **D**
  - 1.2: rajapintataulukkoon `join` ja `part`.
  - 3.3: `game`-kentät `seats`, `party` ja `leash`, aloitus istuimittain ja virheet.
  - 3.4: autosaven laskenta ja `on arrive` ryhmälle.
  - 5.3: moottorin komponentti `seat` ja jäsen ilman `sight`-komponenttia.
  - 11.1: hover, kamera ja `map`-alueen määre `turn`.
  - 11.3: `keyboard N` -osat, tunniste, `join`, `leave_seat` ja konteksti `JOIN`.
  - 12: ääniefekti.
  - 13: `JOIN` isojen moottorinimien joukkoon.
  - 14: otsikkorivit.
- **RS**
  - 2.4: kontekstinimi `players`.
  - 4.1: tapahtumat `join` ja `part`.
  - 7: ryhmän tekoäly.
  - 9: kohtauksen kartta.
  - 10.1: `control(e, seat:)` ja esimerkit `seat_of(victim)`.
  - 11.1: istuintaulu tallennukseen.
- **M**
  - 1: istuin periaatteisiin.
  - 2.5: tarkistukset.
  - 3: pelaajan kenttä istuintauluksi, lukitus ja datan jäsen jonoon.
  - 4: syöteloki ja `players`-järjestys.
  - 5.1–5.4: aloitus, tallennus, checkpointit ja autosave.
  - 6.1.2, 6.1.3, 6.1.5 ja 6.1.6: toimija, vaihto, valinta ja ohjauksen siirto.
  - 6.1.7: `join` ja `part` lopun töinä.
  - 6.1.8 ja 6.7: aktiiviset kartat, kellot, jäädytys ja väliaikaiset kartat.
  - 6.2: tapahtumat, kontekstinimet ja karttataulu.
  - 6.8: ääni.
  - 7.1: kamera, `view_width` ja `view_height`.
  - 7.2: yhteinen näkö ja `in_view`.
  - 7.3–7.5: kehotteet, `more` ja valikot.
  - 8 ja 8.1: liittymisruutu, reititys, `leave_seat`, `pass` ja klikkausmatka.
  - 9.1, 9.3–9.6: sanasto, signatuurit ja tapahtumat.
  - 11: suljettu kysymys useista aktiivisista kartoista.
- **STD**
  - 1.3: moduulit.
  - 2: prioriteetit.
  - 3: ryhmä.
  - 5: hihna, paikanvaihto, `TAKE_EXIT` ja `SWITCH`.
  - 6.1: `LEADER_OF`.
  - 7: ystävällinen tuli.
  - 8: käsittelijät.
  - 9: viestit.
  - 13: muutokset muihin dokumentteihin.
  - 14: avoimet kysymykset.
  - `lib/std` päivitetään speksin jälkeen.

### Vaihe 1: tila ja ajoitin

- `SeatId(u8)` ja istuintaulu pelin tilassa: numero, jäsenet ja ohjattava. Komponentti `seat` ja nimet `player` ja `players` ratkaistaan istuintaulusta.
- Ajoittimen toimijasääntö (luku 5.1), vuoron valinta `wake`-käsittelijöiden jälkeen ja ohjauksen siirto (luku 5.2).
- `control`-semantiikka (luku 5.3) ja tapahtumat `join` ja `part` toiminnon lopun töinä (M 6.1.7).
- Ytimen rajapinta: `fn input(&mut self, seat: SeatId, action: Action)` ja `fn awaiting(&self) -> Option<SeatId>`. Ydin ei tiedä laitteista mitään.
- **Valmis, kun** kahden istuimen skenaariotestit ajavat vuoroja oikeassa järjestyksessä ja vaihtotesti osoittaa, ettei ilmaisia vuoroja synny. Nykyiset golden-transkriptit eivät muutu.

### Vaihe 2: yhteinen näkö

- Näkökenttävälimuisti entiteettiä kohti. Se mitätöidään liikkeessä, tilen muutoksessa ja `sight`-komponentin muutoksessa.
- Unioni bittijoukkona kartta-alueen kartalle. Muisti, `fog`, välikehysten näkyvyys ja hover lukevat unionia.
- `sees` kokoelmalla ja `in_view`.
- **Valmis, kun** välikehystestit tuottavat saman tuloksen yhdellä jäsenellä kuin nykyinen malli ja oikean unionin kahdella.

### Vaihe 3: istuimet ja syöte (odottaa tallennusta ja syötelokia)

- Käyttöliittymän puolella `DeviceId`: näppäimistöosa, peliohjain tai hiiri. Reititin kuvaa laitteen istuimeksi ja hylkää muiden istuinten painallukset (luku 6.5).
- Liittymisruutu (konteksti `JOIN`), `join` kesken pelin, `leave_seat` ja latauksen uudelleenliittyminen.
- Tallennukseen istuintaulu, ja syötelokiin rivit muodossa (istuin, toiminto).
- `idle` istuimen vuorokohtaisena.
- **Valmis, kun** tallennus ja lataus säilyttävät istuimet ja toisto tuottaa saman pelin.

### Vaihe 4: esitys

- Kameran ryhmäseuranta, `follow(camera)` ilman kohdetta, `view_width` ja `view_height`.
- Vuoron merkki.
- **Valmis, kun** hajaantuvan ryhmän kamera vaihtaa `player`-seurantaan ja palaa ryhmään, kun ryhmä mahtuu taas alueeseen.

### Vaihe 5: vakiokirjasto

- `SPAWN_PLAYER`, `JOIN_PLAYER`, `PART_PLAYER`, `LEADER_OF`, hihna, `TAKE_EXIT`, `GROUP_ARRIVAL`, viestit, `PROVOKE` ja `SWITCH`. Etualan dialogi `EXIT.LEAVE_OTHERS` odottaa kohtausten toteutusta.
- **Valmis, kun** STD:n omat testit läpäisevät yhden ja kolmen istuimen ajon.

### Vaihe 6: päätteen käyttöliittymä (odottaa peliohjaimia)

- crossterm ei tue peliohjaimia, joten peliohjaimet luetaan gilrs-kirjastolla omassa säikeessään, ja tapahtumat yhdistetään samaan syötekanavaan näppäimistön kanssa.
- Näppäimistöosat toimivat päätteessä sellaisenaan, koska vuoropohjainen peli tarvitsee vain painallukset eikä näppäinten vapautuksia tai samanaikaisuutta.
- **Valmis, kun** kaksi peliohjainta ja kaksi näppäimistöosaa pelaavat samaa peliä.

### Vaihe 7: toteutustesti

- Pieni yhteistyöpeli samaan tapaan kuin aiemmat toteutustestit: 1–4 istuinta, yhteinen luolasto, uloskäynnit, elvytys alttarilla ja yhden istuimen ryhmätila (`party switch`).
- Löydökset raportoidaan kuten aiemmissa testeissä, ja päätökset viedään päädokumentteihin.

### Vaihe 8: MCP-palvelin (odottaa palvelinta)

- RLML-MCP-palvelimen syöte saa istuinparametrin, ja `render` näyttää yhteisen näön. Tämä mahdollistaa moninpelitestien ajon automaattisesti.

## 10. Jatko

- **Kilpailu ja hot seat.** Istuinkohtainen muisti, näkö ja kamera sekä vaihtoruutu. Malli laajenee siirtämällä muisti kartalta istuimelle.
- **Jaettu ruutu.** Asettelun `map`-alueet istuimille (`- m map seat 1`). Nykyinen sääntö sallii vain yhden `map`- ja `hover`-alueen.
- **Verkkopeli.** Deterministinen ajo ja syöteloki muodossa (istuin, toiminto) sopivat lockstep-malliin.
- **Istuinkohtaiset asetukset.** Esimerkiksi `idle` ja `targeting` istuimittain.
- **Yläraja.** 8 istuinta on moottorin raja. Käytännön raja määräytyy ruudusta ja laitteista.
