# Kohtaukset ja dialogit – toteutusohje

Tämä on ohje Claude Codelle. Siinä kerrotaan, mitä kohtauksista ja dialogeista toteutetaan ja missä järjestyksessä. Speksi on luvuissa [RS 9](roguescript/09-kohtaukset-ja-aikajanat.md) ja [RS 11.1](roguescript/11-tallennus-ja-korvaaminen.md#111-tallennus) sekä moottorin luvuissa [M 4.1](moottori/04-satunnaisuus-ja-toistettavuus.md#41-satunnaisvirrat), [M 5.2–5.4](moottori/05-aloitus-tallennus-ja-checkpointit.md#52-tallennus), [M 6.1.6–6.1.7](moottori/06-ajo.md#616-monivaiheinen-komento), [M 7.4–7.5](moottori/07-esitys.md#74-loki-ja-dialogit) ja [M 8](moottori/08-syote.md) (kontekstit `SCENE` ja `DIALOGUE`). Ohje ei muuta speksiä. Vaihe 0 tuo speksiin ne kohdat, jotka toteutus pakottaa.

## Päätökset

- **Korutiini ensin, aikajana sitten.** Dialogi ja haarautuva tarina ovat korutiinikohtauksia, ja ne vapauttavat eniten muuta työtä: `EXIT.LEAVE_OTHERS`, komennon `wait scene`, `wait confirm` ja `open`, luku- ja tekstikentät sekä `on scene` -tallennuksen. Aikajanat (kello, raidat, tweenit) tulevat niiden jälkeen, ja taustakohtaukset viimeisinä.
- **Kohtaus kulkee kehyksinä kuten komento.** Tulkki ajaa komennon rungon kehyksinä ja pysähtyy `wait`-kohtaan (vaihe 3 tallennuksessa). Korutiinikohtaus käyttää samaa koneistoa, ja pysähtymiskohtia ovat `say`, `choice`, `wait confirm`, `wait scene`, `open`, `wait number` ja `wait text`. Ajoitin palauttaa silloin uuden tilan `Status::Scene`, ja käyttöliittymä vastaa (`Runner::scene_confirm`, `scene_choose(i)`, `scene_answer(v)`), kuten se vastaa nyt syötteen odotukseen.
- **Luonnos on tilannekuva.** Etualan kohtauksen muutokset tehdään suoraan tilaan, ja kohtauksen alussa otetaan tilannekuva (sama `Snapshot` kuin checkpointissa). `restart` palauttaa kuvan ja ajaa kohtauksen alusta. Kohtauksen loppu ei tee mitään erityistä, koska muutokset ovat jo tilassa. Ulospäin tämä näkyy samoin kuin speksin luonnos (RS 9.8): tallennus on estetty, eikä pelin aika kulje.
- **Esityksen virta.** Kohtaus saa oman satunnaisvirtansa pelin siemenestä, kohtauksen nimestä ja käynnistyksen järjestysnumerosta (M 4.1). Pelin virta ei etene kohtauksessa, joten `restart` ja ohitus toistuvat samanlaisina.
- **Syöteloki.** Kohtauksen vastaukset ovat lokin rivejä: `scene.confirm`, `scene.choose N` ja `scene.answer ARVO`, sekä `skip` ja `restart`. Toisto ajaa ne (M 4.4).
- **Esimerkkipeli.** Uusi pieni peli `esimerkit/kohtaus` toimii testinä: kuningas, jonka kanssa puhutaan (dialogi, `hide when` ja `disabled when`), portti, joka aukeaa aikajanalla, ja vartija taustakohtauksessa. Peli on kirjoitettu speksin esimerkkien mukaan, joten se testaa myös niitä.

## Lähtötilanne (commit 7853d7f)

- **Parseri ja tarkistus.** `ItemKind::Scene` ja `ItemKind::Timeline` (raidat, rivit, merkit ja ehdot) jäsentyvät, ja tyyppitarkistus tuntee `say`-, `start`-, `stop`- ja `choice`-kutsut.
- **Ajossa** `eval.rs:283` hylkää `wait scene`-, `wait confirm`-, `open`-, `choice`- ja `wait number/text`-lauseet. `start` ei ole moottorin funktio vaan kohtauksen kutsu, joka kaatuu ("a scene cannot be called"). `say` puuttuu. Esitysfunktiot (`play`, `pan`, `shake`, `fade`, `reveal`) ovat tyhjiä.
- **Sovellus.** Kontekstit `SCENE` ja `DIALOGUE` oletussidontoineen ovat olemassa, mutta niitä ei käytetä. Valikkopino ja kerrokset (`menu.rs`) sopivat `choice`-valikolle, ja vaiheen 5 latausluettelo näyttää, miten dynaaminen valikko tehdään.
- **Odottava komento.** Polku (`rt::path`) numeroi jo kaikki odotuskohdat, myös `wait scene`-, `open`- ja `wait number` -kohdat, joten komennon odotukset vaativat vain ajon.
- **Vakiokirjasto.** `EXIT.LEAVE_OTHERS` on määritelty, mutta `USE_EXIT` kirjoittaa toistaiseksi viestin dialogin sijaan (STD 5 ja 14).

## Vaiheet

Jokaisen vaiheen jälkeen:
- `cargo test --workspace` on vihreä,
- golden-transkriptit ja Kapteeni Aaltolan testit eivät muutu,
- vaiheen testit ajavat syötelokinsa toistona samaksi tilaksi.

Jokainen vaihe on oma haaransa ja commitinsa.

### 0. Speksin aukot

Tehty 25.11.2026:
- **Syötelokin rivit** M 4.4:ään. Valinta kirjataan kohdan nimellä eikä järjestysnumerolla, koska piilotetut kohdat muuttavat numeroita. Aikajanan kello ei ole syöte.
- **Sama kohtaus** voi olla jonossa useaan kertaan (M 6.1.7).
- **Käynnistyslaskuri** kuuluu tilaan ja tallennukseen (M 3.1 ja M 5.2).
- **Kohtaus ilman pelaajaa:** M 8.2 sanoo jo, että kohtausta ohjaa avaajan istuin, ja jos avaaja ei ole jäsen, mikä tahansa istuin. Esimerkkipelin `begin`-käsittelijän käynnistämää kohtausta ohjaa siis mikä tahansa istuin.

Alla on alkuperäinen tehtävälista.

- **Sama kohtaus jonossa kahdesti.** Speksi ei sano, mitä tapahtuu, kun samaa kohtausta pyydetään jonoon toistamiseen ennen kuin edellinen on alkanut. Ehdotus: molemmat ajetaan, koska jono on aloitusjärjestyksessä.
- **Kohtaus pelaajan kuoltua.** Kenen istuin ohjaa kohtausta, kun avaajaa ei ole (M 8.2 sanoo: mikä tahansa istuin)? Tarkistetaan, että `say` toimii ilman pelaajaa.
- **Syötelokin rivit kohtauksessa** M 4.4:ään (yllä).
- Muutokset kirjataan muutoslokeihin.

### 1. Korutiinikohtaus ja `say`

Tehty 25.11.2026 haarassa `claude/kohtaukset-1`:
- **Ydin (`rt::scene`):**
  - `start(X())` laittaa kohtauksen jonoon (`State::scene_queue`), ja jono alkaa toiminnon lopussa sekä ennen ensimmäistä vuoroa `begin`-käsittelijän jälkeen.
  - Kohtaus kulkee kehyksinä: `say` ja `wait confirm` pysäyttävät sen (`World::pause`, `Status::Scene`), ja `wait scene` ajaa alikohtauksen samalla pinolla.
  - Luonnos on tilannekuva. `restart` palauttaa sen ja poistaa ajon viestit, ja `skip` jättää repliikit ja `log`-viestit pois.
  - Esityksen virta lasketaan siemenestä, nimestä ja käynnistyslaskurista (`State::scene_starts`).
  - Syötelokin rivit ovat `scene.confirm`, `skip` ja `restart`, ja toisto ajaa ne.
- **Kohtauskutsu** on arvo `Value::Scene`. `say` toimii vain kohtauksessa.
- **Sovellus:**
  - dialogi-ikkuna kartan alareunassa: puhujan nimi reunassa ja rivitetty repliikki, `wait confirm` näkyy merkkinä `▼`,
  - konteksti `SCENE` (`enter` tai `space` kuittaa, `tab` ohittaa, `r` aloittaa alusta) ja napautus kuittaa,
  - tallennus on estetty kohtauksen aikana, joten myös `quit` jättää tallentamatta,
  - lokin ryhmä `SPEECH` näyttää repliikit.
- **Pääte ilman käyttöliittymää:** `rlml run` kuittaa näppäimillä `enter` ja `space`, ohittaa näppäimellä `tab` ja aloittaa alusta näppäimellä `r`.
- **Ei tehty:** `rewind`, joka tulee aikajanojen kanssa. Kohtauksen ohjaajaa ei vielä rajata avaajan istuimeen, vaan mikä tahansa laite kuittaa.
- **Valmiusehto muuttui.** Esimerkkipelin kohtaukset tarvitsevat heti valintoja ja tekstikentän (vaihe 2), joten vaihe 1 testattiin omalla pelillä (`crates/rlml-core/tests/scenes.rs` ja sovellustesti). Oz-esimerkin portinvartija näyttää ensimmäisen repliikkinsä ja pysähtyy nimen kysymiseen.

Alla on alkuperäinen tehtävälista.

- **Ytimessä:**
  - `start(X(…))` laittaa etualan kohtauksen jonoon (nimi ja argumentit) ja palauttaa kahvan.
  - Toiminnon lopun vaihe 6 (M 6.1.7) aloittaa jonon kohtaukset aloitusjärjestyksessä.
  - Kohtauksen `map` ja kontekstinimet ratkaistaan RS 9:n mukaan.
- **Pysähtymiskohdat:**
  - `say(AVAIN, …, speaker:)` pysäyttää kohtauksen tilaan `Status::Scene(Say { avain, argumentit, puhuja })`.
  - `log` toimii kuten muualla.
  - `wait confirm` on `say` ilman tekstiä.
  - `wait scene Y()` ajaa alikohtauksen samalla pinolla.
  - `return` ja rungon loppu päättävät kohtauksen. Kun jono tyhjenee, ajo jatkuu saman mikroaskeleen seuraavasta toiminnosta.
- **Luonnos ja kohtauksen toiminnot:** kohtauksen alussa otetaan tilannekuva. `restart` palauttaa sen, ja `skip` ajaa loput ohitussäännöin (RS 9.8: `say` jätetään väliin, ja `choice` pysäyttää ohituksen).
- **Pelin aika** pysähtyy, ja tallennus on estetty (`save_blocked` myös kohtauksessa).
- **Sovellus:**
  - modaalinen dialogi-ikkuna (M 7.4), jossa on puhujan nimi ja rivitetty teksti,
  - konteksti `SCENE`: `confirm` kuittaa, ja `skip` ja `restart` toimivat,
  - `say`-repliikit menevät myös viestiryhmään `SPEECH`, jos peli ottaa sen lokiinsa.
- **Valmis, kun** esimerkkipelin kuningas puhuu kaksi repliikkiä, jotka kuitataan. Lisäksi ohitus ja alusta aloitus toimivat, ja toisto tuottaa saman tilan.

### 2. Dialogit, valikot ja kentät

Tehty 25.11.2026 haarassa `claude/kohtaukset-2`:
- **`choice`:**
  - `hide when`, `disabled when` ja kohdan parametrit arvioidaan kerralla (`World::choice_items`).
  - Valittu kohta ajaa lohkonsa, ja kohtaus jatkuu `choice`-lauseen jälkeen.
  - Säädettävä kohta (`adjust … pool`) liikkuu yhden kerrallaan välinsä ja poolinsa rajoissa.
  - Ohitus pysähtyy valintaan.
- **`open VALIKKO(…)` ja `x = open …`** odottavat valikon sulkeutumista. `x` saa arvon `none`, kunnes listavalikot toimivat. Paneeli aukeaa modaalisena kuten sidonnasta.
- **`x = wait number(a..b)` ja `x = wait text(N)`:** luku välin sisällä, teksti leikattuna, ja peruutus antaa `none`.
- **Syötelokin rivit:** `scene.choose`, `scene.adjust`, `scene.answer` ja `scene.close`. Toisto ajaa ne.
- **Sovellus:**
  - valinta on valikko kontekstissa `DIALOGUE`: himmeät kohdat, säädettävä kohta muodossa `Voima ◂ 12 ▸`, vasen ja oikea säätävät, ja `confirm` nostaa arvoa yhdellä,
  - dialogilla ei ole `back`-toimintoa,
  - tekstikentässä näkyvät kirjoitettu teksti ja raja; `enter` antaa vastauksen, `esc` peruu ja napautus antaa vastauksen.
- **`rlml run`:** numeronäppäin valitsee näkyvän kohdan, ja `enter` peruu kentän ja sulkee `open`-odotuksen.
- **Speksin tarkennus.** Speksin mukaan `choice`-kohtien ehdot arvioidaan kerran (RS 9.7). Säädettävän kohdan jälkeen ne arvioidaan uudelleen, jotta Valmis-kohta aukeaa, kun pisteet on jaettu, kuten RS 9.7:n `CREATE` vaatii. Tarkennus on kirjattu lukuun RS 9.7 ja muutoslokiin.
- **Väliaikainen sääntö.** Aikajanan käynnistys (`start`) jätetään väliin ajon varoituksella, kunnes vaiheet 4 ja 5 on tehty, joten Oz-esimerkki on pelattavissa.
- **Testit:** `crates/rlml-core/tests/scenes.rs` (`KING.TALK`, `GNOME.RIDDLE` ja `CREATE` sovitettuina sekä `open`) ja sovellustesti, joka pelaa Oz-esimerkin portinvartijan, Ozin ja Variksenpelättimen dialogit.

Alla on alkuperäinen tehtävälista.

- **`choice`:**
  - kohdat tekstiavaimina,
  - `hide when` ja `disabled when` arvioidaan kerralla `choice`-lauseen alkaessa,
  - kohdan parametrit,
  - säädettävä kohta `adjust … pool`.
- **Sovellus:** valinta näytetään valikkona kontekstissa `DIALOGUE`.
- **`open` korutiinissa:**
  - `open VALIKKO(…)` odottaa, kunnes valikko suljetaan,
  - `x = open` palauttaa listavalikon valinnan, kun listavalikot ovat käytettävissä,
  - `open PANEELI(…)` näyttää paneelin modaalisena.
- **Luku- ja tekstikentät:** `x = wait number(a..b)` ja `x = wait text(N)`. Sovellus saa kentät, jotka puuttuvat README:n mukaan.
- **Valmis, kun** RS 9.7:n `KING.TALK`, `GNOME.RIDDLE` ja `CREATE` toimivat esimerkkipelissä.

### 3. Odottava komento ja vakiokirjasto

- **Komennon odotukset:** `wait scene`, `wait confirm`, `open`, `wait number` ja `wait text` komennossa. Ne päättävät toiminnon, ja komento jatkuu kohtauksen jälkeen uutena toimintona (M 6.1.6). Polku ja tallennus ovat jo valmiina.
- **Tallennus:**
  - `on scene` tallentaa kohtausta edeltävän tilan (M 5.4),
  - jonossa oleva kohtaus tallentuu nimenä ja argumentteina,
  - ladattaessa kohtaus ajetaan alusta.
- **Vakiokirjasto:** `USE_EXIT` käynnistää jälleen dialogin `EXIT.LEAVE_OTHERS`. Väliaikainen viesti `MESSAGE.EXIT.OTHERS_AWAY` poistuu, ja STD 5 ja 14 päivitetään.
- **Valmis, kun:**
  - kolmen istuimen ryhmä kysyy portailla, lähdetäänkö, ja `YES` vie kaikki,
  - komento, joka odottaa `wait scene` -kohtaa, jatkuu oikein myös tallennuksen ja latauksen yli.

### 4. Aikajanat etualalla

- **Kello** laskee esitysaikaa. Käyttöliittymä vie sitä eteenpäin `tick`-kutsuissa, ja välikehykset näkyvät.
- **Rivit:** alku, kesto, merkit, `+`-ajat, saman hetken järjestys ja `when … else`.
- **Raidat** kulkevat rinnakkain, ja `loop` toistaa raitaa. `stop(track:)` pysäyttää silmukan.
- **Tweenit:**
  - `move` kestolla etenee kokonaisina ruutuina (reitti lasketaan kerran, ja este pysäyttää),
  - joukon `tile` ja `carve` etenevät ruutu kerrallaan,
  - yhden ruudun muutos tehdään heti ja häivytetään.
- **`say`** pysäyttää kellon. Ohitus (taulukko RS 9.8) ja `rewind` toimivat.
- **Esitysfunktiot** `pan`, `shake`, `fade` ja `play` näkyvät päätteessä niin hyvin kuin pääte sallii, ja loput jäävät tyhjiksi. Kirjataan, mitkä ne ovat.
- **Valmis, kun** RS 9.1:n `GATE_OPENS` ja RS 9.3:n `AUDIENCE` toimivat esimerkkipelissä, ja ohitettu kohtaus päätyy samaan tilaan kuin loppuun katsottu.

### 5. Taustakohtaukset

- **`background`:**
  - pelin aika kierroksina,
  - rivit ajetaan kierroksen rajalla (M 6.1.4),
  - tweenien askeleet ajetaan kohtauksen herätyksinä.
- **`background realtime`:** esitysaika, joka kulkee myös syötettä odotettaessa. Ei tilan muutoksia.
- **Hallinta:** `start(…, scope: map, bind: e)`, `stop(kahva)` ja useita yhtä aikaa.
- **Tallennus:** nimi, argumentit, `scope`, `bind`, kello, silmukoiden vaiheet ja keskeneräisen tweenin jäljellä oleva reitti (RS 11.1). Toisto.
- **Valmis, kun** RS 9.9:n `PATROL` kulkee esimerkkipelissä tallennuksen ja latauksen yli samoin kuin ilman.

## Ei tässä vaiheessa

- Graafisen esitystavan sujuva liuku (`smooth`), äänitiedostot ja `render audioscript`.
- `rewind` aikajanalla, jos se osoittautuu raskaaksi. Tarvittaessa siitä tehdään oma vaiheensa.
