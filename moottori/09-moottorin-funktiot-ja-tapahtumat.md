[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 9. Moottorin funktiot ja tapahtumat

Luku on moottorin funktioiden ja tapahtumien viite. Sama luettelo ohjaa latauksen tarkistuksia (luku [2.5](02-lataus.md#25-tarkistukset)), ohitusta ja reaaliaikaisia taustakohtauksia (luku [6.4](06-ajo.md#64-kohtaukset)) sekä ketjukutsun ratkaisua ([RS 2.7](../roguescript/02-yleinen-syntaksi.md#27-ketjukutsu)).

## 9.1 Sanasto

Kaikki moottorin nimet ovat pieniä.

| Laji | Nimet |
| --- | --- |
| komponentit | `render`, `movement`, `ai`, `sight`, `light`, `facing`, `timer`, `interior`, `shape`, `label`, `wire`, `blocks`, `hearing`, `sound`, `name`, `revealed`, `seat` |
| tapahtumat | `begin`, `turn`, `wake`, `created`, `removed`, `enter`, `leave`, `arrive`, `depart`, `hear`, `join`, `part` |
| toiminnot | `save`, `quit`, `pass`, `repeat`, `help`, `confirm`, `back`, `close`, `resume`, `skip`, `restart`, `rewind`, `new_game`, `load_game`, `exit`, `next`, `prev`, `cursor.<suunta>`, `toggle`, `travel_to`, `look`, `cursor`, `follow`, `verbs`, `focus`, `join`, `leave_seat`; toiminto `follow` on eri nimi kuin kameran funktio `follow(camera, e)`, koska toiminnot esiintyvät vain sidonnoissa ja valikon kohdissa |
| hiiren nimet | `click`, `rclick`, `mclick`, `wheel.up`, `wheel.down` (luku [8.1](08-syote.md#81-hiiri)) |
| asettelun lähteet | `map`, `hover`, `sentence`, `menu` (luku [7.1](07-esitys.md#71-asettelu)) |
| kontekstinimet | `self`, `holder`, `actor`, `it`, `player`, `players`, `game`, `profile`, `map`, `camera` |
| `game`-kentät | `start`, `at`, `seed`, `turn`, `time`, `call_depth`, `loop_steps`, `keys`, `title`, `layout`, `naming`, `masking`, `walk`, `walk_stop`, `walk_avoid`, `leader`, `rest`, `follow_distance`, `follow_ai`, `seats`, `party`, `leash` |
| entiteetin kentät | `at`, `map`, `kind`, `slot`, `stack` |
| tilen ominaisuudet | `blocks`, `only`, `lit`, `forget` ([D 7.5](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)) |
| valikon määreet | `list`, `final`, `verbs` ([D 11.2](../dataformaatit/11-kayttoliittyman-data.md#112-valikot)) |
| tyylin määreet | `series` ([D 9.2](../dataformaatit/09-ulkoasu.md#92-tilejen-ulkoasu-tilesglyphs)) |
| tila ja ilmentymät | `spawn`, `destroy`, `add`, `remove`, `move`, `tile`, `tag`, `entities`, `control`, `travel`, `follow`, `noise` (luku [6.8](06-ajo.md#68-äänet-ja-kuuleminen)) |
| ruutujoukot | `tiles`, `at`, `near`, `from`, `rect`, `line`, `path`, `label`, `edge`, `pos`, `region` |
| listat | `insert`, `remove_at`, `index_of` ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)) |
| satunnaisuus | `choose`, `shuffled`, `pick`, `chance` |
| pakat ja taulut | `draw`, `peek`, `put`, `add`, `reset`, `copy`, `shuffle`; taululle vain `draw` ja `spawn` |
| kyselyt | `min`, `max`, `distance`, `nearest`, `sees`, `blocks`, `tile_of`, `hears`, `heard`, `adjacent`, `ray`, `text_of`, `seats`, `seat_of`, `members`, `active`, `controlled`, `leashed`, `in_view`, `view_width`, `view_height` |
| generointi | `carve`, `footprint`, `stamp`, `external`, `scatter` |
| kohtaukset | `start`, `stop`, `say`, `log` |
| esitys | `play`, `pan`, `shake`, `fade`, `light`, `fly`, `reveal`, `today` |
| tallennus | `checkpoint`, `rollback`, `autosave`, `delete_save` |
| datan lohkot | `patch`, `new`, `drop` ([D 4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)) |

`give` ja `bump` eivät ole moottorin nimiä. Ne ovat vakiokirjaston `STD.GIVE` ja `STD.BUMP` (luku [9.7](#97-raja-vakiokirjastoon)).

## 9.2 Funktion kuvaus

Jokaisesta funktiosta kerrotaan samat asiat:

- **Parametrit** tyyppeineen ja oletuksineen. Ensimmäinen parametri on ketjukutsun vastaanottaja: `x.f(a)` on `f(x, a)` ([RS 2.7](../roguescript/02-yleinen-syntaksi.md#27-ketjukutsu)). Muun parametrin tyyppi voi olla vaihtoehtojen joukko (`n: number|dice|all`).
- **Paluu.** Funktio, joka vain muuttaa vastaanottajaansa, palauttaa vastaanottajan, jotta ketju jatkuu (`LOOT.RARE.copy().shuffle().draw(3)`). Epäonnistuminen on arvon palauttavalla funktiolla `none` ja totuusarvon palauttavalla `false`.
- **Laji:**
  - *muuttaa:* muuttaa pelin tilaa ja käyttää kutsujan virtaa,
  - *muuttaa vastaanottajaa:* muuttaa pelin tilaa vain, jos vastaanottaja kuuluu siihen; paikallinen kopio tai lista kelpaa kyselyssä ([RS 5.5](../roguescript/05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)),
  - *kysely:* ei muuta tilaa; satunnaisuus kulkee kutsujan virrasta, kyselyssä kopiosta (luku [4.1](04-satunnaisuus-ja-toistettavuus.md#41-satunnaisvirrat)),
  - *esitys:* ei muuta tilaa ja käyttää esityksen virtaa.
- **Konteksti:** kaikkialla, vain generaattorissa tai vain korutiinissa.
- **Lähettää:** moottorin tapahtumat järjestyksessä (luku [9.6](#96-tapahtumat)).

Laji ratkaisee latauksen tarkistukset ja ohituksen yhdellä säännöllä:

| Laji | `query`, `present fn` | `background realtime` | Ohitettaessa ([RS 9.8](../roguescript/09-kohtaukset-ja-aikajanat.md#98-luonnos-ohitus-ja-alusta-aloitus)) |
| --- | --- | --- | --- |
| muuttaa | latausvirhe | latausvirhe | suoritetaan heti; tilaa muuttavan tweenin jäljellä olevat askeleet tehdään heti järjestyksessä |
| muuttaa vastaanottajaa | sallittu vain paikalliselle kopiolle tai listalle | latausvirhe | suoritetaan heti |
| kysely | sallittu | sallittu | suoritetaan |
| esitys | sallittu `present fn`:ssä, latausvirhe `query`:ssa | sallittu | jätetään väliin, paitsi `always`-rivi |

**Version valinta.** Lataus valitsee funktion version ensimmäisen argumentin staattisesta tyypistä. Ainoa poikkeus on `from`, jonka versiot erottaa kolmas argumentti. Paikallisen nimen tyyppi päätellään paluutyypistä (`hand = LOOT.RARE.copy()` on `deck`). Kokoelman ja listan jäsenen tyyppi tulee jäsenlajista ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)). Entiteetin nimen kutsu on `card` ([RS 2.8](../roguescript/02-yleinen-syntaksi.md#28-ison-nimen-kutsu)), joten `spawn(NKI(…), at: …)` valitsee kortin version. Jos sopivaa versiota ei ole, kutsu on latausvirhe, joka luettelee versiot.

## 9.3 Funktiot

Lyhenteet: *alue* on `area`, ja sen paikalla käyvät myös `pos`, kartalla oleva entiteetti, ruutujen lista (`list pos`) ja `label(…)` ([RS 6.2](../roguescript/06-ruudut-ja-paikkahaku.md#62-operaatiot)). *Joukko* on kokoelma, lista, ruutujoukko, suodatettu joukko, jokeri tai tilen indeksiväli ([RS 5.4](../roguescript/05-funktiot-ja-lausekkeet.md#54-satunnaisuus)). Entiteettien lista käy kokoelman paikalla, missä kokoelmaa vain luetaan ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)).

**Tila ja ilmentymät**

| Kutsu | Paluu | Laji | Huomiot |
| --- | --- | --- | --- |
| `spawn(x: name, at: alue)` | `entity`, epäonnistuessa `none` | muuttaa | lähettää `created` (luku [3.2](03-pelin-tila.md#32-ilmentymän-syntyminen)); arpoo ruudun joukosta |
| `spawn(x: card, at: alue)` | `collection` | muuttaa | kortti: määrä ja ylikirjoitukset; määrä synnyttää erilliset ilmentymät; `none`-kortti tuottaa tyhjän; logiikan kortti tulee entiteetin nimen kutsusta (`NKI(NOTE: k)`, [RS 2.8](../roguescript/02-yleinen-syntaksi.md#28-ison-nimen-kutsu)), ja sen ylikirjoitukset sovelletaan ennen `created`-tapahtumaa |
| `spawn(x: deck, at: alue)` | `collection` | muuttaa | nostaa yhden kortin ja synnyttää sen; sama taululle (`x: table`) |
| `spawn(x, into: collection)` | kuten yllä | muuttaa | synnyttää kokoelmaan `at`-argumentin sijaan; samanlaiset kappaleet liittyvät pinoon; täysi kokoelma: `none` tai tyhjä ja ajon varoitus |
| `destroy(e: entity)` | `none` | muuttaa | lähettää `removed`; jäsenet tuhotaan (luku [9.5](#95-ilmentymän-elinkaari-ja-komponentit)) |
| `add(e: entity, k: KOMPONENTTI(…))` | `entity` | muuttaa | lisää komponentin tai korvaa samannimisen (luku [9.5](#95-ilmentymän-elinkaari-ja-komponentit)) |
| `remove(e: entity, k: name)` | `entity` | muuttaa | poistaa komponentin; ei tapahtumaa (luku [9.5](#95-ilmentymän-elinkaari-ja-komponentit)) |
| `move(e: entity, to: alue\|collection, via: path\|line = path)` | `bool` | muuttaa | kartalla kantajan tai entiteetin kartalla; tyhjä joukko tai täysi kokoelma `false`; siirtää yhden kappaleen, ja pino siirretään silmukalla `e.stack`; tapahtumat luvussa [9.6](#96-tapahtumat) |
| `tile(p: alue, t: name\|none, layer: name = oletuskerros, label: name)` | `bool` | muuttaa | arpoo ruudun joukosta; `none` tyhjentää kerroksen |
| `tag(a: area, n: name)` | `area` | muuttaa | nimeää joukon kaikki ruudut ([RS 6.4](../roguescript/06-ruudut-ja-paikkahaku.md#64-siirtyminen-kartalta-toiselle)) |
| `entities(in: area = tiles())` | `collection` | kysely | luontijärjestyksessä |
| `control(e: entity, seat: number = none)` | `entity` | muuttaa | tekee `e`:stä pelaajan; herätys luvun [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin) mukaan; `seat:` tekee `e`:stä istuimen jäsenen ja ohjattavan, ja istuimen on oltava liitetty (luku [6.1.10](06-ajo.md#6110-istuimet-ja-ryhmä)) |
| `travel(e: entity, map: name, at: name = START, align: bool = false)` | `entity` | muuttaa | lähettää `leave`, `depart` ja `arrive` (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)); saapumisruutu arvotaan, ellei `align`; nykyinen kartta sallittu; puuttuva nimiö on virhe; siirto tehdään toiminnon lopussa ennen etualan kohtauksia (luku [6.1.7](06-ajo.md#617-toiminto)) |

**Ruutujoukot** (kaikki kyselyjä)

| Kutsu | Paluu | Huomiot |
| --- | --- | --- |
| `tiles()` | `area` | koko kartta |
| `at(h: name)` | `area` | ruudut, joiden jokin kerros on hakujoukon jäsen |
| `near(a: alue, d: number\|range)` | `area` | luku N on väli 0..N; väli antaa renkaan |
| `from(a: alue, x: number, y: number)` | `area` | siirtymä ankkurista |
| `from(a: alue, d: number, s: dir)` | `area` | suunta voi olla suhteellinen (`facing`) |
| `rect(k: pos, size: size)` | `area` |  |
| `rect(k: pos, width: number, height: number)` | `area` | koko luvuista ([RS 6.1](../roguescript/06-ruudut-ja-paikkahaku.md#61-joukon-tuottavat-funktiot)) |
| `line(a: alue, b: alue)` | `area` |  |
| `path(a: alue, b: alue, for: entity = none)` | `area` | reitti järjestyksessä; `.last` on määränpää; for: liikkujan liikkumistapa, through-joukko, esto sekä kentät dirs ja corners, ilman sitä vain tilejen move-esto ja kahdeksan suuntaa; lähtöruutu ei kuulu reittiin; `for`-reitillä estävä toimija (`ai`, pelaaja tai istuimen jäsen) on läpipääsevä, mutta sen ruutu maksaa neljä askelta, jotta reitti kiertää jonon, jos voi |
| `label(n: name)` | `area` | nimiöryhmän ja nimettyjen entiteettien ruudut |
| `edge(s: dir)` | `area` | reunarivi tai -sarake lukujärjestyksessä |
| `pos(x: number, y: number)` | `pos` |  |
| `region(p: pos, of: area)` | `area` | `of`-joukon yhtenäinen osa, johon `p` kuuluu (4-naapurusto); käy kaikkialla, esimerkiksi pelaajan huone `region(player.at, of: at(FLOOR))` |

**Reitinhaku.** `path` on A\*-haku. Askel viereiseen ruutuun maksaa 1 kaikissa kahdeksassa suunnassa, ja `for`-reitillä estävän toimijan ruutu maksaa 4. Heuristiikka on Chebyshev-etäisyys lähimpään kohderuutuun. Avoimista ruuduista laajennetaan pienin arvio (kustannus + heuristiikka), tasatilanteessa pienin heuristiikka ja sitten ensimmäinen lukujärjestyksessä. Naapurit käydään järjestyksessä `north`, `northeast`, `east`, `southeast`, `south`, `southwest`, `west`, `northwest`, ja ruudun edeltäjäksi jää ensimmäinen, joka tuotti sen pienimmän kustannuksen. Haku päättyy, kun kohdejoukon ruutu otetaan laajennettavaksi. Jos lähtöruutu kuuluu kohteeseen, reitti on tyhjä. Ilman reittiä tulos on tyhjä joukko.

**Satunnaisuus** (kyselyjä, kutsujan virta, algoritmit luvussa [4.1](04-satunnaisuus-ja-toistettavuus.md#41-satunnaisvirrat))

| Kutsu | Paluu | Huomiot |
| --- | --- | --- |
| `choose(j: joukko)` | jäsen tai `none` | suljettu lukuväli antaa kokonaisluvun, avoin väli on virhe (`choose(4..9)`, [RS 5.4](../roguescript/05-funktiot-ja-lausekkeet.md#54-satunnaisuus)); tyhjä väli (`0..-1`) ja tyhjä joukko antavat `none` |
| `shuffled(j: joukko)` | `collection` tai `list` | uusi joukko samaa lajia, ei muuta mitään; ruutujoukosta `list pos` arvontajärjestyksessä |
| `pick(j: joukko, n: number)` | `collection` tai `list` | samaa lajia; ruutujoukosta `list pos`; liian pieni joukko: kaikki jäsenet ja ajon varoitus |
| `chance(p: percent)` | `bool` |  |

Tyhjä väli on tyhjä joukko kaikissa tuottavissa käytöissä: `for` ei kierrä, ja `pick` ja `shuffled` palauttavat tyhjän ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)).

**Listat** ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit); muuttavat vastaanottajaa, joten paikallinen lista kelpaa kyselyssä ja kentän lista muuttaa pelin tilaa)

| Kutsu | Paluu | Huomiot |
| --- | --- | --- |
| `insert(l: list, i: number, v)` | `list` | lisää paikkaan `i`; `i == l.count` lisää loppuun; muu alueen ulkopuolinen indeksi on käyttöhetken virhe |
| `remove_at(l: list, i: number)` | `list` | poistaa paikasta `i`; alueen ulkopuolinen indeksi on käyttöhetken virhe |
| `index_of(l: list, v)` | `number` tai `none` | kysely; ensimmäinen paikka |

`l[i]`, `l[i] = v` ja `l += v` ovat operaattoreita. Kentän lista noudattaa kokoaan ja `sort`-järjestystään jokaisen kirjoituksen jälkeen ([D 5.1](../dataformaatit/05-komponentit-ja-entiteetit.md#51-komponentit)).

**Pakat ja taulut**

| Kutsu | Paluu | Laji | Huomiot |
| --- | --- | --- | --- |
| `shuffle(d: deck)` | `deck` | muuttaa vastaanottajaa | kutsujan virta |
| `draw(d: deck, n: number\|dice\|all = 1)` | `collection card` | muuttaa vastaanottajaa | kortit nostojärjestyksessä |
| `draw(t: table, n: number\|dice = 1)` | `collection card` | kysely | kutsujan virta |
| `peek(d: deck, n: number = 1)` | `collection card` | kysely | näkee vain nostopinon |
| `put(d: deck, card: card\|name, bottom: bool = false, shuffle: bool = false)` | `deck` | muuttaa vastaanottajaa | `bottom` ja `shuffle` yhdessä on virhe |
| `add(d: deck, other: deck, shuffle: bool = false)` | `deck` | muuttaa vastaanottajaa | litistää toisen pakan määrittelyn |
| `reset(d: deck)` | `deck` | muuttaa vastaanottajaa | poistaa määrittelyn ulkopuoliset kortit |
| `copy(d: deck)` | `deck` | kysely | uusi paikallinen kopio; kutsujan virta |

Pakan `.count` on kenttä, ei kutsu. Esto ja maasto (kyselyjä): blocks(p: alue, what: move|sight|name, for: entity = none, tiles: bool = true, except: entity = none) -> bool soveltaa tilen ominaisuuksien portaan ja kerrossäännön ([D 7.5](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)) sekä entiteettien blocks-komponentin (luku [3.4](03-pelin-tila.md#34-moottorin-komponentit)); for jättää liikkujan itsensä pois ja käyttää sen through-joukkoa, ja joukolle tulos on tosi, jos jokin ruutu estää. Kun `what` on `move` ja `for` on annettu, kysely katsoo myös liikkujan liikkumistavan: tile tai entiteetti, joka estää `movement.mode`-tavan (`blocks WALK`) tai sallii vain muita tapoja (`only`), estää liikkujan. Näin `blocks(p, move, for: e)` riittää liikekomennon ehdoksi (STD `CAN_STEP`). tile\_of(p: pos, layer: name = oletuskerros) -> name|none palauttaa kerroksen tilen. `tiles: false` jättää tilet pois, jolloin `blocks` katsoo vain entiteettien omaa estoa. `except` jättää pois vielä yhden entiteetin, esimerkiksi paikkaa vaihtavan toimijan oman ruutunsa estosta: `blocks(actor.at, move, for: other, except: actor)` kertoo, voiko `other` astua ruutuun, josta `actor` on lähdössä. `adjacent(e: entity) -> area` palauttaa ruudut, joihin `e` voi astua yhdellä askeleella sen `movement`-kenttien `dirs` ja `corners` mukaan. Kohderuudun omaa estoa se ei katso, koska törmäys on liikekomennon asia. **Linja ja lento.** `line(a, b)` on Bresenhamin viiva ruudusta `a` ruutuun `b`, molemmat mukaan lukien; vinon askeleen tasatilanne ratkeaa vaakasuunnan hyväksi, joten tulos on deterministinen mutta ei symmetrinen. `ray(a: pos, b: pos, for: entity = none, range: number = 0, mode: name = none) -> area` palauttaa lentoreitin ilman lähtöruutua: linjan `a`:sta `b`:hen ja `range`-arvolla samaa suuntaa jatkaen, kunnes reitin pituus on `range`. Reitti päättyy ennen ensimmäistä ruutua, jonka tile estää arvolla `move` tai `mode`, tai jossa entiteetin `blocks` sisältää arvon `mode`. Olento, joka estää vain liikkeen, ei pysäytä lentoa, vaan osumat ovat logiikan asia. Esitysfunktio `fly(tile: name, route: area, speed: time)` piirtää ammuksen reittiä pitkin, eikä se muuta tilaa.

**Kartat ja kamera.** `spawn(x: name|card|deck|table, map: name, at: name)` palauttaa saman kuin `spawn`, muuttaa tilaa ja synnyttää toisen kartan nimiöön (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)). `follow(c: camera, e: entity = none) -> none` muuttaa tilaa ja vaihtaa kameran kohteen, ja ilman kohdetta palauttaa oletuksen (luku [7.1](07-esitys.md#71-asettelu)). `e.map`, `e.kind`, `e.slot` ja `e.stack` ovat kenttiä, eivät kutsuja (luvut [3.4](03-pelin-tila.md#34-moottorin-komponentit) ja [6.2](06-ajo.md#62-tapahtumat)).

**Kyselyt**

| Kutsu | Paluu | Huomiot |
| --- | --- | --- |
| `min(a: number, b: number)`, `max(…)` | `number` |  |
| `distance(a: alue, b: alue)` | `number` | Chebyshev |
| `nearest(j: joukko, a: alue)` | jäsen tai `none` | tasatilanteessa joukon järjestys; ruutujoukolle lähin ruutu |
| `sees(a: entity, b: alue)` | `bool` | `sight`-komponentti ja valaistus (luku [7.2](07-esitys.md#72-kartan-piirtäminen)); `revealed` ja `reveal` eivät vaikuta |
| `sees(a: collection, b: alue)` | `bool` | tosi, jos yksikin jäsen näkee; tyhjä kokoelma ei näe |
| `in_view(x: alue)` | `bool` | yhteinen näkökenttä, ilman jäseniä viimeinen yhteinen näkökenttä (luku [7.2](07-esitys.md#72-kartan-piirtäminen)) |
| `seats()` | `list number` | liitetyt istuimet numerojärjestyksessä; yksinpelissä `[1]` (luku [6.1.10](06-ajo.md#6110-istuimet-ja-ryhmä)) |
| `seat_of(e: entity)` | `number` tai `none` | jäsenen istuin |
| `members(n: number)` | `collection` | istuimen elävät jäsenet jäseneksi tulon järjestyksessä |
| `active(n: number)` | `entity` tai `none` | istuimen ohjattava |
| `controlled(e: entity)` | `bool` | `e` odottaa vuorollaan istuimensa syötettä: istuimen ohjattava, säännöllä `party each` jokainen elävä jäsen, ja säännöllä `party switch` jokainen jäsen, kun ohjattava ei voi toimia (luku [6.1.10](06-ajo.md#6110-istuimet-ja-ryhmä)) |
| `leashed(who: entity, to: pos)` | `bool` | talutushihna epää istuimen ohjattavan askeleen ruutuun `to`, koska se levittäisi saman kartan jäsenten rajaavan suorakulmion kartta-aluetta kahta ruutua pienempää suuremmaksi; `false` ilman `leash`-kenttää, muulle kuin ohjattavalle ja yksin kartalla (luku [6.1.10](06-ajo.md#6110-istuimet-ja-ryhmä)) |
| `view_width()`, `view_height()` | `number` | kartta-alueen kirjoitettu koko soluina, ilman sitä kartan koko; ei riipu ruudun koosta (luku [7.1](07-esitys.md#71-asettelu)) |
| `text_of(key: name, …)` | `text` | kielen mukainen teksti paikkamerkit täytettyinä; vain kyselyssä, esityksessä, paneelissa, listavalikossa ja `naming`-kyselyssä (luku [7.3](07-esitys.md#73-tekstit)) |

**Generointi** (pelin virta; `footprint` on kysely, muut muuttavat; `external` vain generaattorissa)

| Kutsu | Paluu | Huomiot |
| --- | --- | --- |
| `carve(a: area, t: name, layer: name = oletuskerros, from: area)` | `none` | asettaa koko joukon tilen; `carve(tiles(), WALL)` täyttää kartan |
| `footprint(p: name, at: pos)` | `area` | prefabin piirretyt ruudut, kun sen vasen yläkulma on `at`; `none`, jos prefab ei mahdu kartalle |
| `stamp(p: name, at: pos)` | `none` | painaa prefabin tilet, nimiöt ja entiteetit; prefabin tyhjät ruudut jättävät alla olevan |
| `external(program: text, …)` | `bool` | vain generaattorissa; ajaa ulkoisen ohjelman ja painaa sen kartan (alla) |
| `scatter(x: name\|card\|deck\|table, count: number\|range\|dice, in: area)` | `collection` | ensimmäinen parametri kuten `spawn`; jokainen synnytys saa eri ruudun joukosta (kortin kappaleet samaan ruutuun), ja varattuja ruutuja se ei väistä; jos ruudut loppuvat, loput jätetään synnyttämättä ja moottori kirjaa ajon varoituksen |

`region` on ruutujoukkojen kysely (yllä), ja generaattorit käyttävät sitä usein. Algoritmit ovat vakiokirjastoa. `STD.SCATTER_TILES`, `STD.SMOOTH`, `STD.ROOMS`, `STD.CONNECT` ja `STD.PLACE` on kirjoitettu RogueScriptillä ([STD 10](../vakiokirjasto/10-generointi.md)), ja ne käyttävät yllä olevia primitiivejä. Kirjaston lähdekoodi määrää algoritmin ja satunnaisvirran käytön, joten sama siemen tuottaa saman kartan kaikissa toteutuksissa. Niitä varten `nearest` hyväksyy myös ruutujoukon ja palauttaa lähimmän ruudun, `pos`-arvolla on kentät `x` ja `y`, `rect` hyväksyy muodon `rect(kulma, leveys, korkeus)` ja `choose` lukuvälin ([RS 5.4](../roguescript/05-funktiot-ja-lausekkeet.md#54-satunnaisuus) ja [6.1](../roguescript/06-ruudut-ja-paikkahaku.md#61-joukon-tuottavat-funktiot)).

**Ulkoinen generaattori.** `external("tools/wfc", depth: 3)` ajaa ohjelman ja painaa sen tuottaman kartan generoitavaan karttaan kuten `stamp`.

- **Kutsu.** Ohjelma saa nimetyt argumentit komentorivillä (`--depth 3`) sekä aina kartan nimen `--name`, `--width`, `--height` ja `--seed`. Siemen nostetaan pelin virrasta yhdellä nostolla, joten virta etenee samoin riippumatta ohjelmasta.
- **Tulos.** Ohjelma tulostaa stdoutiin samanlaisen karttatiedoston kuin piirretty kartta: otsikon `map NIMI [size LxK]`, piirroksen, `legend:`-osion ja valinnaisen `vars:`-osion ([D 8.1](../dataformaatit/08-kartat-ja-prefabit.md#81-karttatiedosto)). Moottori jäsentää ja tarkistaa sen samalla koodilla kuin `maps/`-kansion tiedostot, joten nimiöt (`as`), entiteetit (`with`) ja kartan muuttujat toimivat. Tulosteen voi tallentaa `.rlml`-tiedostoksi ja avata tai korjata käsin.
- **Otsikko.** Otsikon nimen on vastattava `--name`-argumenttia. Jos generaattorilla on `size`, otsikon koon on vastattava sitä; ilman `size`-määrettä kartta saa koon tulosteesta. `transient` ja vyöhykkeiden liitokset ratkaistaan kartan määrittelyssä, joten tulosteen otsikossa ne ovat virhe. Ristiriita on käyttöhetken virhe.
- **Determinismi.** Ohjelman on tuotettava samalla siemenellä sama kartta. Moottori ei voi valvoa sitä, mutta kartat säilyvät tallennuksessa, joten poikkeama vaikuttaa vain uudelleengenerointiin (`transient`, uuden pelin laskenta).
- **Polku.** Ohjelman on oltava pelin kansiossa. Absoluuttinen polku tai `..` on virhe.
- **Lupa.** Moottori kysyy pelaajalta luvan, kun peli ajaa ohjelmaa ensimmäisen kerran. Lupa koskee peliä ja ohjelmaa, ja se tallentuu asetuksiin. Asetus `- external all` sallii kaikki ilman kyselyä ja `- external none` kieltää kaikki; oletus on `ask` ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)).
- **Aikaraja.** `- external_timeout 10s`, oletus 10 sekuntia.
- **Paluuarvo.** `true`, kun kartta painettiin. `false`, kun ohjelmaa ei voi ajaa: alusta ei salli prosesseja, ohjelma puuttuu tältä käyttöjärjestelmältä tai lupa puuttuu. Generaattori voi silloin käyttää RogueScript-varaversiota.
- **Virheet.** Nollasta poikkeava paluukoodi, aikarajan ylitys tai virheellinen karttalohko on käyttöhetken virhe, ja ilmoitus näyttää ohjelman stderr-tulosteen.

**Kohtaukset ja tallennus**

| Kutsu | Paluu | Laji | Konteksti |
| --- | --- | --- | --- |
| `start(k: scene, scope: map, bind: entity\|collection)` | `handle` taustakohtaukselle, muuten `none` | muuttaa | kaikkialla; etualan kohtaus alkaa toiminnon lopussa siirtojen jälkeen (luku [6.1.7](06-ajo.md#617-toiminto)) |
| `stop(h: handle)`, `stop(track: name)` | `none` | muuttaa | kaikkialla; `track` vain kohtauksessa |
| `say(key: name, …, speaker: entity)` | `none` | esitys | korutiini, etualan aikajana, pelin ajan taustakohtaus |
| `log(key: name, …, speaker: entity)` | `none` | esitys | kaikkialla |
| `checkpoint()`, `rollback()`, `autosave()` | `none` | muuttaa | kaikkialla; tehdään toiminnon lopussa (luvut [5.3](05-aloitus-tallennus-ja-checkpointit.md#53-checkpointit), [5.4](05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus) ja [6.1.7](06-ajo.md#617-toiminto)) |
| `delete_save(target: checkpoint\|auto\|all)` | – | muuttaa | kaikkialla; tehdään toiminnon lopussa (luku [5.5](05-aloitus-tallennus-ja-checkpointit.md#55-tallennuksen-poisto)) |

`wait`, `open`, `emit` ja `cancel` ovat avainsanoja, eivät funktioita ([RS 2.4](../roguescript/02-yleinen-syntaksi.md#24-varatut-sanat-ja-paikalliset-nimet)). `wait` ja `open` ovat sallittuja komennon rungossa ja korutiinikohtauksissa (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)). `open` hyväksyy valikon ja paneelin (luku [7.7](07-esitys.md#77-paneelit)).

**Esitys** (esitys, esityksen virta; kestollisella aikajanan rivillä tweeni)

| Kutsu | Paluu | Huomiot |
| --- | --- | --- |
| `play(sound: name, at: alue)` | `none` |  |
| `pan(c: camera, to: alue)` | `none` |  |
| `shake(c: camera, strength: number)` | `none` |  |
| `fade(v: light, to: percent)` | `none` |  |
| `light(a: area)` | `light`: alueen valot |  |
| `reveal(a: area)` | `none` | lisää ruudut pelaajan muistiin (luku [7.2](07-esitys.md#72-kartan-piirtäminen)); muisti on pelaajan näkymää, joten kutsu on sallittu kaikkialla, myös käsittelijässä, mutta ei muuta pelin kulkua |
| `today()` | päiväys, kentät `year`, `month`, `day` | koneen kalenterin mukainen päivä; esitystä, joten sallittu `present fn`:ssä ja paneelin kentissä mutta ei kyselyssä eikä tilaa muuttavassa logiikassa |

Tyyppien nimet ovat RogueScriptin tyyppiluettelossa ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)): kahva on `handle`, kohtaus `scene`, kamera `camera` ja valo `light`. Pakan ja taulun nostot palauttavat kortteja (`collection card`) ja `STD.ROOMS` alueita (`list area`, [STD 10](../vakiokirjasto/10-generointi.md)).

## 9.4 Ylikuormitukset

Samalla nimellä on useampi versio vain, kun ensimmäisen parametrin tyyppi erottaa ne (poikkeukset `from` ja `rect`):

| Nimi | Versiot |
| --- | --- |
| `draw` | pakka: muuttaa vastaanottajaa · taulu: kysely |
| `add` | pakka ja pakka · entiteetti ja komponentti |
| `remove` | entiteetti ja komponentti · sanakirja ja avain ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)) |
| `sees` | entiteetti · kokoelma |
| `spawn`, `scatter` | nimi → `entity` (`spawn`) · kortti, pakka tai taulu → `collection` |
| `from` | kolmas argumentti luku: siirtymä · kolmas argumentti suunta: suunnassa |
| `stop` | kahva · `track:` |
| `rect` | kulma ja koko · kulma, leveys ja korkeus (argumenttien määrä) |

Sama nimi on myös kahdessa lajissa: `at` (funktio, nimetty parametri ja `game`-kenttä), `label` ja `light` (komponentti ja funktio) sekä `repeat` (RogueScriptin silmukka ja moottorin toiminto), `join` (moottorin toiminto ja tapahtuma), `seats` (`game`-kenttä ja funktio) ja `turn` (tapahtuma, `game`-kenttä ja kartta-alueen määre). Sulut ja paikka erottavat ne, joten ne ovat sallittuja.

## 9.5 Ilmentymän elinkaari ja komponentit

`remove` poistaa komponentin ja `destroy` ilmentymän. Tapahtuma `removed` kuuluu ilmentymän tuholle, ja se on `created`-tapahtuman pari. `remove` ei lähetä sitä.

**`remove(e, KOMP)`** poistaa komponentin kenttineen ja käsittelijöineen. Tapahtumaa ei lähetetä. Puuttuvan komponentin poisto ei tee mitään.

- Moottorin komponentin poistolla on vaikutus: `ai` poistaa herätyksen, ellei `e` ole pelaaja tai istuimen jäsen; `seat` irrottaa jäsenen istuimeltaan, ja herätys poistuu, ellei sillä ole `ai`-komponenttia; `timer` poistaa ajastimet; `render`, `sight` ja `light` lakkaavat piirtämästä, näkemästä ja valaisemasta; `revealed` lopettaa piirron näkökentän ulkopuolella.
- Jos komponentin kentässä on kokoelma, sen jäsenet tuhotaan kuten `destroy`-kutsussa.

**`add(e, KOMP(…))`** lisää komponentin. Moottori luo sen samoin kuin syntymässä (luku [3.2](03-pelin-tila.md#32-ilmentymän-syntyminen), askeleet 2–5): argumentit ovat ylikirjoitukset, puuttuvat kentät saavat oletusarvon, nopat heitetään ja kokoelmat täytetään.

- Jos samanniminen komponentti on jo olemassa, uusi korvaa sen. Tila uusiutuu siis alusta (`add(self, POISONED(turns: 5))`).
- `ai` laittaa ilmentymän herätysjonoon, jos se on kartalla (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)). Tapahtumaa ei lähetetä.
- Komponentin lisäys tai poisto kokoelman jäsenelle voi muuttaa pinoja (luku [3.3](03-pelin-tila.md#33-kokoelmat)).

**`destroy(e)`** lopettaa ilmentymän neljässä vaiheessa:

1. Moottori lähettää `removed(e)`. Ilmentymä on vielä ehjä ja paikallaan, joten käsittelijä voi pudottaa tavarat (`spawn(…, at: e)` tai `move(item, to: e.at)`).
2. Kokoelmiin jääneet jäsenet tuhotaan samoin vaiheittain. Kantaja käsitellään ennen jäseniään, jotta sen `removed`-käsittelijä ehtii tyhjentää kokoelmat.
3. Herätys poistuu, `bind`-kohdetta seuraavat taustakohtaukset päättyvät ([RS 9.9](../roguescript/09-kohtaukset-ja-aikajanat.md#99-taustakohtaukset)) ja ilmentymä irrotetaan sijainnistaan. Tapahtumia `leave` ja `depart` ei lähetetä.
4. Moottori laskee ilmentymän nimen (luku [7.3](07-esitys.md#73-tekstit)) ja säilyttää sen kahvassa, ja kahva kuolee. Kentän luku antaa `none` ilman varoitusta, `has` on epätosi, ja vertailu `==` toimii yhä. Kahva säilyttää lajinsa (`kind`), joten `is` toimii ja tekstin paikkamerkki näyttää säilytetyn nimen. Tilaa muuttava kutsu kuolleelle kahvalle on käyttöhetken virhe, paitsi `destroy`, joka ei tee mitään. Tapahtuma kuolleelle subjektille jätetään jakamatta (luku [6.2](06-ajo.md#62-tapahtumat)).

## 9.6 Tapahtumat

Yleinen käsittelijä saa subjektin ensimmäisenä parametrina. Komponentin käsittelijässä subjekti on `self`, eikä sitä kirjoiteta; sama koskee pelin omia tapahtumia (luku [6.2](06-ajo.md#62-tapahtumat)). Parametrin nimi saa olla sama kuin kontekstinimi (`on arrive(map, p)`, [RS 2.4](../roguescript/02-yleinen-syntaksi.md#24-varatut-sanat-ja-paikalliset-nimet)). Moottorin tapahtumilla on vain vaiheet `on` ja `after` (luku [6.2](06-ajo.md#62-tapahtumat)).

| Tapahtuma | Subjekti | Yleinen käsittelijä | Komponentissa | Laukeaa | Lähettäjät |
| --- | --- | --- | --- | --- | --- |
| `begin` | `game` | `begin()` | – | uuden pelin askel 8, kerran pelikerrassa (luku [5.1](05-aloitus-tallennus-ja-checkpointit.md#51-uusi-peli)) | aloitus |
| `turn` | ilmentymä; yleisille `game` | `turn()` | `turn()` | kierroksen rajalla, vaihe 1 (luku [6.1.4](06-ajo.md#614-mikroaskeleen-vaiheet)) | ajoitin |
| `wake` | toimija | `wake(who)` | `wake()` | herätyksen alussa, ennen valintaa tai jatkoa (luku [6.1.5](06-ajo.md#615-vuoron-kulku)) | ajoitin |
| `created` | uusi ilmentymä | `created(e)` | `created()` | syntymän askel 7: kokoelmat täytetty ja ilmentymä sijainnissaan, ennen herätysjonoa; jäsenet ennen kantajaa (luku [3.2](03-pelin-tila.md#32-ilmentymän-syntyminen)) | legend, `spawn`, `scatter`, generaattori, `add` kokoelmineen |
| `removed` | tuhottava ilmentymä | `removed(e)` | `removed()` | tuhon vaihe 1: ilmentymä ehjä ja paikallaan; kantaja ennen jäseniä (luku [9.5](#95-ilmentymän-elinkaari-ja-komponentit)) | `destroy`, kokoelmakomponentin `remove` |
| `enter` | siirtyjä | `enter(who, p, prev)` | `enter(p, prev)` | siirron jälkeen; `prev` on lähtöruutu tai `none` kokoelmasta tultaessa | `move` |
| `leave` | siirtyjä | `leave(who, p)` | `leave(p)` | siirron jälkeen; `p` on lähtöruutu | `move`, `travel` |
| `arrive` | siirtyjä | `arrive(who, map, p)` | `arrive(map, p)` | kartalle saapumisen jälkeen; kohdekartta on ladattu ja sulatettu, ja `p` on saapumisruutu | `travel`, myös nykyiseen karttaan; `move` kokoelmasta |
| `depart` | siirtyjä | `depart(who, map, p)` | `depart(map, p)` | lähdön jälkeen; `map` ja `p` ovat lähtökartta ja lähtöruutu | `travel`, myös nykyiseen karttaan; `move` kokoelmaan |
| `join` | `game` | `join(seat)` | – | toiminnon lopussa siirtojen jälkeen, kun istuin liittyy kesken pelin (luku [8.2](08-syote.md#82-istuimet-ja-laitteet)) | liittyminen |
| `part` | `game` | `part(seat)` | – | toiminnon lopussa siirtojen jälkeen, kun istuin poistuu; jäsenillä on vielä `seat` | `leave_seat`, latauksen ohitettu istuin |
| `hear` | kuulija | `hear(who, s)` | `hear(s)` | herätyksen alussa ennen vuoroa, jokaisesta uudesta kuuluvasta äänestä kovimmasta alkaen (luku [6.8](06-ajo.md#68-äänet-ja-kuuleminen)) | ajoitin |

**Sijainnin muutokset.** Moottori tekee siirron ensin ja lähettää tapahtumat sen jälkeen, joten käsittelijä näkee siirtyjän uudessa sijainnissaan.

| Muutos | Funktio | Tapahtumat järjestyksessä |
| --- | --- | --- |
| syntymä | legend, `spawn`, `scatter`, generaattori | `created` |
| ruudusta toiseen samalla kartalla | `move` | `leave`, `enter` |
| kartalta toiselle tai samalle kartalle | `travel` | `leave`, `depart`, `arrive` |
| kartalta kokoelmaan | `move(…, to: kokoelma)` | `leave`, `depart` |
| kokoelmasta kartalle | `move(…, to: alue)` | `arrive`, `enter` (`prev` on `none`) |
| kokoelmasta toiseen | `move` | ei tapahtumia |
| tuho | `destroy` | `removed` |

## 9.7 Raja vakiokirjastoon

Moottori lähettää tapahtuman vain, kun kaksi ehtoa täyttyy:

1. **Moottori tekee muutoksen itse.** Sijainti, elinkaari ja aika muuttuvat vain moottorin funktioissa, joten vain moottori voi kertoa niistä luotettavasti.
2. **Tapahtumassa ei ole pelin käsitettä.** ”Astui ruutuun” on mekaniikkaa. ”Törmäsi”, ”tuli viereen” ja ”sai esineen” tulkitsevat muutosta pelin säännöillä.

Kaikki muu rakennetaan vakiokirjastossa moottorin tapahtumien ja funktioiden päälle (vakiokirjasto, luku [4](../vakiokirjasto/04-tapahtumat.md)):

| Vakiokirjasto | Rakennetaan | Miksi ei moottorissa |
| --- | --- | --- |
| `STD.GIVE(to, x)` | `spawn(x, into: …)` tai `move(x, to: …)` repun kokoelmaan | moottori ei tiedä, mikä kokoelma on reppu |
| `STD.CAN_STEP`, `STD.BLOCKER`, `STD.DO_STEP` | `adjacent`, `blocks`, `entities`, `move` ja `noise` | kulkukelpoisuus ja liikkeen ääni ovat pelin sääntöjä |
| `STD.BUMP` | liikekomennossa, kun kohderuudussa on estävä entiteetti (`STD.BLOCKER`) | moottori ei tunne kulkukelpoisuutta (luku [10](10-mita-moottori-ei-tee.md)) |
| `STD.STRIKE` | lähitaistelun ydin, joka lähettää `STD.HIT` | osuminen on pelin sääntö |
| `STD.ADJACENT`, `STD.APART` | vanhan ja uuden naapuruston erotuksesta | naapurusto (4 vai 8, lasketaanko esineet) on pelin sääntö |
| `STD.PICKED_UP`, `STD.DROPPED` | poiminta- ja pudotuskomennoissa | inventaario on vakiokirjastoa |
| `STD.DIED` | `STD.APPLY_DAMAGE` (`cause`-parametrin kanssa) | kuolema on vakiokirjastoa |
| `STD.SCATTER_TILES`, `STD.SMOOTH`, `STD.ROOMS`, `STD.CONNECT`, `STD.PLACE` | `carve`, `region`, `footprint`, `stamp` ([STD 10](../vakiokirjasto/10-generointi.md)) | algoritmi on pelin valinta, ja lähdekoodi määrää toistettavuuden |

Peli, joka ei tarvitse näitä, korvaa tai poistaa käsittelijät, eikä se maksa niistä mitään (luku [1.2](01-rajaus-ja-periaatteet.md#12-periaatteet)).

**Kattavuus.** Vierekkäisyys on moottorin tapahtumajoukon testi. Vakiokirjasto saa naapuruston kaikki muutokset kiinni tapahtumilla `enter` (lähtöruutu `prev`), `created`, `removed`, `arrive` ja `depart`, koska syntymä, tuho ja siirtymä ohittavat `enter`- ja `leave`-tapahtumat. Jos jokin muutos jää näiden ulkopuolelle, moottorin tapahtumajoukko on vajaa.

Siksi `arrive` ja `depart` kertovat myös ruudun (`p`). Kartalta lähteneen siirtyjän lähtöruutua ei muuten tietäisi, koska tapahtumat lähetetään siirron jälkeen. `leave` ei riitä, koska se laukeaa myös kartan sisäisessä siirrossa, jolloin naapurusto lasketaan `enter`-tapahtumasta.

**Näkyvyys on raja-alue.** Näkökentän laskee moottorin `sight`-komponentti, mutta vakiokirjasto voi verrata näkyviä `wake`-käsittelijässä edelliseen herätykseen. Toimija reagoi silloin omalla vuorollaan, mikä riittää tekoälylle. Moottorin tapahtuma tarvitaan vasta, jos pitkä komento halutaan keskeyttää heti (luku [11](11-avoimet-kysymykset.md)). Toistuva komento (`count`) tarkistaa näkyvyyden itse silmukassaan.

## 9.8 Esimerkkijäljet

Jäljet kertovat tapahtumien järjestyksen, ja ne kuuluvat moottorin testeihin.

```
spawn(GOBLIN, at: pos(3, 4))      // GOBLIN-kokoelmassa DAGGER
  created DAGGER · created GOBLIN

move(goblin, to: pos(4, 4))
  leave goblin (3,4) · enter goblin (4,4) prev (3,4)

move(dagger, to: goblin.at)       // tikari kokoelmasta maahan
  arrive dagger CAVE · enter dagger (4,4) prev none

destroy(goblin)                   // removed-käsittelijä ei pudota mitään
  removed GOBLIN

travel(player, FIELD_B, at: WEST, align: true)
  leave player (39,7) · depart player FIELD_A (39,7) · arrive player FIELD_B (0,7)

travel(player, LEVEL, at: START)  // LEVEL on transient: luodaan uudelleen
  leave player (12,5) · depart player LEVEL (12,5) · created … · arrive player LEVEL (3,9)

travel(robot, STAGE, at: LEFT)    // sama käsittelijä: travel(kitten, …) ja start(ENDING(…))
travel(kitten, STAGE, at: RIGHT)
start(ENDING(robot, kitten))
  toiminnon loppu: leave robot · depart robot FIELD · arrive robot STAGE
                   · leave kitten · depart kitten FIELD · arrive kitten STAGE
                   · ENDING alkaa, map = STAGE
```

---

← [8. Syöte](08-syote.md) · [Sisällys](README.md) · [10. Mitä moottori ei tee](10-mita-moottori-ei-tee.md) →
