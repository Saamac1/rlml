# Kehitystyökalut – toteutusohje

Tämä on ohje Claude Codelle. Siinä kerrotaan, miten moottoriin tehdään slotit, inspector, hot reload ja nimetyt kehitysmodit, ja missä järjestyksessä. Pohjana on dokumentti [RLML – slotit, inspector, hot reload ja modit](https://claude.ai/artifact/Cos8z216UKw8qW3rQ6LPX8) (26.11.2026). Tämä ohje sovittaa sen nykyiseen koodiin (commit `afddbf5`), ja kohdat, joissa dokumentti ja koodi eroavat, on koottu lukuun [Erot dokumenttiin](#erot-dokumenttiin).

Ohje ei muuta speksiä. Vaiheet, jotka tuovat speksiin jotain uutta (`reload`- ja inspector-rivit syötelokiin, ruudun alkuperäiset arvot RogueScriptiin, kehitysmodien kerros), kirjaavat sen speksiin ja muutoslokiin ennen koodia.

## Tavoitteet ja rajaus

- Nopeampi tulkki ilman tavukoodia tai AOT-kääntäjää.
- Ajonaikainen tilan tarkastelu ja hallittu muokkaus.
- Muutokset käyttöön ajon aikana toiminnon rajalla, peruttavasti.
- Toimiva kokeilu siirtyy projektiin tarkistettuna, diffinä.

Ei tavukoodia eikä käännöstä C:hen, Rustiin tai JS:ään. Hot reload on vain kehitystilassa, ei julkaistussa pelissä eikä web-versiossa.

## Päätökset

Dokumentista:
- **Kehitysmodit ladataan** perustiedostojen ja include-tiedostojen jälkeen mutta ennen pelaajan `overrides/`-tiedostoja, aktivointijärjestyksessä.
- **Muuttamaton kenttä.** Kenttä on pelin muuttamaton, kun sen arvo on sama kuin vanhan määrittelyn arvo. Erillistä merkintää ei tarvita.
- **Kartat.** Jo luodut kartat päivitetään kolmisuuntaisella yhdistämisellä. Kartan tila kirjaa muutetut ruudut alkuperäisine ja nykyisine arvoineen, joten piirroksen kopiota ei tarvita.
- **Web-versio** on jakeluversio eikä tue hot reloadia.
- **Breakpointit** tehdään kevyenä versiona: tulkki kutsuu koukkua jokaisen lauseen kohdalla, ja tila on silloin vain luettavissa.

Tämän ohjeen omat päätökset, jotka nykyinen koodi pakottaa:
- **Malli vaihdetaan kokonaan.** `World<'m>`, `Runner<'m>`, `Session<'m>` ja `App<'m>` lainaavat mallia (`&'m Model`), joten ladattua mallia ei voi muuttaa paikallaan. Hot reload rakentaa uuden mallin ja siirtää tilan siihen tilannekuvana (`Snapshot`), kuten `Runner::load` jo tekee tallennukselle. Kehitystilassa jokainen ladattu malli vuodetaan (`Box::leak`), kuten web-versio jo tekee. Muistia kuluu latausta kohden, mutta lainausrakenne säilyy eikä elinaikoja tarvitse kirjoittaa uusiksi.
- **Lowering on välimuisti mallin rinnalla.** Slotit ja nimitaulut lasketaan mallista latauksessa, ja ne kuuluvat malliin samalla tavalla kuin `m.code`. Tallennus, syöteloki ja rungon tiiviste (`body_hash`) käyttävät edelleen lähteen jäsennyspuuta ja nimiä, joten lowering ei muuta niitä.

## Lähtötilanne (commit `afddbf5`)

- **Lokaalit.** `Env.locals` on `BTreeMap<String, Value>`, ja jokainen nimen luku on merkkijonohaku. Iso nimi resolvoidaan jokaisella kerralla (`World::resolve` → `Model::resolve`).
- **Kehykset kopioivat.** `Frame::Block`, `For`, `While` ja `Repeat` omistavat lauseensa (`Vec<Stmt>`). `run_to_end` kopioi koko rungon (`body.to_vec()`), ja `run_frames` kopioi jokaisen lauseen ennen ajoa (`stmts[*i].clone()`). Funktiot, kyselyt ja käsittelijät ajetaan `run_frames`-silmukassa, mutta kutsu itse on Rustin rekursio (`call_function` → `run_to_end`), ja kutsusyvyyden laskevat `enter` ja `leave`.
- **Käsittelijöiden jakelu.** `World::handlers` käy jokaisessa `emit`-kutsussa läpi koko `m.code`-listan, vertaa tapahtuman nimeä merkkijonona ja kloonaa jokaisen osuvan käsittelijän jäsennyspuun (`Arc::new(h.clone())`). Valmiita jakelutauluja ei ole.
- **Odottava komento** tallentuu nimillä jo nyt: `Suspended.env` sisältää `locals`-taulun nimineen, ja `waiting_fits` hylkää komennon, jonka rungon tiiviste on muuttunut (M 6.1.6).
- **Kartan tila tallentuu kokonaan.** `MapState.tiles` tallentuu jokaisen kerroksen jokaisena ruutuna, eikä tila tiedä, mitkä ruudut peli on muuttanut.
- **Mallin lataus.** `load::load` ja `check_game` lukevat tiedostot `Files`-rajapinnan kautta. `patch`-, `new`- ja `drop`-merkinnät toimivat mallissa (`Modifier::Patch`, `model.rs`), mutta pelaajan `overrides/`-hakemistoa ei vielä lueta automaattisesti, vaikka D 3.2 niin sanoo.
- **Syöteloki** (`rt::input`) on rivejä (aika, istuin, toiminto). Rivejä on komennoille, matkoille, istuimille ja kohtauksille, mutta ei latauksille eikä tilan muokkauksille.
- **Ajanotto.** `rt/profile.rs` on pelaajan profiili (M 5.6), ei suorituskyvyn mittari. Mittauksia on vain selainversion esimerkissä `crates/rlml-web/examples/replay.rs`, joka mittaa piirron ja askelten ajat.
- **Inspector ja MCP.** Kumpaakaan ei ole. MCP-palvelin on moninpelisuunnitelman vaihe 8 ([RLML-moninpeli.md](RLML-moninpeli.md)), ja se käyttää samaa rajapintaa kuin inspector.

## Erot dokumenttiin

- **Mittaus `profile.rs`:llä.** Tiedosto on pelaajan profiili. Mittaus tarvitsee oman työkalunsa (vaihe 1).
- **Tallennus nimillä.** Dokumentin mukaan odottavan komennon lokaalit pitää alkaa kirjoittaa nimillä. Ne kirjoitetaan nimillä jo nyt, joten vaiheessa 2 riittää, että slotit muunnetaan takaisin nimiksi tallennettaessa ja nimistä sloteiksi ladattaessa.
- **Polun esimerkki `game.round`.** Kierroksen kenttä on `game.turn`. Inspectorin polut käyttävät samoja nimiä kuin RogueScript (`game.turn`, `entity#12.HP.now`), joten niitä ei tarvitse opetella erikseen.
- **Ruudun alkuperäiset arvot "ominaisuuksina".** Ruutu ei ole RogueScriptissä olio, jolla olisi kenttiä, vaan tile luetaan funktiolla `tile_of(p, layer)` (M 9.3). Luonteva muoto on siksi funktiopari, esimerkiksi `tile_original(p, layer)` ja `tile_changed(p)`. Syntaksi päätetään vaiheessa 4 ennen koodia.
- **Hot reload ja malli.** Dokumentti puhuu nimi- ja dispatch-taulujen uudelleenrakennuksesta paikallaan. Malli on lainattu kaikkialle, joten taulut rakennetaan uuteen malliin, ja tila siirretään siihen tilannekuvana (Päätökset).
- **Pelaajan `overrides/`.** Dokumentti olettaa sen toimivaksi kerrokseksi, jonka alle modit asettuvat. Hakemiston lukeminen puuttuu, ja se tehdään vaiheen 5 alussa, koska latausjärjestys perustuu siihen.
- **Toisto latauksen yli.** `reload`-rivi kertoo modijoukon ja kerrosten tiivisteet, mutta toisto tarvitsee myös tiedostot, jotka silloin ladattiin. Avoin kysymys (alla).

## Vaiheet

Jokaisen vaiheen jälkeen:
- `cargo test --workspace` on vihreä, ja `cargo fmt --all` on ajettu,
- Kapteeni Aaltolan testit ja golden-transkriptit eivät muutu,
- vanhalla versiolla tehdyt tallennukset latautuvat.

Jokainen vaihe on oma haaransa ja commitinsa, ja jokainen on käyttökelpoinen yksinään.

### 1. Mittaus

Tehty 26.11.2026 haarassa `claude/kehitystyokalut-1`:
- **`rlml bench PELI [--seed N] [--keys "…" | --keys-file TIEDOSTO] [--repeat N]`** ajaa näppäimet kuten `rlml run`, mutta ilman piirtoa, ja tulostaa latauksen, uuden pelin ja pelaamisen ajat.
- **Laskurit (`rt::stats`)** ovat ominaisuuden `stats` takana (`cargo build -p rlml --features stats`). Ilman ominaisuutta ne kääntyvät tyhjiksi. Ominaisuuden kanssa ajo hidastuu noin 10 %.
  - Laskettavat asiat ovat ajetut lauseet, kopioidut rungot, lokaalien luvut, isojen nimien resolvoinnit, määrittelyhaut, funktiokutsut, `emit`-kutsut, käsittelijöitä etsittäessä läpikäydyt määrittelyt ja ajetut käsittelijät.
  - Laskurit toistuvat samoina samalla siemenellä.
- **Ajanotto.** Koneella ei ole profilointityökalua, ja järjestelmä estää suorituskykylaskurit (`perf_event_paranoid` 4). `stats` ottaa siksi aikaa myös käsittelijöiden haulle, näkökentälle, reitinhaulle, kuulemiselle, tekoälyn valinnalle ja komennoille. Jaksot ovat osin päällekkäisiä: esimerkiksi komennon aika sisältää sen lähettämät tapahtumat ja kuulemisen.
- **Generointikarttaa ei voi vielä mitata**, koska moottori ei aja generaattoreita ("generators are not supported yet"). Sen tilalla mitataan Oz-esimerkki, jossa taustakohtaus kulkee joka kierros. Generointikartan mittaus tehdään, kun generaattorit toimivat.

**Perustaso** (commit `2283b51` + vaihe 1, julkaisukäännös laskureineen, siemen 1):

| Mittaus | Painalluksia | Aika / painallus | Lauseita / painallus | Tapahtumia / painallus | Läpikäytyjä määrittelyjä / painallus | Käsittelijöitä ajettu / painallus |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Kapteeni Aaltola, `testit/lapipeluu.keys` | 565 | 0,89 ms | 32 | 41 | 14 941 | 2,5 |
| Minipeli, `s` × 500 | 500 | 1,16 ms | 170 | 16 | 3 568 | 3,0 |
| Oz-esimerkki, nimi ja 300 odotusta | 323 | 0,20 ms | 7 | 8 | 2 166 | 0,7 |

| Mittaus | Käsittelijöiden haku | Näkökenttä | Kuuleminen | Tekoäly | Reitit |
| --- | ---: | ---: | ---: | ---: | ---: |
| Kapteeni Aaltola | 49 % | 20 % | 4 % | 0 % | 0 % |
| Minipeli | 12 % | 0 % | 30 % | 29 % | 0 % |
| Oz-esimerkki | 37 % | 31 % | 0 % | 0 % | 2 % |

Mitä tästä seuraa vaiheelle 2:
- **Käsittelijöiden haku on suurin yksittäinen kulu.** Jokainen `emit` käy läpi koko koodilistan: Aaltolassa 41 tapahtumaa painallusta kohden käy läpi noin 15 000 määrittelyä, vaikka käsittelijöitä ajetaan 2,5. Valmiit jakelutaulut poistavat tämän lähes kokonaan.
- **Näkökenttä** on toiseksi suurin Aaltolassa ja Oz-esimerkissä. Se ei kuulu tämän suunnitelman vaiheisiin, mutta se on seuraava kohde, kun jakelutaulut on tehty.
- **Nimihaut ja runkojen kopiot** ovat lukumäärinä suuria (minipelissä 110 resolvointia, 66 määrittelyhakua ja 95 runkokopiota painallusta kohden), mutta niiden osuutta ajasta ei voi mitata erikseen ilman profilointityökalua. Ero näkyy vaiheen 2 jälkeen samalla mittauksella.

Komennot perustasoon:

```
cargo build --release -p rlml --features stats
target/release/rlml bench ~/repos/kapteeni-aaltola/roguelike --keys-file ~/repos/kapteeni-aaltola/roguelike/testit/lapipeluu.keys
target/release/rlml bench esimerkit/mini --keys s --repeat 500
target/release/rlml bench esimerkit/kohtaus --keys "enter enter enter $(for i in $(seq 1 20); do printf 's %.0s' $(seq 1 15); printf 'enter '; done)"
```

Alla on alkuperäinen tehtävälista.

- **Työkalu** `rlml bench PELI [--keys …] [--rounds N]`, joka ajaa näppäinsarjan tai kierroksia ilman piirtoa ja tulostaa ajan sekä laskurit: lauseet, nimihaut, `emit`-kutsut, käsittelijäkutsut ja kloonatut kehykset. Laskurit ovat kehityskäännöksen ominaisuus (`cfg(feature = "stats")`), joten julkaistu ajo ei maksa niistä.
- **Perustaso** Kapteeni Aaltolasta, minipelistä ja yhdestä raskaasta generointikartasta. Tulokset kirjataan tähän ohjeeseen, jotta vaiheen 2 hyöty näkyy.
- **Valmis, kun** mittaus toistuu samoina laskureina samalla siemenellä ja näyttää, kuinka suuri osa ajasta kuluu nimihakuun, kehysten kopiointiin ja käsittelijöiden etsintään.

### 2. Lowering ja slotit

**Jakelutaulut tehty** 26.11.2026 haarassa `claude/jakelutaulut`. Vaiheen muut osat (slotit, rungon nimitaulu, jaetut kehykset ja isojen nimien resolvointi latauksessa) ovat yhä tekemättä.
- **`rt::dispatch`.** Malli laskee kerran (`Model::dispatch`, `OnceLock` kuten muutkin mallin välimuistit) jokaisen tapahtuman ja vaiheen käsittelijät valmiiksi järjestettyinä: ensin prioriteetti ja sitten latausjärjestys, ja komponentin sisällä kirjoitusjärjestys (M 6.2).
  - Tapahtuman nimi ratkaistaan käsittelijän nimiavaruudessa samalla kertaa.
  - `override … as NIMI` -sääntö ratkaistaan samalla kertaa.
  - Käsittelijän jäsennyspuu jaetaan (`Arc`) eikä sitä kloonata kutsulla.
- **`World::handlers`** lukee tapahtuman listan taulusta ja tarkistaa ajon aikana vain, onko subjektilla komponentin käsittelijän komponentti.
- **Käytös ei muuttunut.** `rlml run` tuottaa kaikissa kolmessa mittauspelissä rivilleen saman tulosteen kuin ennen. Kaikki testit ja Kapteeni Aaltolan läpipeluut menevät läpi.

| Mittaus | Aika / painallus ennen | Jälkeen | Läpikäytyjä määrittelyjä / painallus ennen | Jälkeen | Käsittelijöiden haku ennen | Jälkeen |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Kapteeni Aaltola | 0,89 ms | 0,43 ms | 14 941 | 73 | 49 % | 2 % |
| Minipeli | 1,16 ms | 1,04 ms | 3 568 | 7 | 12 % | 0 % |
| Oz-esimerkki | 0,20 ms | 0,12 ms | 2 166 | 2 | 37 % | 1 % |

Ajat on mitattu laskureiden kanssa. Ilman laskureita Kapteeni Aaltolan läpipeluu nopeutui 0,80 ms:sta 0,42 ms:iin painallusta kohden. Suurimmat kulut ovat nyt näkökenttä (Kapteeni Aaltolassa 43 %, Oz-esimerkissä 50 %) sekä minipelin kuuleminen (34 %) ja rottien tekoäly (33 %).

Alla on alkuperäinen tehtävälista.

- **Lowering-vaihe** tarkistuksen (`rs::check`) jälkeen tuottaa jokaisesta rungosta version, jossa lokaalit ovat slotteja ja isot nimet on resolvoitu latausajan tauluihin.
- **Lokaalit.** `Env.locals` muuttuu muotoon `Vec<Value>`, ja jokainen slotti alustetaan arvoon `none`. Slotit jaetaan deterministisesti esittelyjärjestyksessä, eikä niitä käytetä uudelleen sisemmissä lohkoissa.
- **Rungon nimitaulu.** Jokaisesta slotista kirjataan nimi, tyyppi, näkyvyysalue rakenteellisena polkuna (alku ja loppu) ja esittelypaikka. Sitä käyttävät tallennus, inspector ja virheilmoitukset.
- **Jaetut kehykset.** Kehykset viittaavat lauseisiin jaetusti (`Arc<[Stmt]>` ja indeksi). `run_to_end` ja `run_frames` eivät enää kopioi runkoja eivätkä lauseita.
- **Jakelutaulut.** Käsittelijät lasketaan latauksessa tapahtumittain ja vaiheittain valmiiksi järjestettyyn listaan (prioriteetti ja latausjärjestys, M 6.2), ja `override … as NIMI` -sääntö ratkaistaan samalla. `emit` lukee taulua eikä kloonaa jäsennyspuuta.
- **Tallennus** muuntaa slotit nimiksi kirjoitettaessa ja nimet sloteiksi luettaessa. Tallennusmuoto ei muutu. `body_hash` lasketaan edelleen lähteen jäsennyspuusta.
- **Valmis, kun** ulkoinen käytös ei muutu: kaikki testit, toistot ja vanhat tallennukset toimivat sellaisinaan, ja vaiheen 1 mittaus näyttää eron.

### 2b. Näkökenttä (mittauksen löydös)

Tehty 26.11.2026 haarassa `claude/nakokentta`. Tämä ei ollut alkuperäisessä suunnitelmassa, vaan jakelutaulujen jälkeen näkökenttä oli suurin kulu (Kapteeni Aaltolassa 43 %, Oz-esimerkissä 50 %).
- **Syy.** Jokainen ruutu, jonka yli näkökenttä tai valo katsoo, kysyi `blocks(p, sight)`. Se kävi läpi kaikki pelin entiteetit (`entities_at`) ja rakensi tilen nimiketjun uusina merkkijonoina (`tile_property`). Myös `tile_flag` varasi kaksi merkkijonoa jokaista valaistuskyselyä varten.
- **Korjaus.**
  - Näkökenttä ja valon ulottuvuus keräävät kartan näköä estävät entiteetit kerran ruutujoukoksi (`sight_blockers`).
  - Tilen oma esto (`tile_blocks`) ja liput (`tile_flag`) muistetaan tilen nimen mukaan ilman varauksia. Ne riippuvat vain mallista, joten muistia ei tarvitse tyhjentää.
  - Pelin tilaa ei pidetä välimuistissa, joten mitään ei tarvitse mitätöidä.
- **Käytös ei muuttunut.** `rlml run` tuottaa Kapteeni Aaltolassa, minipelissä, Oz-esimerkissä ja ryhmäesimerkissä rivilleen saman tulosteen kuin ennen.

| Mittaus | Aika / painallus ennen | Jälkeen | Näkökenttä ennen | Jälkeen |
| --- | ---: | ---: | ---: | ---: |
| Kapteeni Aaltola | 0,43 ms | 0,30 ms | 103 ms | 44 ms |
| Minipeli | 1,04 ms | 0,88 ms | 1 ms | 1 ms |
| Oz-esimerkki | 0,12 ms | 0,11 ms | 20 ms | 12 ms |

Ilman laskureita Kapteeni Aaltolan läpipeluu vie nyt 0,30 ms painallusta kohden, kun perustaso oli 0,80 ms. Seuraavat kulut ovat minipelin rottien tekoäly (37 %) ja kuuleminen (28 %) sekä näkökentän oma varjonheitto ja valaistu-tarkistus.

### 3. Inspector, luku

Tehty 26.11.2026 haarassa `claude/inspector-luku`:
- **`rt::inspect`.** `Runner::inspect(polku)` palauttaa tilan puuna (`Node`), ja `Node::lines` antaa sen sisennettyinä riveinä.
  - **Juuret:** `game`, `maps`, `map.NIMI`, `entities`, `entity#N`, `seats`, `wait`, `wait#N` ja `scenes`. Tyhjä polku antaa yhteenvedon.
  - **Polut:** polut jatkuvat nimillä kuten RogueScriptissä, esimerkiksi `game.turn`, `game.STORY.stage`, `entity#12.HP.now` ja `wait#12.locals.steps`.
  - **Komponentit:** komponentin voi nimetä koko nimellä (`STD.HP`) tai viimeisellä osalla (`HP`), kun se on yksiselitteinen.
  - **Arvot:** listat ja sanakirjat näytetään sisältöineen.
  - **Entiteetti:** näyttää määrittelynsä tiedoston ja rivin, istuimen ja herätyksen.
  - **Kohtaukset:** näyttävät etualan kohtauksen, jonon, taustakohtaukset kahvoineen ja satunnaisvirran tilan.
- **Lukeminen ei muuta tilaa.** Testi vertaa tilaa ennen ja jälkeen kaikkien juurten luvun.
- **`rlml inspect PELI [--keys "…"] [POLKU …]`** pelaa näppäimet ja tulostaa polut. Väärä polku kerrotaan sen kohdalla, ja muut tulostuvat.
- **Kehitystila `rlml play --dev`.** `f12` avaa inspectorin pelin päälle: yhteenvedon, `game`-tilan, pelaajan, odottavat komennot ja kohtaukset. Paneeli selataan `MENU`-kontekstin näppäimillä, ja `esc` tai `f12` sulkee sen. Pelin aika seisoo, ja pelin näppäimet eivät toimi paneelin ollessa auki.
- **Ei vielä:**
  - Odottavan komennon lokaaleista näytetään kaikki, jotka runko on asettanut. Vain odotuskohdassa näkyvät tulevat rungon nimitaulun mukana (vaihe 2).
  - Määrittelystä tulevan arvon alkuperä kenttäkohtaisesti tulee modien mukana (vaihe 5).
  - MCP-työkalut tehdään moninpelin vaiheen 8 palvelimen kanssa samalle rajapinnalle.
- **Testit:** `crates/rlml-core/tests/inspect.rs` (polut, virheet, odottavan komennon lokaalit ja lukeminen ilman muutoksia) ja sovellustesti `f12_opens_the_inspector_in_development_mode`.

Alla on alkuperäinen tehtävälista.

- **Ytimen rajapinta** `rt::inspect`. Se lukee tilaa milloin tahansa ja palauttaa rakenteen, jota TUI, web ja MCP voivat näyttää.
- **Mitä näkyy:**
  - `game`, kartat, entiteetit ja niiden komponentit kenttineen nimillä,
  - odottavan komennon kehys ja sen lokaalit nimitaulun kautta, vain ne, jotka ovat näkyvissä sen kohdassa,
  - etualan kohtaus, taustakohtaukset kahvoineen, satunnaisvirtojen tila (vain luku) ja myöhemmin aktiiviset modit,
  - määrittelystä tulevan arvon alkuperä: tiedosto ja rivi, myöhemmin myös modi.
- **Polut** samalla syntaksilla kuin RogueScript: `game.turn`, `entity#12.HP.now`, `wait.locals.target`. Sama syntaksi toimii luvussa, kirjoituksessa ja MCP-työkaluissa.
- **TUI-paneeli** kehitystilassa: näppäin avaa inspectorin pelin päälle, ja pelin aika pysähtyy kuten valikossa.
- **MCP.** Lukutyökalut tehdään samaan aikaan moninpelin vaiheen 8 palvelimeen, tai palvelin aloitetaan niistä.
- **Valmis, kun** inspector näyttää Kapteeni Aaltolan tilan ja odottavan komennon lokaalit oikein, ja lukeminen ei muuta toistoa.

### 4. Hot reload perustiedostoille

Ensin speksiin: syötelokin rivi `reload` (M 4.4), ruudun alkuperäiset arvot RogueScriptiin (RS 6 ja M 9.3) ja muutettujen ruutujen lista tallennukseen (M 5.2).

- **Pyyntö.** Tiedostomuutos tai komento pyytää uudelleenlatausta, ja moottori odottaa toiminnon rajaa. Etualan kohtauksen, dialogin tai `open`-valikon aikana odotetaan niiden loppuun, eli samaa ehtoa kuin tallennuksessa (`World::save_blocked`).
- **Lataus.** Koko kerrosjoukko ladataan ja tarkistetaan uudelleen. Jos tulee virhe, vanha malli jää voimaan ja virheet näytetään.
- **Tilan siirto.** Tila siirretään uuteen malliin tilannekuvana (Päätökset). Määrittelymuutokset koskevat vain uusia instansseja. Odottava komento, jonka rungon tiiviste muuttui, hylätään varoituksella, kuten jo nyt (`drop_changed_wait`).
- **Mitä vaihtuu heti.** Käsittelijät, komennot, tekoälyt, kyselyt ja tekstit vaihtuvat seuraavasta toiminnosta alkaen. Käynnissä oleva taustakohtaus jatkaa vanhalla aikajanalla loppuun: sen rivit rakennetaan kutsusta (`bg_plans`), joten vanha suunnitelma pidetään kahvan välimuistissa siihen asti.
- **Muutetut ruudut.** `MapState` kirjaa jokaisen pelin muuttaman ruudun alkuperäisen arvon kerroksittain siitä hetkestä alkaen, kun kartta on ladattu tai generoitu. Muutoksia tulevat tekemään `tile`, `carve` ja aikajanan tweenit. Lista tallentuu tallennukseen, ja vanhat tallennukset latautuvat ilman sitä.
- **Karttojen yhdistäminen** kolmesta versiosta: vanha piirros, uusi piirros ja muistissa oleva kartta.
  1. Ruutu, jota peli ei ole muuttanut, saa uuden piirroksen arvon.
  2. Pelin muuttama ruutu säilyy.
  3. Jos sama ruutu on muuttunut sekä piirroksessa että pelissä, pelin muutos jää voimaan ja siitä tulee varoitus.
  4. Lopuksi tarkistetaan, ettei pelaaja tai yksikään entiteetti ole estävässä ruudussa ja että nimiöt (`label`) ovat yhä olemassa. Ongelmat näytetään varoituksina.
- **Samat tarkistukset** tehdään, kun tallennettu peli ladataan ja kartan piirros on muuttunut tallennuksen jälkeen.
- **Syöteloki** saa rivin `reload`, jossa on kerrosten tiivisteet.
- **Valmis, kun** käsittelijän muutos näkyy seuraavassa toiminnossa ilman uutta peliä, ja kartan uusi seinä ilmestyy ilman, että avattu ovi sulkeutuu.

### 5. Kehitysmodit

- **Ensin `overrides/`.** Pelaajan `overrides/`-hakemisto luetaan latauksessa viimeisenä, kuin sen lohkoissa olisi `patch` (D 3.2).
- **Modi** on nimetty hakemisto, esimerkiksi `dev/rotat-vaikeampi/`, jossa on samaa RLML- ja RogueScript-syntaksia kuin pelissä ja samat `patch`-, `drop`- ja `new`-merkinnät.
- **Otsake** kertoo nimen, lyhyen kuvauksen ja pelin version tiivisteen, jonka päälle modi on tehty. Muuttuneesta pohjasta tulee varoitus, mutta modi ladataan.
- **Latausjärjestys** on perustiedostot, include-tiedostot, kehitysmodit aktivointijärjestyksessä ja `overrides/`. Jos kaksi modia muuttaa samaa avainta, viimeisenä ladattu voittaa ja siitä tulee varoitus.
- **Aktivointi.** Aktiivisten modien vaihto on uudelleenlataus (vaihe 4).
- **Tallennus ja toisto.** Tallennus kirjaa aktiiviset modit tiivisteineen. Jos tallennuksen lataa ilman samoja modeja, tulee varoitus ja peli jatkuu nykyisillä määrittelyillä. Toisto alkaa modijoukolla, ja jokainen modin lataus tai poisto on `reload`-rivi.
- **Olemassa olevien päivitys.** Modi voi pyytää, että myös olemassa olevat instanssit päivitetään. Päivitys koskee vain kenttiä, joiden arvo on yhä vanhan määrittelyn mukainen, joten haavoittuneen rotan `hp` ei palaudu.
- **Valmis, kun** modin voi kytkeä päälle ja pois kesken pelin, ristiriidasta tulee varoitus ja inspector näyttää arvon alkuperän.

### 6. Inspector, muokkaus

Ensin speksiin: syötelokin inspector-rivi (M 4.4).

- **Kirjoitus** on sallittu vain toiminnon rajalla, ei etualan kohtauksen aikana.
- **Tarkistus.** Polku muunnetaan nimitaulun avulla slotiksi tai kentäksi, ja arvon tyyppi tarkistetaan.
- **Syöteloki ja tallennus.** Muokkaus kirjataan syötelokiin omana rivinään, joten toisto säilyy. Tallennus merkitään inspectorilla muokatuksi.
- **Tallenna modiin.** Jos arvo tulee määrittelystä, esimerkiksi rotan `HP`, inspector kirjoittaa sen `patch`-riviksi aktiiviseen kehitysmodiin. Instanssin oma pelin aikana muuttunut tila ei ole määrittelyä.
- **Valmis, kun** muokattu peli toistuu identtisenä, ja "tallenna modiin" tuottaa modin, joka uudessa pelissä antaa saman arvon.

### 7. Vienti include-tiedostona

- **`rlml mod export NIMI`** kopioi modin projektiin omaksi tiedostokseen ja lisää peliin include-rivin. `patch`-semantiikka pysyy samana.
- **Tarkistus.** Resolvoitu määrittelyjoukko lasketaan ennen vientiä (perus + modi) ja sen jälkeen (uusi perus). Jos joukot eroavat, vienti hylätään ja erot näytetään.
- **Valmis, kun** viedyn ja modina ladatun pelin määrittelyjoukot ovat samat.

### 8. Yhdistäminen perustiedostoihin

- **`rlml mod merge NIMI`** hakee jokaiselle patchille kohdemäärittelyn lähdepaikan jäsennyspuusta ja korjaa tekstiä pisteenä: vain muuttuva arvo korvataan, joten kommentit ja muotoilu säilyvät.
- **Merkinnät.** `drop` poistaa kentän tai määrittelyn, `new` lisää kentän määrittelyn loppuun, ja muuttunut runko korvataan kokonaisena.
- **STD ja muut suojatut tiedostot.** Jos kohde on vakiokirjastossa tai tiedostossa, jota ei haluta muuttaa, patch jää jäännösmodiin ja siitä raportoidaan.
- **Diff.** Tulos näytetään diffinä, ja tiedostot kirjoitetaan vasta hyväksynnän jälkeen. Sama tarkistus kuin vaiheessa 7. Lopuksi modi arkistoidaan, eikä työkalu tee committia.
- **Valmis, kun** yhdistetyn pelin määrittelyjoukko on sama kuin modina ladatun, ja kommentit ovat paikallaan.

### 9. Breakpointit ja askellus (kevyt)

- **Koukku.** `run_frames` kutsuu koukkua jokaisen lauseen kohdalla. Ilman breakpointeja kustannus on yksi haara lausetta kohden.
- **Breakpoint** sidotaan lähderiviin ja muunnetaan latauksessa rakenteelliseksi poluksi.
- **Pysähdys.** Kun breakpoint osuu, frontend ajaa oman debug-silmukkansa koukun sisällä. Tilaa voi tarkastella inspectorilla mutta ei muokata, koska toiminto on kesken.
- **Askellus** (into, over, out) perustuu kutsusyvyyteen, jonka `enter` ja `leave` jo laskevat.
- **MCP.** Breakpoint palauttaa tilan kuvan ja jatkaa vasta erillisestä kutsusta, tai ajo pysähtyy omaan säikeeseensä. Ratkaistaan tässä vaiheessa.
- **Valmis, kun** TUI pysähtyy käsittelijän riville, näyttää lokaalit ja askeltaa seuraavaan lauseeseen.

## Avoimet kysymykset

- **Toisto latauksen yli.** `reload`-rivi kertoo tiivisteet, mutta toisto tarvitsee ladatut tiedostot. Vaihtoehdot: toisto hylkää rivin, jonka tiivisteet eivät täsmää nykyisiin tiedostoihin, tai syötelokin rinnalle tallennetaan kunkin latauksen tiedostot. Päätetään vaiheessa 4.
- **Ruudun alkuperäisten arvojen syntaksi.** Funktiopari kuten `tile_original(p, layer)` ja `tile_changed(p)` vai jokin muu muoto (Erot dokumenttiin). Päätetään vaiheessa 4.
- **Inspectorin muokkaus moninpelissä.** Kenen istuimen nimissä muokkaus kirjataan syötelokiin. Päätetään vaiheessa 6.

## Ei tässä suunnitelmassa

- Tavukoodi, AOT-käännös ja käännös C:hen, Rustiin tai JS:ään.
- Hot reload web-versiossa.
- Täydet breakpointit, joissa ajo palaa frontendin pääsilmukkaan kesken funktion. Ne vaatisivat tulkin uudelleenkirjoituksen jatkettaviksi kehyksiksi.
