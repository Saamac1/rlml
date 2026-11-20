# rlml-web – toteutusohje

Ohje Claude Codelle. Suunnitelma ja perustelut ovat claude.ai-dokumentissa
"rlml-web – suunnitelma" (20.11.2026). Tämä tiedosto kertoo, mitä tehdään
ja missä järjestyksessä.

## Päätökset

- Piirto: terminaalin merkit canvasille. Ei tilesettejä.
- Ensimmäinen versio pelaa vain Kapteeni Aaltolan seikkailua
  (`~/kapteeni-aaltola/roguelike`) paketoituna. Ei pelivalikkoa pelin
  valintaan eikä pelaajan omaa kansiota.
- `rlml-web` on crate tässä repossa kuten `rlml-tui`.
- Ei tallennusta, profiilia eikä ääniä (ydin ei vielä tallenna).
- Nopeus mitataan vaiheessa 4: paneelikyselyt joka kuvassa.

## Lähtötilanne (commit b845e79)

- `rlml-core` ei käytä käyttöjärjestelmää. Tiedostot tulevat
  `load::Files`-traitin kautta, ja `load::MemFiles` on valmiina.
- `rlml-tui`:n `app`, `draw`, `input`, `menu`, `cursor` ja `panel` käyttävät
  vain `rlml-core`a ja `termplate-core`a. Ainoa OS-kutsu on
  `draw.rs`:n `rgb()`, joka lukee `COLORTERM`:in.
- OS-sidottua on vain `term.rs` (crossterm, `Instant`, `thread::sleep`) ja
  `pad.rs` (gilrs-säie). `pad.rs`:n `Raw` ja `PadState` ovat puhtaita.
- `App` on tapahtumaohjattu: `key(b, now)`, `mouse_press(b, x, y, now)`,
  `pointer_at(x, y)`, `pad(id, p, now)`, `tick(now)`, `resize(w, h)`,
  `wait()`, `frame_time()`, `draw() -> Frame`, välikuvat `app.frames` ja
  `app.quit`.

## Vaiheet

Jokaisen vaiheen jälkeen `cargo test --workspace` on vihreä, `rlml play`
toimii ennallaan ja Kapteeni Aaltolan testit menevät läpi:
`~/kapteeni-aaltola/roguelike/testit/aja.sh ~/rlml/target/debug/rlml`.
Jokainen vaihe on oma commitinsa.

### 1. Wasm-tarkistus

    rustup target add wasm32-unknown-unknown
    cargo build -p rlml-core --target wasm32-unknown-unknown

Odotus: menee läpi ilman muutoksia. Jos ei mene, korjaa ydin ennen muuta.

Tulos: meni läpi ilman muutoksia. Ytimessä ei ole `Instant`-, `SystemTime`-,
säie-, `env`- eikä `fs`-kutsuja, jotka kääntyisivät mutta kaatuisivat
wasmissa ajettaessa.

### 2. Truecolor parametriksi

`draw.rs`:n `rgb()` ei lue ympäristöä. Truecolor-lippu annetaan `View`lle
(tai `App::new`:lle), ja `term.rs` lukee `COLORTERM`:in kuten nyt.
Selain antaa aina `true`.

### 3. Crate-jako: `rlml-app`

- Uusi crate `crates/rlml-app`, riippuvuudet `rlml-core` ja `termplate-core`
  (sama git-rev kuin nyt). Ei crosstermia eikä gilrsiä.
- Siirrä sinne `app.rs`, `draw.rs`, `input.rs`, `menu.rs`, `cursor.rs`,
  `panel.rs` sekä `pad.rs`:stä `Raw` ja `PadState`. Testit `tests/app.rs`,
  `tests/draw.rs` ja `tests/input.rs` siirtyvät mukana.
- `rlml-tui`:hin jäävät `term.rs` ja `pad.rs`:n `spawn()`.
- Siirrä `term.rs`:n tapahtumien kokoaminen (kaikki odottavat tapahtumat
  ennen kuvaa, hiiren liikkeistä vain viimeinen) `rlml-app`iin funktioksi,
  jota molemmat frontendit kutsuvat.
- Tarkista lopuksi:
  `cargo build -p rlml-app --target wasm32-unknown-unknown`.

### 4. `rlml-web`, minimi

- Crate `crates/rlml-web`, `crate-type = ["cdylib", "rlib"]`, riippuvuudet
  `rlml-app`, `rlml-core`, `wasm-bindgen`, `js-sys`, `web-sys`.
  Workspaceen jäseneksi; varmista, ettei `cargo test --workspace` yritä
  ajaa sitä natiivina, jos se ei käänny (`cfg(target_arch = "wasm32")`).
- **Paketti:** skripti `crates/rlml-web/bundle.sh PELIKANSIO` tekee
  `game.json`-tiedoston `{ "main": "game.rlml", "game": {polku: sisältö},
  "engine": {polku: sisältö} }` pelikansiosta ja repon `lib/`-kansiosta.
  Vain `.rlml`, `.md`, `.rlml.md` ja `.rogue` (M 2.1), ei `testit/`-kansiota.
  Sivu hakee sen fetchillä ja antaa sen wasmille, joka rakentaa `MemFiles`in
  ja ajaa `rlml_core::check_game`. Latausvirheet näytetään sivulla.
- **Elinikä:** `App<'m>` lainaa `Model`ia. Wasm-bindgenin luokka ei voi
  kantaa lainaa, joten `Box::leak` malli kerran pelin latauksessa.
- **Worker:** wasm ja `App` ajetaan Web Workerissa. Pääsäie lähettää
  syötteen viesteinä ja saa takaisin muuttuneet solut (diff edelliseen
  `Frame`en workerissa). Aika on `performance.now()`, odotus
  `setTimeout(app.wait())`, joka perutaan uuden syötteen tullessa.
  Välikuvat `app.frames` lähetetään `frame_time()`-välein.
- **Piirto:** canvas, monospace-fontti, solun koko fontin mitoista,
  `devicePixelRatio` huomioiden. Leveät merkit (`unicode-width` 2) vievät
  kaksi solua kuten terminaalissa.
- **Näppäimet:** `keydown` → `termplate_core::keys::Binding`. Kuvaa
  `KeyboardEvent.key` samoihin nimiin kuin `termplate-term`in `map_event`
  crosstermin tapahtumille, jotta pelin sidonnat (esim. `h j k l`, nuolet,
  `esc`, `.`) toimivat samoin. `preventDefault` sidotuille näppäimille.
- **Valmis, kun** Kapteeni Aaltola pelautuu selaimessa alusta pikasiirtoon
  1 ja lokissa on samat rivit kuin `rlml run` samoilla näppäimillä.
  Mittaa `draw()`- ja `tick()`-ajat ja kirjaa ne tähän tiedostoon.

Tulos (20.11.2026):

- Ajo: `crates/rlml-web/build.sh ~/kapteeni-aaltola/roguelike` ja
  `python3 -m http.server -d crates/rlml-web/www 8000`, sitten
  <http://localhost:8000/> (`?seed=N`, `?stats` näyttää ajat kulmassa).
  Tarvitsee `wasm-bindgen-cli`n samassa versiossa kuin `Cargo.toml`
  (0.2.130). `web-sys`iä ei tarvittu: canvas, fetch ja ajastimet ovat
  JavaScriptissä, wasm saa paketin `JSON.parse`n oliona.
- Rust-puoli on natiivisti testattava (`keys`, `cells`, `bundle`,
  `game`); vain `wasm.rs` on `cfg(target_arch = "wasm32")`.
  Wasm-riippuvuudet ovat vain wasm-kohteelle, joten
  `cargo test --workspace` ei käännä niitä.
- Pääsäie lataa saman wasm-moduulin vain `key_binding`ia varten, jotta
  `preventDefault` osuu synkronisesti pelin sitomiin näppäimiin (worker
  lähettää niiden nimet latauksen jälkeen). Cmd-yhdistelmät jäävät
  selaimelle.
- Loki: `jsc/smoke.sh` ajaa oikean wasm-moduulin ja `game.json`in
  workerin kutsuilla macOS:n JavaScriptCoressa. Kaikki neljä
  `testit/*.keys`-läpipeluuta ja pikasiirto 1 antavat rivilleen saman
  lokin kuin `rlml run`. Selaimessa pelaamista en ole voinut kokeilla.
- Ajat, 100×34 solua, wasm JavaScriptCoressa (release), ms:

  | läpipeluu            | draw ka | p95 | max | input ka | max | tick |
  |----------------------|--------:|----:|----:|---------:|----:|-----:|
  | lapipeluu            | 23 | 33 | 38 | 1,7 |  8 | < 0,01 |
  | kotimatka            | 27 | 43 | 70 | 1,7 |  8 | < 0,01 |
  | kuoppa-ja-paineaalto | 14 | 16 | 27 | 2,2 | 26 | < 0,01 |
  | maito-ei-riita       | 15 | 18 | 26 | 1,9 | 15 | < 0,01 |

  `draw` = `App::draw()` ja muuttuneiden solujen koodaus; koodaus on
  alle 1 % siitä (`examples/replay.rs` natiivisti: `draw` ka 29,8 ms,
  pelkkä `App::draw()` 29,7 ms). `input` on näppäimet ja vuoro. Natiivi
  release ei ole nopeampi kuin wasm. **Piirto on pullonkaula:** 15–30
  ms kuvaa kohden (enimmillään 70 ms) näkyy viiveenä, kun kuvia tulee
  peräkkäin (kävely, välikuvat). Suunnitelma arveli syyksi
  paneelikyselyt joka kuvassa; sitä ei ole vielä varmistettu. Seuraava
  askel on profiloida `App::draw()`.

### 5. Loput syötteestä

- Hiiri: pikselit soluiksi, `mouse_press` ja `pointer_at`, liikkeet
  koottuina (vaihe 3). Kapteeni Aaltola käyttää klikkausmatkaa ja hoveria.
- Koko: `ResizeObserver` → sarakkeet ja rivit → `app.resize`.
- Peliohjain: gilrs tukee wasmia, joten kokeile ensin gilrsiä workerin
  ulkopuolella pollattuna joka kuvassa (`next_event`, ei säiettä) ja anna
  `Raw` → `PadState` → `app.pad`. Jos gilrs ei toimi, Gamepad API
  JavaScriptissä tuottaa samat `Raw`-tapahtumat.
- `exit` palaa alkuun, koska välilehteä ei voi sulkea.

## Ei tässä vaiheessa

Pelivalikko usealle pelille, File System Access API, OPFS ja `Store`-trait
(tehdään tallennuksen kanssa), äänet, tilesetit ja ulkoinen generaattori
(`external` palauttaa `false`, M 9.3).
