[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 5. Komponentit ja entiteetit

Kieli käyttää ECS-mallia. Entiteetti on nimi ja joukko komponentteja, eikä sillä ole lajia. Moottori ei tunne yhtään kirjoittajan komponenttia nimeltä, joten isot nimet (`HP`, `HEALTH`) ovat täysin kirjoittajan valinta. Moottori lukee vain omia komponenttejaan, jotka kirjoitetaan pienillä (luku [5.3](#53-moottorin-komponentit)).

## 5.1 Komponentit

```
component HP
- now number
- max number default now

component MELEE
- damage dice

component DROPS
- loot table

component RESIST
- FIRE percent
- COLD percent

component BOSS
component FLYING
```

Kenttä on rivi `- nimi tyyppi [default arvo]`. Kenttien nimet ovat sanastoa ja kirjoitetaan pienillä, paitsi nimellä indeksoitavat kentät (`RESIST.FIRE`, luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)).

Tyypit:

| Tyyppi | Arvo |
| --- | --- |
| `number` | kokonaisluku |
| `percent` | prosenttiarvo `%`-merkillä (`50%`) |
| `dice` | noppa, joka heitetään joka käytöllä |
| `bool` | `true` tai `false` |
| `text` | vapaa merkkijono (alla) |
| `deck` | pakan nimi, eli viittaus jaettuun pakkaan (luku [6](06-pakat-ja-taulut.md)) |
| `deck copy` | ilmentymän oma kopio pakasta (luku [6.3](06-pakat-ja-taulut.md#63-kopiot)) |
| `table` | taulun nimi, eli viittaus tauluun (luku [6.5](06-pakat-ja-taulut.md#65-taulut)) |
| `name` | nimetyn asian nimi, esimerkiksi tile |
| `dir` | suunta tai `none` (luku [2.4](02-yleinen-syntaksi.md#24-arvot)) |
| `side` | suhteellinen suunta tai `none` (luku [2.4](02-yleinen-syntaksi.md#24-arvot)) |
| `pos` | sijainti `X,Y` (luku [2.4](02-yleinen-syntaksi.md#24-arvot)) |
| `area` | ruutujoukko; datassa alkuarvo on tyhjä, ja logiikka asettaa sen (RogueScript, luku [6](../roguescript/06-ruudut-ja-paikkahaku.md)) |
| `card` | entiteetin nimi ja valinnaiset kenttien ylikirjoitukset (luku [6.4](06-pakat-ja-taulut.md#64-kenttien-ylikirjoitus)) |
| `entity` | viittaus entiteetin ilmentymään tai `none` |
| `collection [N] [merge MERKKI] [modifies]` | kokoelma, jonka jäsenet ovat toisia entiteettejä (luku [5.4](#54-kokoelmat)) |
| `list T [N] [sort [KENTTÄ] [desc]]` | järjestetty arvojono (alla) |
| `label` | kartan paikallinen nimiö (alla) |
| `dict AVAIN ARVO` | sanakirja (alla) |
| `handle` | taustakohtauksen kahva tai `none` (RogueScript, luku [9.9](../roguescript/09-kohtaukset-ja-aikajanat.md#99-taustakohtaukset)) |

- **Suuntatyypit erillään:** `side`-arvo `dir`-kentässä on latausvirhe, ja päinvastoin.
- **Oletusarvo:** `default` antaa kentälle arvon, jos entiteetti ei anna sitä. Arvo voi olla vakio tai saman komponentin toinen kenttä (`default now`). Ilman oletusarvoa puuttuva kenttä on 0, puuttuva `bool`-kenttä `false`, puuttuva `pos`-kenttä `0,0`, puuttuva `entity`-kenttä `none`, puuttuva lista tyhjä ja puuttuva `area` tyhjä joukko.
- **Kokoelman alkusisältö:** kokoelmakentän arvo ja `default` voivat olla nosto pakasta tai taulusta tai entiteetti (`- items collection 26 default KIT.EMPTY_HANDED all copy`, luku [5.4](#54-kokoelmat)).
- **Pakka- ja taulukenttä:** `deck`-kentän arvo on pakan nimi ja `table`-kentän arvo taulun nimi. Kenttä hyväksyy vain oman lajinsa, joten taulu `deck`-kentässä on latausvirhe, ja päinvastoin. Pakalle ja taululle ei ole yhteistä kenttätyyppiä: niiden nostot tuottavat samanlaisia kortteja, mutta lähteet ovat eri lajia. `deck copy`-kentän arvo on pakka, josta ilmentymä saa oman kopionsa syntyessään (luku [6.3](06-pakat-ja-taulut.md#63-kopiot)).
- Tyhjä `card`-kenttä ei tarkoita mitään entiteettiä.
- **Merkki (tag)** on komponentti ilman kenttiä (`BOSS`, `FLYING`).
  - **Nimiökenttä.** Tyyppi `label` on kartan paikallinen nimiö (luku [8.5](08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)). Lataus ei ratkaise sitä yleisellä haulla vaan tarkistaa sen kohdekartasta, kun saman entiteetin karttakenttä on vakio; muuten tarkistus on käyttöhetkellä. Esimerkki: `component EXIT` kentillä `- to name` ja `- at label default START`.
  - **Sanakirja.** Tyyppi `dict AVAIN ARVO` on avain–arvo-taulu (RogueScript, luku [2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)), jonka avain on `name`, `number` tai `entity`. Datassa arvo annetaan pareina (`- STANCE groups RAT -20 GOBLIN 10`); entiteettiavaimisen sanakirjan alkuarvo on aina tyhjä.
- **Lista.** Tyyppi `list T` on järjestetty arvojono (RogueScript, luku [2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)). Jäsenen tyyppi `T` on luku, nimi, teksti, `pos`, `area` tai komponentti, jolloin jäsenet ovat komponenttiarvoja (`list SCORE`). Datassa arvot kirjoitetaan välilyönnein erotettuina järjestyksessä: `- BESTIARY order MONSTER.KESTREL MONSTER.EMU MONSTER.BAT`. Komponenttiarvojen ja alueiden listan alkuarvo on aina tyhjä. `N` on listan enimmäispituus, ja `sort` pitää listan järjestyksessä arvon tai komponenttiarvon kentän mukaan, suurin ensin sanalla `desc`. Täysi lista pudottaa lisäyksessä järjestyksessä viimeisen tai ilman `sort`-sanaa vanhimman.
- **Määrittely skriptissä.** Komponentin voi määritellä myös RogueScriptillä (`component HP(now: number, max: number = now)`), ja muodot ovat sama laji. Nimellä on yksi määrittely, ja käsittelijät ovat mahdollisia vain skriptimuodossa (RogueScript, luku [3.2](../roguescript/03-komponentit.md#32-datassa-ja-skriptissä)).

**Teksti.** Tyyppi `text` on vapaa merkkijono lainausmerkeissä (`"Iolo"`). Se ei käänny kielen mukaan, ja rivinvaihto siinä on latausvirhe. Tekstit, jotka käännetään, ovat aina tekstiavaimia (luku [10](10-tekstit.md)).

## 5.2 Entiteetit

Entiteetti koostetaan komponenteista. Monikenttäinen komponentti annetaan nimeämällä kentät, ja yhden kentän komponentilla kentän nimi jää pois:

```
entity GOBLIN
- HP now 8
- MELEE 1d6
- movement mode WALK speed 10
- render GLYPH.GOBLIN
- DROPS LOOT.COMMON

entity BAT
- HP now 3
- MELEE 1d3
- render GLYPH.BAT
- FLYING

entity GOBLIN.ARCHER extends GOBLIN
- MELEE 1d4
- RESIST FIRE 20%
```

- **Kenttien arvot:** `- HP now 8 max 10` nimeää kentät. Puuttuva kenttä saa oletusarvonsa, joten `- HP now 8` tarkoittaa 8/8, koska `max` on määritelty muodossa `default now`.
- **Yhden kentän komponentti** annetaan ilman kentän nimeä: `- MELEE 1d6`. Jos komponenttiin lisätään myöhemmin toinen kenttä, lyhenne on latausvirhe, joka luettelee kentät. Vanha rivi ei siis hiljaa osoita väärään kenttään.
- **Noppa lukukentässä:** jos `number`-kentälle annetaan noppa tai väli (`- ENCHANT -1..3`), se heitetään, kun entiteetin ilmentymä syntyy, ja tulos tallentuu ilmentymään.
- **Nimellä indeksoidut kentät** annetaan samalla tavalla: `- RESIST FIRE 100% COLD -50%`.
- **Perintä:** `entity GOBLIN.ARCHER extends GOBLIN` kopioi emon komponentit, ja lapsen omat rivit korvaavat samannimiset komponentit. Rivi `- drop KOMPONENTTI` poistaa emolta perityn komponentin (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)). Lapsi on samalla emonsa lajia, joten emoa koskevat viittaukset kattavat sen. `extends` on ainoa perinnän muoto.
- **Laji ajossa.** Ilmentymän moottorikenttä `kind` on määrittely, josta se luotiin: `GOBLIN.ARCHER`-ilmentymän `kind` on `GOBLIN.ARCHER`, vaikka se on myös `GOBLIN`-lajia (RogueScript, luku [2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)).
- **Kirjainkoko entiteetin rivillä:** iso sana on kirjoittajan komponentti, pieni sana moottorin komponentti (luku [5.3](#53-moottorin-komponentit)) tai kiinteä avainsana. Pienten sanojen joukko on kiinteä, joten jäsennin erottaa ne.
- **Tekstit:** entiteetin nimi ja kuvaus haetaan kielitiedostosta avaimilla `GOBLIN.NAME` ja `GOBLIN.DESC` (luku [10](10-tekstit.md)).
- **Sijainti:** jokainen entiteetin ilmentymä on joko kartan ruudussa tai toisen entiteetin kokoelmassa. Sijainti on suhde eikä komponentti. Kartan legend sijoittaa ilmentymiä ruutuihin (luku [8.2](08-kartat-ja-prefabit.md#82-legend)) ja kokoelman alkusisältö kokoelmiin (luku [5.4](#54-kokoelmat)). Ruudussa olevan ilmentymän sijainti voi toimia paikkahaun ankkurina (luku [8.8](08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)).

## 5.3 Moottorin komponentit

Moottori lukee muutamaa komponenttia nimeltä. Ne ovat sanastoa ja kirjoitetaan pienillä. Niiden kentät ja kirjoitusasu kuuluvat dataformaattiin, mutta niiden merkitys kuvataan moottoridokumentissa (“[RLML-moottorin vastuut](../moottori/README.md)”, luku [3.4](03-tiedostot-ja-include.md#34-automaattinen-tallennus-autosave)), paitsi `render` (luku [9](09-ulkoasu.md)) ja `label` sekä `wire` (luku [8.5](08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)):

| Komponentti | Kentät |
| --- | --- |
| `render` | `tile name` |
| `movement` | `mode name`, `speed number default 10` |
| `ai` | `behavior name` |
| `sight` | `radius number`, `arc number default 360`, `dir dir`, `dark number default 1` |
| `light` | `radius number`, `arc number default 360` |
| `facing` | `dir dir` (ei `none`) |
| `timer` | `TAPAHTUMA interval`, `delay`, `count` (rivi kutakin ajastinta kohti) |
| `interior` | `map name` |
| `shape` | `prefab name` |
| `label` | `name name` |
| `wire` | `name name` |
| `revealed` | – (merkki) |
| `seat` | `n number` |

- **Moottori määrittelee ne.** Kirjoittaja ei kirjoita niille `component`-lohkoa, ja sellainen on latausvirhe. Omaa dataa varten tehdään oma komponentti isolla nimellä.
- **Kirjoitusasu** on sama kuin muilla komponenteilla: `- sight radius 10 arc 90 dir south` ja yhden kentän lyhenne `- render GLYPH.GOBLIN`.
- `facing` antaa suhteellisille suunnille vertailukohdan, myös paikkahaun `from`-osassa (luku [8.8](08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)).
- **Näkö ja valaistus.** `sight`-komponentin `radius` on näkösäde valaistuihin ruutuihin ja `dark` näkösäde pimeisiin ruutuihin. Valaistu ruutu on ruutu, jonka jokin kerros on `lit`-tile, sen kahdeksan naapuria tai ruutu, jota `light`-lähde valaisee (luku [7.5](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles); moottoridokumentti, luku [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen)). Sokeus kirjoitetaan säteinä 0 (`actor.sight.radius = 0` ja `actor.sight.dark = 0`): olento näkee vain oman ruutunsa. Komponentin poisto ei sovi sokeuteen, koska pelaaja (istuimen jäsen) ilman `sight`-komponenttia näkee koko kartan.
- **Istuin.** `seat` tekee entiteetistä ihmispelaajan istuimen jäsenen: `- seat 1`. Yleensä logiikka antaa sen kutsulla `control(e, seat: n)`, ja datan rivi on testejä ja valmiita ryhmiä varten. Jäsen on toimija ilman `ai`-komponenttiakin. Numero yli `game`-lohkon `seats`-ylärajan on latausvirhe (luku [3.3](03-tiedostot-ja-include.md#33-aloitus); moottoridokumentti, luku [6.1.2](../moottori/06-ajo.md#612-toimijat-ja-ajoitin)).
- **Paljastettu.** `revealed` piirtää entiteetin myös näkökentän ulkopuolella, esimerkiksi hirviöiden aistimisen ajan. Se ei muuta näkökenttää, joten `sees` ja säännöt pysyvät ennallaan (moottoridokumentti, luku [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen)).
  - **Esto ja läpäisy.** Moottorin komponentti `blocks` antaa entiteetille saman eston kuin tilen ominaisuus (luku [7.5](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)): `- blocks move sight`. `movement` ja `sight` saavat kentän `through`, tilejoukon, jonka esto ei koske niitä: `- movement mode PHASE through WALLS` ja `- sight radius 6 through WALLS`. `movement` saa myös kentät `dirs` (4 tai 8, oletus 8) ja `corners` (oletus `false`, eli vinoaskel seinän kulman ohi on kielletty): `- movement dirs 4`. Ammuksen esto on liikkumistapa `MISSILE`: `- WINDOW blocks move MISSILE` pysäyttää kävelijän ja ammuksen mutta ei näköä (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).
- **Aika.** `movement`-komponentin `speed` skaalaa komentojen hinnan viiveeksi, ja normaalinopeus on 10. `timer`-rivien ajat ovat kierroksia (moottoridokumentti, luku [6.1](../moottori/06-ajo.md#61-aika-ja-vuorot)).
- **Lisäys ja poisto ajossa.** Logiikka voi lisätä ja poistaa moottorin komponentin kuten minkä tahansa komponentin (`add`, `remove`). Esimerkiksi `ai`-komponentin lisäys laittaa ilmentymän herätysjonoon (moottoridokumentti, luku [9.5](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)).

**Nimi.** Moottorin komponentti `name` kentällä `text text` korvaa entiteetin näyttönimen tekstiavaimen sijaan: `- name "Iolo"` (luku [10](10-tekstit.md)). Kuvaus tulee yhä avaimesta `.DESC`.

**Kirjoitettavuus.** Moottorin komponenttien kentät ovat logiikalle kirjoitettavia, ja muutos vaikuttaa seuraavasta laskennasta (moottoridokumentti, luku [3.4](../moottori/03-pelin-tila.md#34-moottorin-komponentit)). Logiikka, joka kirjoittaa kenttään, tarvitsee komponentin: esimerkiksi nopeutta muuttava tila edellyttää, että entiteetillä on `movement`-rivi.

## 5.4 Kokoelmat

Kokoelma on komponentin kentän tyyppi. Kuka tahansa entiteetti, jolla on kokoelmakenttä, voi kantaa toisia entiteettejä:

```
component PACK
- items collection 26 merge STACKABLE

component CHEST
- items collection

entity PLAYER
- HP now 12
- PACK items KIT.FIGHTER all copy

entity CHEST.OLD
- render GLYPH.CHEST
- CHEST items LOOT.CHEST 1d3 copy
```

Kenttä on `- nimi collection [N] [merge MERKKI] [modifies]`.

- **N** on paikkojen enimmäismäärä, ja pino vie yhden paikan. Ilman lukua paikkoja ei rajoiteta.
- **Indeksi:** jäsen saa pienimmän vapaan indeksin (0, 1, 2, …), ja indeksi näytetään kirjaimena (`a` on 0). Kirjaimet ovat siis kokoelman ominaisuus eivätkä erillinen asetus. Ilmentymän paikka näkyy logiikalle kenttänä `slot` (RogueScript, luku [2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)).
- **`merge MERKKI`** tekee pinoja jäsenistä, joilla on annettu merkki. Pino on joukko täysin samanlaisia jäseniä: sama entiteetin nimi ja kaikki kentät samat. Jokainen kappale on oma ilmentymänsä, mutta pinon jäsenet jakavat paikan ja kirjaimen ja näkyvät listoissa yhtenä rivinä, jossa on lukumäärä. `ARROW ENCHANT 0` ja `ARROW ENCHANT 1` ovat eri pinoja. Kun kappaleen kenttä muuttuu, se erkanee omaksi pinokseen, ja kun kaksi pinoa tulee samanlaisiksi, ne yhdistyvät. Merkki on kirjoittajan valinta, eikä moottori tunne merkkiä `STACKABLE`.
- **`modifies`** merkitsee kokoelman, jonka jäsenet vaikuttavat kantajaansa. Merkityksen kuvaa RogueScript-dokumentti.
- **Alkusisältö:** kokoelmakentän arvo tai oletus on nosto pakasta tai taulusta (luvut [6.2](06-pakat-ja-taulut.md#62-tila-sekoitus-ja-nosto) ja [6.5](06-pakat-ja-taulut.md#65-taulut)) tai entiteetti. Kokoelma täytetään, kun omistajan ilmentymä syntyy (esimerkiksi kartan legendistä, `spawn`-kutsusta tai `game`-lohkosta). Kaksi samaa kauppiasta nostaa jaetusta pakasta eri tavarat. Jos jokaisen pitää saada sama valikoima, nosto tehdään kopiosta (`WARES.SMITH all copy`, luku [6.3](06-pakat-ja-taulut.md#63-kopiot)).
- **Täyttyminen:** jos sisältö ei mahdu N paikkaan, ylimääräiset jäsenet jätetään luomatta ja moottori kirjaa varoituksen. Pinoon mahtuva kappale ei vie uutta paikkaa. Lataus varoittaa jo etukäteen, jos `all`-nosto pakasta, jossa ei ole noppia, ei mahdu.
- **Monta kokoelmaa:** entiteetillä voi olla useita kokoelmakomponentteja, ja hirviö, jolla on `PACK`, kantaa esineitä samoin kuin pelaaja.
- **Kokoelma ja tilejoukko ovat eri asioita:** kokoelma kantaa entiteettien ilmentymiä, tilejoukko luettelee tilejä haun ja vertailun tarpeisiin (luku [7.3](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#73-tilejoukot-tileset)).

## 5.5 Pelikerran tila: game

`game` on pelikerran tila. Sen pienet rivit ovat moottorin kenttiä (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)), ja isot rivit ovat komponentteja kuten entiteetillä:

```
game CRYPT
- start TOWN
- WIND dir east
- KING_QUEST 0
- CALENDAR day 1 season SPRING
- PERMADEATH
```

- **Entiteetin rivit ilman entiteettiä.** Komponenttirivit kirjoitetaan kuten entiteetin rivit. Kirjaston nimetön `game`-lohko lisää komponentteja, ja saman komponentin myöhempi rivi korvaa aiemman (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- **Ei sijaintia:** `game` ei ole kartalla eikä kokoelmassa, joten se ei kelpaa paikkahaun ankkuriksi.
- **Yleinen tila** (tunnistetut esineet, juonen vaiheet, kalenteri, dialogien liput ja pelikerran asetukset, kuten vaikeustaso ja permadeath) kirjoitetaan `game`-komponentteihin, ja sen alkuarvot annetaan tässä lohkossa. Alkuarvot ovat osa laskettua aloitustilaa (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)), ja tila tallentuu pelin tallennukseen. Pelikertojen yli säilyvä tila kuuluu profiiliin (luku [3.5](03-tiedostot-ja-include.md#35-profiili-profile)).
- **Logiikassa** `game` on kontekstinimi (RogueScript, luku [10.3](../roguescript/10-aloitus-ja-checkpointit.md#103-pelikerran-tila-game)): komponentin kenttä luetaan ja muutetaan polulla `game.KING_QUEST`, ja merkki testataan muodossa `game has PERMADEATH`. Moottorin kentät, myös rajat `call_depth` ja `loop_steps`, ovat vain luettavia.

---

← [4. Nimiavaruudet](04-nimiavaruudet.md) · [Sisällys](README.md) · [6. Pakat ja taulut](06-pakat-ja-taulut.md) →
