[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 4. Satunnaisuus ja toistettavuus

## 4.1 Satunnaisvirrat

Moottori ylläpitää kolmea virtaa ([RS 5.4](../roguescript/05-funktiot-ja-lausekkeet.md#54-satunnaisuus)):

| Virta | Alustus | Käyttäjät | Tallennetaan |
| --- | --- | --- | --- |
| pelin virta | pelin siemen (`game.seed`) | `fn`, komento, käsittelijä, tekoäly, generaattori, datan nopat, pakkojen sekoitukset ja taulut | kyllä |
| kyselyn kopio | pelin virran nykytila sekoitettuna kutsujaan ja kyselyn nimeen | `query` | ei, virta ei etene |
| esityksen virta | siemen, kohtauksen nimi ja käynnistyksen järjestysnumero | `present fn`, kohtausten esitysrivit | ei |

Moottori valitsee virran kutsujan lajin perusteella. Kirjoittaja ei valitse virtaa itse.

**Algoritmi.** Kaikki toteutukset käyttävät samaa generaattoria, jotta sama siemen tuottaa saman pelin ([STD 1.1](../vakiokirjasto/01-rajaus-periaatteet-ja-lataus.md#11-mikä-std-on)):

- **Generaattori** on xoshiro256\*\*. Sen neljän 64-bittisen sanan tila alustetaan siemenestä neljällä peräkkäisellä SplitMix64-tulosteella. Siemen on 64-bittinen etumerkitön luku, ja negatiivinen `game.seed` luetaan kahden komplementtina.
- **Väli.** `below(n)` (n ≥ 1) on Lemiren menetelmä ilman vinoumaa: x = seuraava tuloste, m = x · n 128-bittisenä, ja jos m:n alin 64 bittiä on alle (2⁶⁴ − n) mod n, nostetaan uusi x. Tulos on m:n ylin 64 bittiä. Kokonaislukuväli `a..b` on `a + below(b − a + 1)`, ja tyhjä väli ei nosta mitään.
- **Noppa.** `NdM+K` on N erillistä heittoa `1 + below(M)` järjestyksessä, ja niiden summaan lisätään K. Väli lukukentässä (`-1..3`) on yksi heitto välistä.
- **`chance(p)`** nostaa aina yhden heiton `1 + below(100)` ja on tosi, kun heitto on enintään p.
- **Joukosta.** `choose` nostaa indeksin `below(count)` joukon järjestyksessä (luku [4.2](#42-määrätty-järjestys)), ja tyhjä joukko ei nosta mitään. `pick(j, n)` ja `shuffled` ovat Fisher–Yatesin sekoitus alusta päin: paikalle i (0, 1, …) vaihdetaan jäsen `i + below(count − i)`, ja `pick` lopettaa n:n paikan jälkeen. Pakan sekoitus on sama koko nostopinolle.
- **Sekoitusfunktio.** `mix(h, x)` on yksi SplitMix64-askel tilasta `h xor x`. Nimi sekoitetaan tavu kerrallaan UTF-8-muodossa, ja nimien väliin sekoitetaan tavu 0.
- **Kyselyn kopio.** h on kutsujan virran neljän tilasanan xor. Siihen sekoitetaan kutsujan täysi nimi ja kyselyn täysi nimi, ja kopio alustetaan h:sta kuten pelin virta siemenestä. Kyselyn sisäinen kysely johtaa kopionsa samoin sen kyselyn kopiosta, joka sitä kutsuu.
- **Esityksen virta.** h on siemen, johon sekoitetaan kohtauksen täysi nimi ja sen käynnistyksen järjestysnumero, ja virta alustetaan h:sta.

## 4.2 Määrätty järjestys

Toistettavuus vaatii, että satunnaisuuden lisäksi kaikki järjestys on määrätty:

- **Joukot:** entiteetit luontijärjestyksessä, ruudut lukujärjestyksessä (ylhäältä alas, vasemmalta oikealle). Poikkeus ovat istuinten jäsenet `players` ja `members(n)`, jotka ovat istuinnumeron ja jäseneksi tulon järjestyksessä (luku [6.1.10](06-ajo.md#6110-istuimet-ja-ryhmä)). Listat säilyttävät oman järjestyksensä ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)).
- **Käsittelijät:** prioriteetti, sitten latausjärjestys; kun sama käsittelijä osuu useaan kohteeseen, subjekti tulee ennen `modifies`-jäseniä ja jäsenet luontijärjestyksessä (luku [6.2](06-ajo.md#62-tapahtumat), [RS 4](../roguescript/04-tapahtumat-ja-kasittelijat.md)).
- **Herätykset:** hetki, sitten järjestysnumero (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)).
- **Mikroaskeleen vaiheet:** kierros, ajastimet, taustakohtaukset ja herätykset kiinteässä järjestyksessä (luku [6.1.4](06-ajo.md#614-mikroaskeleen-vaiheet)).
- **Toiminnon loppu:** siirrot, `rollback`, `checkpoint`, `delete_save`, `autosave` ja profiili sekä etualan kohtaukset kiinteässä järjestyksessä (luku [6.1.7](06-ajo.md#617-toiminto)).
- **Elinkaari:** kokoelman jäsenten `created` ennen kantajan omaa, kantajan `removed` ennen jäsenten omia (luvut [3.2](03-pelin-tila.md#32-ilmentymän-syntyminen) ja [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)).
- **Aikajanat:** rivit ajan mukaan, saman ajan rivit kirjoitusjärjestyksessä ([RS 9.1](../roguescript/09-kohtaukset-ja-aikajanat.md#91-rivi)).
- **Kohtausten jono:** aloitusjärjestys.
- **Aloitus:** nopat heitetään kiinteässä laskentajärjestyksessä (luku [5.1](05-aloitus-tallennus-ja-checkpointit.md#51-uusi-peli)).
- **Tiedostot:** jokerit aakkosjärjestyksessä.
- **Paikat:** latausjärjestyksessä (luku [2.4](02-lataus.md#24-korvaaminen-ja-paikkaaminen)).

## 4.3 Yksi ruutu joukosta

Kun käyttöpaikka tarvitsee yhden ruudun ja saa joukon (`at`, `move`, `spawn`, `tile`), moottori arpoo ruudun kutsujan virrasta indeksillä `below(count)` joukon lukujärjestyksessä (lista omassa järjestyksessään, luku [4.1](#41-satunnaisvirrat)) ([D 8.8](../dataformaatit/08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from), [RS 6.2](../roguescript/06-ruudut-ja-paikkahaku.md#62-operaatiot)). Siirtymässä saapumisruutu arvotaan samoin, ellei `align` pyydä samaa indeksiä (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)).

## 4.4 Syöteloki

Syöteloki kirjaa pelikerran syötteet siinä järjestyksessä kuin moottori ajaa ne. Pelin kulku määräytyy kokonaan lähtötilasta ja lokista: sama data, sama lähtötila ja sama loki tuottavat saman pelin. Toisto ajaa lokin rivi kerrallaan ilman laitteita, ja `rlml run` -transkriptit ovat toistoja.

- **Lähtötila.** Uudessa pelissä lähtötila on aloituksen syötteet: siemen, profiilin tilannekuva ja liitettyjen istuinten määrä (luku [5.1](05-aloitus-tallennus-ja-checkpointit.md#51-uusi-peli)). Toisto laskee niistä checkpointin 0 kuten uusi peli. Lokin lähtötila voi olla myös tallennus (luku [5.2](05-aloitus-tallennus-ja-checkpointit.md#52-tallennus)). Silloin toisto lataa tallennuksen ja ajaa lokin siitä eteenpäin.
- **Rivi** on (istuin, toiminto, argumentit) ja hetki `game.time`, jolloin moottori ajoi sen. Istuin on se, jonka syötettä moottori odotti, tai `join`-rivillä uusi istuin. Yksinpelissä istuin on 1.
- **Kierrokset ilman vuoroa.** Kun moottori ei odota kenenkään syötettä, kierrokset kulkevat `round`-tahdissa tai `pass`-painalluksesta (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)). Ne eivät ole rivejä. Toisto ajaa kierroksia, kunnes seuraavan rivin hetki on käsillä.
- **Hetki on tarkistus.** Toisto vertaa jokaisen rivin hetkeä omaan `game.time`-arvoonsa ennen rivin ajoa. Jos hetket eroavat, data tai moottori on muuttunut lokin kirjoittamisen jälkeen. Toisto pysähtyy silloin ajon virheeseen, joka kertoo rivin, eikä ajoa jatketa väärällä syötteellä.
- **Loppu.** Lokin viimeinen rivi on `end` hetkineen, joten toisto ajaa myös viimeisen syötteen jälkeiset kierrokset.

**Mitä kirjataan.** Lokiin kirjataan syöte, jonka vaikutus riippuu muustakin kuin pelin tilasta. Rivi kirjataan vasta, kun moottori ajaa toiminnon, ei painalluksen hetkellä.

| Syöte | Rivi |
| --- | --- |
| pelaajan komento | komento täytettyine argumentteineen, myös parametrikyselyn vastaukset, `count`-lähteen luku ([RS 7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)) ja verbivalikon klikkaukset (luku [8.1](08-syote.md#81-hiiri)); näppäimiä, klikkauksia ja kyselyn vaiheita ei kirjata |
| `idle`-ajastimen laukeaminen | ajettu komento kuten pelaajan komento; `idle` on pelaajan asetus eikä pelin dataa ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)), joten toisto ei laske komentoa uudelleen (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)) |
| klikkausmatka | `travel_to` kohderuutuineen, koska osoitin ei kuulu tilaan; toisto laskee askeleet uudelleen (luku [8.1](08-syote.md#81-hiiri)) |
| seuraaminen | `follow` kohteineen, myös `leader`-kyselyn antama kohde, ja jokainen seuraajan vuoro rivinä `follow.step`, jonka toisto laskee uudelleen; matkan askeleet ovat samoin rivejä |
| matkan tai seuraamisen katkaisu | `stop`; painallus, joka katkaisee, ja vanhentuneen ennakkokäskyn peruutus kirjataan näin, koska talteen otettu näkökenttä ei kuulu tilaan |
| ennakkokäsky | käskyn matka tai seuraaminen silloin, kun moottori ajaa sen, ei silloin, kun käsky annettiin |
| istuimet | `join` ja `leave_seat` (luku [8.2](08-syote.md#82-istuimet-ja-laitteet)) |
| valikon kohta, joka ajaa komennon tai pelin toiminnon | ajettu komento tai toiminto kuten pelaajan komento |
| kohtauksen kuittaus (`say`, `wait confirm`, modaalinen paneeli) | `scene.confirm` |
| dialogin valinta | `scene.choose KOHTA` kohdan nimellä eikä järjestysnumerolla, koska piilotetut kohdat muuttavat numeroita ([RS 9.7](../roguescript/09-kohtaukset-ja-aikajanat.md#97-dialogit)) |
| säädettävä kohta | `scene.adjust KOHTA +1` tai `-1` |
| luku- ja tekstikenttä | `scene.answer ARVO`, peruutus `scene.answer none` |
| `open`-valikon sulkeminen kohtauksessa | `scene.close`; valikon kohdat kirjataan kuten muutkin valikon toiminnot |
| ohitus, alusta aloitus ja kelaus | `skip`, `restart` ja `rewind` ([RS 9.8](../roguescript/09-kohtaukset-ja-aikajanat.md#98-luonnos-ohitus-ja-alusta-aloitus)) |

**Mitä ei kirjata.** Esitys ei kuulu lokiin, koska se ei muuta tilaa: osoitin ja hover, kursorin liike, valikkojen selaus, lukunäkymä, välilehdet, kamera, valittu verbi ennen komentoa ja ennakkokäsky ennen ajoa. Moottorin toiminnot, jotka eivät muuta pelin kulkua, eivät myöskään kuulu lokiin: `save`, `help` ja taukovalikon avaus ja sulku. Logiikan omat kutsut (`rollback`, `checkpoint`, `autosave`) seuraavat lokin riveistä, joten niistä ei kirjata rivejä. Etualan aikajanan kello ei ole syöte: toisto ajaa aikajanan rivit niiden järjestyksessä, ja vain pelaajan vastaukset ovat rivejä. Siksi kohtaus toistuu samanlaisena, vaikka pelaaja katsoi sen eri tahdissa. Ohituksen hetkeä ei myöskään kirjata: ohitus kirjaa jäljellä olevat repliikit ja viestit lokiin, joten tila ja loki ovat samat, missä kohdassa pelaaja ohittikin ([RS 9.8](../roguescript/09-kohtaukset-ja-aikajanat.md#98-luonnos-ohitus-ja-alusta-aloitus)). Toisto ajaa aikajanan kellon seuraavaan pysähdykseen ennen seuraavaa riviä.

**Lokin loppu.** `quit`, `new_game` ja `load_game` päättävät lokin. Ladattu peli aloittaa uuden lokin, jonka lähtötila on ladattu tallennus. Profiili kirjoitetaan tiedostoon pelikerran aikana (luku [5.6](05-aloitus-tallennus-ja-checkpointit.md#56-profiili)), mutta toisto käyttää lähtötilan tilannekuvaa. Siksi toisto ei riipu koneen profiilista, eikä se kirjoita profiilia.

**Muoto** on moottorin oma kuten tallennuksen. Esitysmuoto alla on havainnollistus, ei tiedostomuoto:

```
seed 7  seats 2
0      1 STD.MOVE dir: north
120    2 STD.MOVE dir: west
240    1 travel_to pos(14, 6)
240    1 follow.step
360    1 stop
360    2 STD.WAIT
840    3 join
960    end
```

---

← [3. Pelin tila](03-pelin-tila.md) · [Sisällys](README.md) · [5. Aloitus, tallennus ja checkpointit](05-aloitus-tallennus-ja-checkpointit.md) →
