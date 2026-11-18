[RLML](../README.md) › [RogueScript](README.md)

# 2. Yleinen syntaksi

## 2.1 Tiedostot

RogueScript kirjoitetaan kahdella tavalla:

- **Upotettuna** RLML-datatiedoston (`.rlml`, `.md` tai `.rlml.md`) markdown-koodilohkoon, jonka kielitunniste on `roguescript`. Koodilohkojen ulkopuolella pätevät dataformaattien säännöt, joten data ja logiikka voivat olla samassa tiedostossa.
- **Omana tiedostonaan**, jonka pääte on `.rogue`. Tiedosto on kokonaan RogueScriptiä ilman koodilohkoja.

Tiedosto otetaan mukaan päätteen kanssa kuten datatiedosto (`include logic/combat.rogue`). Tiedoston `namespace`-rivi koskee myös sen koodilohkoja.

## 2.2 Lohkot ja kommentit

Lohko alkaa kaksoispisteeseen päättyvällä rivillä, ja sen sisältö sisennetään neljällä välilyönnillä. Sääntö koskee kaikkia otsikoita: `component`, `fn`, `query`, `command`, `ai`, `generator`, `scene`, käsittelijät ja ohjausrakenteet. Sarkain on latausvirhe.

Yksirivinen määrittely ilman lohkoa ei pääty kaksoispisteeseen: `component HP(now: number)`, `event`, `command MOVE.NORTH = MOVE(north)` ja `map … generate`.

Kommentit alkavat merkeillä `//` ja jatkuvat rivin loppuun. Datan `>`-kommentti ei käy, koska `>` on vertailuoperaattori.

```roguescript
// Myrkky: tila, joka poistaa itsensä
component POISONED(turns: number = 5, damage: dice = 1d2):
    on turn():
        HURT(self, damage)
        turns -= 1
        if turns <= 0:
            remove(self, POISONED)
```

## 2.3 Nimet

- Isot nimet ovat kirjoittajan määrittelyjä kuten datassa (`HURT`, `POISONED`).
- Pienet nimet ovat moottorin sanoja tai paikallisia nimiä (`damage`, `self`, `turn`).
- Nimiavaruudet, `extends` ja `override` toimivat kuten datassa. Ne koskevat myös komponentteja ja vakioita (luku [3.3](03-komponentit.md#33-nimiavaruudet)).
  - **Nimeämiskäytäntö.** Komennot ovat verbejä (`ATTACK`, `MOVE`, `TAKE_EXIT`), komponentit substantiiveja (`MELEE`, `HP`, `EXIT`) ja tekoälyt toimijan nimiä (`HUNTER`, `GUARD`). Kaikki lajit jakavat täydet nimet, joten käytäntö estää törmäykset ennen kuin lataus joutuu ilmoittamaan niistä. Vakiokirjasto noudattaa sitä.

## 2.4 Varatut sanat ja paikalliset nimet

Paikallinen nimi on muuttuja, parametri tai silmukan muuttuja. Se kirjoitetaan pienillä, ja se ei saa olla varattu sana. Varatut sanat ovat suljettu luettelo:

- **avainsanat:** `component`, `event`, `on`, `before`, `after`, `as`, `priority`, `when`, `hide`, `disabled`, `fn`, `present`, `query`, `command`, `cost`, `from`, `aim`, `ai`, `generator`, `map`, `transient`, `generate`, `size`, `scene`, `timeline`, `background`, `realtime`, `track`, `loop`, `mark`, `always`, `if`, `elif`, `else`, `for`, `in`, `where`, `repeat`, `while`, `break`, `match`, `choice`, `emit`, `wait`, `confirm`, `open`, `return`, `cancel`, `has`, `no`, `is`, `and`, `or`, `not`, `namespace`, `override`, `extends`
- **kontekstinimet:** `self`, `holder`, `actor`, `it`, `player`, `players`, `game`, `map`, `camera`, `profile`
- **arvot:** `true`, `false`, `none`, `all`, suunnat (`north` …) ja suhteelliset suunnat (`front` …)

Säännöt:

- Varattu sana paikallisena nimenä on latausvirhe. Poikkeus on moottorin tapahtuman parametri, joka nimeää saman asian kuin kontekstinimi (`on arrive(map, p)`).
- Muut moottorin nimet eivät ole varattuja. Paikallinen nimi saa olla sama kuin moottorin funktio tai kenttä, koska funktio kutsutaan suluilla ja kenttä haetaan polusta.
- Tyyppien nimet ovat omassa nimiavaruudessaan, joten `dir: dir`, `deck: deck`, `table: table` ja `list: list name` ovat sallittuja.
- Nimetyn argumentin nimi saa olla varattu sana (`in:`, `size:`), koska kaksoispiste erottaa sen.
- **Kontekstisanat.** `map`, `ai` ja `from` ovat avainsanoja vain yhdessä paikassa ja muualla nimiä. `map` ja `ai` aloittavat määrittelyn (`map X generate`, `ai HUNTER:`), ja `from` on parametrin lähde (`potion: entity from actor.PACK.items`). Lausekkeessa `map` on kontekstinimi, `ai` moottorin komponentti (`self.ai.behavior`, `has ai`) ja `from(…)` moottorin funktio. Paikallisena nimenä ne ovat silti latausvirheitä. Parametrin lähde `count` (luku [7.1](07-komennot-ja-tekoaly.md#71-komennot)) on avainsana vain lähteen paikalla, joten se kelpaa myös paikalliseksi nimeksi ja kentäksi (`.count`, `ROOMS(count: 6..10)`).

## 2.5 Parametrit ja argumentit

- Parametrit ovat suluissa, ja niillä on tyyppi ja valinnainen oletus: `(target: entity, amount: number = 1)`.
- Argumentit annetaan paikan tai nimen mukaan: `HURT(goblin, 5)` tai `HURT(goblin, amount: 5)`. Nimetyt argumentit tulevat paikallisten jälkeen.
- Tekstin parametrit annetaan isoilla nimillä (`TARGET: target`) ja moottorin parametrit pienillä (`speaker: kitten`).

## 2.6 Tyypit

| Tyyppi | Arvo |
| --- | --- |
| `number`, `percent`, `dice`, `dir`, `side`, `name`, `entity` | kuten datassa |
| `range` | kokonaislukuväli `a..b`, esimerkiksi `4..9`; avoin väli `a..` vain hakevassa käytössä (alla) |
| `text` | vapaa merkkijono (alla) |
| `card` | kortti: kohteen nimi, määrä ja kenttien ylikirjoitukset (luvut [2.8](#28-ison-nimen-kutsu) ja [5.5](05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)) |
| `collection` | entiteettien kokoelma kuten datassa |
| `collection card`, `collection name` | korttien tai nimien joukko (alla) |
| `list T` | järjestetty, indeksoitava arvojono (alla) |
| `dict AVAIN ARVO` | sanakirja (alla) |
| `deck` | pakka: jaettu pakka nimellä tai kopio (luku [5.5](05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)) |
| `deck copy` | komponentin kenttä, johon ilmentymä saa oman kopion pakasta (luku [5.5](05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)) |
| `table` | taulu nimellä (luku [5.5](05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)) |
| `pos` | kartan ruutu `pos(5, 3)` |
| `bool` | `true` tai `false` |
| `area` | ruutujoukko (luku [6](06-ruudut-ja-paikkahaku.md)) |
| `command` | komento arvona (luvut [2.8](#28-ison-nimen-kutsu) ja [7.2](07-komennot-ja-tekoaly.md#72-tekoäly)) |
| `scene` | kohtaus arvona (luvut [2.8](#28-ison-nimen-kutsu) ja [9](09-kohtaukset-ja-aikajanat.md)) |
| `component` | komponentti arvona, esimerkiksi `add`-kutsulle (luku [2.8](#28-ison-nimen-kutsu)) |
| `handle` | taustakohtauksen kahva; kuolee, kun kohtaus päättyy (luku [9.9](09-kohtaukset-ja-aikajanat.md#99-taustakohtaukset)) |
| `camera` | kamera, kontekstinimi `camera` |
| `light` | alueen valot, `light(…)` (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)) |

**Kokoelman jäsenlaji.** Kokoelma voi sisältää entiteettejä, kortteja tai nimiä. Laji kirjoitetaan tyypin perään kuten `deck copy`, ja pelkkä `collection` on entiteettejä. Lataus päättelee lajin paikallisille nimille, joten se kirjoitetaan vain parametrin ja paluuarvon tyyppiin.

| Laji | Tuottajat |
| --- | --- |
| `collection` (entiteetit) | `entities()`, kokoelmakentät, `spawn`, `scatter` |
| `collection card` (kortit) | `draw`, `peek` |
| `collection name` (nimet) | jokeri (`ITEM.*`, myös tekstiavaimet `TIP.*`), tilen indeksiväli (`GLYPH.NKI.1..20`) |

`where`, `shuffled` ja `pick` säilyttävät lajin, ja `choose`, `.first` ja `.last` palauttavat jäsenen tyypin. Siksi `for card in pakka.draw(3)` antaa `card`-arvon, ja lataus valitsee `spawn(card, …)`-kutsulle oikean version (moottoridokumentti, luku [9.2](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#92-funktion-kuvaus)). Datan kokoelmakentät sisältävät aina entiteettejä. Muut arvojoukot, kuten alueet ja luvut, ovat listoja (alla).

**Jokerin laji.** Jokeri vastaa ryhmän kaikkia täysiä nimiä lajista riippumatta: entiteettejä, karttoja, tilejä, tekstiavaimia ja muita (dataformaatit, luku [2.1](../dataformaatit/02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). Jos jäsenet ovat eri lajeja eikä käyttöpaikka rajaa lajia (esimerkiksi `say` hyväksyy vain tekstiavaimia), jokeri on latausvirhe, joka luettelee lajit. Tekstiavainten jokeri vastaa kaikkien ladattujen kielitiedostojen avainten yhdistettä, joten sama pelikerta saa saman joukon kielestä riippumatta, ja valitusta kielestä puuttuva avain näkyy avaimena. Esimerkiksi `LORE.*.pick(20)` arpoo 20 eri kuvausavainta, jotka voi tallentaa `name`-kenttään ja näyttää viestin paikkamerkillä.

**Lista.** `list T` on järjestetty jono samantyyppisiä arvoja. Jäsenen tyyppi `T` on mikä tahansa arvotyyppi paitsi lista, sanakirja, pakka tai kokoelma: luku, nimi, teksti, `pos`, `area`, entiteettikahva tai komponenttiarvo (`list SCORE`). Ero kokoelmaan on se, että kokoelmakenttä on entiteettien *sijainti*, ja lista on *arvo*, jonka voi tallentaa kenttään, antaa parametrina ja palauttaa.

- **Kirjoitusasu.** `[]` on tyhjä lista ja `[a, b]` lista annetuista jäsenistä. Jäsenen tyyppi päätellään jäsenistä ja lisäyksistä, ja eri tyyppien sekoitus on latausvirhe. Tyhjä lista saa tyypin käyttöpaikastaan.
- **Operaatiot.** `l[i]` lukee jäsenen nollasta alkaen, ja alueen ulkopuolinen indeksi antaa `none`. `l[i] = v` kirjoittaa olemassa olevaan paikkaan, ja alueen ulkopuolinen kirjoitus on käyttöhetken virhe. `l += v` lisää loppuun, `insert(l, i, v)` paikkaan `i`, `remove_at(l, i)` poistaa paikasta `i`, ja `index_of(l, v)` palauttaa ensimmäisen paikan tai `none`. `.count`, `.first`, `.last`, `for`, `in` ja joukkofunktiot (`choose`, `pick`, `shuffled`, `where`) toimivat kuten kokoelmalla.
- **Arvo.** Sijoitus ja parametri kopioivat listan, joten kentän listaa muutetaan kentän kautta: `game.BESTIARY.order += MONSTER.TROLL`.
- **Entiteettijoukkona.** Lista entiteettikahvoista (`list entity`) käy `collection`-parametrin ja -paluuarvon paikalle, koska kyselyn tulosjoukko on kahvojen joukko eikä sijainti. Entiteetti siirretään kokoelmakenttään vain `spawn`- tai `move`-kutsulla.
- **Kenttänä.** Komponentin kenttä `list T [N] [sort [KENTTÄ] [desc]]` pitää listan enintään `N` jäsenen mittaisena ja järjestyksessä. `sort` järjestää lisäyksen jälkeen jäsenet arvon mukaan tai komponenttiarvon kentän mukaan, ja täysi lista pudottaa järjestyksessä viimeisen. Ilman `sort`-sanaa täysi lista pudottaa vanhimman. Lista kuuluu pelin tilaan ja tallennukseen (dataformaatit, luku [5.1](../dataformaatit/05-komponentit-ja-entiteetit.md#51-komponentit)).

**Väli.** `range` on kokonaislukuväli, päät mukaan lukien. Toisin kuin noppa, väli ei heitä itseään luettaessa: se pysyy arvona, kunnes `choose` arpoo siitä luvun (`want = choose(count)`, luku [8.3](08-generointi.md#83-vakiokirjaston-generointi)). Luku ei muutu väliksi, koska `near` tulkitsee luvun N välinä 0..N, joten `range`-parametri vaatii välin (`ROOMS(count: 6..6)`). Avoimen välin `a..` saa käyttää vain hakevasti, kuten datan tilevälit (dataformaatit, luku [7.4](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#74-hakeva-ja-tuottava-paikka)):

- **Hakeva käyttö** sallii avoimen välin: `in` (`d in 5..`), `match`-kohta (`10..:`), `near`-etäisyys (`near(self, 2..)`) ja tilen indeksi haussa (`WATER.4..`).
- **Tuottava käyttö** vaatii molemmat päät: `choose`, `for`, `pick` ja `shuffled`. Avoin väli niissä on latausvirhe, kun väli on vakio, ja muuten käyttöhetken virhe.
- **Tyhjä väli.** Väli, jonka yläraja on alarajaa pienempi (`0..-1`), on tyhjä joukko: `for` ei kierrä, `pick` ja `shuffled` palauttavat tyhjän, `choose` palauttaa `none`, ja `in` on aina epätosi. Väli `0..n - 1` toimii siksi myös, kun `n` on 0.

**Ruudut kuuluvat kartalle.** `pos`- ja `area`-arvo kantaa karttansa, vaikka kirjoitusasu ei näytä sitä. `pos(5, 3)` ja ruutujoukkofunktiot tuottavat arvon kontekstin kartalle (`map`), jonka moottoridokumentin luku [6.2](../moottori/06-ajo.md#62-tapahtumat) määrittelee kullekin kontekstille. Eri karttojen ruudut eivät ole koskaan yhtä suuret, ja `move` tai `spawn` toisen kartan ruutuun on käyttöhetken virhe: karttojen välillä siirrytään `travel`-kutsulla. Poikkeus on atlaksen lähialue: hakujen tulos voi sisältää naapurilehden ruutuja, ja `move` naapurilehden ruutuun on reunan ylitys (moottoridokumentti, luku [6.10](../moottori/06-ajo.md#610-atlas)). Komponentin kenttänä tai kartan muuttujana tallennettu `area` (dataformaatit, luvut [5.1](../dataformaatit/05-komponentit-ja-entiteetit.md#51-komponentit) ja [8.4](../dataformaatit/08-kartat-ja-prefabit.md#84-kartan-muuttujat-vars)) säilyttää karttansa.

Datan sijaintia `5,3` ei kirjoiteta RogueScriptissä, koska pilkku erottaa argumentit.

**Sanakirja.** `dict AVAIN ARVO` on avain–arvo-taulu. Avain on `entity`, `name` tai `number`, ja arvo mikä tahansa tyyppi paitsi kokoelma, pakka tai sanakirja. `{}` on tyhjä sanakirja. `d[k]` lukee arvon, ja puuttuva avain antaa `none` ilman varoitusta. `d[k] = v`, `+=` ja `-=` kirjoittavat, `k in d` testaa avaimen, `remove(d, k)` poistaa sen, `.count` on koko, `d.keys` on avainten lista lisäysjärjestyksessä, ja `for k in d` käy avaimet samassa järjestyksessä. Kuollut kahva pysyy avaimena, kunnes se poistetaan. Komponentin kenttänä sanakirja kuuluu pelin tilaan ja tallennukseen (dataformaatit, luku [5.1](../dataformaatit/05-komponentit-ja-entiteetit.md#51-komponentit)).

**Nimiö.** `label` on kartan paikallinen nimi (dataformaatit, luvut [5.1](../dataformaatit/05-komponentit-ja-entiteetit.md#51-komponentit) ja [8.5](../dataformaatit/08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)). Lataus ei ratkaise sitä yleisellä nimenhaulla, vaan tarkistaa sen kohdekartasta, kun kartta on tiedossa.

**Ääni.** `sound` on kuultu ääni (moottoridokumentti, luku [6.8](../moottori/06-ajo.md#68-äänet-ja-kuuleminen)). Sen kentät ovat vain luettavia: `.kind` on äänilaji, `.at` lähteen ruutu, `.source` lähde-entiteetti tai `none` ja `.level` kuultu taso. Tekoäly voi siis mennä ääntä kohti näkemättä lähdettä: `heard(self).first.at`.

**Teksti.** `text` on vapaa merkkijono, jonka tuottavat merkkijonoliteraali (`"Iolo"`), `wait text`, `text ask` ja `text_of`. Sitä ei käännetä, ja se on tarkoitettu näyttöön: moottorin komponentti `name` (moottoridokumentti, luku [3.4](../moottori/03-pelin-tila.md#34-moottorin-komponentit)), tekstin paikkamerkki ja nimen kysely (luku [9.6](09-kohtaukset-ja-aikajanat.md#96-say-ja-log)). Vertailu `==` toimii, `.count` on pituus koodipisteinä, ja `a + b` yhdistää kaksi tekstiä.

- `text_of(AVAIN, PARAM: …) -> text` palauttaa avaimen tekstin valitulla kielellä paikkamerkit täytettyinä. Tulos riippuu kielestä, joten funktio on sallittu vain kyselyissä (`query`), esityksessä (`present fn`) sekä paneelin ja listavalikon lausekkeissa. Tilaa muuttavassa logiikassa se on latausvirhe, koska tilaan tallennettu teksti tekisi tallennuksesta kielisidonnaisen.
- `today()` palauttaa koneen kalenterin päivän (kentät `year`, `month` ja `day`). Se on esitystä, joten se on sallittu esityksessä (`present fn`) ja paneelin kentissä, esimerkiksi hautakiven vuosiluvussa, mutta ei kyselyssä eikä tilaa muuttavassa logiikassa (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).

**Kokoelman indeksi.** `kokoelma[i]` lukee jäsenen paikasta `i` nollasta alkaen kokoelman järjestyksessä, ja alueen ulkopuolinen indeksi antaa `none`. Indeksi on vain luettava, ja se toimii kaikille jäsenlajeille. Jokerin tuottamat nimet ovat luonnollisessa järjestyksessä, jossa nimen numero-osat verrataan lukuina, joten `DUNGEON.*[depth - 1]` valitsee tason kartan (`L2` ennen `L10`).

**Pinot.** Kokoelmassa, jolla on `merge MERKKI` (dataformaatit, luku [5.4](../dataformaatit/05-komponentit-ja-entiteetit.md#54-kokoelmat)), täysin samanlaiset jäsenet ovat pino. Jokainen kappale on oma ilmentymänsä, mutta pinon jäsenet jakavat paikan ja kirjaimen ja näkyvät listassa yhtenä rivinä. Kappaleen juonti on `destroy(e)`, heitto `move(e, to: …)` ja koko pinon siirto `for x in e.stack: move(x, to: …)`. Kun yhden kappaleen kenttä muuttuu, se ei ole enää samanlainen ja erkanee omaksi pinokseen.

**Entiteetin moottorikentät.** Moottori ylläpitää entiteetille vain luettavia kenttiä, jotka luetaan polulla kuten komponentin kentät:

| Kenttä | Arvo |
| --- | --- |
| `e.at` | ruutu, jossa `e` on; kokoelmassa `none` |
| `e.map` | kartta, jolla `e` on (moottoridokumentti, luku [6.2](../moottori/06-ajo.md#62-tapahtumat)) |
| `e.kind` | `name`: määrittely, josta ilmentymä luotiin (lapsi eikä vanhempi); säilyy kuolleella kahvalla |
| `e.slot` | `number`: paikka kokoelmassa nollasta, pinon jäsenillä sama; kartalla `none` |
| `e.stack` | `collection`: `e` ja sen kanssa täysin samanlaiset jäsenet samassa `merge`-kokoelmassa; muualla pelkkä `e` |

`e.kind` on tunnistuksen, kuolinsyyn ja pistelistan avain: `game.IDENT.known[item.kind]`. `e.stack.count` on pinon koko.

## 2.7 Ketjukutsu

Moottorin funktion voi kutsua ensimmäisen argumenttinsa perään kirjoitettuna: `x.f(a, b)` on sama kuin `f(x, a, b)`. Ketju luetaan vasemmalta oikealle, joten sisäkkäisiä sulkuja ei tarvita:

```roguescript
for card in DECOYS.copy().draw(18):
    scatter(card, count: 1, in: tiles() - near(home, 1))

hand = LOOT.RARE.copy()
hand.shuffle()
```

- **Sulut tekevät kutsun.** Pieni nimi pisteen jälkeen on kenttä tai moottorin komponentti (`deck.count`, `self.render.tile`), ja sulut tekevät siitä kutsun (`deck.draw()`). Tilaa muuttava kutsu näkyy siksi aina kutsuna, myös ilman argumentteja.
- **Vain moottorin funktiot.** Kirjoittajan funktiot (isot nimet) kutsutaan funktiomuodossa, koska polun iso osa on komponentti tai nimiavaruus (luku [3.3](03-komponentit.md#33-nimiavaruudet)). `goblin.HURT(5)` näyttäisi komponentin `HURT` polulta.
- **Vastaanottaja** on funktion ensimmäinen argumentti, ja sen on käytävä ensimmäisen parametrin tyypiksi. Muut argumentit annetaan paikan tai nimen mukaan kuten ennenkin: `deck.put(card, bottom: true)`. Funktioiden parametrit, paluutyypit ja vastaanottajan tyypit ovat moottoridokumentin luvussa [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot).
- **Ketju jatkuu.** Funktio, joka vain muuttaa vastaanottajaansa (`shuffle`, `put`, `add`, `reset`, `remove`), palauttaa vastaanottajan: `LOOT.RARE.copy().shuffle().draw(3)`.
- **Sama funktio.** Muoto ei muuta funktiota: funktion laji, satunnaisvirta ja tilan muutosten tarkistukset (luku [5](05-funktiot-ja-lausekkeet.md)) ovat samat kummassakin muodossa.
- **Sitovuus.** Ketjukutsu sitoo tiukemmin kuin operaattorit, väli ja `where`. Väli tai suodatettu joukko vastaanottajana tarvitsee siksi sulut: `(GLYPH.NKI.1..20).choose()` ja `(near(self, 1) where it in FLOOR).choose()`. Ilman sulkuja `GLYPH.NKI.1..20.choose()` olisi väli, jonka yläraja on `20.choose()`, mikä on latausvirhe. Jokeri käy vastaanottajaksi sellaisenaan: `ITEM.*.pick(18)`.
- **Kumpi muoto.** Ketjumuoto sopii, kun ensimmäinen argumentti on käsiteltävä asia (pakka, joukko, kokoelma). Funktiomuoto sopii, kun argumentit ovat tasavertaisia (`distance(a, b)`, `min(a, b)`), kun kutsu on efekti maailmaan (`spawn`, `move`, `log`) tai kun vastaanottaja tarvitsisi sulut. Molemmat muodot ovat aina sallittuja.

## 2.8 Ison nimen kutsu

`NIMI(…)` tekee sen, minkä nimen laji kertoo. Lataus ratkaisee lajin nimenhaun jälkeen:

| Laji | `NIMI(…)` | Tyyppi |
| --- | --- | --- |
| `fn`, `query`, `present fn` | suoritetaan | paluutyyppi |
| komento | arvo | `command` |
| kohtaus | arvo | `scene` |
| komponentti | arvo | `component` |
| entiteetti | kortti: nimetyt argumentit ovat kenttien ylikirjoituksia, paljas luku tai noppa on määrä | `card` |
| `ai` | suoritetaan vain toisen tekoälyn rungossa samalla `self`-arvolla (luku [7.2](07-komennot-ja-tekoaly.md#72-tekoäly)); muualla latausvirhe | `command` |
| tapahtuma | latausvirhe; tapahtuma lähetetään sanalla `emit` | – |

Arvo suoritetaan vain moottorin kautta: tekoälyn paluuarvona (luku [7.2](07-komennot-ja-tekoaly.md#72-tekoäly)), `start`-kutsulla, `wait scene` -odotuksella (luku [9](09-kohtaukset-ja-aikajanat.md)), `add`-kutsulla (luku [3.1](03-komponentit.md#31-määrittely)) tai kortin synnyttävällä funktiolla (`spawn`, `scatter`, `GIVE`, `put`). Muuten arvon voi sitoa nimeen, antaa argumenttina ja palauttaa kuten muutkin arvot.

**Kortti entiteetin nimestä.** `NKI(NOTE: key, render: GLYPH.NKI.7)` on kortti, jonka kohde on `NKI` ja jonka ylikirjoitukset ovat kuten datan kortissa (dataformaatit, luku [6.4](../dataformaatit/06-pakat-ja-taulut.md#64-kenttien-ylikirjoitus)): argumentin nimi on komponentti, ja yhden kentän komponentin arvo annetaan suoraan, moninkenttäisen komponenttiarvona (`HP: HP(now: 3)`). Moottorin komponentit nimetään pienillä (`render:`, `label:`). Paljas ensimmäinen argumentti on määrä: `RATION(3)` synnyttää kolme annosta ja `ARROW(2d4, ENCHANT: 1)` heittää määrän kerran. `spawn` soveltaa ylikirjoitukset syntymässä ennen `created`-tapahtumaa (moottoridokumentti, luku [3.2](../moottori/03-pelin-tila.md#32-ilmentymän-syntyminen)), joten käsittelijät näkevät ilmentymän valmiina. Tuntematon komponentti tai kenttä on latausvirhe.

```roguescript
thing = spawn(NKI(NOTE: key, render: looks[i]), at: free).first
LOOT.RARE.put(SWORD(ENCHANT: -3, CURSED: CURSED()))
```

**Moottorin komponentti arvona.** Moottorin komponentin pieni nimi kutsuna on samoin komponenttiarvo: `add(p, ai(behavior: FOLLOWER))` ja `add(actor, movement(mode: SHIP, speed: 10))`. Moottorin komponenttien kentät ovat myös kirjoitettavia (`actor.render.tile = GLYPH.SHIP`), ja muutos vaikuttaa seuraavasta laskennasta (moottoridokumentti, luku [3.4](../moottori/03-pelin-tila.md#34-moottorin-komponentit)).

`remove(e, POISONED)` saa paljaan nimen ilman sulkuja, ja lataus tarkistaa, että nimi on komponentti.

---

← [1. Rajaus ja periaatteet](01-rajaus-ja-periaatteet.md) · [Sisällys](README.md) · [3. Komponentit](03-komponentit.md) →
