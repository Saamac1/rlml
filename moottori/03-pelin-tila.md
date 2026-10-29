[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 3. Pelin tila

## 3.1 Tilan osat

Moottori omistaa pelin koko tilan:

- **Ilmentymät** komponentteineen ja kenttien arvoineen.
- **Sijainnit.** Jokainen ilmentymä on joko kartan ruudussa tai toisen ilmentymän kokoelmassa. Sijainti on suhde, ei komponentti ([D 5.2](../dataformaatit/05-komponentit-ja-entiteetit.md#52-entiteetit)).
- **Pelikerran tila `game`:** moottorin kentät `start`, `at`, `seed`, `turn` ja `time`, rajat `call_depth` ja `loop_steps` (luku [2.6](02-lataus.md#26-virheiden-lajit)) sekä kirjoittajan komponentit ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus), [D 5.5](../dataformaatit/05-komponentit-ja-entiteetit.md#55-pelikerran-tila-game)). Lisäksi pelikerralle lukittu automaattisen tallennuksen paikkamäärä (luku [5.4](05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)).
- **Profiilin näkymä:** profiilin tilannekuva pelikerran alusta ja pelikerran omat kirjoitukset (luku [5.6](05-aloitus-tallennus-ja-checkpointit.md#56-profiili)). Itse profiili on moottorin oma tiedosto eikä kuulu pelikerran tilaan.
- **Pelaaja:** viimeisimmän `control`-kutsun kohde ([RS 10.1](../roguescript/10-aloitus-ja-checkpointit.md#101-aloitus)).
- **Kartat:** kaikki ladatut kartat (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)): ruutujen maastokerrokset, nimiöt, johtimet, muuttujat (`vars:`) sekä pelaajan muistamat ja paljastetut ruudut (luku [7.2](07-esitys.md#72-kartan-piirtäminen)).
- **Ajoitus:** herätysjono järjestysnumeroineen, seuraava järjestysnumero ja karttojen kellot (luku [6.1.8](06-ajo.md#618-aktiiviset-kartat-ja-jäädytys)).
- **Pakkojen tila:** jokaisen jaetun pakan ja `deck copy` -kentän kopion nostopino ja poistopakka ([D 6.2](../dataformaatit/06-pakat-ja-taulut.md#62-tila-sekoitus-ja-nosto)). Taulut ovat tilattomia ([D 6.5](../dataformaatit/06-pakat-ja-taulut.md#65-taulut)).
- **Satunnaisvirtojen tila** (luku [4](04-satunnaisuus-ja-toistettavuus.md)).
- **Kohtausten tila:** etualan jono, käynnissä olevat taustakohtaukset kahvoineen ja pelin ajan tweenien jäljellä olevat reitit ([RS 9.1](../roguescript/09-kohtaukset-ja-aikajanat.md#91-rivi)).

## 3.2 Ilmentymän syntyminen

Kun ilmentymä syntyy (legendistä, pakasta, taulusta, `spawn`-kutsusta tai generaattorista), moottori

1. kopioi entiteetin komponentit perintäketjun kautta (`extends`) ja poistaa `drop`-riveillä poistetut ([D 5.2](../dataformaatit/05-komponentit-ja-entiteetit.md#52-entiteetit)), ja asettaa kentän `kind` määrittelyn nimeksi,
2. soveltaa kenttien ylikirjoitukset: datan kortin, legendin `with`-osan tai logiikan kortin (`NKI(NOTE: k)`, [RS 2.8](../roguescript/02-yleinen-syntaksi.md#28-ison-nimen-kutsu)) ([D 6.4](../dataformaatit/06-pakat-ja-taulut.md#64-kenttien-ylikirjoitus)),
3. antaa puuttuville kentille oletusarvot tai nollan ([D 5.1](../dataformaatit/05-komponentit-ja-entiteetit.md#51-komponentit)),
4. heittää `number`-kenttien nopat ja välit ja tallentaa tuloksen ilmentymään,
5. täyttää kokoelmat alkusisällöllä ja heittää nopat ilmentymää kohti ([D 5.4](../dataformaatit/05-komponentit-ja-entiteetit.md#54-kokoelmat)); jäsenet syntyvät samoilla askelilla, joten niiden `created` lähetetään ennen kantajan omaa,
6. asettaa ilmentymän sijaintiinsa, ruutuun tai kokoelmaan,
7. lähettää tapahtuman `created`,
8. laittaa ilmentymän herätysjonoon, jos sillä on `ai`-komponentti ja se on kartalla (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)).

Kortin määrä synnyttää niin monta erillistä ilmentymää, ja jokainen käy askeleet läpi erikseen. Kortin nopat ja ylikirjoitukset heitetään kerran korttia kohti, joten kappaleet ovat samanlaisia ([D 6.1](../dataformaatit/06-pakat-ja-taulut.md#61-pakka)). Koska ylikirjoitukset sovelletaan askeleessa 2, `created`-käsittelijä näkee ilmentymän lopullisine kenttineen; syntymän jälkeen kirjoitettu kenttä ei näy sille.

Syntymä ei lähetä tapahtumia `enter` eikä `arrive`. `created` on syntymän ainoa tapahtuma, ja sen käsittelijä näkee ilmentymän valmiina ja sijainnissaan (`self.at`).

Kartan latauksessa syntyvien ilmentymien `created`-käsittelijät eivät näe pelaajaa, koska `player` on `none` ennen ensimmäistä `control`-kutsua ([RS 10.1](../roguescript/10-aloitus-ja-checkpointit.md#101-aloitus)).

Ilmentymän tuho ja komponentin poisto kuvataan luvussa [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit).

## 3.3 Kokoelmat

- **Paikat ja indeksit.** Moottori antaa jäsenelle pienimmän vapaan indeksin ja näyttää sen kirjaimena. Indeksi näkyy logiikalle kenttänä `e.slot` (luku [3.4](#34-moottorin-komponentit)).
- **Pinot.** `merge MERKKI` -kokoelmassa jokainen kappale on oma ilmentymänsä, mutta täysin samanlaiset jäsenet (sama `kind` ja kaikki kentät samat) muodostavat pinon, joka jakaa yhden paikan ja kirjaimen. Moottori pitää pinot ajan tasalla: kun jäsenen kenttä muuttuu, se erkanee omaksi pinokseen pienimpään vapaaseen paikkaan, ja kun kaksi pinoa tulee samanlaisiksi, ne yhdistyvät pienempään paikkaan. Kokoelman koko `N` laskee pinoja eikä kappaleita, ja `.count` laskee kappaleita. Pino näkyy logiikalle kenttänä `e.stack` (luku [3.4](#34-moottorin-komponentit)).
- **Täyttyminen.** Jäsenet, jotka eivät mahdu, jätetään luomatta, ja moottori kirjaa varoituksen. Kappale, joka liittyy olemassa olevaan pinoon, mahtuu aina. `spawn(…, into:)` ja `move(…, to:)` täyteen kokoelmaan epäonnistuvat samoin (luku [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).
- **`modifies`.** Moottori jakaa kantajan tapahtumat kokoelman jäsenille (luku [6.2](06-ajo.md#62-tapahtumat)).
- **Siirrot.** Moottori tuntee kokoelman vain sijaintina: se siirtää ilmentymän kokoelmaan ja sieltä pois mutta ei tiedä, mikä kokoelma on reppu. Valinta kuuluu vakiokirjastolle (`STD.GIVE`). Siirtojen tapahtumat ovat luvussa [9.6](09-moottorin-funktiot-ja-tapahtumat.md#96-tapahtumat).

## 3.4 Moottorin komponentit

Moottori määrittelee pienillä kirjoitetut komponentit, ja vain se antaa niille merkityksen ([D 5.3](../dataformaatit/05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)):

| Komponentti | Moottori |
| --- | --- |
| `render` | piirtää ilmentymän annetulla tilellä |
| `movement` | lukee liikkumistavan ja nopeuden; nopeus skaalaa komennon hinnan viiveeksi, ja normaalinopeus ja oletus on 10 (luku [6.1.3](06-ajo.md#613-hinta-ja-nopeus)) |
| `ai` | tekee ilmentymästä toimijan ja kutsuu nimettyä tekoälyfunktiota sen vuorolla (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)) |
| `sight` | laskee näkökentän säteillä `radius` (valaistut ruudut) ja `dark` (pimeät ruudut, oletus 1): `sees()`, `fog`-kerros ja `remember` (luku [7.2](07-esitys.md#72-kartan-piirtäminen)) |
| `light` | laskee valon säteen ja kaaren; valaistu ruutu näkyy koko näkösäteellä (luku [7.2](07-esitys.md#72-kartan-piirtäminen)) |
| `facing` | antaa suhteellisille suunnille vertailukohdan |
| `timer` | lähettää ajastetut tapahtumat kierroksen rajalla (luku [6.1.4](06-ajo.md#614-mikroaskeleen-vaiheet)) |
| `interior` | liittää ilmentymään oman kartan |
| `shape` | antaa ilmentymälle prefabin muodon |
| `label`, `wire` | nimiö ja johdin, jotka liikkuvat ilmentymän mukana ([D 8.5](../dataformaatit/08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)) |
| `revealed` | piirtää ilmentymän myös pelaajan näkökentän ulkopuolella; ei muuta näkökenttää eikä `sees`-kyselyä (luku [7.2](07-esitys.md#72-kartan-piirtäminen)) |

`movement`-kentän `mode`, `timer`-rivien tapahtumat ja kentät sekä `interior`- ja `shape`-komponenttien tarkka merkitys ovat vielä kuvaamatta (luku [11](11-avoimet-kysymykset.md)). `timer` laskee kierroksia, ja sen ajoitus on kuvattu luvussa [6.1](06-ajo.md#61-aika-ja-vuorot). Moottorin komponentin lisäämisen ja poistamisen vaikutukset ovat luvussa [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit). Komponentti blocks antaa entiteetille saman eston kuin tilen ominaisuus ([D 7.5](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)): move, sight tai liikkumistapa. Näkö, valo ja reitinhaku käyttävät tilejen ja entiteettien estoa. Kentät movement.through ja sight.through ovat tilejoukkoja, joiden esto ei koske liikkujaa tai katsojaa: aave kulkee ja näkee seinien läpi. `movement` saa lisäksi kentät `dirs` (4 tai 8, oletus 8) ja `corners` (oletus `false`): vinoaskel on kielletty, jos jompikumpi sen sivuruuduista estää liikkeen, ellei `corners` ole `true`. Reitinhaku ja `adjacent` noudattavat näitä kenttiä.

**Sokeus.** Sokea olento kirjoitetaan säteinä 0 (`sight.radius = 0`, `sight.dark = 0`): se näkee vain oman ruutunsa. `sight`-komponentin poisto ei sovi sokeuteen, koska pelaaja ilman sitä näkee koko aktiivisen kartan (luku [7.2](07-esitys.md#72-kartan-piirtäminen)).

**Nimi ja kirjoitettavat kentät.** Moottorin komponentti `name` (kenttä `text: text`) korvaa entiteetin näyttönimen: `{NIMI}` ja `.NAME` näyttävät tekstin avaimen sijaan, ja `DESC` tulee yhä avaimesta (luku [7.3](07-esitys.md#73-tekstit)). Moottorin komponenttien kentät ovat logiikalle kirjoitettavia (`actor.render.tile = …`, `actor.movement.mode = SHIP`). Muutos vaikuttaa seuraavasta laskennasta: nopeus seuraavaan viiveeseen, liikkumistapa seuraavaan estokyselyyn ja `sight` seuraavaan näkökenttään. Pienellä kirjoitettu komponentin nimi kutsuna on komponenttiarvo kuten iso nimi (`add(p, ai(behavior: FOLLOWER))`, [RS 2.8](../roguescript/02-yleinen-syntaksi.md#28-ison-nimen-kutsu)).

**Entiteetin moottorikentät.** Moottori ylläpitää jokaiselle ilmentymälle vain luettavia kenttiä ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)):

| Kenttä | Arvo |
| --- | --- |
| `e.at` | ruutu kartalla; kokoelmassa ylimmän kantajan ruutu |
| `e.map` | kartta (luku [6.2](06-ajo.md#62-tapahtumat)) |
| `e.kind` | määrittely, josta ilmentymä luotiin; säilyy kuolleella kahvalla |
| `e.slot` | indeksi kokoelmassa nollasta, pinon jäsenillä sama; kartalla `none` |
| `e.stack` | `e` ja sen kanssa samassa pinossa olevat jäsenet luontijärjestyksessä; pinon ulkopuolella pelkkä `e` |

---

← [2. Lataus](02-lataus.md) · [Sisällys](README.md) · [4. Satunnaisuus ja toistettavuus](04-satunnaisuus-ja-toistettavuus.md) →
