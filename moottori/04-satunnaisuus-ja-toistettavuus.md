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

---

← [3. Pelin tila](03-pelin-tila.md) · [Sisällys](README.md) · [5. Aloitus, tallennus ja checkpointit](05-aloitus-tallennus-ja-checkpointit.md) →
