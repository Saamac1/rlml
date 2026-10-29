[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 1. Rajaus ja periaatteet

## 1.1 Vastuunjako

| Kerros | Kuka kirjoittaa | Vastaa |
| --- | --- | --- |
| Data | pelin tekijä | mitä pelissä on: entiteetit, kartat, pakat, taulut, tekstit, ulkoasu, näppäimet |
| RogueScript | pelin tekijä | miten peli toimii: komponenttien käsittelijät, komennot, tekoäly, kohtaukset |
| Vakiokirjasto `STD` | moottorin mukana, korvattavissa | pelin yleiset säännöt: vahinko, kuolema, inventaario ja `GIVE`, pelaajan luonti, liikkeen ja lähitaistelun ytimet (`CAN_STEP`, `BLOCKER`, `DO_STEP`, `STRIKE`), törmäys (`BUMP`), vierekkäisyys (luku [9.7](09-moottorin-funktiot-ja-tapahtumat.md#97-raja-vakiokirjastoon)) ja generointialgoritmit ([STD 10](../vakiokirjasto/10-generointi.md)) |
| Moottori | moottorin tekijä | lataus, tila, ajo, esitys, syöte ja tallennus |

Raja kulkee kirjainkoossa: moottori lukee vain pienillä kirjoitettuja nimiä eikä tunne yhtään isolla kirjoitettua nimeä ([D 2.1](../dataformaatit/02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). Vakiokirjasto on isoilla kirjoitettua RLML:ää ja RogueScriptiä nimiavaruudessa `STD`, joten se on moottorin näkökulmasta tavallista pelin sisältöä. Säännön poikkeukset ovat avoin kysymys (luku [11](11-avoimet-kysymykset.md)).

## 1.2 Periaatteet

- **Moottori ei tunne pelin käsitteitä.** Kuolema, vahinko, inventaario ja pelaaja ovat sääntöjä ja sisältöä, eivät moottorin ominaisuuksia ([RS 1](../roguescript/01-rajaus-ja-periaatteet.md)). Moottori tarjoaa tapahtumat ja funktiot, joiden varaan säännöt rakennetaan.
- **Tyhjä peli ei sisällä mitään.** Moottori käynnistää järjestelmän vain, jos sen osio on kirjoitettu ([D 1.1](../dataformaatit/01-rajaus-ja-periaatteet.md#11-periaatteet)). Ilman ääni-ilmoituksia ei ole ääntä, ilman `terrain`-osiota on yksi maastokerros, ilman `profile`-lohkoa ei ole profiilia.
- **Toistettavuus on moottorin lupaus.** Sama data ja sama siemen tuottavat saman aloituksen, samat nopanheitot ja saman kulun samoilla syötteillä (luku [4](04-satunnaisuus-ja-toistettavuus.md)). Profiili kuuluu syötteisiin: pelikerta näkee profiilin aloitushetken tilannekuvan (luku [5.6](05-aloitus-tallennus-ja-checkpointit.md#56-profiili)).
- **Virheet kerrotaan aikaisin ja tarkasti.** Kaikki, minkä voi tarkistaa ilman pelin tilaa, tarkistetaan latauksessa (luku [2.5](02-lataus.md#25-tarkistukset)).
- **Projekti on vain luettavaa.** Moottori kirjoittaa ajon aikana vain käyttäjädatakansioon ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)). Poikkeus on äänten renderöinti kehityksessä (luku [7.6](07-esitys.md#76-ääni)).

---

[Sisällys](README.md) · [2. Lataus](02-lataus.md) →
