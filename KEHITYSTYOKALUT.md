# RLML – modit ja debuggaus

[RLML](README.md) › Modit ja debuggaus

Tämä ohje kertoo pelintekijälle, miten RLML-peliä muutetaan ja tutkitaan kesken pelin:
- kokeilut tehdään kehitysmodeina,
- tila luetaan ja kirjoitetaan inspectorilla,
- koodi pysäytetään breakpointeilla.

Lopuksi toimiva kokeilu siirretään peliin tarkistettuna.

Tarkemmat säännöt ovat speksissä:
- kehitysmodit, vienti ja yhdistäminen: [D 3.6](dataformaatit/03-tiedostot-ja-include.md#36-kehitysmodit-dev)
- uudelleenlataus, inspector ja breakpointit: [M 5.7](moottori/05-aloitus-tallennus-ja-checkpointit.md#57-uudelleenlataus-kehitystilassa).

## Kehitystila

Kaikki tämän ohjeen työkalut toimivat kehitystilassa, joka käynnistetään valitsimella `--dev`:

```
rlml play PELI --dev
```

Kehitystila on vain pääteversiossa. Julkaistu peli ja web-versio eivät lue `dev/`-hakemistoa eivätkä avaa työkaluja.

Kehitystilan näppäimet:
- `f5` lataa pelin tiedostot uudelleen.
- `f6` avaa modilistan.
- `f12` avaa inspectorin. Peli seisoo, kun inspector on auki.

## Uudelleenlataus

`f5` lukee ja tarkistaa pelin tiedostot uudelleen ja jatkaa niillä samasta tilanteesta. Muokkaa tiedostoa editorissa, paina `f5`, ja muutos on käytössä seuraavasta toiminnosta alkaen.

- **Milloin:** lataus tehdään toimintojen välissä. Kohtauksen, dialogin tai valikon aikana se odottaa niiden loppuun.
- **Virhe:** jos uusissa tiedostoissa on virheitä, peli jatkaa vanhoilla ja näyttää virheet.
- **Mitä vaihtuu heti:** käsittelijät, komennot, tekoäly, kyselyt ja tekstit.
- **Määrittelyt:** muutettu määrittely koskee vain uusia ilmentymiä. Jo luodut rotat pitävät arvonsa, ellei modi pyydä `update existing`.
- **Kartat:** muuttunut piirros yhdistetään pelattavaan karttaan.
  - Ruudut, joita peli ei ole muuttanut, saavat uuden piirroksen.
  - Pelin muuttamat ruudut, kuten avattu ovi tai kaivettu seinä, säilyvät.
  - Ristiriidasta ja kielletyssä ruudussa seisovasta hahmosta tulee varoitus.
- **Odottava komento:** jos sen runko muuttui, komento hylätään varoituksella.
- **Toisto:** syöteloki aloittaa latauksesta uuden jakson, joten peli toistuu myös latauksen yli.

## Kehitysmodit

Kehitysmodi on kokeilu, jonka voi kytkeä päälle ja pois muuttamatta pelin omia tiedostoja. Jokainen modi on oma hakemistonsa pelin kansiossa:

```
dev/
    vaikea/
        mod.rlml
        rotat.rlml
```

**Otsake** `mod.rlml` nimeää modin, ja sen täytyy olla omassa tiedostossaan.

```
mod VAIKEA
- about "Rotat ovat sitkeämpiä."
- base 3f2a9c1e5b7d4e8a
- update existing
```

- `about` on lyhyt kuvaus, joka näkyy `f6`-listassa.
- `base` on sen pelin version tiiviste, jonka päälle modi tehtiin.
  - Jos peli on muuttunut, lataus varoittaa mutta lataa modin silti.
  - Rivi on vapaaehtoinen.
- `update existing` päivittää myös jo luodut ilmentymät, mutta vain kentät, joita peli ei ole muuttanut. Haavoittuneen rotan `HP` ei siis palaudu.

**Sisältö** on tavallista RLML:ää ja RogueScriptiä. Olemassa olevaa määrittelyä muutetaan `patch`-lohkolla:

```
patch entity RAT
- HP now 9
- drop hearing
- new light radius 2
```

- Rivi ilman merkintää korvaa saman avaimen rivin, esimerkiksi `HP`.
- `drop` poistaa rivin.
- `new` lisää uuden rivin lohkon loppuun.
- Modissa voi olla myös kokonaan uusia määrittelyjä, käsittelijöitä ja funktioita.

**Päälle kytkeminen:**
- Käynnistettäessä valitsimella `--mod NIMI`, jonka voi toistaa: `rlml play PELI --dev --mod vaikea --mod hurja`.
- Kesken pelin `f6`-listasta, jossa `[x]` on päällä ja `[ ]` pois. Valinta lataa pelin uudelleen.
- Valinta ei jää voimaan seuraavaan käynnistykseen.

**Latausjärjestys:**
1. pelin omat tiedostot
2. modit kytkentäjärjestyksessä
3. viimeisenä pelaajan `overrides/`.

Jos kaksi modia muuttaa saman rivin, myöhemmin ladattu voittaa ja lataus varoittaa.

**Tallennus ja syöteloki** kirjaavat käytössä olleet modit:
- Toisilla modeilla ladattu tallennus antaa varoituksen.
- Syötelokin toisto vaatii samat modit.

## Inspector

Inspector näyttää pelin tilan polkuina ja kirjoittaa arvoja toimintojen välissä.
- `f12` avaa sen pelin päälle.
- Nuolinäppäimet vierittävät.
- `esc` sulkee.

**Polut** ovat samoja nimiä kuin RogueScriptissä:

| Polku | Näyttää |
| --- | --- |
| `game`, `game.turn`, `game.TALLY.count` | Pelin tila ja sen komponentit |
| `entities`, `entity#12`, `entity#12.HP.now` | Entiteetit, yhden entiteetin komponentit ja kentät |
| `entity#12.patched` | Mistä tiedostosta ja riviltä paikattu arvo tuli |
| `maps`, `map.HALL` | Kartat, nimiöt ja muutetut ruudut |
| `seats`, `wait`, `wait#12`, `wait#12.locals.steps` | Istuimet ja odottavat komennot nimineen |
| `scenes` | Käynnissä olevat kohtaukset |

Komponentin voi nimetä koko nimellä (`STD.HP`) tai pelkällä viimeisellä osalla (`HP`), jos se on yksikäsitteinen.

**Kirjoitus:** `:` avaa inspectorin alariville komentorivin.

```
:entity#12.HP.now = 1
:game.TALLY.count = 5
:wait#4.locals.steps = 10
```

- Arvo luetaan kentän tyypin mukaan, ja väärä tyyppi on virhe (`x` ei ole luku).
- Kirjoittaa voi vain olemassa oleviin kenttiin. Moottorin omat arvot, kuten `game.turn`, ovat vain luettavia.
- Muokkaus kirjataan syötelokiin, joten peli toistuu sellaisenaan.
- Tallennus merkitään muokatuksi.

**Tallennus modiin:** kun rivin loppuun lisää `> MODI`, entiteetin kenttä kirjoitetaan myös määrittelyn `patch`-riviksi modiin.

```
:entity#12.HP.now = 7 > omat
```

- Rivi lisätään tiedostoon `dev/omat/inspector.rlml`, ja puuttuva otsake luodaan.
- Modi kytkeytyy päälle seuraavassa uudelleenlatauksessa, jolloin jokainen uusi rotta saa arvon 7.
- Vain entiteetin kentän voi tallentaa modiin. `game`-lohkon ja odottavan komennon arvot ovat pelin omaa tilaa eivätkä määrittelyä.

**Komentoriviltä** sama onnistuu ilman päätettä:

```
rlml inspect PELI --keys "l l j" entity#12 game.turn
rlml inspect PELI --set entity#0.HP.now=1 entity#0.HP
```

1. `--keys` pelaa näppäimet ensin.
2. `--set` kirjoittaa arvon niiden jälkeen.
3. Lopuksi polut tulostetaan.

Ilman polkuja komento listaa juuret.

## Modin vienti ja yhdistäminen peliin

Toimiva kokeilu tehdään osaksi peliä jommallakummalla kahdesta komennosta. Kummankin tulos on uusi peliversio, joka ilman `--mod`-valitsinta on sama kuin ennen modin kanssa ladattuna.

```
dev/NIMI/ ──┬── mod export ── tiedostot mods/NIMI/, include-rivi ──┬── tarkistus ── dev/.arkisto/NIMI/
            └── mod merge ─── rivit määrittelyihin, loput mods/NIMI/ ┘
```

Ennen kirjoitusta komento lataa pelin muistissa kahdesti: modin kanssa ja muutettuna ilman modia. Sitten se vertaa ratkaistuja määrittelyjä. Jos ne eroavat, mitään ei kirjoiteta ja erot luetellaan.

**`rlml mod export PELI NIMI`** pitää muutokset omana tiedostonaan:
- Modin tiedostot kopioidaan kansioon `mods/NIMI/` sellaisinaan ilman `mod`-otsaketta.
- Pelin päätiedoston loppuun tulee kommentti `> Modi NIMI: kuvaus` ja rivi `include mods/NIMI/**`.
- `--dry-run` näyttää, mitä kirjoitettaisiin.

**`rlml mod merge PELI NIMI`** kirjoittaa muutokset alkuperäisiin määrittelyihin:
- Korvattava rivi muuttuu siellä, missä se on kirjoitettu, ja vain sen muuttuva osa vaihtuu. Esimerkiksi `- HP now 3 **heikko**` muuttuu muotoon `- HP now 9 **heikko**`, joten kommentit ja muotoilu säilyvät.
- `drop` poistaa rivin, ja uusi rivi kirjoitetaan lohkon viimeisen rivin perään.
- Mitä ei voi yhdistää näin, viedään kuten `mod export` vie, ja komento luettelee ne. Näitä ovat:
  - uudet määrittelyt
  - RogueScript
  - kielitiedostot
  - alalohkot
  - vakiokirjaston rivien paikkaukset.
- Komento näyttää muutokset diffinä ja kysyy `write these changes? [y/N]`.
  - `--yes` kirjoittaa kysymättä.
  - `--dry-run` vain näyttää.

Kumpikin komento siirtää lopuksi modin kansioon `dev/.arkisto/NIMI/`, jota lataus ja `f6`-lista eivät lue. Kumpikaan ei tee committia, joten muutokset voi katsoa versionhallinnasta ennen commitia.

## Breakpointit ja askellus

Breakpoint pysäyttää pelin ennen RogueScript-rivin lausetta ja näyttää, missä ollaan ja mitä nimissä on. Se asetetaan muodossa `TIEDOSTO:RIVI`:

```
rlml play PELI --dev --break logic/tarina.rlml:217 --break move.rlml:59
```

- Tiedoston voi nimetä polun lopulla (`move.rlml`), jos se on yksikäsitteinen. Myös vakiokirjaston tiedostot käyvät.
- Rivi, jolla ei ole lausetta (tyhjä rivi tai otsake), siirtyy saman määrittelyn seuraavaan lauseeseen.
- Kesken pelin inspectorin `:`-rivi ottaa komennot:
  - `break TIEDOSTO:RIVI`
  - `unbreak TIEDOSTO:RIVI`
  - `breaks`, joka listaa breakpointit.

**Pysähdysnäkymä** näyttää:
- lähdekoodin pysähdysrivin ympäriltä, ja pysähdysrivi on merkitty `→`-merkillä
- kutsuketjun (`STD.MOVE › STEP`)
- rungon nimet arvoineen
- `self`-, `actor`- ja `holder`-entiteetit.

| Näppäin | Mitä tekee |
| --- | --- |
| `c`, `f8` tai `esc` | Jatkaa seuraavaan breakpointiin |
| `s` tai `f11` | Askeltaa seuraavaan lauseeseen, myös kutsun sisään (into) |
| `n` tai `f10` | Askeltaa saman rungon seuraavaan lauseeseen kutsujen yli (over) |
| `o` | Jatkaa kutsujan seuraavaan lauseeseen (out) |
| `:` | Lukee polun: `locals.NIMI` tai inspectorin polku, esimerkiksi `entity#4.HP` |

Pysähdyksessä tilaa voi lukea mutta ei kirjoittaa, koska toiminto on kesken. Pysähdys ei muuta tilaa eikä syötelokia, joten peli etenee samoin kuin ilman breakpointteja.

**Ilman päätettä** `rlml run` tulostaa jokaisen pysähdyksen rivinä näppäimen ja ruudun väliin ja jatkaa:

```
rlml run PELI --keys "l l" --break move.rlml:59

> l  STD.MOVE.EAST
break <engine>/lib/std/move.rlml:59 in STD.MOVE (depth 1): dir = east, self = entity#4 HERO, actor = entity#4 HERO
```

`rlml run` kiinnittää breakpointit vasta ensimmäisestä näppäimestä. Siksi `begin`-käsittelijät ajetaan ennen sitä pysähtymättä.

## Pikaopas

| Tehtävä | Komento tai näppäin |
| --- | --- |
| Kehitystila | `rlml play PELI --dev` |
| Uudelleenlataus | `f5` |
| Modit päälle käynnistettäessä | `--mod NIMI`, toistettava |
| Modit päälle ja pois kesken pelin | `f6` |
| Inspector | `f12`; sulku `esc` |
| Arvon kirjoitus | `:POLKU = ARVO` |
| Arvo myös modiin | `:POLKU = ARVO > MODI` |
| Tila komentoriviltä | `rlml inspect PELI [--keys "…"] [--set POLKU=ARVO] [POLKU …]` |
| Breakpoint | `--break TIEDOSTO:RIVI` tai `:break TIEDOSTO:RIVI` |
| Breakpointin poisto ja lista | `:unbreak TIEDOSTO:RIVI`, `:breaks` |
| Pysähdyksestä eteenpäin | `c` jatka, `s` into, `n` over, `o` out, `:` lue |
| Pysähdykset ilman päätettä | `rlml run PELI --keys "…" --break TIEDOSTO:RIVI` |
| Modi peliin include-tiedostona | `rlml mod export PELI NIMI [--dry-run]` |
| Modi peliin pistekorjauksina | `rlml mod merge PELI NIMI [--dry-run] [--yes]` |
