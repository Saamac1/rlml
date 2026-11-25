# RLML-vakiokirjasto (STD): muutokset

[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md) › Muutokset

Uusin ensin. Lisää uusi merkintä listan alkuun.

**Muutokset 25.11.2026 (ryhmä uloskäynnillä):** kunnes kohtaukset toimivat, `USE_EXIT` kirjoittaa viestin `MESSAGE.EXIT.OTHERS_AWAY` ja epäonnistuu, kun muut jäsenet ovat kaukana, sen sijaan että käynnistäisi dialogin `EXIT.LEAVE_OTHERS`. Aiemmin peli kaatui siihen (luvut [5](05-komennot.md), [9](09-aanet-viestit-ja-tekstit.md) ja [14](14-avoimet-kysymykset.md)).

**Muutokset 25.11.2026 (portaat askeleella):**
- `EXIT`-komponentilla on kenttä `step`. Istuimen jäsen, joka astuu tällaisen uloskäynnin ruutuun, käyttää sitä (`EXIT_STEP`).
- `TAKE_EXIT`-komennon runko on ydin `USE_EXIT`.
- `EXIT_CELL` on valmis `walk_avoid`-kysely.
- Kirjattiin avoin kysymys siitä, miten muut kuin jäsenet käyttävät portaita, kun moottori ajaa useaa aktiivista karttaa (luvut [3](03-komponentit-ja-oletuspelaaja.md), [5](05-komennot.md), [8](08-kasittelijat.md) ja [14](14-avoimet-kysymykset.md)).
- Minipelin portaat toimivat askeleella, ja sen napeista poistuivat portaat ja ohita (luku [12](12-minipeli-std-lla.md)).

**Muutokset 25.11.2026 (minipelin kosketusnapit):** minipelissä on pystyssä olevan puhelimen asettelu `PHONE` kosketusnappeineen, `walk STD.MOVE` karttaa napauttamalla kävelyyn ja taukovalikon `SAVE` (luku [12](12-minipeli-std-lla.md)).

**Muutokset 17.11.2026 (seurattava):** `LEADER_OF` antaa ensin lähimmän ihmisen ohjaaman jäsenen (`controlled`) ja vasta sitten lähimmän muun jäsenen, joten `follow` ei ala seurata toisen istuimen seuraajaa, joka seisoo yhtä lähellä (luku [6.1](06-tekoaly.md#61-seuraaja)).

**Muutokset 8.11.2026 (ohjaus lyhyillä vuoroilla):** `LEADER_OF`, `WAIT` ja `FOLLOWER` moottorin seuraamisen kenttinä `leader`, `rest` ja `follow_ai` (luku [6.1](06-tekoaly.md#61-seuraaja)).

**Muutokset 7.11.2026 (tanssijumi):** `STEP` ei vaihda paikkaa kahden ihmisen ohjaaman hahmon välillä (`controlled`), joten samaan suuntaan kulkevat pelaajat eivät jää vaihtamaan paikkaa keskenään (luku [5](05-komennot.md)).

**Muutokset 7.11.2026 (ryhmäesimerkki):** paikanvaihto ystävän kanssa ei enää esty törmääjän omasta estosta (`except: actor`, luku [5](05-komennot.md)), ja `LEADER_OF` ei anna jäsenelle johtajaksi jäsentä itseään (luku [6.1](06-tekoaly.md#61-seuraaja)).

**Muutokset 6.11.2026 (moninpeli):** `SPAWN_PLAYER` luo hahmon jokaiselle istuimelle, `JOIN_PLAYER` liittyvälle ja `PART_PLAYER` tekee poistuvan istuimen jäsenistä seuraajia (luvut [6.1](06-tekoaly.md#61-seuraaja) ja [8](08-kasittelijat.md)). `LEADER_OF` palauttaa ilman kohdetta lähimmän jäsenen (luku [6.1](06-tekoaly.md#61-seuraaja)). Talutushihna `LEASHED`, paikanvaihto jäsenen kanssa, ryhmän `TAKE_EXIT` ja `GROUP_ARRIVAL` sekä `SWITCH` ja `SWITCH.PREV` ovat uusia (luku [5](05-komennot.md)). Jäsenet eivät suutu toisiinsa (`PROVOKE`, luku [7](07-suhteet-ja-vihamielisyys.md)). `WITNESSED` käyttää yhteistä näkökenttää, viestit nimeävät jäsenen, kun pelaajia on useampi (`SOLO`, `ALLY_PICKUP`, `ALLY_DROP`), ja kuuloviesti kirjataan kerran ajanhetkessä ryhmälle (luvut [8](08-kasittelijat.md) ja [9](09-aanet-viestit-ja-tekstit.md)). Moduulit ja prioriteetit ovat luvuissa [1.3](01-rajaus-periaatteet-ja-lataus.md#13-lataus-ja-moduulit) ja [2](02-prioriteettivyohykkeet.md), avoimet kysymykset luvussa [14](14-avoimet-kysymykset.md).

**Muutokset 31.10.2026 (ajon ensimmäinen versio):** viestien järjestys kuolemassa on avoin kysymys (luku [14](14-avoimet-kysymykset.md)). Minipelin luolat ovat valaistuja (`default … lit`, luku [12](12-minipeli-std-lla.md)).

**Muutokset 31.10.2026 (tyyppitarkistus):** `SPAWN_PLAYER` ottaa hallintaan kortista syntyneen kokoelman ensimmäisen jäsenen (`spawn(…).first`), koska kortin `spawn` palauttaa kokoelman ja `control` ottaa entiteetin (luku [8](08-kasittelijat.md); moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).

**Muutokset 31.10.2026 (lataajan ensimmäinen versio):** STD on tiedostoina kansiossa `lib/std/`. `FLEE`-komennon parametri on taulukossa `threat` kuten koodissa, koska `from` on parametrin lähteen avainsana (luku [5](05-komennot.md)). Lataajan löytämät avoimet kohdat ovat luvussa [14](14-avoimet-kysymykset.md).

**Muutokset 30.10.2026 (kielitiedoston otsikko):** STD:n kielitiedosto alkaa rivillä `language fi` (luku [9](09-aanet-viestit-ja-tekstit.md)). Minipeli on kokonaisena kansiossa `esimerkit/mini/` karttoineen, ja luku [12](12-minipeli-std-lla.md) viittaa siihen eikä enää testidokumenttiin.

**Muutokset 25.10.2026 (robotfindskitten-toteutustesti):** moduulin `move` riippuvuudet ovat relations, combat ja sound (luku [1.3](01-rajaus-periaatteet-ja-lataus.md#13-lataus-ja-moduulit)). Oletussuhteen ja paikanvaihdon sääntö on kirjattu törmäykseen (luku [5](05-komennot.md)), ja paikanvaihdon ehto on avoin kysymys (luku [14](14-avoimet-kysymykset.md)).

**Muutokset 25.10.2026 (Rogue-toteutustesti):** tapahtumat `HIT` ja `DIED` sekä funktio `HURT` saavat viimeiseksi parametrin `cause`, jonka käsittelijä saa jättää pois (luvut [4](04-tapahtumat.md) ja [8](08-kasittelijat.md)). Liikkeen ja iskun säännöt ovat ytimissä `CAN_STEP`, `BLOCKER`, `DO_STEP` ja `STRIKE`, joita `STEP`, `ATTACK` ja pelin omat silmukat kutsuvat (luku [5](05-komennot.md)). `PICK_UP` siirtää esineen kutsulla `move(…, to:)` (luku [5](05-komennot.md)).

**Muutokset 25.10.2026 (Ultima 1 -toteutustesti):** seuraajan tekoäly `FOLLOWER` ja komponentti `FOLLOW` moduulissa `lib/std/ai.rlml` (luku [6.1](06-tekoaly.md#61-seuraaja)).
