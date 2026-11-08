[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 10. Mitä moottori ei tee

- **Ei pelin sääntöjä.** Vahinko, kuolema, parantuminen, inventaarion säännöt ja tilavaikutukset ovat vakiokirjastoa tai peliä.
- **Ei inventaariota.** Moottori siirtää ilmentymiä kokoelmiin ja pitää pinot ajan tasalla mutta ei tiedä, mikä kokoelma on reppu. Esineen antaminen on `STD.GIVE` (luku [9.7](09-moottorin-funktiot-ja-tapahtumat.md#97-raja-vakiokirjastoon)).
- **Ei pelaajan luontia.** Moottori ei tunne pelaajaentiteettiä nimeltä. Logiikka luo pelaajan ja antaa sen hallintaan funktiolla `control`, ja vakiokirjasto tekee tämän oletuksena ([RS 10.1](../roguescript/10-aloitus-ja-checkpointit.md#101-aloitus)).
- **Ei kulkukelpoisuutta.** Paikkahaku tuntee tilet mutta ei tiedä, mihin ruutuun voi astua ([D 13](../dataformaatit/13-avoimet-kysymykset.md)). Liikkeen ehdot ja törmäys (`STD.CAN_STEP`, `STD.BUMP`) kirjoitetaan komentoon. Klikkausmatka ja seuraaminen vain ajavat pelin `walk`- ja `rest`-komentoja tai `follow_ai`-tekoälyä (luku [8.1](08-syote.md#81-hiiri)).
- **Ei tulkitsevia tapahtumia.** Moottori kertoo, mitä tapahtui sijainnille, elinkaarelle ja ajalle. Törmäys, vierekkäisyys, poiminta ja kuolema ovat vakiokirjaston tapahtumia (luku [9.7](09-moottorin-funktiot-ja-tapahtumat.md#97-raja-vakiokirjastoon)).
- **Ei checkpointien ajoitusta eikä pelin loppua.** Moottori tarjoaa `checkpoint()`-, `rollback()`- ja `delete_save()`-kutsut, ja logiikka päättää niiden käytöstä. Kuolema ei päätä peliä, ja voitto on logiikan kohtaus ja tarvittaessa `final`-valikko (luvut [5.3](05-aloitus-tallennus-ja-checkpointit.md#53-checkpointit) ja [7.5](07-esitys.md#75-valikot)).
- **Ei tekoälyn päätöksiä.** Moottori kutsuu tekoälyfunktiota ja suorittaa sen palauttaman komennon.
- **Ei komentojen toistoa.** Toistolaskuri antaa luvun komennolle, ja komento toistaa itse (luku [8](08-syote.md)).
- **Ei pistelistaa.** Moottori tallentaa profiilin, mutta pistelistan sisältö, järjestys ja hautakivi ovat pelin dataa ja logiikkaa ([D 3.5](../dataformaatit/03-tiedostot-ja-include.md#35-profiili-profile)).
- **Ei isoja nimiä.** Moottori ei tunne yhtään kirjoittajan komponenttia, pakkaa, taulua, viestiä tai merkkiä nimeltä, ei myöskään vakiokirjaston nimiä.
- **Ei ajonaikaista kirjoitusta projektiin,** paitsi äänten renderöinti kehityksessä.

---

← [9. Moottorin funktiot ja tapahtumat](09-moottorin-funktiot-ja-tapahtumat.md) · [Sisällys](README.md) · [11. Avoimet kysymykset](11-avoimet-kysymykset.md) →
