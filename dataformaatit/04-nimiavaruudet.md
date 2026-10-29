[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 4. Nimiavaruudet

## 4.1 Periaate

Nimiavaruus on sama asia kuin pistenotaation ryhmä (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). `LOOT.RARE` on nimiavaruuden `LOOT` jäsen riippumatta siitä, onko tiedostossa `namespace`-riviä.

Nimiavaruudet tuovat kieleen neljä asiaa:

- **lyhennyksen:** tiedoston nimet saavat etuliitteen kirjoittamatta sitä joka kerta
- **nimenhaun:** lyhyt nimi etsitään ensin omasta nimiavaruudesta, sitten ulompaa ja lopuksi vakiokirjastosta
- **juuriviittauksen:** `.NIMI` viittaa aina nimiavaruuden ulkopuoliseen nimeen
- **korvausmerkinnän:** `override` kertoo, että määrittely korvaa tarkoituksella toisen tiedoston määrittelyn.

Nimiavaruus ei ole perintää, kuten ryhmäkään. Se ei tuo näkyvyysrajoja eikä yksityisiä nimiä: jokaiseen nimeen voi viitata täydellä nimellä mistä tahansa.

## 4.2 Määrittely

`namespace`-rivi kirjoitetaan tiedoston alkuun ennen ensimmäistä määrittelyä. Kommentit ja markdown-otsikot saavat edeltää sitä.

```
namespace UNDEAD

entity SKELETON
- HP now 6
- render GLYPH.SKELETON

entity SKELETON.ARCHER extends SKELETON
- MELEE 1d6

table LOOT 1d10
- 1..7 BONE
- 8..10 none
```

- Määrittelyjen nimet ovat suhteellisia. Yllä määritellään `UNDEAD.SKELETON`, `UNDEAD.SKELETON.ARCHER` ja `UNDEAD.LOOT`. Ryhmitelty nimi jatkaa etuliitettä.
- Nimiavaruus voi olla monitasoinen: `namespace UNDEAD.CRYPT`.
- Rivi koskee vain omaa tiedostoaan, myös sen RogueScript-koodilohkoja. Se ei periydy tiedostoihin, jotka tiedosto ottaa mukaan `include`-rivillä, eikä se vaikuta tiedostoon, joka ottaa tämän tiedoston mukaan.
- Tiedostossa on enintään yksi `namespace`-rivi. Toinen rivi tai rivi määrittelyjen jälkeen on latausvirhe.
- Nimiavaruuden nimi noudattaa nimen muotoa (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). Nimiavaruutta ei määritellä erikseen, joten sama nimiavaruus voi jakautua moneen tiedostoon.
- Tiedosto ilman `namespace`-riviä määrittelee nimensä juureen.
- Kielitiedosto (luku [10](10-tekstit.md)) voi aloittaa `namespace`-rivillä, ja sen tekstiavaimet ovat silloin suhteellisia.

## 4.3 Nimiavaruuteen kuuluvat nimet

Nimiavaruus koskee yleisiä nimiä: entiteetit, komponentit, vakiot, pakat, taulut, kartat, prefabit, värit, tilet, tilejoukot, valikot, asettelun alueet, paneelit, äänet, äänilajit ja tekstiavaimet sekä RogueScriptin yleiset nimet.

**Kenttäpolut.** Komponentti voi olla nimiavaruudessa (`UNDEAD.HP`), vaikka piste on myös kentän haun merkki. Polku `target.UNDEAD.HP.now` ratkaistaan latauksen jälkeen: polun isot osat sovitetaan pisimpään olemassa olevaan komponentin nimeen, ja loput osat ovat kenttiä. Nimellä indeksoitava kenttä, joka näyttää samalta kuin ryhmitelty komponentti (kenttä `RESIST.FIRE` ja komponentti `FIRE` nimiavaruudessa `RESIST`), on latausvirhe, joka kertoo molemmat paikat. Datan rivillä (`- UNDEAD.HP now 6`) polkua ei ole, joten siellä nimi on aina komponentti.

Nimiavaruuden ulkopuolelle jäävät:

- **Komponenttien kentät.** `RESIST.FIRE` on kenttä eikä nimiavaruuden `RESIST` jäsen.
- **Maastokerrokset.** Kerroksen nimi (`GROUND`, `COVER`) on koko pelin yhteinen rakenne kuten komponentin kenttä, joten kahdessa kirjastossa ei voi olla eri kerroksia samalla nimellä (luku [7.2](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#72-maastokerrokset-terrain)).
- **Kartan nimiöt**, jotka ovat jo valmiiksi oman karttansa sisäisiä.
- **Sanasto ja moottorin nimet** (pienet kirjaimet) sekä pelin nimi (`game CRYPT`, `profile CRYPT`).

Nimenhaku toimii samoin riippumatta siitä, missä paikassa nimi on: nimiavaruudessa `UNDEAD` taulun rivi `BONE` etsii nimeä `UNDEAD.BONE`, sitten `BONE` ja lopuksi `STD.BONE`.

## 4.4 Nimenhaku

Nimet ratkaistaan vasta, kun kaikki tiedostot on ladattu, joten määrittelyjen järjestys tiedostoissa ei vaikuta hakuun. Kun tiedoston nimiavaruus on `A.B` ja viittaus on `N`, haku kokeilee järjestyksessä nimiä

1. `A.B.N`
2. `A.N`
3. `N`
4. `STD.N`

ja ensimmäinen olemassa oleva voittaa. Tiedostossa ilman `namespace`-riviä haku kokeilee nimiä `N` ja `STD.N`. Sama sääntö koskee ryhmiteltyä viittausta: nimiavaruudessa `UNDEAD` viittaus `SFX.HIT` kokeilee nimiä `UNDEAD.SFX.HIT`, `SFX.HIT` ja `STD.SFX.HIT`.

- **Juuriviittaus.** Pisteellä alkava nimi (`.LOOT`) ohittaa haun ja viittaa suoraan juureen. Sitä tarvitaan, kun oma nimiavaruus sisältää saman lyhyen nimen.
- Täysi viittaus vakiokirjastoon kirjoitetaan `STD.NIMI`.
- **Peittäminen on sallittua.** Sisempi nimi peittää ulomman ilman varoitusta. Poikkeus on vakiokirjasto (alla).
- **Puuttuva nimi** on latausvirhe, joka kertoo kaikki kokeillut nimet.
- **Lajin tarkistus tehdään haun jälkeen.** Haku ei ohita väärän lajista nimeä etsiäkseen oikean lajista ulompaa. Jos `LOOT` löytyy nimellä `UNDEAD.LOOT` mutta on tilejoukko eikä pakka tai taulu, nosto siitä on latausvirhe. Tämä pitää haun ennustettavana, ja siksi viittauksessa ei tarvita lajin etuliitettä.
- **Tilen indeksi** ei ole osa nimeä haussa: `WATER.3` etsii tileä `WATER` ja lisää indeksin (luku [7.1](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#71-tilen-nimi-muunnelma-ja-indeksi)).
- Virheilmoitukset ja korvauslistaus näyttävät aina täydet nimet.

**Vakiokirjaston nimen peittäminen.** Kun juuren nimi on sama kuin vakiokirjaston nimi, lataus antaa varoituksen. Peittäminen jakaisi nimen kahtia huomaamatta: vakiokirjaston tiedostot viittaavat omaan nimeensä, mutta pelin lyhyt viittaus löytäisi pelin oman. Vakiokirjaston määrittely muutetaan korvaamalla sen nimi (`override tileset STD.FLOODABLE`) tai paikkaamalla se (`patch tileset STD.FLOODABLE`, luku [4.8](#48-lohkojen-yhdistäminen-patch-new-ja-drop)), ja oma erillinen nimi kannattaa valita toisin.

## 4.5 Korvaaminen ja override

Korvaaminen koskee täysiä nimiä. `UNDEAD.LOOT`, juuren `LOOT` ja `STD.LOOT` ovat eri nimiä, eivätkä ne korvaa toisiaan.

Säännöt:

- Myöhempi saman lajin määrittely samalla täydellä nimellä korvaa aiemman. Datassa ja RogueScriptissä kirjoitetut komponentit ja kartat ovat sama laji (luku [1.2](01-rajaus-ja-periaatteet.md#12-rajapinnat-pelilogiikkaan)).
- Eri lajien törmäys samalla täydellä nimellä on latausvirhe.
- Korvaus toisesta tiedostosta ilman merkintää antaa latausvaroituksen.
- Tarkoituksellinen korvaus merkitään sanalla `override`, joka vaientaa varoituksen.
- `override` ilman korvattavaa on latausvirhe. Tarkistus tehdään vasta koko latauksen jälkeen, joten korvaavan tiedoston ei tarvitse latautua alkuperäisen jälkeen. Jos korvattavia on useita, latausjärjestys ratkaisee, ja `override`-merkitty määrittely voittaa merkitsemättömän järjestyksestä riippumatta. Kaksi `override`-määrittelyä samalle nimelle on latausvirhe.
- Saman tiedoston sisäinen korvaus toimii ilman varoitusta.

Merkintä kirjoitetaan määrittelyn otsikon eteen tai luettelorivin alkuun:

```
override table STD.LOOT.COMMON 1d10
- 1..6 COIN 2d6
- 7..10 none

constants
- override BURN_DAMAGE 1d6
```

- Nimiavaruudessa korvauksen nimi on suhteellinen kuten muutkin määrittelyt: nimiavaruudessa `UNDEAD` rivi `override table LOOT` korvaa nimen `UNDEAD.LOOT`. Juuren nimi korvataan merkinnällä `.LOOT`.
- `overrides/`-kansio (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)) ei korvaa vaan paikkaa: sen lohkot käsitellään kuin niissä olisi `patch` (luku [4.8](#48-lohkojen-yhdistäminen-patch-new-ja-drop)). Koko lohkon korvaus merkitään siellä sanalla `override` kuten muuallakin.
- Tiedostotason korvaus hakupolun kautta (pelin oma `lib/hp.rlml` korvaa vakiokirjaston tiedoston, luku [3](03-tiedostot-ja-include.md)) ei tarvitse merkintää, koska vakiokirjaston tiedostoa ei silloin ladata lainkaan.

Merkintä paljastaa kaksi yleistä virhettä: kirjoitusvirheen, joka loisi hiljaa uuden nimen (`override table LOOT.COMON`), ja kirjaston muutoksen, joka poistaa nimen, jota peli korvaa.

## 4.6 Jokerit

Jokerit toimivat kuten luvussa [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto), ja `LOOT.*` vastaa nimiavaruuden suoria jäseniä ja `LOOT.**` kaikkia tasoja. Jokerin etuosa ratkaistaan samalla haulla kuin muutkin nimet: nimiavaruudessa `UNDEAD` jokeri `LOOT.*` tarkoittaa ensimmäistä olemassa olevaa ryhmistä `UNDEAD.LOOT.*`, `LOOT.*` ja `STD.LOOT.*`. `.LOOT.*` viittaa juuren ryhmään, eikä `**` juuressa ulotu vakiokirjastoon.

## 4.7 Kirjastot

- Moottorin vakiokirjasto on nimiavaruudessa `STD`. Jokainen vakiokirjaston tiedosto alkaa rivillä `namespace STD`. Haun viimeinen vaihe (luku [4.4](#44-nimenhaku)) tekee sen nimet käytettäviksi lyhyesti, joten tavallinen peli ei kirjoita etuliitettä.
- Pelin oma vakiokirjastotiedosto (esimerkiksi `lib/hp.rlml`, joka korvaa vakiokirjaston version hakupolussa) kirjoitetaan myös nimiavaruuteen `STD`, jotta muut vakiokirjaston tiedostot löytävät sen nimet.
- Muut kirjastot kannattaa kirjoittaa omaan nimiavaruuteensa, jotta ne eivät törmää peliin, vakiokirjastoon tai toisiinsa. Kirjaston nimeksi kannattaa valita sama sana kuin sen kansio (`lib/undead/` → `namespace UNDEAD`), mutta moottori ei vaadi sitä.
- Pelin omat tiedostot voivat jättää `namespace`-rivin pois. Juuressa olevat nimet ovat pelin omia.

## 4.8 Lohkojen yhdistäminen: patch, new ja drop

Sama lohko voi saada sisältöä monesta tiedostosta: kirjasto määrittelee, peli muuttaa ja pelaaja säätää. Yhdistäminen etenee rivin avaimen mukaan.

- **Rivin avain.** Jokaisella lohkolla on avain, joka tunnistaa rivin. Saman avaimen myöhempi rivi korvaa aiemman koko rivin, ja uusi avain lisää rivin.
- **Nimettömät osiot** yhdistyvät aina. Rivin korvaus toisesta tiedostosta antaa latausvaroituksen, ellei rivin alussa ole `override` (luku [4.5](#45-korvaaminen-ja-override)).
- **Nimetty lohko** korvautuu oletuksena kokonaan (luku [4.5](#45-korvaaminen-ja-override)). Rivien muuttaminen merkitään otsikon edessä sanalla `patch`:

| Lohko | Rivin avain | Yhdistyminen |
| --- | --- | --- |
| `constants`, `colors`, `settings` | nimi | nimetön osio: rivit yhdistyvät aina |
| `autosave` | `slots` tai kohta | nimetön osio: rivit yhdistyvät aina |
| `tiles`, `tiles.glyphs` | tile tai sarjarivin väli | nimetön osio |
| `messages`, `panels`, `logs`, `presentation` | avain, alue, luokka tai asetus | nimetön osio |
| `layers` | kerroksen nimi | nimetön osio, uusi kerros loppuun |
| `bus` | väylän numero | rivit yhdistyvät aina |
| `game`, `profile` | kenttä tai komponentti | nimetty päätiedostossa, kirjastossa nimetön osio |
| `entity` | komponentti | `patch`: rivit |
| `keys`, `pad` | toiminto | `patch`: rivit |
| `menu` | kohdan avain | `patch`: rivit, uusi kohta loppuun |
| `log` | luokka | `patch`: rivit |
| `tileset` | jäsen | `patch`: jäsenet lisätään aina |
| `deck` | ei avainta | `patch`: kortit lisätään loppuun |
| `table`, `terrain`, `map`, `prefab`, `layout`, `panel` | – | vain koko lohko |

```
patch keys GAME
- MOVE.NORTH k up w num8 i
- new LOOK l
- drop help

patch deck LOOT.ARTIFACT
- RING.TELEPORT

patch entity GOBLIN
- HP now 10
- drop DROPS
```

- Olemassa oleva avain korvautuu ilman merkintää, koska `patch` kertoo jo tarkoituksen.
- Uusi avain merkitään rivin alussa sanalla `new`. Rivi ilman `new`-sanaa, jonka avainta ei ole, on latausvirhe, ja virheilmoitus kertoo lohkon olemassa olevat avaimet. `new` olemassa olevalle avaimelle on latausvirhe.
- `drop AVAIN` poistaa rivin. `drop` puuttuvalle avaimelle on latausvirhe.
- **Avaimettomat lohkot.** `deck`-lohkon kortit ja `tileset`-lohkon jäsenet lisätään aina, eikä `new`-sanaa käytetä. `drop` poistaa pakasta kaikki kortit, joiden kohde on annettu nimi, ja tilejoukosta annetun jäsenen.
- **Perintä.** Sama `drop` toimii entiteetin perinnässä: `entity BAT.TAME extends BAT` ja rivi `- drop MELEE` poistaa emolta perityn komponentin (luku [5.2](05-komponentit-ja-entiteetit.md#52-entiteetit)).
- **Järjestys.** Paikat sovelletaan koko latauksen jälkeen latausjärjestyksessä. `override` ratkaistaan ensin, joten paikka koskee voittanutta määrittelyä. Pelaajan `overrides/`-tiedostot sovelletaan viimeisinä.
- **Virheet.** `patch` ilman kohdelohkoa on latausvirhe, kuten `override` ilman korvattavaa. `patch` taululle, `terrain`-osiolle tai piirrokselle on latausvirhe, koska taulun välien kattavuus ei säily rivien lisäyksessä ja piirros on yksi kokonaisuus.
- **`new` ja `patch` vain paikassa.** `new` on sallittu `patch`-lohkossa ja `overrides/`-tiedostoissa. Muualla uusi nimi ja uusi rivi ovat tavallisia määrittelyjä.
- **RogueScript.** Paikka koskee datan lohkoja. Komponentin kenttämäärittely ja RogueScriptin määrittelyt korvautuvat aina kokonaan (RogueScript, luku [3.2](../roguescript/03-komponentit.md#32-datassa-ja-skriptissä)).

Pelaajan `overrides/`-tiedostoissa jokainen lohko on paikka, myös nimettömät osiot: olemassa oleva rivi korvautuu, ja uusi rivi tai lohko merkitään sanalla `new` (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)). Poikkeus on `profile`, jota `overrides/` ei voi paikata (luku [3.5](03-tiedostot-ja-include.md#35-profiili-profile)).

---

← [3. Tiedostot ja include](03-tiedostot-ja-include.md) · [Sisällys](README.md) · [5. Komponentit ja entiteetit](05-komponentit-ja-entiteetit.md) →
