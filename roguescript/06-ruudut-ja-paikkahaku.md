[RLML](../README.md) › [RogueScript](README.md)

# 6. Ruudut ja paikkahaku

`area` on kartan ruutujoukko. Funktiot tuottavat joukkoja, ja joukkoja yhdistetään operaattoreilla. Datan paikkahaku (dataformaatit, luku [8.8](../dataformaatit/08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)) on saman mallin rajattu muoto.

## 6.1 Joukon tuottavat funktiot

- **Ankkuri** on sijainti, entiteetti, ruutujoukko tai `label(…)`. Entiteetti tarkoittaa sen nykyistä ruutua, ja joukko on jäsentensä tulosten yhdiste. Kokoelmassa oleva entiteetti ei ole kartalla, joten se tuottaa tyhjän joukon.
- **Etäisyys** on luku tai väli. Luku N tarkoittaa väliä `0..N`, ja väli `2..4` antaa renkaan. Metriikka on Chebyshev.
- **Suunta** on absoluuttinen (`east`) tai suhteellinen (`front`). Suhteellinen suunta luetaan ankkurientiteetin `facing`-komponentista.
- Kartan ulkopuolelle osuvat ruudut putoavat joukosta pois. Atlaksen lehdellä haku ulottuu yhden lehtirajan yli naapurilehtiin, ja vasta lähialueen ulkopuolelle osuvat ruudut putoavat pois (moottoridokumentti, luku [6.10](../moottori/06-ajo.md#610-atlas)).
- **Kartta.** Funktiot käyttävät kontekstin `map`-karttaa, jonka moottoridokumentin luku [6.2](../moottori/06-ajo.md#62-tapahtumat) määrittelee kullekin kontekstille.

| Funktio | Ruudut |
| --- | --- |
| `tiles()` | koko kartta |
| `changed()` | ruudut, joita peli on muuttanut kartan latauksen jälkeen; `changed(layer: GROUND)` rajaa kerrokseen. Alkuperäinen tile luetaan kyselyllä `tile_original(p)`, joten `for p in changed(): tile(p, tile_original(p))` palauttaa kartan piirretyksi |
| `at(HAKU)` | ruudut, joiden jokin kerros on hakujoukon jäsen (tile, tilen väli tai tilejoukko) |
| `near(ANKKURI, ETÄISYYS)` | ruudut ankkurin lähellä |
| `from(ANKKURI, X, Y)` | ruutu siirtymän päässä ankkurista |
| `from(ANKKURI, ETÄISYYS, SUUNTA)` | ruutu annetussa suunnassa |
| `rect(KULMA, KOKO)` | suorakulmio, esimerkiksi `rect(pos(2, 2), 10x5)` |
| `rect(KULMA, LEVEYS, KORKEUS)` | suorakulmio luvuista, esimerkiksi `rect(p, choose(4..9), 5)` |
| `line(A, B)` | suora viiva |
| `path(A, B)` | moottorin reitinhaun reitti |
| `label(NIMIÖ)` | nimiöryhmän ruudut ja nimettyjen entiteettien ruudut |
| `edge(SUUNTA)` | kartan reunarivi tai -sarake (`north`, `east`, `south` tai `west`) lukujärjestyksessä (luku [6.4](#64-siirtyminen-kartalta-toiselle)) |
| `region(P, of: JOUKKO)` | `JOUKKO`-joukon yhtenäinen osa, johon `P` kuuluu; esimerkiksi pelaajan huone `region(player.at, of: at(FLOOR))` |

## 6.2 Operaatiot

- `a and b` on leikkaus, `a or b` yhdiste ja `a - b` erotus.
- `a where ehto` suodattaa, ja `it` on ruutu: `near(self, 1) where entities(in: it).count == 0` (luku [5.3](05-funktiot-ja-lausekkeet.md#53-lausekkeet)).
- `a.count` on koko, ja `p in a` testaa jäsenyyden.
- **Paljastus.** `reveal(a)` lisää joukon ruudut kartan muistiin, joka on kaikkien pelaajien yhteinen, kuin ne olisi nähty, esimerkiksi taikakartan lukemisessa `reveal(tiles())`. Se ei tuo ruutuja näkökenttään, joten `sees` ei muutu. Entiteetin voi näyttää näkökentän ulkopuolella moottorin komponentilla `revealed` (moottoridokumentti, luku [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen)).
- **Muunnos.** Sijainti, kartalla oleva entiteetti ja ruutujen lista (`list pos`) käyvät ruutujoukoksi; sijainti ja entiteetti tuottavat yhden ruudun joukon.
- **Yksi ruutu joukosta.** Kun funktio tarvitsee yhden ruudun (`move`, `spawn`, `tile`) ja saa joukon, se arpoo ruudun kutsujan satunnaisvirrasta.
- **Tyhjä joukko ei ole virhe.** Yhden ruudun tarvitseva funktio epäonnistuu silloin: `move` ja `tile` palauttavat `false`, `spawn` palauttaa `none` tai tyhjän kokoelman. Poikkeus on `travel`, jonka puuttuva kohdenimiö on virhe (luku [6.4](#64-siirtyminen-kartalta-toiselle)).
- **Entiteetit ruuduissa:** `entities(in: joukko)`, ja ilman argumenttia kaikki nykyisen kartan entiteetit.

## 6.3 Datan vastineet

| Data | RogueScript |
| --- | --- |
| `at FLOOR` | `at(FLOOR)` |
| `at FLOOR near START 3` | `at(FLOOR) and near(label(START), 3)` |
| `at START near 3` | `near(label(START), 3)` |
| `at 2,-1 from START` | `from(label(START), 2, -1)` |
| `at 2 front from GUARD` | `from(label(GUARD), 2, front)` |
| `at 5,3` | `pos(5, 3)` |

Erot dataan: ankkuri voi olla mikä tahansa entiteetti, myös `player`, ja tyhjä tulos on tyhjä joukko eikä virhe.

## 6.4 Siirtyminen kartalta toiselle

Nimiöt kertovat, mihin kohtaan kohdekartalla saavutaan (dataformaatit, luku [8.5](../dataformaatit/08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)). Siirtyminen on logiikkaa: käsittelijä päättää, milloin entiteetti siirtyy, ja moottori sijoittaa sen nimiön mukaan.

| Funktio | Tekee |
| --- | --- |
| `travel(entiteetti, KARTTA, at: NIMIÖ, align: bool = false)` | siirtää entiteetin kohdekartan nimiöön |
| `edge(SUUNTA)` | kartan reunarivi tai -sarake ruutujoukkona lukujärjestyksessä (luku [6.1](#61-joukon-tuottavat-funktiot)) |
| `tag(joukko, NIMIÖ)` | antaa joukon jokaiselle ruudulle nimiön |

```roguescript
generator FIELD(depth: number) size 40x20:
    carve(tiles(), GRASS)
    tag(edge(east), EAST)
    tag(edge(west), WEST)

map FIELD_A generate FIELD(depth: 1)
map FIELD_B generate FIELD(depth: 1)

on enter(who, p, prev) when seat_of(who) != none and p in label(EAST):
    travel(who, FIELD_B, at: WEST, align: true)

on enter(who, p, prev) when seat_of(who) != none and p in label(WEST):
    travel(who, FIELD_A, at: EAST, align: true)
```

- **Saapumisruutu arvotaan.** Oletuksena saapumisruutu arvotaan kohderyhmästä kutsujan virrasta.
- **Saapuminen samaan indeksiin.** `align: true` vie ruutuun, jolla on kohderyhmässä sama indeksi kuin siirtyjän lähtöruudulla omassa nimiöryhmässään. Ruudulla on enintään yksi nimiö (dataformaatit, luku [8.5](../dataformaatit/08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)), joten lähtöryhmää ei tarvitse nimetä. Jos kohderyhmässä on vähemmän ruutuja, saavutaan viimeiseen. Näin saman kokoiset reunat yhdistyvät ilman koordinaatteja: `edge(east)` ja `edge(west)` ovat molemmat lukujärjestyksessä ylhäältä alas. Jos siirtyjä ei seiso nimiöryhmässä, ruutu arvotaan.
- **Puuttuva `at`** vie nimiöön `START`.
- **Puuttuva nimiö** kohdekartassa on virhe, ei tyhjä joukko (luku [6.2](#62-operaatiot)). Piirretylle kartalle lataus tarkistaa sen etukäteen, ja generoidulle kartalle se on käyttöhetken virhe.
- **Tapahtumat.** `travel` lähettää tapahtumat `leave(who, p)` ja `depart(who, map, p)` lähtökartalla ja `arrive(who, map, p)` kohdekartalla, mutta ei tapahtumaa `enter` saapumisruudussa. Pelaaja ei siksi pomppaa heti takaisin. Siirto tehdään toiminnon lopussa ennen jonossa olevia etualan kohtauksia (moottoridokumentti, luku [6.1.7](../moottori/06-ajo.md#617-toiminto)), joten `enter`-käsittelijä voi kutsua `travel`-funktiota, eivätkä saman tapahtuman muut käsittelijät näe siirtoa. Saman käsittelijän `start`-kutsu käynnistää kohtauksen, joka alkaa vasta siirron jälkeen kohdekartalla (luku [9](09-kohtaukset-ja-aikajanat.md)).
- **Sama kartta.** `travel` siirtyjän nykyiseen karttaan on sallittu. `transient`-kartta hylätään ja luodaan uudelleen, joten saman nimen kartta voi olla joka kerta uusi taso. Pysyvä kartta, sen kello ja tila säilyvät, ja siirtyjä vain siirtyy nimiöön. Tapahtumat ovat molemmissa samat kuin muussakin `travel`-kutsussa. Ero `move(e, to: label(X))`-kutsuun on saapuminen: `arrive` laukeaa, ja sen myötä esimerkiksi automaattinen tallennus `on arrive`.

```roguescript
command DESCEND() cost 120:
    if not actor.at in STAIRS:
        return false
    game.DEPTH.now += 1
    travel(actor, DUNGEON, at: ARRIVAL)          // DUNGEON on transient: uusi taso
```

- **Kartan nimi.** Kohdekartta kirjoitetaan pelkällä nimellä (`FIELD_B`). `map` on nykyisen kartan kontekstinimi, joten `map.FIELD_B` olisi nykyisen kartan kenttä.
- **`tag` ja `tile`.** `tile(paikka, …, label: NIMIÖ)` nimeää yhden ruudun ja arpoo sen, jos paikka on joukko (luku [6.2](#62-operaatiot)). `tag` nimeää joukon kaikki ruudut, joten ryhmä syntyy yhdellä kutsulla.

---

← [5. Funktiot ja lausekkeet](05-funktiot-ja-lausekkeet.md) · [Sisällys](README.md) · [7. Komennot ja tekoäly](07-komennot-ja-tekoaly.md) →
