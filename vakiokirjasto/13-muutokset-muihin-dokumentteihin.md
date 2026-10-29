[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 13. Muutokset muihin dokumentteihin

Yhtenäistetty 25.10.2026. Muiden dokumenttien STD-esimerkit viittaavat nyt tähän dokumenttiin:

- **RogueScript 4:** vyöhykkeet pätevät, kun vakiokirjaston include on pelin ensimmäinen, ja esimerkki on `STD.HIT`, `STD.DIED` ja `MESSAGE.COMBAT.HIT`. Käsittelijä saa jättää oletusarvoiset loppuparametrit pois, joten `cause` ei riko vanhoja käsittelijöitä.
- **RogueScript 5.1 ja 7.2:** `HOSTILES` ja `HUNTER` viittaavat lukuun [6](06-tekoaly.md), ja tekoälyn esimerkki on nyt `GUARD`. Tekoäly voi kutsua toista tekoälyä (`HUNTER()`).
- **RogueScript 8.3:** generointikoodi on luvussa [10](10-generointi.md), ja `ROOMS` palauttaa listan `list area`.
- **RogueScript 10.1:** `SPAWN_PLAYER` sisältää oletuspelaajan haaran (luku [8](08-kasittelijat.md)).
- **Dataformaatit 3:** include-esimerkit käyttävät tiedostoa `lib/std.rlml`, ja kansiolistaus mainitsee `lib/std/`.
- **Moottoridokumentti 9.7:** vakiokirjaston tapahtumataulukko viittaa lukuun [4](04-tapahtumat.md).

---

← [12. Minipeli STD:llä](12-minipeli-std-lla.md) · [Sisällys](README.md) · [14. Avoimet kysymykset](14-avoimet-kysymykset.md) →
