[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 9. Ulkoasu

Tyylitiedostoja on yksi kutakin esitystapaa kohti, ja asetus `- style ascii` valitsee käytettävän (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)).

```
colors
- STONE 5a5a6e
- DARK 3a3a4a
- WOOD a0702a
- RED c84040

tiles.glyphs ascii
- default ? fg ff00ff
- WALL # fg STONE bg DARK
- DOOR + fg WOOD bg DARK
- DOOR.open ' fg WOOD bg DARK
- WATER ~ fg 3a6ac0 bg 0a1a30
- WATER.1..2 ~ fg 8ab0e0
- BLOOD , fg a02020
- GLYPH.GOBLIN g fg 4a9a3a

panels
- LOG border single fg STONE bg DARK

logs
- MESSAGE.COMBAT fg RED

presentation
- frame 15ms
```

## 9.1 Värit

Värit nimetään `colors`-osiossa (`- NIMI hex`), ja nimeä voi käyttää hex-arvon sijaan kaikissa tyylin riveissä. Hex-arvo kirjoitetaan kuudella pienellä heksadesimaalimerkillä ilman `#`-merkkiä.

## 9.2 Tilejen ulkoasu: tiles.glyphs

Tilen ulkoasu on tilen ominaisuus `glyphs` (luku [7.5](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)), mutta sen arvo riippuu esitystavasta. Siksi se kirjoitetaan tyylitiedostoon omaan osioonsa, jonka otsikko on `tiles.glyphs ESITYSTAPA`:

```
tiles.glyphs ascii
- WALL # fg STONE bg DARK
- DOOR.open ' fg WOOD

tiles.glyphs spritesheet "tiles16.png" 16x16
- WALL 12
- DOOR.open 13
```

- **ASCII:** `tiles.glyphs ascii` antaa jokaiselle tilelle merkin ja värit: `- TILE MERKKI [fg VÄRI] [bg VÄRI]`.
- **Merkki.** Merkki on yksi Unicode-koodipiste kuten piirroksessa (luku [8.1](08-kartat-ja-prefabit.md#81-karttatiedosto)): yhdistävät merkit, liitosmerkki ja variaatiovalitsimet ovat latausvirheitä, ja leveä merkki antaa latausvaroituksen (`- GLYPH.HEART ♥ fg RED`). Välilyönti kirjoitetaan `' '` kuten legendissä (`- FLOOR ' ' bg DARK`), ja heittomerkki itse kirjoitetaan sellaisenaan (`- DOOR.open ' fg WOOD`).
- **Graafinen:** `tiles.glyphs spritesheet "tiles16.png" 16x16` lukee kuva-arkin, ja sen rivit antavat kuvan indeksin (`- WALL 12`). `fg` ja `bg` ovat valinnaisia, ja graafisessa esitystavassa ne ovat sävytyksiä.
- **Haku** on sama kuin muilla tilen ominaisuuksilla (luku [7.5](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)): tarkin nimi, välirivit, perusnimi ja `default`. Tilejoukot eivät kelpaa riveiksi (luku [7.4](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#74-hakeva-ja-tuottava-paikka)).
- **Otsikon muoto** on `tiles.OMINAISUUS ESITYSTAPA [lisät]`. Pisteen jälkeen sallitaan vain esitystavoittain annettavat ominaisuudet, eli toistaiseksi `glyphs`. Paperiesitystavan otsikot `maps.theme` ja `areas.labels` kuvataan luvussa [9.6](#96-paperikartta-paper). Esimerkiksi `tiles.blocks` on latausvirhe, koska esto ei riipu esitystavasta.
- **Erilliset rivit:** `tiles.glyphs`-rivillä annetaan vain ulkoasu, eikä `tiles`-rivillä anneta ulkoasua. Säännöt ja ulkoasu pysyvät näin eri tiedostoissa (luku [1.1](01-rajaus-ja-periaatteet.md#11-periaatteet)).
- **Monta osiota ja korvaus:** saman esitystavan osiot yhdistyvät riveittäin kuten `tiles`-osiot (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)), ja rivin korvaus merkitään `- override WALL # fg RED` (luku [4.5](04-nimiavaruudet.md#45-korvaaminen-ja-override)).
- **Entiteetin tile** tulee moottorin komponentista `render` (`- render GLYPH.GOBLIN`, luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)).

**Sarjarivi.** Indeksiväli voi saada ulkoasunsa sarjana merkkijonosta ja väriluettelosta, kun ulkoasu arvotaan, esimerkiksi tunnistamattomille juomille tai robotfindskittenin esineille:

```
tiles.glyphs ascii
- GLYPH.NKI.1..36 series "&*+?!%" fg RED GREEN YELLOW BLUE MAGENTA CYAN

tiles.glyphs spritesheet "tiles16.png" 16x16
- GLYPH.NKI.1..36 series 200..235
```

- **ASCII-muoto** on `- TILE.a..b series "MERKIT" [fg VÄRI ...] [bg VÄRI ...]`. Kun merkkejä on M ja `fg`-värejä C, indeksin i merkki on merkkijonon kohta (i − a) mod M ja väri luettelon kohta ((i − a) / M) mod C. Yllä 6 merkkiä ja 6 väriä antavat 36 eri yhdistelmää. Yksi väri tai `bg`-luettelo toimii samoin.
- **Graafinen muoto** on `- TILE.a..b series ALKU..LOPPU`: indeksi i saa kuvan ALKU + (i − a), ja välien pituuksien on oltava samat.
- **Väli** on suljettu. Avoin väli sarjarivinä on latausvirhe, koska sarjalla on oltava pituus.
- **Haku.** Sarjarivi on välirivi (luku [7.1](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#71-tilen-nimi-muunnelma-ja-indeksi)), joten yksittäisen indeksin oma rivi voittaa sen. Merkkijonon merkit noudattavat yllä olevia merkkisääntöjä, ja välilyönti merkkijonossa on tyhjä merkki.

**Istuimet.** Moninpelissä istuimen jäsen haetaan ensin tilellä `TILE.SEATn`, joten `- GLYPH.HERO.SEAT2 @ fg 60a0ff` antaa istuimen 2 sankarille oman värin. Ilman riviä haku päätyy perusnimeen `GLYPH.HERO` (moottoridokumentti, luku [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen)).

## 9.3 Paneelit, lokit ja esitys

- `panels` antaa asettelun alueille (luku [11.1](11-kayttoliittyman-data.md#111-asettelu)) reunat ja värit.
- `logs` värittää viestit ryhmän tai avaimen mukaan (luku [11.4](11-kayttoliittyman-data.md#114-viestit-ja-loki)).
- **Valikot ja dialogit:** valikon ulkoasu haetaan kontekstin nimellä (`MENU`, `DIALOGUE`). Dialogi-ikkunan ulkoasu sekä valikon ja dialogin himmennettyjen kohtien väri (`disabled when`, luku [11.2](11-kayttoliittyman-data.md#112-valikot)) ovat tyylitiedoston asioita.
- `presentation`-osion `frame` on animaation välikehyksen kesto. Välikehys on mikroaskel, jossa näkyvä tila muuttui (moottoridokumentti, luku [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen)). Arvo `0` näyttää vain tilan seuraavalla syötteen odotuksella.

## 9.4 Maastokerrosten piirtäminen

Kun pelissä on useampi maastokerros (luku [7.2](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#72-maastokerrokset-terrain)), ruudusta piirretään näkyvä tile, eli ylin ei-tyhjä kerros, myös `pass`-kerros:

- **Taustan läpäisy:** jos näkyvän tilen tyylirivillä ei ole `bg`-arvoa, tausta haetaan seuraavasta ei-tyhjästä kerroksesta alaspäin. Verijälki (`- BLOOD , fg a02020`) näkyy siis lattian päällä lattian värisellä taustalla, ja matala vesi ilman taustaa näyttää alla olevan maaston.
- Graafisessa esitystavassa kerrokset piirretään päällekkäin alhaalta ylös.
- **Tyhjä ruutu:** jos kaikki kerrokset ovat tyhjiä, ruutu on `none`.

## 9.5 Konsolin kerrokset

Konsolin piirtokerrokset piirretään listan järjestyksessä, ja myöhempi peittää aiemman:

```
layers
- GROUND map remember
- ITEMS entities where it has no movement remember
- ACTORS entities where it has movement smooth 100ms
- FOG fog
```

Rivi on `- NIMI LÄHDE [where ehto] [remember] [smooth AIKA]`.

- **Lähde** on `map` (maasto kerroksineen), `entities` (entiteetit) tai `fog` (näkökentän ulkopuolinen alue).
- `where` suodattaa lähteen entiteetit kerrokseen niiden komponenttien perusteella, ja ehdossa `it` tarkoittaa suodatettavaa entiteettiä (RogueScript, luku [5.3](../roguescript/05-funktiot-ja-lausekkeet.md#53-lausekkeet)). Kerroksetta jäävä entiteetti jää piirtämättä, ja lataus varoittaa siitä.
- `remember` jättää viimeksi nähdyn tilanteen näkyviin. `map`-kerroksessa `forget`-tilet (luku [7.5](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)) unohtuvat, kun ne poistuvat näkökentästä. Logiikan paljastamat ruudut (`reveal`, moottoridokumentti luku [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen)) tulevat muistiin kuin ne olisi nähty.
- `smooth` liu’uttaa liikkeen annetussa ajassa, ja se on käytössä vain graafisessa esitystavassa.
- Kerroksen ulkoasu (`alpha`, värit) kuuluu tyylitiedostoon.
- **Nimet:** konsolin piirtokerrokset (`layers`) ja ruudun maastokerrokset (`terrain`) ovat eri asioita. Molemmat nimetään isoilla, mutta ne eivät viittaa toisiinsa.

## 9.6 Paperikartta: paper

Esitystapa `paper` piirtää kartan tulostettavaksi mustavalkoiseksi vektorikartaksi: rosoinen rantaviiva, viivoitettu meri ja kallio, maaston ja esineiden symbolit, alueiden nimet ja selite. Sitä käyttävät tulostettavien karttojen työkalut, kuten karttageneraattori. Peli ei valitse sitä asetuksella `- style`, eikä sillä ole konsolin kerroksia. Tulosteen paperikoko, ruudun koko ja viivavahvuudet ovat työkalun asetuksia eivätkä dataa.

```
maps.theme paper
- default sea
- DECK ship
- HOLD ship
- CAVE cave

tiles.glyphs paper
- WALL.crystal solid straight pattern runes
- FLOOR.bridge floor pattern planks
- MARSH shrubs
- KRAKEN decor

areas.labels paper
- level 0 size 11 spacing wide
- level 1 size 8.5
- SYNKKA_SYVA size 9 place water
- VAARAVUORI in MERIKARTTA outline dotted
```

**Kartan piirtotapa: maps.theme.** Rivi `- KARTTA TAPA` valitsee kartan piirtotavan, ja `- default TAPA` antaa sen muille kartoille. Ilman riviä tapa on `sea`.

| Tapa | Piirto |
| --- | --- |
| `sea` | merikartta: rosoinen rantaviiva, meri vaakaviivoitettuna (matalikko tummin), vuoret, kukkulat, metsät, pensaikko, palmut ja joet |
| `cave` | luola: luonnonseinät rosoisina ja rakennetut seinät suorina, kallio viivoitettuna avoimen tilan vierestä |
| `ship` | laiva: runko umpiviivoitettuna, kansilankut, ympärillä vaakaviivoitettu meri |

**Tilen piirto: tiles.glyphs paper.** Paperiesitystavassa tilen ulkoasu on piirtotapa eikä merkki. Rivi on `- TILE [LAJI] [määreet] [decor]`, ja haku on sama kuin luvussa [9.2](#92-tilejen-ulkoasu-tilesglyphs): tarkin nimi, perusnimi ja `default`.

- **Lajit merikartalla:** `water`, `grass`, `shrubs`, `conifer`, `deciduous`, `palm`, `mountain`, `hill`, `river`, `treasure` ja `kraken`.
- **Lajit luolassa ja laivassa:** `solid` (seinä, kallio, runko), `floor` (kuljettava lattia), `void` (ulkopuoli: meri tai tyhjä), `door`, `stairs` ja `exit` (poistuminen kartalta).
- **Määreet:** `rough` ja `straight` (seinän muoto), `thin` (ohut seinä, esimerkiksi kaide), `pattern planks|boards|gangplank|moss|runes` (lattian tai seinän kuvio), `symbol NIMI` (piirretty symboli), `locked` (ovi) sekä `up` ja `down` (portaat).
- **Symbolit:** `chest`, `key`, `gold`, `gem`, `goblet`, `amulet`, `ornament`, `pillar`, `barrel`, `cannon`, `mast`, `helm`, `crate`, `table`, `anchor`, `crew`, `torch`, `window`, `port`, `lantern`, `hatch` ja `ladder`.
- `decor` piirtää tilen tai entiteetin kartalle mutta jättää sen pois selitteestä. Kraken on koriste, ei kartan merkki.
- **Laji puuttuu.** Rivi ilman lajia (`- KRAKEN decor`) antaa vain määreet, ja laji tulee haun seuraavalta riviltä, jolla laji on: tilellä perusnimen ja `default`-rivin, entiteetillä vanhempien ja `render`-tilen rivin (alla). Jos lajia ei löydy mistään, lataus varoittaa ja nimeää tilen tai entiteetin, ja piirtäjä valitsee lajin itse. Varoitus kertoo siis jokaisen paikan, jonka piirto ei ole datassa.
- **Entiteetin rivi.** Paperiesitystavassa rivin nimi voi olla tile tai entiteetti (`- CHEST symbol chest`). Entiteetin nimen rivi ei luo tileä eikä mainitse sitä (luku [7](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md)), joten se ei törmää entiteetin nimeen. Nimi, joka on sekä entiteetin että tilen nimi, ei ole mahdollinen, koska yleiset nimet ovat yksikäsitteisiä (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)).
- **Entiteetit** (`with` legendissä ja `at:`-rivillä) piirretään lattian päälle. Haku on entiteetin oma rivi, sitten sen vanhempien rivit (`extends`, luku [5.2](05-komponentit-ja-entiteetit.md#52-entiteetit)) ja lopuksi sen `render`-tilen rivi (luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)). Työkalu, joka ei lue entiteettien määrittelyjä, hakee vain entiteetin nimellä.
- **Poistumat.** Nimiöllä merkitty `exit`-ruutu saa tekstikseen nimiön näkyvän nimen avaimella `<KARTTA>.<NIMIÖ>.NAME` (luku [10](10-tekstit.md)).

**Alueiden nimet: areas.labels.** Alueet (luku [8.11](08-kartat-ja-prefabit.md#811-nimetyt-alueet-areas)) kirjoitetaan karttaan niiden näkyvällä nimellä. Rivi on `- level N [in KARTTA] [määreet]` tai `- ALUE [in KARTTA] [määreet]`.

- **Haku:** määreet kootaan tässä järjestyksessä, ja myöhempi määre voittaa aiemman: piirtäjän oletus, tason rivi ilman `in`-rajausta, tason rivi `in KARTTA`, alueen rivi ilman `in`-rajausta ja lopuksi alueen rivi `in KARTTA`. Rajattu rivi voittaa siis rajaamattoman rivien järjestyksestä riippumatta. Saman vaiheen rivit (kaksi riviä `- VAARAVUORI …`) yhdistyvät, ja niistä myöhempi voittaa saman määreen. `in KARTTA` rajaa rivin yhteen karttaan, ja ilman sitä rivi koskee kaikkia karttoja, joilla on samanniminen alue tai sama taso.
- `size N` on tekstin korkeus, jossa 16 on yhden ruudun korkeus. Oletus on tasolla 0 `11`, tasolla 1 `8.5` ja syvemmällä `7`.
- `spacing wide|normal` on kirjainväli. Tason 0 oletus on `wide`.
- `place land|water|center` sijoittaa nimen alueen maaruutujen painopisteeseen, syvimpään vesikohtaan tai alueen keskelle. Ilman määrettä nimi menee maalle, jos alueesta on maata vähintään neljännes.
- `at X,Y` antaa nimen keskipisteen ruutuina, ja desimaalit ovat sallittuja (`at 12.5,8`).
- `hide` jättää alueen nimen piirtämättä.
- `outline dotted` piirtää alueen rajan pisteviivana.
- Piirtäjä siirtää nimeä, jos se osuisi toiseen nimeen, selitteeseen tai korkeaan maastoon.

**Otsikot.** Paperiesitystapa tuo luvun [9.2](#92-tilejen-ulkoasu-tilesglyphs) otsikkomuodon rinnalle kaksi uutta: `maps.theme paper` ja `areas.labels paper`. Ne ovat esitystavan omia, joten muussa esitystavassa (`maps.theme ascii`) ne ovat latausvirheitä.

---

← [8. Kartat ja prefabit](08-kartat-ja-prefabit.md) · [Sisällys](README.md) · [10. Tekstit](10-tekstit.md) →
