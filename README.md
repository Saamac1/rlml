# RLML 0.3

RLML on roguelike-pelien kuvauskieli: pelin data kirjoitetaan tekstitiedostoina (kartat, entiteetit, pakat, käyttöliittymä), pelilogiikka RogueScriptillä, ja moottori lataa ja ajaa ne. Speksi on jaettu neljään dokumenttiin, ja jokainen dokumentti lukuihin, yksi tiedosto lukua kohden.

| Dokumentti | Lyhenne | Sisältö |
|---|---|---|
| [RLML-dataformaatit](dataformaatit/README.md) | D | mitä kirjoittaja kirjoittaa datana: syntaksi, tiedostot, kartat, ulkoasu, tekstit ja käyttöliittymä |
| [RogueScript](roguescript/README.md) | RS | pelilogiikan kieli: tapahtumat, funktiot, komennot, generointi ja kohtaukset |
| [RLML-moottorin vastuut](moottori/README.md) | M | mitä moottori lupaa tehdä: lataus, pelin tila, ajo, esitys, syöte ja tallennus |
| [RLML-vakiokirjasto](vakiokirjasto/README.md) | STD | RLML:llä ja RogueScriptillä kirjoitettu kirjasto, joka tekee tavallisesta roguelikesta pelattavan |

## Sisällys

### [RLML-dataformaatit](dataformaatit/README.md) (D)

- [1. Rajaus ja periaatteet](dataformaatit/01-rajaus-ja-periaatteet.md)
- [2. Yleinen syntaksi](dataformaatit/02-yleinen-syntaksi.md)
- [3. Tiedostot ja include](dataformaatit/03-tiedostot-ja-include.md)
- [4. Nimiavaruudet](dataformaatit/04-nimiavaruudet.md)
- [5. Komponentit ja entiteetit](dataformaatit/05-komponentit-ja-entiteetit.md)
- [6. Pakat ja taulut](dataformaatit/06-pakat-ja-taulut.md)
- [7. Tilet, maastokerrokset, tilejoukot ja ominaisuudet](dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md)
- [8. Kartat ja prefabit](dataformaatit/08-kartat-ja-prefabit.md)
- [9. Ulkoasu](dataformaatit/09-ulkoasu.md)
- [10. Tekstit](dataformaatit/10-tekstit.md)
- [11. Käyttöliittymän data](dataformaatit/11-kayttoliittyman-data.md)
- [12. Äänten ilmoitukset](dataformaatit/12-aanten-ilmoitukset.md)
- [13. Avoimet kysymykset](dataformaatit/13-avoimet-kysymykset.md)
- [14. Osiot ja otsikkorivit](dataformaatit/14-osiot-ja-otsikkorivit.md)
- [Muutokset](dataformaatit/muutokset.md)

### [RogueScript](roguescript/README.md) (RS)

- [1. Rajaus ja periaatteet](roguescript/01-rajaus-ja-periaatteet.md)
- [2. Yleinen syntaksi](roguescript/02-yleinen-syntaksi.md)
- [3. Komponentit](roguescript/03-komponentit.md)
- [4. Tapahtumat ja käsittelijät](roguescript/04-tapahtumat-ja-kasittelijat.md)
- [5. Funktiot ja lausekkeet](roguescript/05-funktiot-ja-lausekkeet.md)
- [6. Ruudut ja paikkahaku](roguescript/06-ruudut-ja-paikkahaku.md)
- [7. Komennot ja tekoäly](roguescript/07-komennot-ja-tekoaly.md)
- [8. Generointi](roguescript/08-generointi.md)
- [9. Kohtaukset ja aikajanat](roguescript/09-kohtaukset-ja-aikajanat.md)
- [10. Aloitus ja checkpointit](roguescript/10-aloitus-ja-checkpointit.md)
- [11. Tallennus ja korvaaminen](roguescript/11-tallennus-ja-korvaaminen.md)
- [12. Kielioppi](roguescript/12-kielioppi.md)
- [13. Avoimet kysymykset](roguescript/13-avoimet-kysymykset.md)
- [Kohtausopas](roguescript/kohtausopas.md)
- [Muutokset](roguescript/muutokset.md)

### [RLML-moottorin vastuut](moottori/README.md) (M)

- [1. Rajaus ja periaatteet](moottori/01-rajaus-ja-periaatteet.md)
- [2. Lataus](moottori/02-lataus.md)
- [3. Pelin tila](moottori/03-pelin-tila.md)
- [4. Satunnaisuus ja toistettavuus](moottori/04-satunnaisuus-ja-toistettavuus.md)
- [5. Aloitus, tallennus ja checkpointit](moottori/05-aloitus-tallennus-ja-checkpointit.md)
- [6. Ajo](moottori/06-ajo.md)
- [7. Esitys](moottori/07-esitys.md)
- [8. Syöte](moottori/08-syote.md)
- [9. Moottorin funktiot ja tapahtumat](moottori/09-moottorin-funktiot-ja-tapahtumat.md)
- [10. Mitä moottori ei tee](moottori/10-mita-moottori-ei-tee.md)
- [11. Avoimet kysymykset](moottori/11-avoimet-kysymykset.md)
- [Muutokset](moottori/muutokset.md)

### [RLML-vakiokirjasto](vakiokirjasto/README.md) (STD)

- [1. Rajaus, periaatteet ja lataus](vakiokirjasto/01-rajaus-periaatteet-ja-lataus.md)
- [2. Prioriteettivyöhykkeet](vakiokirjasto/02-prioriteettivyohykkeet.md)
- [3. Komponentit ja oletuspelaaja](vakiokirjasto/03-komponentit-ja-oletuspelaaja.md)
- [4. Tapahtumat](vakiokirjasto/04-tapahtumat.md)
- [5. Komennot](vakiokirjasto/05-komennot.md)
- [6. Tekoäly](vakiokirjasto/06-tekoaly.md)
- [7. Suhteet ja vihamielisyys](vakiokirjasto/07-suhteet-ja-vihamielisyys.md)
- [8. Käsittelijät](vakiokirjasto/08-kasittelijat.md)
- [9. Äänet, viestit ja tekstit](vakiokirjasto/09-aanet-viestit-ja-tekstit.md)
- [10. Generointi](vakiokirjasto/10-generointi.md)
- [11. Korvaaminen](vakiokirjasto/11-korvaaminen.md)
- [12. Minipeli STD:llä](vakiokirjasto/12-minipeli-std-lla.md)
- [13. Muutokset muihin dokumentteihin](vakiokirjasto/13-muutokset-muihin-dokumentteihin.md)
- [14. Avoimet kysymykset](vakiokirjasto/14-avoimet-kysymykset.md)
- [Muutokset](vakiokirjasto/muutokset.md)

## Esimerkit

- [Minipeli](esimerkit/mini/) (`esimerkit/mini/`): vakiokirjaston valmiustestin peli kokonaisena, kaksi luolaa, rotat ja portaat ilman RogueScriptiä ([STD 12](vakiokirjasto/12-minipeli-std-lla.md)). Portaille astuminen vie toiseen luolaan. Pystyssä olevassa puhelimessa lokin alla ovat kosketusnapit: suunnat, odota ja valikko (`esc`).
- [Ryhmä](esimerkit/ryhma/) (`esimerkit/ryhma/`): paikallisen moninpelin esimerkki ([RLML-moninpeli](RLML-moninpeli.md), [M 6.1.10](moottori/06-ajo.md#6110-istuimet-ja-ryhmä)). Kaksi istuinta pelaa vuorotellen samalla näppäimistöllä. Istuimella 1 on seuralainen Robin, jota pelaaja voi ohjata (`tab` ja `shift+tab`), ja talutushihna pitää ryhmän 20×8 solun kartta-alueen sisällä.

## Lataaja

`crates/` sisältää Rust-toteutuksen: lataajan, joka lukee pelin ja vakiokirjaston (`lib/std.rlml`, `lib/std/`) ja tarkistaa ne latausvirheiden ja -varoitusten osalta ([M 2](moottori/02-lataus.md)). RogueScript jäsennetään kokonaan ([RS 12](roguescript/12-kielioppi.md)), ja lataus tarkistaa logiikasta nimet, paikalliset nimet, odotusten ja kyselyjen säännöt sekä moottorin funktiot. Tyypit päätellään ja tarkistetaan asteittain: tyyppi, jota ei voi päätellä, ei aiheuta virhettä.

Ajo ([M 3–6](moottori/README.md)) on mukana myös ilman käyttöliittymää: `rlml run` pelaa uuden pelin, jossa jokainen näppäin on yksi pelaajan vuoro, ja tulostaa jokaisen vuoron jälkeen kartan, HP:n ja lokin. Sama siemen ja samat näppäimet tuottavat aina saman tulosteen. `--log-out FILE` kirjoittaa pelikerran syötelokin ([M 4.4](moottori/04-satunnaisuus-ja-toistettavuus.md#44-syöteloki)), myös `rlml play` -komennolla, ja `rlml replay PELI LOKI` toistaa sen. `rlml bench` ajaa näppäimet ilman piirtoa ja kertoo, kauanko lataus ja pelaaminen kestivät. Ominaisuuden `stats` kanssa (`cargo build --release -p rlml --features stats`) se näyttää myös tulkin laskurit ja ajan, joka kuluu käsittelijöiden hakuun, näkökenttään, kuulemiseen, tekoälyyn ja komentoihin ([RLML-kehitystyokalut](RLML-kehitystyokalut.md), vaihe 1). `rlml inspect PELI --keys "…" POLKU` tulostaa pelin tilan polulla, esimerkiksi `game.turn` tai `entity#12.HP.now`, ja `rlml play --dev` avaa saman inspectorin pelin päälle näppäimellä `f12` (vaihe 3).

```sh
cargo run -p rlml -- check esimerkit/mini
cargo run -p rlml -- run esimerkit/mini --seed 7 --keys "s s s j h h"
cargo test
```

## Pelaaminen

`rlml play` pelaa pelin päätteessä ([M 7](moottori/07-esitys.md) ja [M 8](moottori/08-syote.md)). Komento tarvitsee oikean päätteen, koska se lukee näppäimiä suoraan. Värit ovat 24-bittisiä, kun pääte kertoo tukevansa niitä (`COLORTERM=truecolor` tai `24bit`, esimerkiksi iTerm2, kitty ja WezTerm), ja muuten lähimmät 256 värin paletin värit, jotka näkyvät myös macOS:n Terminalissa.

```sh
cargo run -p rlml -- play esimerkit/mini --seed 7
```

Minipelin näppäimet tulevat sen tiedostosta [`ui/main.rlml`](esimerkit/mini/ui/main.rlml):

| Näppäin | Toiminto |
| --- | --- |
| `h` `j` `k` `l` tai nuolet | liiku tai lyö viereistä |
| `s` | odota vuoro |
| `>` `<` | portaat alas ja ylös |
| `z` | seuraava kierros heti, kun pelaajalla ei ole vuoroa (esimerkiksi kuoleman jälkeen) |
| `esc` | taukovalikko auki ja kiinni |
| `ctrl+c` | lopettaa heti |

Valikossa `j` `k` ja nuolet liikuttavat kursoria, `enter` ja `space` valitsevat ja `esc` tai `backspace` palaa. Pelin aika pysähtyy valikon ajaksi. Taukovalikon Lopeta-kohta päättää pelikerran ja avaa moottorin valikon, josta voi aloittaa uuden pelin uudella siemenellä tai poistua. Jos pelissä on `title`-valikko ([D 3.3](dataformaatit/03-tiedostot-ja-include.md#33-aloitus)), peli alkaa siitä ja Lopeta palaa siihen.

Kun pelaaja kuolee, maailma jatkaa kierros kerrallaan pelin `round`-tahdissa. Asetus `idle` antaa vuoron komennolle tai pelaajan tekoälylle, jos näppäintä ei paineta ([D 3.2](dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)). Ajon virhe lopettaa pelin, ja se tulostetaan päätteen palauttamisen jälkeen tiedoston, rivin ja sarakkeen kanssa.

**Moninpeli.** Ryhmäesimerkissä (`cargo run -p rlml -- play esimerkit/ryhma`) istuin 1 pelaa näppäimillä `h` `j` `k` `l` tai nuolilla ja odottaa näppäimellä `.`, ja istuin 2 pelaa näppäimillä `w` `a` `s` `d` ja odottaa näppäimellä `q`. Kummankin istuimen näppäimet toimivat vain sen omalla vuorolla. Kytketty peliohjain ottaa istuimen 2 näppäimistöosalta 2: ristiohjain tai vasen tatti liikkuu, `south` odottaa, `rb` ja `lb` vaihtavat ohjattavaa ja `start` avaa taukovalikon. Kun ohjain irrotetaan, istuin 2 palaa näppäimille `wasd`. Istuimella 1 `tab` vaihtaa istuimen 1 ohjattavaa Robinin ja sankarin välillä, ja `shift+tab` vaihtaa takaisin. Istuimen 1 sankari on keltainen Aino ja istuimen 2 sininen Eero, ja vuorossa olevan hahmon ruutu välähtää ja vilkkuu käänteisin värein. Robin seuraa lähintä sankaria, kun sitä ei ohjata. Seinää päin kävely kirjoittaa lokiin ja vie vuoron, toisin kuin minipelissä. Kamera seuraa koko ryhmää, ja kun ryhmä ei mahdu kartta-alueelle, se seuraa vuorossa olevaa hahmoa. Hiiri on yhteinen ja seuraa vuoroa: klikkaus kartalla kuljettaa vuorossa olevan sankarin ruutuun askel kerrallaan, ja kartan alla oleva rivi kertoo, mitä osoittimen alla näkyy. `f` (istuin 2: `r`, peliohjain: `north`) seuraa lähintä toista sankaria. Matka ja seuraaminen katkeavat mihin tahansa näppäimeen ja pysähtyvät, kun näkyviin tulee uusi toimija. `c` (istuin 2: `x`, peliohjain: `select`) avaa istuimen oman kursorin, joka pysyy auki vuorojen yli: suuntanäppäimet (istuin 2: `wasd`) siirtävät sitä, `enter` (istuin 2: `q`) kulkee sen ruutuun ja `f` (`r`) seuraa sen alla olevaa. Toisen istuimen vuorolla sama painallus jättää ennakkokäskyn, joka ajetaan istuimen seuraavalla vuorolla, ja `backspace` (`x`) peruu käskyn ja sitten sulkee kursorin. Kartan oikealla puolella on verbivalikko (Tutki, Lyö, Odota, Vaihda), joka näyttää vuorossa olevan istuimen pikanäppäimet. Hiirellä klikataan verbiä ja sitten kohdetta, ja kartan alla oleva lauserivi näyttää, mitä on tekemässä. Näppäimillä `v` (istuin 2: `g`, peliohjain: `lt`) siirtää fokuksen valikkoon, `enter` valitsee verbin ja kohde valitaan kursorilla. Oikea klikkaus peruu verbin, ja ilman verbiä se avaa ruudun viereen verbikolikon, jossa ovat vain ruutuun sopivat verbit; kursorissa saman tekee `v` (istuin 2: `g`, peliohjain: `west`). Komento, jolla on kysyttäviä parametreja, kysyy ne näppäimiltäkin: `i` (Tutki) näyttää listan näkyvistä kohteista kirjaimineen, kohdistettava parametri avaa kohdistimen kantaman sisällä tai asetuksen `targeting` mukaan listan tai valitsee lähimmän, ja suunta kysytään yhdellä suuntanäppäimellä; hiirellä klikataan kohdetta, ja oikea klikkaus tai `backspace` peruu ilman ajan kulumista. Valikot toimivat hiirellä: osoitin korostaa rivin, klikkaus valitsee ja klikkaus valikon ohi sulkee sen. Peli alkaa liittymisruudusta: näppäimistöosa 1 on istuin 1, osa 2 liittyy näppäimellä `e` ja peliohjain napilla `start`, ja sama näppäin aloittaa pelin. Ladattu peli kysyy laitteen jokaiselle tallennetulle istuimelle, ja `tab` ohittaa istuimen. Pelissä, jossa istuimia voi olla vaihteleva määrä, vapaa laite liittyy kesken pelin `join`-toiminnolla, ja taukovalikon `leave_seat` poistaa istuimen. `rlml run` ajaa näppäimet aina vuorossa olevalle istuimelle.

**Selaimessa.** Minipeli pelautuu osoitteessa <https://saamac1.github.io/rlml/> ja kohtausten esimerkkipeli (Ozin valtaistuinsali, `esimerkit/kohtaus`) osoitteessa <https://saamac1.github.io/rlml/kohtaus/>. Molemmissa on pystypuhelimessa kosketusnapit. Työnkulku [`.github/workflows/pages.yml`](.github/workflows/pages.yml) ajaa testit jokaisesta `main`in pushista, kääntää selainversion (`crates/rlml-web/build.sh esimerkit/mini`), paketoi kohtauspelin samalle sivulle alikansioon `kohtaus/` ja julkaisee molemmat `gh-pages`-haaraan.

**Tallennus.** `rlml play` tallentaa pelin datakansioon (`~/.local/share/rlml/PELI` Linuxilla tai `--data DIR`): tallennukset kansioon `saves/` ja profiilin ([D 3.5](dataformaatit/03-tiedostot-ja-include.md#35-profiili-profile)) tiedostoon `profile`. Lopetus tallentaa pelin `autosave`-osion mukaan ([D 3.4](dataformaatit/03-tiedostot-ja-include.md#34-automaattinen-tallennus-autosave)), ja moottorin valikon kohta Load game jatkaa tallennuksesta. Pelin oma `save`-sidonta tallentaa omaan paikkaan.

Moottorin valikon Reset profile nollaa profiilin.

Puuttuu vielä: listavalikot, paneelien kentät, kohtaukset ja dialogit, luku- ja tekstikentät ja äänet.

## Avoimet kysymykset

Jokaisessa dokumentissa on oma lukunsa avoimille kysymyksille:

- [Dataformaatit, luku 13](dataformaatit/13-avoimet-kysymykset.md)
- [RogueScript, luku 13](roguescript/13-avoimet-kysymykset.md)
- [Moottori, luku 11](moottori/11-avoimet-kysymykset.md)
- [Vakiokirjasto, luku 14](vakiokirjasto/14-avoimet-kysymykset.md)

## Kirjoitusohjeet

- **Yksi luku, yksi tiedosto.** Tiedoston nimi on luvun numero kahdella numerolla ja otsikko pienin kirjaimin ilman ääkkösiä (`11-kayttoliittyman-data.md`). Luvun otsikko on tiedoston `#`-otsikko ja aliluvut `##`-otsikoita (`## 11.1 Asettelu`).
- **Numerointi pysyy.** Lukujen ja alilukujen numerot ovat viittausten kohteita. Uusi luku tai aliluku saa seuraavan vapaan numeron, eikä olemassa olevia numeroita siirretä.
- **Viittaukset ovat linkkejä.** Saman dokumentin luku: `luku [11.1](11-kayttoliittyman-data.md#111-asettelu)`. Toinen dokumentti: `([M 7.1](../moottori/07-esitys.md#71-asettelu))` tai `(moottoridokumentti, luku [7.1](../moottori/07-esitys.md#71-asettelu))`. Ankkuri on GitHubin muoto otsikosta: pienet kirjaimet, välimerkit pois ja välilyönnit viivoiksi (`## 3.4 Automaattinen tallennus: autosave` → `#34-automaattinen-tallennus-autosave`).
- **Muutokset kirjataan.** Jokaisella dokumentilla on `muutokset.md`, jonka alkuun lisätään päivätty merkintä (`**Muutokset 29.10.2026 (aihe):** …`) viittauksineen muuttuneisiin lukuihin.
- **Piirrokset koodilohkoon.** Kartat, prefabit, asettelut ja paneelit kirjoitetaan koodilohkoon, jotta tasaus säilyy eikä `#`-rivejä tulkita otsikoiksi.
- **Uusi luku** lisätään dokumentin `README.md`:n sisällykseen, tämän tiedoston sisällykseen sekä viereisten lukujen ala-reunan navigointiriveille.
