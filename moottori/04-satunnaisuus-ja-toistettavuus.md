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

## 4.2 Määrätty järjestys

Toistettavuus vaatii, että satunnaisuuden lisäksi kaikki järjestys on määrätty:

- **Joukot:** entiteetit luontijärjestyksessä, ruudut lukujärjestyksessä (ylhäältä alas, vasemmalta oikealle). Listat säilyttävät oman järjestyksensä ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)).
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

Kun käyttöpaikka tarvitsee yhden ruudun ja saa joukon (`at`, `move`, `spawn`, `tile`), moottori arpoo ruudun kutsujan virrasta ([D 8.8](../dataformaatit/08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from), [RS 6.2](../roguescript/06-ruudut-ja-paikkahaku.md#62-operaatiot)). Siirtymässä saapumisruutu arvotaan samoin, ellei `align` pyydä samaa indeksiä (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)).

---

← [3. Pelin tila](03-pelin-tila.md) · [Sisällys](README.md) · [5. Aloitus, tallennus ja checkpointit](05-aloitus-tallennus-ja-checkpointit.md) →
