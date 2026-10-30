[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 14. Avoimet kysymykset

- **Paikanvaihto oletussuhteella.** Oletussuhde on tasan `FRIENDLY`, joten kaikki entiteetit ilman suhteita ovat ystäviä, ja paikanvaihdon rajaa vain tekoälyehto (luku [5](05-komennot.md)). Vaatiiko paikanvaihto yksilö- tai ryhmäsuhteen eikä pelkkää alignmentin oletusta? Muutos koskisi seuraajia, joiden suhde on nyt oletus (luku [6.1](06-tekoaly.md#61-seuraaja)).

**Lataajan ensimmäinen versio 31.10.2026.** STD on nyt tiedostoina kansiossa `lib/std/`, ja lataaja tarkistaa sen. Seuraavat kohdat jouduttiin arvaamaan:

- **Moduulien riippuvuudet.** Luvun [1.3](01-rajaus-periaatteet-ja-lataus.md#13-lataus-ja-moduulit) taulukon mukaan `core` ei tarvitse muita moduuleja, mutta `DEFAULT.PLAYER` käyttää komponentteja `HP`, `MELEE`, `PACK`, `ALIGNMENT` ja `FACTION`, tekoälyä `HUNTER` ja tileä `GLYPH.PLAYER`. Pelkkä `include lib/std/core.rlml` antaa siksi seitsemän latausvirhettä. Samoin `combat` käyttää datassa tileä `GLYPH.MISSILE` ja logiikassa nimiä `HOSTILES` (ai), `PACK` (items) ja `SOUND.SHOT` (sound). Pitäisikö `core`:n ottaa mukaan nämä moduulit, vai siirretäänkö `DEFAULT.PLAYER` omaan moduuliinsa?
- **Moduulittomat nimet.** Taulukko ei sijoita nimiä `SNEAK`, `SNEAKING`, `FIGHT`, `FEED`, `FOOD`, `LISTENER`, `HEAR_MESSAGE`, `WITNESSED`, `FOLLOW`, `LEADER_OF`, `FOLLOWER`, `AMMO` eikä vakioita `TOLERANCE` … `KIN_RANGE`. Toteutus sijoittaa ne näin: liike (`SNEAK`, `SNEAKING`, `FIGHT`, `FEED`, `FOOD`), ääni (`LISTENER`, `HEAR_MESSAGE`), core (`WITNESSED`), tekoäly (`FOLLOW`, `LEADER_OF`, `FOLLOWER`), taistelu (`AMMO`) ja suhteet (vakiot).
- **Kortin ylikirjoitus `PLAYER HERO HP 20`.** Luvun [3](03-komponentit-ja-oletuspelaaja.md) esimerkki kirjoittaa kaksikenttäisen `HP`:n ilman kentän nimeä, mikä on latausvirhe (dataformaatit, luvut [5.2](../dataformaatit/05-komponentit-ja-entiteetit.md#52-entiteetit) ja [6.4](../dataformaatit/06-pakat-ja-taulut.md#64-kenttien-ylikirjoitus)). Pitäisikö esimerkin olla `- PLAYER HERO HP now 20`?
- **Englanninkieliset tekstit.** Luku [1.3](01-rajaus-periaatteet-ja-lataus.md#13-lataus-ja-moduulit) lupaa tiedoston `lib/std/text/en.rlml`, mutta luku [9](09-aanet-viestit-ja-tekstit.md) antaa vain suomenkieliset tekstit. Toteutuksessa on toistaiseksi vain `fi.rlml`.

**Ratkaistu 25.10.2026 (robotfindskitten-toteutustesti):**

- **Moduulin `move` riippuvuudet** ovat relations, combat ja sound (luku [1.3](01-rajaus-periaatteet-ja-lataus.md#13-lataus-ja-moduulit)).

**Ratkaistu 25.10.2026 (Rogue-toteutustesti):**

- **Kuoleman syy** on `HIT`-, `HURT`- ja `DIED`-parametri `cause` (luku [4](04-tapahtumat.md)).
- **Askel ja isku silmukassa:** ytimet `CAN_STEP`, `BLOCKER`, `DO_STEP` ja `STRIKE` (luku [5](05-komennot.md)). Kaukoaseiden linjafunktio on nyt `STRIKE_LINE`.
- **`PICK_UP`** käyttää moottorin `move(…, to:)`-muotoa (luku [5](05-komennot.md)).

**Ratkaistu 24.10.2026:**

- **Ryhmät** ovat entiteettejä, ja `is` hyväksyy nimiarvon (luku [7](07-suhteet-ja-vihamielisyys.md)). Ryhmien nimet ovat tarkistettuja entiteettien nimiä, joten vapaita nimiä ei tarvita.
- **Tekstipääte** `{NIMI:PÄÄTE}` näyttää nimen muun tekstin kuin `NAME` (dataformaatit, luku [10](../dataformaatit/10-tekstit.md)).
- **Reitti** ei sisällä lähtöruutua (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).
- **Generointi:** `NOISE` on nyt `SCATTER_TILES` (luku [10](10-generointi.md)).
- **Ulkoasu:** STD toimittaa tyylirivit omille tileilleen (luku [1.3](01-rajaus-periaatteet-ja-lataus.md#13-lataus-ja-moduulit)).
- **Kuollut pelaaja** näkee viimeisen näkökenttänsä, joten `WITNESSED` toimii kuoleman jälkeen (luku [8](08-kasittelijat.md)).
- **Törmäys** syntyy vain liikkeen estosta (`tiles: false`, luku [5](05-komennot.md)).
- **Suunnat:** oletus on kahdeksan suuntaa ilman seinän kulman ohitusta, ja `movement`-kentät `dirs` ja `corners` muuttavat sitä (luku [5](05-komennot.md)).
- **Kaukoaseet:** kaksi lentotapaa, suora ja kaari, ja osuma ampujan taidon ja kohteen pelastusheiton vertailuna (luku [5.1](05-komennot.md#51-kaukoaseet)). Aluevaikutukset kuuluvat myöhempään vaiheeseen.

**Ratkaistu 25.10.2026 (valmiustesti 2):**

- **Viholliset** ovat vastavuoroisia (`ENEMIES`), hyökkäys synnyttää vihaa (`PROVOKE`), ja alignment rajaa lyöntejä (luku [7.1](07-suhteet-ja-vihamielisyys.md#71-viholliset-ja-ystävät)). Tarkoituksellinen lyönti neutraaliin on `FIGHT`.
- **Pelaajan tekoäly**: oletuspelaajalla on `HUNTER` joutokäyntiä varten (luku [3](03-komponentit-ja-oletuspelaaja.md)).
- **Ammukset** ovat yksi pino, jolla on `AMMO count` (luku [5.1](05-komennot.md#51-kaukoaseet)).
- **Metsästäjä** lyö vain, jos sillä on `MELEE` (luku [6](06-tekoaly.md)).
- **Hiipiminen**: `SNEAK` tekee askeleesta hiljaisen (`SOUND.SNEAK` level 1), mutta askel maksaa 240. Omat askeleet peittävät hiljaisemmat äänet, joten kuunteleva pysyy paikallaan tai hiipii.
- **Ruokinta** on `FEED`, ja ruoka on `FOOD`-komponentti (luvut [3](03-komponentit-ja-oletuspelaaja.md) ja [5](05-komennot.md)).
- **Ystävät** vaihtavat paikkaa törmätessään (luku [5](05-komennot.md)).
- **Kuuloviesti** kirjataan enintään kerran herätyksellä (`LISTENER`, luku [8](08-kasittelijat.md)).
- **Tyylit**: STD ei kirjoita omaa `default`-riviä (luku [3](03-komponentit-ja-oletuspelaaja.md)).

---

← [13. Muutokset muihin dokumentteihin](13-muutokset-muihin-dokumentteihin.md) · [Sisällys](README.md)
