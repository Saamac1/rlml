# RLML-vakiokirjasto (STD): muutokset

[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md) › Muutokset

Uusin ensin. Lisää uusi merkintä listan alkuun.

**Muutokset 31.10.2026 (lataajan ensimmäinen versio):** STD on tiedostoina kansiossa `lib/std/`. `FLEE`-komennon parametri on taulukossa `threat` kuten koodissa, koska `from` on parametrin lähteen avainsana (luku [5](05-komennot.md)). Lataajan löytämät avoimet kohdat ovat luvussa [14](14-avoimet-kysymykset.md).

**Muutokset 30.10.2026 (kielitiedoston otsikko):** STD:n kielitiedosto alkaa rivillä `language fi` (luku [9](09-aanet-viestit-ja-tekstit.md)). Minipeli on kokonaisena kansiossa `esimerkit/mini/` karttoineen, ja luku [12](12-minipeli-std-lla.md) viittaa siihen eikä enää testidokumenttiin.

**Muutokset 25.10.2026 (robotfindskitten-toteutustesti):** moduulin `move` riippuvuudet ovat relations, combat ja sound (luku [1.3](01-rajaus-periaatteet-ja-lataus.md#13-lataus-ja-moduulit)). Oletussuhteen ja paikanvaihdon sääntö on kirjattu törmäykseen (luku [5](05-komennot.md)), ja paikanvaihdon ehto on avoin kysymys (luku [14](14-avoimet-kysymykset.md)).

**Muutokset 25.10.2026 (Rogue-toteutustesti):** tapahtumat `HIT` ja `DIED` sekä funktio `HURT` saavat viimeiseksi parametrin `cause`, jonka käsittelijä saa jättää pois (luvut [4](04-tapahtumat.md) ja [8](08-kasittelijat.md)). Liikkeen ja iskun säännöt ovat ytimissä `CAN_STEP`, `BLOCKER`, `DO_STEP` ja `STRIKE`, joita `STEP`, `ATTACK` ja pelin omat silmukat kutsuvat (luku [5](05-komennot.md)). `PICK_UP` siirtää esineen kutsulla `move(…, to:)` (luku [5](05-komennot.md)).

**Muutokset 25.10.2026 (Ultima 1 -toteutustesti):** seuraajan tekoäly `FOLLOWER` ja komponentti `FOLLOW` moduulissa `lib/std/ai.rlml` (luku [6.1](06-tekoaly.md#61-seuraaja)).
