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

- **Työkalu** `rlml bench PELI [--keys …] [--rounds N]`, joka ajaa näppäinsarjan tai kierroksia ilman piirtoa ja tulostaa ajan sekä laskurit: lauseet, nimihaut, `emit`-kutsut, käsittelijäkutsut ja kloonatut kehykset. Laskurit ovat kehityskäännöksen ominaisuus (`cfg(feature = "stats")`), joten julkaistu ajo ei maksa niistä.
- **Perustaso** Kapteeni Aaltolasta, minipelistä ja yhdestä raskaasta generointikartasta. Tulokset kirjataan tähän ohjeeseen, jotta vaiheen 2 hyöty näkyy.
- **Valmis, kun** mittaus toistuu samoina laskureina samalla siemenellä ja näyttää, kuinka suuri osa ajasta kuluu nimihakuun, kehysten kopiointiin ja käsittelijöiden etsintään.

### 2. Lowering ja slotit

- **Lowering-vaihe** tarkistuksen (`rs::check`) jälkeen tuottaa jokaisesta rungosta version, jossa lokaalit ovat slotteja ja isot nimet on resolvoitu latausajan tauluihin.
- **Lokaalit.** `Env.locals` muuttuu muotoon `Vec<Value>`, ja jokainen slotti alustetaan arvoon `none`. Slotit jaetaan deterministisesti esittelyjärjestyksessä, eikä niitä käytetä uudelleen sisemmissä lohkoissa.
- **Rungon nimitaulu.** Jokaisesta slotista kirjataan nimi, tyyppi, näkyvyysalue rakenteellisena polkuna (alku ja loppu) ja esittelypaikka. Sitä käyttävät tallennus, inspector ja virheilmoitukset.
- **Jaetut kehykset.** Kehykset viittaavat lauseisiin jaetusti (`Arc<[Stmt]>` ja indeksi). `run_to_end` ja `run_frames` eivät enää kopioi runkoja eivätkä lauseita.
- **Jakelutaulut.** Käsittelijät lasketaan latauksessa tapahtumittain ja vaiheittain valmiiksi järjestettyyn listaan (prioriteetti ja latausjärjestys, M 6.2), ja `override … as NIMI` -sääntö ratkaistaan samalla. `emit` lukee taulua eikä kloonaa jäsennyspuuta.
- **Tallennus** muuntaa slotit nimiksi kirjoitettaessa ja nimet sloteiksi luettaessa. Tallennusmuoto ei muutu. `body_hash` lasketaan edelleen lähteen jäsennyspuusta.
- **Valmis, kun** ulkoinen käytös ei muutu: kaikki testit, toistot ja vanhat tallennukset toimivat sellaisinaan, ja vaiheen 1 mittaus näyttää eron.

### 3. Inspector, luku

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
