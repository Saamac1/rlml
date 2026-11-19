[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 8. Kartat ja prefabit

## 8.1 Karttatiedosto

Kartta on markdown-tiedosto. Piirros kirjoitetaan tiedostossa koodilohkoon (luku [2.2](02-yleinen-syntaksi.md#22-rivit-lohkot-ja-luettelot)), mutta alla se on näytetty ilman omaa lohkoa:

```
# Kaupunki

> Pelin aloituskartta.

include legends/common.rlml

map TOWN size 24x4
########################
#....@.........g.......#
#<.................>...#
########################

legend:
- ' ' none
- @ FLOOR as START
- g FLOOR with GOBLIN
- < STAIR.up as STAIRS_UP
- > STAIR.down as STAIRS_DOWN

vars:
- DANGER 2
```

- **Otsikko:** `map NIMI [extends POHJA] [size LxK] [transient] [wrap] [nosave]` (atlaksen lehti, luku [8.12](#812-atlas-karttalehdet)), ja sen perään voi tulla vyöhykkeiden liitos (luku [1.2](01-rajaus-ja-periaatteet.md#12-rajapinnat-pelilogiikkaan)). Lyhyemmät rivit täytetään oikealta tyhjillä ruuduilla ja puuttuvat rivit alhaalta tyhjillä riveillä, joten editorin poistamat loppuvälilyönnit eivät haittaa. Liian pitkä rivi on virhe. Ilman `size`-määrettä koko lasketaan piirroksesta, ja rivien pitää olla yhtä pitkiä.
- **Generoitu kartta** määritellään RogueScriptissä (`map NIMI [transient] generate GENERAATTORI(…)`, RogueScript luku [8.2](../roguescript/08-generointi.md#82-generoitu-kartta)). Se on samaa lajia kuin piirretty kartta, joten siihen viitataan samalla tavalla.
- **Ei pelaajaa.** Kartat eivät sisällä pelaajaa. Pelaajan luo logiikka aloituksessa (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)), ja kartalta toiselle siirrytään nimiöiden kautta (luku [8.5](#85-nimiöt-ryhmät-ja-johtimet)). Näin sama kartta toimii sekä aloituskarttana että tavallisena tasona. Yllä `@` on pelkkä aloituskohta `START`.
- **Piirros:** yksi merkki on yksi ruutu, eikä piirroksessa ole kommentteja. Välilyönti on oletuksena `none` (tyhjä ruutu, prefabissa läpinäkyvä). Legendissä se kirjoitetaan `' '`. Jos välilyönnille annetaan toinen merkitys, tyhjä ruutu merkitään toisella merkillä (`- _ none`).
- **Ruudun sijainti:** piirroksen merkin sarake ja rivi ovat ruudun sijainti `X,Y` (luku [2.4](02-yleinen-syntaksi.md#24-arvot)), ja vasemman yläkulman merkki on `0,0`.
- **Markdown:** otsikot ja `>`-lainaukset ovat vapaata tekstiä (luku [2.5](02-yleinen-syntaksi.md#25-kommentit)).
- **Näkyvä nimi.** Kartan nimi pelaajalle, esimerkiksi tilarivillä tai tulostetussa kartassa, on kielitiedostossa avaimella `<KARTTA>.NAME` (`TOWN.NAME Satama`, luku [10](10-tekstit.md)). Markdown-otsikko ei nimeä karttaa.
- **Include** toimii kuten muuallakin (luku [3](03-tiedostot-ja-include.md)), joten yhteiset legendit voi koota omaan tiedostoonsa. Yksi tiedosto voi sisältää useita karttoja ja prefabeja.
- **Pysyvyys.** Kartta ladataan, kun sille saavutaan ensimmäisen kerran, ja se säilyy sen jälkeen: paluu löytää kartan sellaisena kuin se jäi. `transient`-kartta hylätään, kun pelaaja lähtee siltä, ja se ladataan tai generoidaan uudelleen seuraavalla saapumisella, myös kun `travel` vie nykyiseen karttaan (moottoridokumentti, luku [6.7](../moottori/06-ajo.md#67-siirtyminen-kartalta-toiselle)).

**Otsikon määreet.** `wrap` tekee kartasta kiertävän (moottoridokumentti, luku [6.9](../moottori/06-ajo.md#69-kiertävä-kartta)), ja `nosave` estää pelaajan oman tallennuksen kartalla (moottoridokumentti, luku [5.2](../moottori/05-aloitus-tallennus-ja-checkpointit.md#52-tallennus)). `extends` kuvataan luvussa [8.10](#810-perivä-kartta-extends).

**Piirroksen merkit.** Piirroksessa ja legendin merkkinä saa käyttää mitä tahansa Unicode-merkkiä. Lataus normalisoi tiedoston NFC-muotoon, ja piirroksen sarake on yksi koodipiste, joten hajotettu ä on yksi ruutu. Yhdistävät merkit, liitosmerkki (ZWJ) ja variaatiovalitsimet ovat piirroksessa latausvirhe. Leveä merkki on yksi ruutu mutta antaa latausvaroituksen, koska se rikkoo piirroksen tasauksen. Sääntö koskee kaikkia piirroksia (kartta, prefab, asettelu, paneeli) ja tyylin merkkejä (luku [9.2](09-ulkoasu.md#92-tilejen-ulkoasu-tilesglyphs)), ja nimet pysyvät ASCII-merkkeinä (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)).

## 8.2 Legend

```
- MERKKI TILE [KERROS TILE ...] [with ENTITEETTI...] [as [local] NIMIÖ] [wire NIMIÖ]
```

- **Tile:** ensimmäinen nimi on tile, joka menee oletuskerrokseen. Legend on tuottava paikka (luku [7.4](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#74-hakeva-ja-tuottava-paikka)), joten tilejoukko ja tilen väli eivät kelpaa.
- **Kerrokset:** muut maastokerrokset annetaan tilen jälkeen pareina `KERROS TILE` (luku [8.3](#83-kerrokset-legendissä)).
- `with` lisää ruutuun entiteettejä tai noston pakasta tai taulusta (luvut [6.2](06-pakat-ja-taulut.md#62-tila-sekoitus-ja-nosto) ja [6.5](06-pakat-ja-taulut.md#65-taulut)), joka tehdään kartan latautuessa. Entiteetin kenttiä voi ylikirjoittaa kuten pakan kortissa (`- ! FLOOR with RING_MAIL ENCHANT 2`, luku [6.4](06-pakat-ja-taulut.md#64-kenttien-ylikirjoitus)), myös moottorin komponentteja (`with LIFT label GATE_A`).
- `as` nimeää ruudun nimiöllä, ja `local` tekee nimiöstä prefabin sijoituskohtaisen (luku [8.6](#86-paikalliset-nimiöt-prefabeissa)).
- `wire` antaa ruudulle johtimen (luku [8.5](#85-nimiöt-ryhmät-ja-johtimet)).
- **Useita legendejä:** kartassa voi olla useita `legend:`-alilohkoja ja includella tuotuja legendejä, ja ne yhdistyvät niin, että myöhempi merkki voittaa. Tuntematon merkki piirroksessa on virhe, ja virheilmoitus kertoo rivin ja sarakkeen.

## 8.3 Kerrokset legendissä

```
map MARSH size 12x3
############
#..~~,,~~..#
############

legend:
- # WALL
- . FLOOR
- ~ FLOOR.moss COVER WATER.2
- , GRASS COVER WATER.1 DECAL LEAVES
```

- **Kerrosparit:** ensimmäinen tile menee oletuskerrokseen, ja kerrokset annetaan sen jälkeen pareina `KERROS TILE`. Pareja seuraavat tutut osat `with`, `as` ja `wire`.
- **Kerros ensin:** rivin voi aloittaa myös kerroksen nimellä (`- w COVER WATER.source`), jolloin oletuskerrokseen ei kirjoiteta mitään.
- Kerros, jota legend ei mainitse, saa `fill`-arvonsa tai jää tyhjäksi (luku [7.2](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#72-maastokerrokset-terrain)).
- Nimiöt ja johtimet kuuluvat ruudulle eivätkä kerrokselle, joten ne säilyvät, vaikka ruudun kerroksia muutetaan.

## 8.4 Kartan muuttujat: vars:

`vars:` on alilohko kartan omille muuttujille (`- DANGER 2`). Muuttujat ovat isoja nimiä, ja niiden arvot ovat kartan alkuarvoja. Puuttuva muuttuja on 0, kuten puuttuva kenttä. Kirjastojen käyttämät muuttujat (esimerkiksi syvyys `DEPTH`) ovat samanlaisia muuttujia. Generoidun kartan muuttujat asettaa generaattori.

**Tyypit.** Muuttujan arvo voi olla mitä tahansa komponentin kentän tyyppiä (luku [5.1](05-komponentit-ja-entiteetit.md#51-komponentit)) kokoelmia lukuun ottamatta, myös alue (`area`) ja lista. Datassa tyyppi päätellään arvon muodosta kuten vakioissa, ja generaattori päättelee sen ensimmäisestä sijoituksesta. Myöhempi eri tyyppinen sijoitus on virhe. Puuttuvan muuttujan luku antaa 0 lukuna, tyhjän listan listana ja muuten `none`. Generaattori voi tallentaa esimerkiksi huoneiden listan (`map.ROOMS = ROOMS(…)`, RogueScript luku [8.1](../roguescript/08-generointi.md#81-generaattorit)), ja alue säilyttää karttansa.

## 8.5 Nimiöt, ryhmät ja johtimet

Nimiö nimeää kartan ruudun tai entiteetin, ja se kertoo, mihin kohtaan kartalla saavutaan. Nimiöillä kartat yhdistetään toisiinsa ja kytkimet kohteisiinsa ilman koordinaatteja.

```
map LIFT_ROOM size 15x4
###############
#L...p:::::...#
#..@.::::::...#
###############

legend:
- # WALL
- . FLOOR
- : CHASM
- @ FLOOR as START
- L LEVER.off wire GATE_A
- p CHASM with LIFT label GATE_A
```

- **Ruudun nimiö** merkitään legendiin sanalla `as` tai generaattorissa funktioilla `tile(…, label:)` ja `tag` (RogueScript, luvut [6.4](../roguescript/06-ruudut-ja-paikkahaku.md#64-siirtyminen-kartalta-toiselle) ja [8.1](../roguescript/08-generointi.md#81-generaattorit)). Nimiöt ovat kartan sisäisiä nimiä (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)), joten jokaisella kartalla voi olla oma `START`.
- **Ryhmä:** sama nimiö useassa ruudussa muodostaa ryhmän. Ruudut numeroidaan lukujärjestyksessä, eli ylhäältä alas ja rivillä vasemmalta oikealle, alkaen nollasta. Yksittäinen ruutu on ryhmä, jonka koko on 1.
- `START` on kartan oletussaapumiskohta.
- **Entiteetin nimiö** on moottorin komponentti `label NIMIÖ`. Legendissä se kirjoitetaan entiteetin kentän ylikirjoituksena (`with LIFT label GATE_A`). Nimiö liikkuu entiteetin mukana, toisin kuin ruudun nimiö. Ryhmä kattaa sekä ruudut että entiteetit.
- **Ruudun johdin** `wire NIMIÖ` on legendin määre, joka tallentuu karttaan. Se kertoo, mihin nimiöön ruudun kytkin on kytketty. Ruudulla voi olla sekä `as` että `wire`.
- **Entiteetin johdin** on moottorin komponentti `wire NIMIÖ`.
- **Yksi nimiö kerrallaan:** ruudulla ja entiteetillä on korkeintaan yksi nimiö ja yksi johdin.

**Saapuminen.** Kartalta toiselle siirrytään logiikassa (RogueScriptin `travel`, luku [6.4](06-pakat-ja-taulut.md#64-kenttien-ylikirjoitus)), ja saapumisruutu valitaan nimiöstä:

```
map FIELD_A size 8x3
W......E
W......E
W......E

legend:
- . GRASS
- W GRASS as WEST
- E GRASS as EAST
```

- **Arvonta.** Oletuksena saapumisruutu arvotaan kohderyhmästä siemenen perusteella.
- **Saapuminen samaan indeksiin.** `travel(…, align: true)` vie ruutuun, jolla on kohderyhmässä sama indeksi kuin siirtyjän lähtöruudulla omassa ryhmässään. Jos kohderyhmässä on vähemmän ruutuja, saavutaan viimeiseen. Näin saman kokoiset reunat yhdistyvät ilman koordinaatteja: `FIELD_A`:n `EAST`-ryhmän ruudusta 1 saavutaan `FIELD_B`:n `WEST`-ryhmän ruutuun 1. Jos lähtöruudulla ei ole nimiötä, ruutu arvotaan.
- **Ilman kohdenimiötä** saavutaan nimiöön `START`. Puuttuva nimiö kohdekartassa on virhe: piirretylle kartalle latausvirhe, generoidulle käyttöhetken virhe.
- **Tapahtumat.** Siirtyminen lähettää lähtökartalla tapahtumat `leave` ja `depart` ja kohdekartalla tapahtuman `arrive(who, map, p)`, jossa `p` on saapumisruutu (moottoridokumentti, luku [9.6](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#96-tapahtumat)).
- **Ei paluuta heti.** Saapuminen ei lähetä saapumisruudussa tapahtumaa `enter`, joten pelaaja ei pomppaa heti takaisin (RogueScript, luku [6.4](../roguescript/06-ruudut-ja-paikkahaku.md#64-siirtyminen-kartalta-toiselle)).

## 8.6 Paikalliset nimiöt prefabeissa

Kun sama prefab sijoitetaan karttaan kahdesti, sen nimiöt tulisivat kahteen paikkaan, ja kytkin ohjaisi molempia kopioita. Paikallinen nimiö estää tämän:

```
prefab BELT_ROOM
#########
#L>>>>>.#
#########

legend:
- L LEVER.off wire BELT_A
- > BELT.stop as local BELT_A
```

- `as local NIMI`: sijoitus lisää nimiöön sijoituksen järjestysnumeron, eli ensimmäinen sijoitus saa nimen `BELT_A.1` ja toinen `BELT_A.2`. Sama siemen antaa samat nimet.
- **Oletus on yhteinen nimiö.** Saapumisnimiöiden (esimerkiksi portaiden `STAIRS_UP`) on toimittava jokaisessa sijoituksessa samalla nimellä.
- **Johtimen ratkaisu:** prefabin `wire BELT_A` osoittaa ensin saman sijoituksen paikalliseen nimiöön ja vasta sitten yhteiseen. Kytkin ohjaa siis vain omaa huonettaan. Tulos (`BELT_A.1`) tallentuu ruudun johtimeksi.
- **Entiteetin nimiö:** `with LIFT label local GATE_A` toimii samoin kuin ruudun nimiö.
- **Latausvaroitus** tulee, jos prefabin johdin osoittaa yhteiseen nimiöön, joka on merkitty saman prefabin sisällä. Kyse on lähes aina unohtuneesta `local`-merkinnästä.
- **Piirretyt kartat:** karttaa ei sijoiteta kuin kerran, joten `local` ei tee siinä mitään. Merkintä sallitaan silti, jotta piirretyn huoneen voi myöhemmin muuttaa prefabiksi ilman muutoksia.

## 8.7 Prefabit

Prefab on karttapala, joka painetaan toiseen karttaan. Se on samanmuotoinen kuin kartta, mutta otsikko on `prefab NIMI`, ja sen perään voivat tulla parametrit (luku [1.2](01-rajaus-ja-periaatteet.md#12-rajapinnat-pelilogiikkaan)).

```
prefab VAULT
#####
#.$.#
#####

legend:
- # WALL
- . FLOOR
- $ FLOOR with LOOT.RARE
```

- **Sijoitus on logiikkaa.** Prefab kuvaa vain muotonsa. Sen sijoittaa RogueScriptin generaattori vakiokirjaston funktiolla (`PLACE(VAULT, area: at(WALL))`), ja generaattori päättää myös, sijoitetaanko se (RogueScript, luvut [8.1](../roguescript/08-generointi.md#81-generaattorit) ja [8.3](../roguescript/08-generointi.md#83-vakiokirjaston-generointi)). Siksi prefabin otsikossa ei ole sijoitusehtoa.
- **Läpinäkyvyys:** `none`-ruutu jättää alla olevan ruudun kokonaan ennalleen.
- **Kerrokset:** prefab kirjoittaa vain ne maastokerrokset, jotka sen legend mainitsee, ja jättää muut ennalleen. Yhden kerroksen prefab, esimerkiksi lähde (`- W COVER WATER.source`), jää sen maaston päälle, johon se painetaan.
- **Entiteetit:** `with`-entiteetit syntyvät painettaessa.
- **Ei paikallisia osioita:** painettu prefab on osa toista karttaa, joten prefabin tiedostossa ei saa olla kartan paikallisia osioita.

## 8.8 Paikkahaku: at, near ja from

Paikkahaku valitsee kartalta ruutujoukon. Datassa sitä käyttää `game`-lohkon `at` (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)), ja RogueScript käyttää samoja osia funktioina (`at`, `near`, `from`, RogueScript luku [6](../roguescript/06-ruudut-ja-paikkahaku.md)). Haun tulos on aina ruutujoukko. Jos käyttöpaikka tarvitsee yhden ruudun, se arvotaan joukosta pelin siemenen perusteella.

```
at KOHDE [near [ANKKURI] [ETÄISYYS]]
at SIIRTYMÄ from ANKKURI
```

Kohde on yksi kolmesta, ja laji ratkeaa muodosta:

| Kohde | Esimerkki | Tulos |
| --- | --- | --- |
| sijainti | `at 5,3` | yksi ruutu |
| nimiö | `at START` | nimiöryhmän ruudut ja nimettyjen entiteettien ruudut |
| hakujoukko | `at STAIR.up`, `at WATER.1..2`, `at SHRINES` | ruudut, joiden jokin kerros on joukon jäsen (luku [7.4](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#74-hakeva-ja-tuottava-paikka)) |
| alue | `at VAARAVUORI` | alueen ruudut (luku [8.11](#811-nimetyt-alueet-areas)) |

Luvulla alkava kohde on sijainti. Muu kohde on nimi: ensin katsotaan kartan nimiöt ja alueet, sitten yleinen nimenhaku. Sekaannusta ei synny, koska nimiö ei saa olla sama kuin yleinen nimi (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). Löytynyt yleinen nimi, joka ei ole tile eikä tilejoukko, on latausvirhe.

`near` rajaa kohteen ruudut ankkurin lähelle:

```
at FLOOR near START 3
at FLOOR near GATE_A 2..4
at START near 3
```

- **Ankkuri** on nimiö tai alue. Ruudun nimiö ja alue ovat kiinteitä paikkoja, entiteetin nimiö (`label`, luku [8.5](#85-nimiöt-ryhmät-ja-johtimet)) sen kantajan nykyinen ruutu. Ryhmä on jäsentensä lähialueiden yhdiste. Kokoelmassa oleva entiteetti ei ole kartalla, joten se ei ole ankkuri.
- **Etäisyys** on luku tai väli. Luku N tarkoittaa väliä `0..N`, ja oletus on 1. Väli antaa renkaan: `2..4` jättää ankkurin ja sen lähimmät ruudut pois. Avoin yläraja `2..` on sallittu.
- **Metriikka** on Chebyshev: kaikki kahdeksan naapuria ovat etäisyydellä 1, kuten kahdeksan suuntaa (luku [2.4](02-yleinen-syntaksi.md#24-arvot)).
- **Leikkaus:** `FLOOR near START 3` on lattiaruudut, jotka ovat enintään kolmen ruudun päässä `START`-ruudusta.
- **Lyhenne:** jos `near`-osasta puuttuu ankkuri, ankkuri on kohteen nimiö ja kohde on kaikki ruudut. `at START near 3` tarkoittaa siis mitä tahansa ruutua kolmen ruudun säteellä `START`-ruudusta. Lyhenne edellyttää, että kohde on nimiö.

`from` antaa ruudun suhteessa ankkuriin:

```
at 2,-1 from START
at 3 east from GATE_A
at 2 front from GUARD
```

- Siirtymä on `X,Y` (luku [2.4](02-yleinen-syntaksi.md#24-arvot)) tai `ETÄISYYS SUUNTA`.
- Absoluuttinen suunta (`dir`) on kartan suunta: `3 east` on sama kuin `3,0`. Vinosuunnat siirtävät molempia akseleita, joten `2 northeast` on `2,-2`.
- Suhteellinen suunta (`side`) luetaan ankkurientiteetin `facing`-komponentista. Jos ankkuri on ruudun nimiö tai entiteetiltä puuttuu `facing`, suhteellinen suunta on latausvirhe.
- **Ryhmä ankkurina:** siirtymä lasketaan jokaisesta jäsenestä, ja tulos on niiden joukko.
- Kartan ulkopuolelle osuvat ruudut putoavat joukosta pois.

**Tyhjä tulos.** Jos haku ei tuota yhtään ruutua, se on virhe käyttöhetkellä, ja virheilmoitus kertoo kartan ja koko haun. Lataus tarkistaa etukäteen vain sen, minkä voi tietää ilman pelin tilaa: nimiöt ja nimet ovat olemassa ja sijainti on kartan sisällä, kun kartalla on `size`.

**Sijainti ja nimiö.** Sijainti on hauras, koska se rikkoutuu, kun piirrosta muokataan. Kartat yhdistetään siksi nimiöillä (luku [8.5](#85-nimiöt-ryhmät-ja-johtimet)). Sijainti sopii testaamiseen ja pelaajan `overrides/`-tiedostoon, esimerkiksi suoraan pomohuoneeseen hyppäämiseen (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)).

**Pelaaja ankkurina.** Kartat eivät sisällä pelaajaa (luku [8.1](#81-karttatiedosto)), joten data ei viittaa pelaajaan ankkurina. Pelaajan nimiö annetaan entiteetin määrittelyssä (`- label HERO`) tai synnytettäessä. Aloituksen `at` lasketaan ennen kuin pelaaja on kartalla. RogueScriptissä ankkuri voi olla mikä tahansa entiteettiviittaus, myös `player`.

## 8.9 Sijaintikohtaiset rivit: at:

Alilohko `at:` liittää legendin muotoisen rivin yksittäiseen ruutuun piirroksen päälle. Isolla kartalla jokainen kohde saa näin oman nimiönsä ja entiteettinsä ilman omaa legendimerkkiä.

```
at:
- 34,12 as MOON with SITE.MOON
- 40,7 COVER RUINS as RUINS
- 9,3 as START
```

- **Rivi** on `- X,Y [TILE] [KERROS TILE ...] [with …] [as …] [wire …]`. Osat ovat samat kuin legendissä (luku [8.2](#82-legend)), mutta tile on valinnainen.
- **Päälle.** Rivin tilet korvaavat ruudun kyseiset kerrokset, ja nimiö, johdin ja entiteetit lisätään piirroksen omiin.
- **Virheet.** Sijainnin on oltava kartan sisällä, ja kaksi riviä samalle sijainnille on latausvirhe.
- **Prefabissa** sijainti on suhteessa prefabin vasempaan yläkulmaan, ja `as local` toimii kuten legendissä (luku [8.6](#86-paikalliset-nimiöt-prefabeissa)).

## 8.10 Perivä kartta: extends

`map NIMI extends POHJA` tekee pohjasta uuden kartan omalla nimellään, joten monta kaupunkia voi käyttää samaa piirrosta.

```
map TOWN.MOON extends TOWN.A transient

legend:
- w FLOOR with MERCHANT.WEAPONS.WEST

vars:
- WEALTH 2
```

- Perivä kartta saa pohjan piirroksen, legendin, `vars:`-osion, `at:`-rivit ja alueet (`areas:`).
- Omat `legend:`-rivit korvaavat pohjan rivit merkin mukaan, `vars:`-rivit nimen mukaan ja `at:`-rivit sijainnin mukaan. `areas:`-rivit lisätään aina, joten oma rivi voi laajentaa pohjan aluetta. Uudet rivit lisätään.
- `size` tulee pohjasta, ja oma piirros on latausvirhe. Otsikon `transient`, `wrap` ja `nosave` ovat kartan omia eivätkä periydy.
- Pohja voi itse periä toisen kartan. Kehä tai puuttuva pohja on latausvirhe, samoin generoitu pohja.
- Pohja on tavallinen kartta, jolle voi myös siirtyä. Pelkkänä pohjana käytettävän kartan nimi kannattaa ryhmitellä (`TOWN.A`).

## 8.11 Nimetyt alueet: areas:

Alue nimeää kartan osan, esimerkiksi saaren, vuoren tai salin. Alue on pelkkää merkitystä: se ei muuta ruutuja, vaan kertoo, mihin nimettyyn paikkaan ruutu kuuluu. Pelin tilarivi voi näyttää sen, ja tulostettava kartta kirjoittaa alueen nimen karttaan.

```
map MERIKARTTA size 64x44
…

areas:
- 8,6 size 26x17 as NORSUSAARI
- 30,3 size 7x6 as NORSUSAARI
- 14,9 size 7x5 as VAARAVUORI
- 2,24 size 12x10 as SYNKKA_SYVA
```

- **Rivi** on `- X,Y size LxK as ALUE`: suorakulmio, jonka vasen yläkulma on ruudussa `X,Y` (luku [2.4](02-yleinen-syntaksi.md#24-arvot)) ja koko L × K ruutua. Suorakulmion on oltava kokonaan kartan sisällä, ja sen leveys ja korkeus ovat vähintään 1. Muu on latausvirhe.
- **Yhdiste.** Saman nimen rivit muodostavat yhdessä alueen. Rajaukset saavat olla päällekkäin ja vierekkäin, joten alueen voi koota mistä muodosta tahansa. Yllä `NORSUSAARI` on kahden suorakulmion yhdiste.
- **Nimi** on kartan sisäinen nimi kuten nimiö (luku [8.5](#85-nimiöt-ryhmät-ja-johtimet)), joten jokaisella kartalla voi olla omat alueensa. Sama nimi ei saa olla kartalla sekä alueena että nimiönä, ja se ei saa olla sama kuin yleinen nimi (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). Myös yhdistelmä `<KARTTA>.<ALUE>` on latausvirhe, jos se on yleinen nimi, koska näkyvän nimen avain olisi silloin kaksitulkintainen (luku [10](10-tekstit.md)).
- **Ruutu voi kuulua moneen alueeseen.** Yllä Vaaravuoren ruudut kuuluvat sekä Vaaravuoreen että Norsusaareen.
- **Sisäkkäisyys.** Alue A on alueen B vanhempi, kun kaikki B:n ruudut ovat A:ssa ja A:ssa on enemmän ruutuja. Vanhemmista lähin on se, jossa on vähiten ruutuja, ja tasatilanteessa se, jonka ensimmäinen `areas:`-rivi on myöhempänä. Alueen taso on lähimpien vanhempien ketjun pituus: uloimmat alueet ovat tasolla 0, niiden lähimmät lapset tasolla 1 ja niin edelleen. Taso ei siis kasva, vaikka alue olisi kahden samanruutuisen tai osittain päällekkäisen alueen sisällä. Sisäkkäisyys lasketaan ruuduista, eikä sitä kirjoiteta. Kaksi aluetta, joilla on täsmälleen samat ruudut, eivät ole toistensa vanhempia: ne ovat samalla tasolla, ja lataus varoittaa, koska kaksi nimeä samalle paikalle on yleensä virhe. Tarkimman alueen tasatilanne ratkeaa silloin alla olevalla säännöllä.
- **Osittainen päällekkäisyys.** Jos kaksi aluetta jakaa ruutuja mutta kumpikaan ei ole toisen sisällä eivätkä ruudut ole samat, lataus varoittaa, eikä kummastakaan tule toisen vanhempaa.
- **Tarkin alue.** Ruudun tarkin alue on pienin alue, johon ruutu kuuluu. Tasatilanteessa voittaa alue, jonka ensimmäinen `areas:`-rivi on myöhempänä; perivän kartan omat rivit ovat pohjan rivien jälkeen. Esimerkiksi tilarivi näyttää tarkimman alueen (*Vaaravuori*) tai koko polun (*Norsusaari › Vaaravuori*).
- **Paikkahaku.** Alueen nimi kelpaa paikkahaun kohteeksi ja ankkuriksi (luku [8.8](#88-paikkahaku-at-near-ja-from)): `at VAARAVUORI` on alueen ruudut, ja `at FLOOR near NORSUSAARI 2` on lattiat enintään kahden ruudun päässä saaresta.
- **Näkyvä nimi** on kielitiedostossa avaimella `<KARTTA>.<ALUE>.NAME` (luku [10](10-tekstit.md)), ja alueen nimen piirtäminen kuuluu tyylitiedostoon (luku [9.6](09-ulkoasu.md#96-paperikartta-paper)).
- **Perivä kartta** saa pohjan alueet, ja sen omat `areas:`-rivit lisätään niihin (luku [8.10](#810-perivä-kartta-extends)).
- **Prefabissa** `areas:` on toistaiseksi latausvirhe, koska saman prefabin kaksi sijoitusta toisivat saman alueen kahteen paikkaan (luku [13](13-avoimet-kysymykset.md)).

## 8.12 Atlas: karttalehdet

Atlas jakaa ison maailman karttalehtiin. Jokainen lehti on tavallinen kartta, joten peli lataa ja tallentaa vain ne lehdet, joilla on käyty, ja kartan voi piirtää ja muokata lehti kerrallaan. Lehdet liittyvät toisiinsa reunoistaan, ja moottori siirtää kulkijan reunan yli naapurilehdelle (moottoridokumentti, luku [6.10](../moottori/06-ajo.md#610-atlas)).

```
atlas MAAILMA sheet 64x48 grid 10x14 fill WATER

legend:
- ~ WATER
- . GRASS
- T FOREST.conifer

vars:
- OUTDOOR true

areas:
- 120,40 size 90x60 as NORSUSAARI
```

```
map MAAILMA.B10

(64 × 48 merkin piirros)

at:
- 12,7 as START

areas:
- 3,4 size 6x5 as LAHDE
```

- **Otsikko:** `atlas NIMI sheet LxK grid SxR [fill TILE]`. `sheet` on lehden koko ruutuina ja `grid` lehtien määrä sarakkeina ja riveinä, joten atlaksen koko on S·L × R·K ruutua. `fill` on puuttuvan lehden tile oletuskerroksessa, ja ilman sitä puuttuva lehti on `none`.
- **Lehden nimi** on `NIMI.<sarake><rivi>`: sarake kirjaimin (`A`…`Z`, sitten `AA`, `AB` …) ja rivi numerona alkaen 01. Rivinumerossa on vähintään kaksi numeroa ja niin monta kuin rivien määrä vaatii (`C005`, kun rivejä on yli 99). `A01` on vasen yläkulma, ja `B10` on toinen sarake, kymmenes rivi. Lehden nimi on atlaksen ryhmässä (luku [2.3](02-yleinen-syntaksi.md#23-pistenotaatio)).
- **Lehti** on kartta, jonka otsikossa on lehden nimi. Koko tulee atlaksesta, joten `size` on tarpeeton; jos se annetaan, sen on oltava sama. Lyhyemmät rivit ja puuttuvat rivit täytetään kuten muissakin kartoissa (luku [8.1](#81-karttatiedosto)). `wrap` lehdellä on latausvirhe, koska lehden reunat liittyvät naapureihin.
- **Perintä.** Lehti saa atlaksen `legend:`-, `vars:`- ja `areas:`-rivit samoin kuin perivä kartta pohjaltaan (luku [8.10](#810-perivä-kartta-extends)): lehden omat legendirivit korvaavat merkin ja `vars:`-rivit nimen. Lehden `at:`-rivit ja `areas:`-rivit ovat lehden omia, ja niiden sijainnit ovat lehden koordinaateissa.
- **Atlaksen alueet** ovat atlaksen koordinaateissa, joten saari voi ulottua usealle lehdelle. Lehdellä alue on sen osa, joka osuu lehteen, ja se on lehden alue atlaksen nimellä: `at NORSUSAARI` lehdellä B10 on saaren B10:lle osuvat ruudut. Lehden oma alue ja atlaksen alue eivät saa olla samannimisiä.
- **Lähialue.** Pelissä lehti ja sen naapurit (enintään 3 × 3 lehteä) ovat yhdessä aktiivisia, ja haut, reitit, näkö, valo ja ääni ulottuvat yhden lehtirajan yli (moottoridokumentti, luku [6.10](../moottori/06-ajo.md#610-atlas)). Siksi lehden on oltava vähintään pelin suurimman säteen kokoinen; muuten lataus varoittaa.
- **Puuttuva lehti.** Lehtiä ei tarvitse kirjoittaa kaikkia. Puuttuva lehti on `fill`-tileä, ja sillä on atlaksen legendi, muuttujat ja alueet.
- **Atlas ei ole kartta.** Siihen ei voi siirtyä eikä se ole `game`-lohkon aloituskartta: siirto kirjoitetaan lehteen (`game MAAILMA_PELI start MAAILMA.B10 at START`).
- **Tarkistukset.** Lehti ruudukon ulkopuolella (`MAAILMA.K01`, kun sarakkeita on 10) ja sama lehti kahdesti ovat latausvirheitä. Väärän kokoinen piirros on latausvirhe kuten liian pitkä rivi kartassa.
- **Tiedostot.** Lehdet voivat olla samassa tiedostossa kuin atlas tai omissa tiedostoissaan. Tiedostonimellä ei ole merkitystä (luku [3](03-tiedostot-ja-include.md)); käytäntö on `maailma-B10.rlml.md`.
- **Tekstit.** Atlaksen näkyvä nimi on avaimella `<ATLAS>.NAME` ja atlaksen alueen nimi avaimella `<ATLAS>.<ALUE>.NAME`. Lehti on kartta, joten sen alueen nimi on avaimella `<ATLAS>.<LEHTI>.<ALUE>.NAME` (`MAAILMA.B10.LAHDE.NAME`, luku [10](10-tekstit.md)).

---

← [7. Tilet, maastokerrokset, tilejoukot ja ominaisuudet](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md) · [Sisällys](README.md) · [9. Ulkoasu](09-ulkoasu.md) →
