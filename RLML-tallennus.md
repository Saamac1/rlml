# Tallennus – toteutusohje

Tämä on ohje Claude Codelle. Siinä kerrotaan, mitä tallennuksesta toteutetaan ja missä järjestyksessä. Speksi on luvuissa [M 4](moottori/04-satunnaisuus-ja-toistettavuus.md), [M 5](moottori/05-aloitus-tallennus-ja-checkpointit.md), [M 6.1.6–6.1.7](moottori/06-ajo.md#616-monivaiheinen-komento), [RS 10–11](roguescript/10-aloitus-ja-checkpointit.md) ja [D 3.3–3.5](dataformaatit/03-tiedostot-ja-include.md#33-aloitus). Tämä ohje ei muuta speksiä. Vaihe 0 tuo speksiin ne kohdat, jotka siitä vielä puuttuvat.

## Päätökset

- **Syöteloki ensin, tilannekuva sitten.** Ajo on jo deterministinen: sama siemen ja samat näppäimet antavat aina saman tulosteen. Siksi tehdään ensin syöteloki ja toisto. Ne testaavat sitten tilannekuvaa: tallennus ja lataus mistä tahansa syötteen odotuksesta tuottaa saman jatkon kuin keskeytymätön ajo.
- **Checkpoint ensin muistissa.** `State` ja `World` ovat jo `Clone`. Siksi `checkpoint()` ja `rollback()` voidaan tehdä kloonilla ennen tiedostomuotoa. Tiedostomuoto tulee vasta, kun jokainen tilan osa on tunnistettu.
- **Oma tekstimuoto, ei serdeä.** `rlml-core` riippuu nyt vain kahdesta Unicode-cratesta, ja se käännetään wasmiksi. Tallennus on rivipohjainen tekstitiedosto, jonka moottori kirjoittaa ja lukee itse. Teksti näkyy diffeissä ja golden-testeissä. Muoto on moottorin oma, kuten speksi sanoo (M 5.2), eikä sitä dokumentoida RLML:ksi.
- **Tallennuspaikka on trait.** Ydin ei käytä tiedostojärjestelmää. `rlml-core` saa `Store`-traitin (luku, kirjoitus, nimeäminen, poisto ja lista), ja sen toteuttavat `MemStore` testeille, `rlml` levylle ja myöhemmin `rlml-web` OPFS:lle. Tämä on se `Store`, jonka `RLML-web.md` siirsi tallennuksen yhteyteen.
- **Kohtauksia ei vielä ole.** Ajo hylkää vielä kohtaukset, dialogit ja `open`-valikot (`eval.rs:283`). Tallennukseen varataan niiden paikat, kuten jonossa oleva etualan kohtaus, taustakohtaukset ja tweenit, mutta niitä ei toteuteta. Automaattisen tallennuksen kohta `on scene` tarkistetaan latauksessa, mutta ajossa se ei laukea ennen kohtauksia.
- **Profiili on oma vaiheensa** (vaihe 6). Profiilia ei ole vielä toteutettu (`world.rs:682`). Tallennus varaa profiilin näkymälle paikan, ja vaihe 6 täyttää sen.

## Lähtötilanne (commit 4afdf63)

- **Tila.** `rt::state::State` sisältää ilmentymät, `game`-tietueen, kartat (myös muistetut ruudut), herätykset ja `next_seq`-laskurin, pakat, istuimet, `player`-kentän, `party_each`- ja `leash`-asetukset, istuinmäärän rajan ja viimeisimmän pelaajan kartan. Kaikki on `Clone` ja `PartialEq`.
- **`World`** sisältää tilan lisäksi pelin virran (`Rng`, neljä sanaa) sekä näitä kenttiä:
  - `log`
  - kuuloon liittyvät `sounds`, `sounds_made`, `heard_mark` ja `heard`
  - `last_view`
  - toiminnon aikaiset `created`, `travels`, `switch` ja `depth`

  Toiminnon aikaiset kentät ovat tyhjiä toimintojen välissä.
- **`Runner`** sisältää odottavat komennot (`waiting`, `Suspended` ja `Frame`), `woke`-kartan, odotetun istuimen ja esityksen tilan (`travels`, `orders`). Kommentti `run.rs`:ssä varoittaa jo, että latauksen on säilytettävä `next_seq` tai tyhjennettävä `woke`.
- **Odottava komento** on nyt tulkin kehyspino. `Frame` sisältää kopion jäsennyspuusta (`Vec<Stmt>`, `Expr`). Speksi vaatii rakenteellisen polun ja rungon tiivisteen (M 6.1.6, RS 11.1).
- **Syöte.** `Runner::input(seat, cmd)` ja `play::Session::press(key)` ovat ainoat kohdat, joista pelaajan päätös tulee ytimeen. `rlml run --keys` ja transkriptitestit ajavat niiden kautta.
- **Puuttuu:**
  - `checkpoint`, `rollback`, `delete_save`, `autosave` ja profiili ajossa (allekirjoitukset `sigs.rs`:ssä on jo)
  - `autosave`-osion luku malliin (syntaksi tuntee osion)
  - `save`- ja `load_game`-toiminnot sovelluksessa (`app.rs` tuntee vain `quit`- ja `exit`-toiminnot)

## Vaiheet

Jokaisen vaiheen jälkeen:
- `cargo test --workspace` on vihreä,
- `rlml play` ja `rlml run` toimivat ennallaan,
- olemassa olevat transkriptit eivät muutu,
- Kapteeni Aaltolan testit menevät läpi:
  `~/kapteeni-aaltola/roguelike/testit/aja.sh ~/rlml/target/debug/rlml`.

Jokainen vaihe on oma commitinsa.

### 0. Speksi: syöteloki ja tallennuksen aukot

Tehty 25.11.2026: luku M 4.4, lisäykset M 3.1:een ja M 5.2:een sekä merkintä `moottori/muutokset.md`:hen. Päätökset:
- Viestilokin muistissa olevat viestit tallentuvat esitystä varten.
- Kuulo ja viimeinen näkökenttä ovat tilaa.
- Katkaisurivi on `stop`.
- Lokin rivillä on hetki, ja toisto pysähtyy, jos hetki ei täsmää.
- Syöteloki ei kuulu tallennukseen.

Alla on alkuperäinen tehtävälista.

- **Uusi luku M 4.4 Syöteloki.** Moninpelisuunnitelma lupaa sen (`RLML-moninpeli.md` 4.4), ja M 8 viittaa siihen jo.
  - Rivi on (istuin, toiminto, argumentit). `join` ja `leave_seat` ovat rivejä muiden joukossa.
  - `travel_to` kirjataan kohderuutuineen, `follow` kohteineen, `follow.step` ja katkaiseva painallus omina riveinään (M 8).
  - Lokin alussa ovat aloituksen syötteet: siemen, profiilin tilannekuva ja liitettyjen istuinten määrä (M 5.1).
  - Toisto ajaa lokin alusta tai tallennuksesta, ja tuloksen on oltava sama.
- **Lokiviestit.** Kuuluuko `log` tilaan? Ehdotus: tallennus sisältää viestilokin viimeiset rivit esitystä varten (D 11.4 lukunäkymä), mutta ne eivät vaikuta ajoon. Kirjataan M 3.1:een tai M 5.2:een.
- **Kuulo ja viimeisin näkymä.** `heard`, `heard_mark` ja `last_view` vaikuttavat logiikkaan: `hear`-tapahtuma ja kuolleen pelaajan `sees`. Ne lisätään M 3.1:n tilan osiin, jotta tallennus kattaa ne.
- Muutokset kirjataan `moottori/muutokset.md`:hen ja M 11:n avoimiin kysymyksiin tarvittaessa.

### 1. Syöteloki ja toisto

Tehty 25.11.2026:
- **Syöteloki.** `rt::input` sisältää lokin (`InputLog`, rivit ja tekstimuoto) sekä toiston: `Replay::run` alkaa uudesta pelistä ja `Replay::from_runner` käynnissä olevasta pelistä. Toistoa käyttävät `rlml run --log-out`, `rlml play --log-out` ja `rlml replay`.
- **Muisti tilaan.** Toteutus löysi toistettavuusvirheen: karttamuisti ja viimeinen näkökenttä päivittyivät vain piirrettäessä. Klikkausmatkan reitti kulkee muistettujen ruutujen kautta, joten piirto vaikutti pelin kulkuun. Nyt `World::see` päivittää ne, kun ajo pysähtyy (M 7.2).
- **Testit.** Golden-transkriptit, ytimen matkatestit ja 20 sovellustestiä toistavat lokinsa, ja tilan, virran ja viestien on oltava samat.
- **Nopeus.** Kapteeni Aaltolan testit vievät debug-käännöksellä 12,8 s, ennen 9,5 s. Näkökenttä lasketaan nyt myös jokaisen pysähdyksen kohdalla, eikä vain piirrettäessä.

Alla on alkuperäinen tehtävälista.

- `rlml-core`: `rt::log::InputLog`, jonka `Runner` täyttää jokaisesta ajetusta toiminnosta (`input`, istuinten liittyminen ja poistuminen, matka ja seuraaminen). Pelkkä painallus ei ole lokirivi: rivi kirjataan vasta, kun moottori ajaa toiminnon (M 8.1).
- Lokin tekstimuoto, yksi rivi kerrallaan: `1 move dir:north`, `2 travel_to 14,6`, `1 follow.step`. Otsikkorivillä ovat siemen ja istuimet.
- `rlml run --log-out FILE` kirjoittaa lokin, ja `rlml replay PELI FILE` ajaa sen ja tulostaa saman transkriptin kuin `run`.
- `rlml play` kirjoittaa lokin pelikerran päätteeksi, kun annetaan `--log-out`. Tämä on virheiden toistamista varten.
- **Valmis, kun** jokaisen nykyisen transkriptitestin (mini, ryhmä, matka ja verbit) loki toistettuna tuottaa saman transkriptin, ja matkan sekä seuraamisen lokirivit toistuvat ilman osoitinta.

### 2. Checkpoint ja rollback muistissa

- Toiminnon loppuun lisätään vaiheet 2–4 (M 6.1.7): `rollback`, `checkpoint` ja `delete_save(checkpoint)` tässä järjestyksessä siirtojen ja istuinten jälkeen. Jos samassa toiminnossa kutsutaan kumpaakin, viimeinen pyyntö ratkaisee.
- `Snapshot` sisältää `State`-tilan, pelin virran, kuulon ja viimeisimmän näkymän sekä `Runner`-ajurin `waiting`- ja `woke`-kentät. Se on kloonattu arvo. Checkpoint 0 otetaan `begin`-käsittelijöiden jälkeen.
- `rollback()` palauttaa tilan. Istuimet, jotka on liitetty checkpointin jälkeen, säilyvät tyhjinä (M 5.3). Esityksen tila (`travels`, `orders`) tyhjennetään. Ilman checkpointia (`delete_save(checkpoint)`) rollback kirjaa ajon varoituksen.
- `delete_save(auto|all)` hyväksytään, mutta se vaikuttaa vasta vaiheessa 5.
- STD:n kuolemasäännöt (RS 10.2) eivät muutu. Testipeli `esimerkit/` alle tai testikansioon: alttari kutsuu `checkpoint()`, ja kuolema kutsuu `rollback()`.
- **Valmis, kun**
  - testi kuolee, palaa alttarille ja jatkaa samalla tavalla kuin uusi ajo samasta checkpointista,
  - samassa toiminnossa annetut `checkpoint` ja `rollback` noudattavat järjestystä,
  - ryhmäesimerkissä rollback palauttaa jäsenyydet ja ohjattavat.

### 3. Odottavan komennon polku

- Lataus numeroi jokaisen komennon rungon odotuskohdat ja silmukat lähdejärjestyksessä (RS 11.1) ja laskee rungon tiivisteen jäsennyspuusta. Kommentit ja välilyönnit eivät vaikuta siihen. Tiiviste on oma FNV- tai SplitMix-pohjainen, koska riippuvuuksia ei lisätä.
- `Suspended` muutetaan muotoon: komennon nimi, tiiviste, argumentit, paikalliset nimet, odotuskohdan numero, silmukoiden tilat ulkoa sisään (`repeat`: kierros, `for`: kiinnitetty joukko ja indeksi, `while`: ei mitään) ja `wait N`:n jatkoherätys. Kehyspino rakennetaan polusta uudelleen jatkettaessa (M 6.1.6, "Jatkaminen").
- **Muuttunut runko.** Jos tiiviste tai komento puuttuu, odotus hylätään: muutokset jäävät, hintaa ei peritä, ja ajo antaa varoituksen. `wait N`:n jatkoherätys säilyy.
- **Valmis, kun**
  - RS 7.1:n `DIG` odottaa kolmannella kierroksella ja tallentuu kohtana odotus 0 ja `repeat`-kierros 2 (RS 11.1:n esimerkki),
  - polusta jatkettu komento tuottaa saman transkriptin kuin keskeytymätön,
  - muuttunut runko antaa varoituksen eikä kaada ajoa.

### 4. Tiedostomuoto

- `rt::save`: `write(&Snapshot) -> String` ja `read(&Model, &str) -> RtResult<Snapshot>`.
- Ensimmäinen rivi on `rlml-save 1`, ja sen jälkeen tulevat pelin nimi ja datan tiiviste. Tiiviste on tiedoksi: speksin mukaan ladattu peli käyttää aloitushetken `game`-arvoja (M 5.2), joten eroava data ei estä latausta.
- Osiot vastaavat M 5.2:n taulukkoa:

  | Osio | Sisältö |
  | --- | --- |
  | `game` | moottorin kentät, rajat ja komponentit |
  | `rng` | pelin virran neljä sanaa |
  | `entities` | luontijärjestyksessä, sijainteineen ja kuolleet kahvat lajeineen ja nimineen |
  | `maps` | kerrokset, nimiöt, johtimet, `vars` ja muisti |
  | `wakes` | herätykset ja `next_seq` |
  | `decks` | pakkojen tila |
  | `seats` | istuimet |
  | `waiting` | odottavat komennot polkuina (vaihe 3) |
  | `checkpoint` | sisäkkäinen tilannekuva |
  | `profile` | profiilin näkymä, tyhjä vaiheeseen 6 asti |
  | `hearing` | kuulo |
  | `log` | viestilokin häntä |

- Arvot kirjoitetaan RogueScriptin kirjoitusasun mukaan aina, kun sellainen on (`pos(5, 3)`, `2d6+1`, kortit, nimet), jotta tiedosto on luettava.
- **Kierrostesti.** Testiapuri tallentaa ja lataa jokaisessa syötteen odotuksessa ja ajaa loput syötteet. Tuloksen on oltava sama kuin keskeytymättömässä ajossa. Apuri ajetaan minipelin, ryhmän ja vaiheen 2 testipelin transkripteille.
- **Valmis, kun** kierrostesti on vihreä kaikille transkripteille, ja `read(write(s)) == s` pätee jokaiselle tilannekuvalle.

### 5. Tallennuspaikat, automaattinen tallennus ja valikot

- **`Store`-trait** `rlml-core`en ja `MemStore`. `rlml` saa levytoteutuksen käyttäjädatakansioon `<käyttäjädata>/PELI/saves/`, samaan paikkaan kuin asetukset (D 3.2). Kirjoitus tehdään väliaikaiseen tiedostoon, joka nimetään lopuksi oikeaksi (M 5.4).
- **`autosave`-osio malliin:** `slots`, `manual` ja `quit` sekä kohdat `input every N`, `scene`, `checkpoint` ja `arrive`. Latauksessa tarkistetaan arvot. Paikkamäärä lukitaan pelikerran tilaan (M 5.4, "Lukitus").
- **Ajo:**
  - `autosave()` ja kohdat `on input`, `on checkpoint` ja `on arrive` toiminnon lopun vaiheessa 5
  - ryhmän siirto tallentaa kerran
  - `on input every N` laskee kaikkien istuinten odotukset yhteen, mutta ei matkan eikä seuraamisen askelia (M 8.1)
  - `delete_save(auto|all)`
  - `nosave`-kartta estää oman tallennuksen ja `quit`-tallennuksen
- **Moodit.** `slots 0`, `slots 1` (oma tallennus lopettaa pelin kaikilta istuimilta) ja `slots N` (pino ja omat paikat). Pino järjestyy kirjoitusajan mukaan, ja vanhemmasta tallennuksesta jatkettaessa uudemmat jäävät pinoon (haarautuminen).
- **Sovellus (`rlml-app`):**
  - toiminto `save`, joka on himmennetty `nosave`-kartalla ja kohtauksessa
  - `quit` tallentaa `quit`-rivin mukaan
  - `load_game` moottorin valikossa ja pelin `title`-valikossa
  - tallennuslista (paikka, aika, kartta ja kierros)
  - `new_game` pelin aikana päättää pelikerran kuten `quit` (M 8)
- **Taustakirjoitus.** Ensin kirjoitetaan synkronisesti. Säikeen ja välissä olevien tallennusten ohituksen (M 5.4, "Kirjoitus") voi lisätä, jos kirjoitus näkyy viiveenä. Kirjoitus mitataan ensin Kapteeni Aaltolan suurimmalla kartalla.
- **Valmis, kun**
  - `rlml play esimerkit/mini` tallentaa, lopettaa ja jatkaa ladatusta tallennuksesta,
  - `slots 1` -peli lopettaa pelin omaan tallennukseen,
  - pinon haarautumista testataan `MemStore`-paikalla,
  - kaatuminen kesken kirjoituksen ei jätä rikkinäistä tiedostoa (testi: kirjoitus keskeytetään ennen nimeämistä).

### 6. Profiili

- `profile`-lohkot malliin ja profiilin näkymä tilaan: aloitushetken tilannekuva ja pelikerran omat kirjoitukset (M 5.6).
- Profiilitiedosto `<käyttäjädata>/PELI/profile` `Store`-traitin kautta.
  - Tarkistussumma lasketaan sisällöstä ja pelin nimestä. Jos summa ei täsmää, tiedosto nimetään varmuuskopioksi, ja profiili alkaa alkuarvoista ajon varoituksen kera.
  - Kun data lisää kenttiä, ne lisätään profiiliin oletusarvoineen. Poistetut kentät säilyvät tiedostossa näkymättöminä.
  - Kirjoitus tehdään toiminnon lopun vaiheessa 5. `rollback`, `delete_save` ja lataus eivät peru sitä.
- Profiilin nollaus moottorin valikosta.
- Syötelokin otsikkoon tulee profiilin tilannekuva (vaihe 1 jätti sille paikan).
- **Valmis, kun** pistelista säilyy pelikertojen yli, toisto toimii toisella profiililla samoin ja muokattu tiedosto huomataan.

### 7. Web

- `rlml-web`: `Store` OPFS:n päälle. Jos OPFS ei ole käytettävissä, käytetään `localStorage`a pienille peleille. `Store` pysyy synkronisena ytimen puolella, ja web kirjoittaa välimuistista taustalla.
- `RLML-web.md`:n "Ei tässä vaiheessa" -kohdasta poistetaan `Store`-trait ja OPFS.
- **Valmis, kun** Kapteeni Aaltolan peli selaimessa tallentaa, sivu ladataan uudelleen ja peli jatkuu tallennuksesta.

## Tämän jälkeen

- **Moninpelin vaihe 3** (`RLML-moninpeli.md`): liittymisruutu, `join` kesken pelin, `leave_seat` ja latauksen uudelleenliittyminen. Sen esteet ovat istuintaulu tallennuksessa (vaihe 4) ja syöteloki istuinriveineen (vaihe 1).
- **Kohtaukset (RS 9)** täyttävät varatut osiot: jonossa olevat etualan kohtaukset, taustakohtaukset, tweenit, `on scene` sekä `wait scene`-, `wait confirm`- ja `open`-odotukset polkuina.

## Ei tässä vaiheessa

- Pilvitallennus.
- Tallennusten pakkaus tai salaus.
- Tallennusten siirto eri moottoriversioiden välillä. Muoto on versio 1, ja vanhempaa versiota ei tarvitse lukea ennen kuin muotoa muutetaan.
- Tallennuksen esikatselukuva.
