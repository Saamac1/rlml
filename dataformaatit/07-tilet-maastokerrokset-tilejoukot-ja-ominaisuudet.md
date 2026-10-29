[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 7. Tilet, maastokerrokset, tilejoukot ja ominaisuudet

Tile on ruudun maaston nimi. Tileä ei määritellä omalla lohkollaan: tile on olemassa, kun se esiintyy `tiles`-osiossa (luku [7.5](#75-tilen-ominaisuudet-tiles)), tyylitiedoston `tiles.glyphs`-osiossa (luku [9.2](09-ulkoasu.md#92-tilejen-ulkoasu-tilesglyphs)) tai legendissä. Sen ominaisuudet tulevat `tiles`-osiosta ja ulkoasu tyylitiedoston `tiles.glyphs`-osiosta.

## 7.1 Tilen nimi, muunnelma ja indeksi

```
DOOR
DOOR.open
WATER.3
LAVA.crust.2
WATER.1..2
WATER.4..
```

- **Muunnelma:** tilen nimen pienellä kirjoitettu osa on muunnelma (`DOOR.open`, `STAIR.down`). Perusnimi kattaa muunnelmansa, eli `DOOR` vastaa myös tileä `DOOR.open`.
- **Lukuindeksi:** tilen nimen viimeinen osa voi olla kokonaisluku `1`–`2147483647`. Indeksi tulee aina viimeiseksi, myös muunnelman jälkeen (`LAVA.crust.2`). Etunollat ovat virhe (`WATER.03`).
- **Vain tileillä:** lukuosa on sallittu ainoastaan tilen nimessä. Muiden nimien osat alkavat kirjaimella (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)), joten noppa ja nimi eivät sekoitu.
- **Indeksi 0 on perustile:** `WATER` on indeksi 0, ja `WATER.0` tarkoittaa samaa kuin `WATER` (latausvaroitus kirjoitusasusta).
- **Väli:** kun tileä verrataan tai haetaan (`tiles`- ja `tiles.glyphs`-rivit, tilejoukot, paikkahaku, RogueScriptin joukkofunktiot), indeksi voi olla väli (`WATER.1..2`) tai avoin väli (`WATER.4..`). Väli ei kelpaa siellä, missä tile tuotetaan (legend, kerroksen `fill`). Paikkojen jako kuvataan luvussa [7.4](#74-hakeva-ja-tuottava-paikka).
- **Jokeri ei kata indeksejä.** `GLYPH.NKI.*` vastaa ryhmän `GLYPH.NKI` nimettyjä jäseniä, ei tilen `GLYPH.NKI` indeksejä. Indeksijoukko kirjoitetaan välinä (`GLYPH.NKI.1..20`), jolloin joukko ei riipu siitä, mitä indeksejä tyylitiedosto luettelee.
- **Käyttö:** indeksi kuvaa lukuarvoista tilaa, esimerkiksi nesteen syvyyttä tai kaivettavan maan paksuutta, tai numeroitua muunnelmasarjaa.

**Haku.** Kun tilelle haetaan riviä (tilen ominaisuus, myös ulkoasu), haku etenee tarkimmasta nimestä perusnimeen ja lopuksi `default`-riville. Välirivit ovat porras tarkan nimen ja perusnimen välissä:

```
LAVA.crust.2 → LAVA.crust.1..3 → LAVA.crust → LAVA → default
```

Jos useampi välirivi sisältää indeksin, kapein voittaa, ja tasatilanteessa myöhempi rivi.

## 7.2 Maastokerrokset: terrain

Ilman kerroksia ruudulla on yksi tile. Maastokerrokset jakavat ruudun nimettyihin kerroksiin, joten peite (vesi, laava, lumi, veri) voi olla maaston päällä ilman, että sen alla oleva maasto katoaa.

```
terrain
- BASE fill BEDROCK
- GROUND default
- COVER
- DECAL pass
```

Rivi on `- NIMI [default] [pass] [fill TILE]`. Rivit ovat järjestyksessä alhaalta ylös.

- **Kerros on yksi tile.** Jokaisessa kerroksessa on enintään yksi tile tai `none`.
- **Nimet** ovat pelin nimiä ja kirjoitetaan isoilla (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). Kerroksen nimi ei kuulu nimiavaruuteen (luku [4.3](04-nimiavaruudet.md#43-nimiavaruuteen-kuuluvat-nimet)). Kerrokset ovat koko pelin yhteinen rakenne.
- `default` on kerros, johon tile menee, kun kerrosta ei nimetä (esimerkiksi legendin ensimmäinen tile). Jos `default` puuttuu, oletus on alin kerros. Useampi `default` on latausvirhe.
- `pass` tekee kerroksesta läpikuultavan: se piirretään, mutta ruudun maasto haetaan sen alta. Verijälki, roskat ja lehdet ovat tyypillisiä `pass`-kerroksia. `pass`-kerros ei myöskään estä liikettä eikä näköä (luku [7.5](#75-tilen-ominaisuudet-tiles)).
- `fill TILE` on kerroksen alkuarvo jokaisessa ruudussa, kun kartta luodaan. Ilman sitä kerros on tyhjä. Näin kartan ei tarvitse kertoa peruskalliota jokaiselle ruudulle.
- **Yksi `terrain`-osio:** osio on pelin laajuinen, ja toinen osio on latausvirhe. Sitä ei myöskään voi paikata (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)). Kirjasto, joka tarvitsee tietyn kerroksen, kertoo sen dokumentaatiossaan, ja peli julistaa sen (lataus varoittaa, jos kirjaston käyttämää kerrosta ei ole).
- **Ilman osiota** peli saa yhden kerroksen, joka on sekä `default` että ainoa. Kerroksen nimeä ei tarvita missään.

Kerrosten kirjoittaminen karttaan kuvataan luvussa [8.3](08-kartat-ja-prefabit.md#83-kerrokset-legendissä) ja piirtäminen luvussa [9.4](09-ulkoasu.md#94-maastokerrosten-piirtäminen).

## 7.3 Tilejoukot: tileset

Tilejoukko on nimetty luettelo tilejä, johon voi viitata yhdellä nimellä:

```
tileset FLOODABLE
- FLOOR
- GRASS
- STAIR
- SOIL.1..

tileset WET
- WATER
- MUD
```

- **Rivi** on tile, tilen väli (`SOIL.1..`, luku [7.1](#71-tilen-nimi-muunnelma-ja-indeksi)) tai toinen tilejoukko. Sisäkkäinen joukko yhdistyy osaksi joukkoa, ja kehä (A sisältää B:n, joka sisältää A:n) on latausvirhe.
- **Jäsenyys:** perusnimi kattaa muunnelmansa ja indeksinsä, joten `FLOOR` kattaa myös tilen `FLOOR.moss`, ja `WATER` kattaa jokaisen indeksin ja lähteen `WATER.source`.
- **Nimi** on yleinen nimi, ja se kuuluu nimiavaruuteen kuten tilet (luku [4.3](04-nimiavaruudet.md#43-nimiavaruuteen-kuuluvat-nimet)). Joukko ja tile eivät voi olla samannimisiä.
- **Paikkaaminen:** `patch tileset` lisää jäseniä ja `drop` poistaa niitä (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- **Vain hakevaan paikkaan:** joukko kelpaa sinne, missä tileä verrataan tai haetaan (luku [7.4](#74-hakeva-ja-tuottava-paikka)). Joukosta ei voi tuottaa yhtä tileä, joten joukko legendissä, kerroksen `fill`-arvona, pakan korttina tai taulun rivinä on latausvirhe.
- **Ei ominaisuuksissa:** `tiles`- ja `tiles.glyphs`-osiot eivät hyväksy joukkoja, koska tilen ulkoasu ja ominaisuudet kuuluvat tilelle itselleen.

## 7.4 Hakeva ja tuottava paikka

Tilen nimi esiintyy kahdenlaisessa paikassa:

| Paikka | Missä | Hyväksyy |
| --- | --- | --- |
| tuottava | legendin tile ja kerrosparit, kerroksen `fill`, pakan kortti, taulun rivi | yksi tile (muunnelma ja indeksi sallittu) |
| hakeva | tilejoukon rivi, paikkahaku (luku [8.8](08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)), RogueScriptin vertailut ja haut | tile, tilen väli tai tilejoukko |
| ominaisuuden rivi | `tiles`-osio (luku [7.5](#75-tilen-ominaisuudet-tiles)), `tiles.glyphs`-osio (luku [9.2](09-ulkoasu.md#92-tilejen-ulkoasu-tilesglyphs)) | tile, tilen väli tai `default` |

- Hakevassa paikassa tile ja tilen väli ovat nimettömiä tilejoukkoja. `at STAIR.up` ja `at SHRINES` käsitellään samalla tavalla, ja luvun [7.3](#73-tilejoukot-tileset) jäsenyyssääntö kattaa molemmat: `STAIR` vastaa myös tileä `STAIR.up`, ja `WATER.1..2` vastaa indeksejä 1 ja 2. Hakevan paikan ei siksi tarvitse erottaa tileä joukosta.
- `tiles`- ja `tiles.glyphs`-osiot ovat hakevia paikkoja, mutta ne eivät hyväksy nimettyjä joukkoja (luku [7.3](#73-tilejoukot-tileset)), ja niiden rivit valitaan tarkkuusjärjestyksessä eikä jäsenyydellä (luku [7.1](#71-tilen-nimi-muunnelma-ja-indeksi)).
- **Kerrokset haussa.** Ruutu vastaa hakujoukkoa, jos minkä tahansa sen maastokerroksen tile on joukon jäsen, myös `pass`-kerroksen. Näin `at WATER.source` löytää lähteen, vaikka se on `COVER`-kerroksessa.

## 7.5 Tilen ominaisuudet: tiles

`tiles`-osio antaa tileille ominaisuuksia, jotka moottori lukee. Osion jokainen rivi määrittelee yhden tilen:

```
tiles
- WALL blocks move sight
- DOOR blocks move sight
- DOOR.open blocks none
- DOOR.ajar
- WINDOW blocks move
- CHASM blocks WALK
- WATER.3.. blocks WALK
- FLOOR lit
- FLOOR.dark forget
- default blocks none
```

Rivi on `- TILE [OMINAISUUS arvot ...]`. Ominaisuudet ovat sanastoa ja kirjoitetaan pienillä. Ominaisuus voi saada useita arvoja (`blocks move sight`), ja arvoton ominaisuus (`lit`, `forget`) on lippu.

`TILE` on tile, muunnelma, indeksi, tilen väli (`WATER.3..`) tai `default`. Tilejoukko ei kelpaa, koska ominaisuus kuuluu tilelle itselleen (luku [7.3](#73-tilejoukot-tileset)).

`blocks` kertoo, mitä tile estää:

- `move` estää kaiken liikkeen.
- `sight` estää näön ja valon (moottorin komponentit `sight` ja `light`, luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)).
- Iso nimi on liikkumistapa (`movement mode`, luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)), joten `blocks WALK` estää kävelyn mutta ei esimerkiksi lentoa.
- `none` ei estä mitään, eikä sitä kirjoiteta muiden arvojen kanssa samalle riville.

**Valaistus ja muisti.** `lit` tekee tilestä valaistun: ruutu ja sen kahdeksan naapuria näkyvät koko näkösäteellä (`sight radius`), joten valaistun huoneen lattia näyttää huoneen seinät ja ovet mutta ei oven takaista käytävää. Ruutu, joka ei ole valaistu, näkyy vain `sight`-kentän `dark`-säteellä (luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)). `forget` tekee tilestä unohtuvan: muistava konsolin kerros (`remember`, luku [9.5](09-ulkoasu.md#95-konsolin-kerrokset)) unohtaa ruudun, kun se poistuu näkökentästä, joten pimeän huoneen lattia häviää näkyvistä pelaajan siirtyessä. Molemmat haetaan portaalla kuten muut ominaisuudet, ja `lit` riittää yhdessä ruudun kerroksessa. Säännöt kuvataan moottoridokumentin luvussa [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen).

Muut säännöt:

- **Haku.** Ominaisuus haetaan portaalla: tarkin nimi, välirivit, perusnimi ja lopuksi `default` (luku [7.1](#71-tilen-nimi-muunnelma-ja-indeksi)). Jokainen ominaisuus haetaan erikseen, joten rivi antaa vain ne ominaisuudet, jotka siinä mainitaan, ja muut ominaisuudet haetaan portaalta.
- **Muunnelmat.** `DOOR.open` perisi oven eston, ellei sen rivi antaisi omaa `blocks`-arvoa. `none` kumoaa perityn eston. Rivi ilman ominaisuuksia (`- DOOR.ajar`) julistaa tilen, joka perii kaiken. Kun logiikka vaihtaa ruudun tilen toiseen muunnelmaan, esto vaihtuu tilen mukana. Lippu periytyy samoin: `FLOOR.dark` on valaistu, ellei sen rivi kumoa lippua; yllä se on unohtuva, ja koska `lit` on annettu vain tilelle `FLOOR`, `FLOOR.dark` perii sen. Pimeän muunnelman rivi kirjoitetaan siksi `- FLOOR.dark lit false forget`.
- **Oletus.** Jos portaalta ei löydy arvoa, tile ei estä mitään, ei ole valaistu eikä unohdu. Peli, jossa tuntematon tile on seinä, kirjoittaa `- default blocks move sight`. Peli, jossa koko kartta on valaistu, kirjoittaa `- default lit`.
- **Kerrokset.** Ruutu estää liikkumistavan tai näön, jos jonkin sen maastokerroksen tile estää sen. `pass`-kerros (luku [7.2](#72-maastokerrokset-terrain)) ei osallistu estoon, koska ruudun maasto haetaan sen alta. Tyhjä kerros ei estä.
- **Entiteetit eivät ole tilejä.** Olennon tai esineen esto on moottorin komponentti `blocks` samoilla arvoilla (luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)).
- **Ulkoasu** on ominaisuus `glyphs`, jonka arvo riippuu esitystavasta. Se annetaan tyylitiedoston osiossa `tiles.glyphs ESITYSTAPA` (luku [9.2](09-ulkoasu.md#92-tilejen-ulkoasu-tilesglyphs)) eikä `tiles`-rivillä.
- **Jäsennys.** Ominaisuuksien nimet ja niiden pienet arvot ovat kiinteää sanastoa, eivätkä ne ole päällekkäisiä. Jäsennin tietää siksi, mistä seuraava ominaisuus alkaa.
- **Monta osiota.** `tiles`-osioita voi olla useita, ja niiden rivit yhdistyvät kuten `constants`-osiossa (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)). Saman tilen myöhempi rivi korvaa aiemman koko rivin. Korvaus toisesta tiedostosta merkitään rivin alkuun: `- override DOOR blocks none` (luku [4.5](04-nimiavaruudet.md#45-korvaaminen-ja-override)). Nimiavaruudessa rivin tile on suhteellinen kuten muutkin nimet (luku [4.2](04-nimiavaruudet.md#42-määrittely)).
- **Kirjoitusvirhe.** Jos rivin tileä ei käytetä legendissä, tyylissä eikä pelilogiikassa, lataus antaa varoituksen. Näin kirjoitusvirhe ei luo hiljaa uutta tileä.
- **Esto ja moottori.** Moottori käyttää estoa näkökentässä, valossa ja reitinhaussa. Moottorin `move` ei tarkista estoa, koska moottori ei tunne kulkukelpoisuutta: liikkeen ehdot ja törmäys (`STD.BUMP`) kirjoitetaan liikekomentoon, ja vakiokirjasto tekee tämän oletuksena (moottoridokumentti, luvut [9.7](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#97-raja-vakiokirjastoon) ja [10](../moottori/10-mita-moottori-ei-tee.md)). Näkö, valo ja reitinhaku käyttävät tilejen ja entiteettien estoa samalla kerrossäännöllä, paitsi `through`-joukon tilejä (luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)), ja logiikka lukee eston kyselyllä `blocks` (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).

**Käänteinen esto: `only`.** `only` luettelee liikkumistavat, jotka saavat liikkua tilellä, ja estää kaikki muut: `- WATER only RAFT SHIP HOVER`. Ominaisuus koskee vain liikkumistapoja, joten se ei estä näköä, ääntä eikä ammuksen tapaa `MISSILE`. Samalla rivillä `blocks` ja `only` estävät molemmat, ja `only` haetaan portaalla kuten `blocks`.

---

← [6. Pakat ja taulut](06-pakat-ja-taulut.md) · [Sisällys](README.md) · [8. Kartat ja prefabit](08-kartat-ja-prefabit.md) →
