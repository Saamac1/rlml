[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 2. Yleinen syntaksi

## 2.1 Nimet ja sanasto

Kirjainkoko kertoo lukijalle omistajan poikkeuksetta. Väärä kirjoitus on latausvirhe.

- **NIMET isoilla kirjaimilla.** Kirjoittajan ja kirjastojen määrittelemät nimet: komponentit, entiteetit, pakat, taulut, kartat, prefabit, vakiot, värit, tilet, tilejoukot, maastokerrokset, valikot, asettelun alueet, paneelit, äänet, äänilajit ja tekstiavaimet (`STONE`, `GOBLIN`, `LOOT.COMMON`, `FLOODABLE`, `COVER`).
- **Sanasto pienillä kirjaimilla.** Avainsanat (`tile`, `size`, `legend`, `extends`, `patch`), moottorin komponentit (`render`, `sight`, luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)), komponenttien kentät (`now`, `max`), `game`-lohkon moottorin kentät (`start`, `at`, luku [3.3](03-tiedostot-ja-include.md#33-aloitus)), tilen ominaisuudet (`blocks`, luku [7.5](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)), suunnat (`north`) ja suhteelliset suunnat (`front`), tilen muunnelmat (`DOOR.open`), moottorin toiminnot (`save`, `quit`), näppäinten ja hiiren nimet (`enter`, `esc`, `click`, luku [11.3](11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)) ja hex-värit (`5a5a6e`).
- **Nimellä indeksoitava kenttä on nimi.** Kun kentän nimi on itse arvo, johon viitataan nimellä (vahinkotyyppi `RESIST.FIRE`, varustepaikka `SLOTS.RING`), se kirjoitetaan isoilla.

Moottori lukee vain pienillä kirjoitettuja nimiä, eikä se tunne yhtään isolla kirjoitettua nimeä. Isolla kirjoitettu nimi on siis aina pelin tai kirjaston, myös silloin kun se on vakiokirjastossa. Säännön poikkeukset ovat avoin kysymys (luku [13](13-avoimet-kysymykset.md)). RogueScriptissä pienillä kirjoitetaan lisäksi paikalliset nimet (RogueScript, luku [2.4](../roguescript/02-yleinen-syntaksi.md#24-varatut-sanat-ja-paikalliset-nimet)).

Nimissä sallitaan vain ASCII-merkit `A–Z`, `0–9` ja `_` sekä piste ryhmittelyyn. Tiedostot ovat UTF-8-koodattuja, rivinvaihto on LF, ja tekstit saavat sisältää ääkkösiä, mutta nimet eivät.

- **Nimen muoto.** Nimen jokainen osa (pisteiden väli) alkaa kirjaimella `A–Z` ja jatkuu merkeillä `A–Z`, `0–9` ja `_`. Moniosaisessa nimessä sanat erotetaan alaviivalla (`RING_MAIL`, `LOW_POP`). Alaviiva ei ole nimen alussa tai lopussa, eikä kahta alaviivaa ole peräkkäin. Numerolla alkava nimi (`3D_FIRE`) on virhe, koska se sekoittuisi noppaan (`3d6`). Tilen muunnelman osa on sanastoa ja kirjoitetaan pienillä (`DOOR.open`, `STAIR.down`). Tilen nimen viimeinen osa voi olla lukuindeksi (luku [7.1](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#71-tilen-nimi-muunnelma-ja-indeksi)).
- **Täydet nimet.** Jokaisella yleisellä nimellä on yksi täysi nimi (`LOOT.RARE`, `STD.FLOOR`), ja täydet nimet ovat samassa nimiavaruudessa. Saman lajin uusi määrittely samalla täydellä nimellä korvaa aiemman (luku [4.5](04-nimiavaruudet.md#45-korvaaminen-ja-override)), mutta eri lajien törmäys (esimerkiksi tile ja pakka nimeltä `GOLD`) on latausvirhe, joka kertoo molemmat paikat. Koska täysi nimi on yksikäsitteinen, viittauksessa ei kirjoiteta lajia (`- LOOT.RARE`), ja laji tarkistetaan nimenhaun jälkeen (luku [4.4](04-nimiavaruudet.md#44-nimenhaku)).
- **Paikalliset nimet.** Kartan nimiöt (luku [8.5](08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)) ovat voimassa vain oman karttansa sisällä. Sama nimiö voi esiintyä monella kartalla, mutta se ei saa olla sama kuin yksikään yleinen nimi, jonka nimenhaku löytäisi samalla lyhyellä nimellä. Siksi nimiö ja yleinen nimi eivät sekoitu silloinkaan, kun sama paikka hyväksyy molemmat (paikkahaku, luku [8.8](08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)).
- **Nimeämissuositus.** Tilet kertovat ulkonäöstä. Olentojen ja esineiden tilet kannattaa koota ryhmään (`GLYPH.GOBLIN`) ja pakat ja taulut ryhmään (`LOOT.*`, `SPAWN.*`), jotta ne eivät törmää entiteettien nimiin ja jotta lajin näkee nimestä.
- **Ryhmät.** Pistenotaatiota saa käyttää vapaasti samantyylisten nimien ryhmittelyyn (`LOOT.COMMON`, `LOOT.RARE`, `SFX.HIT`), eikä ryhmää määritellä erikseen. Ryhmät voivat olla monitasoisia (`LOOT.RARE.DEEP`), ja nimi voi olla samalla ryhmän etuosa (`GOBLIN` ja `GOBLIN.ARCHER`). Ryhmä on samalla nimiavaruus (luku [4](04-nimiavaruudet.md)). Ryhmä ei ole perintää: perintä kirjoitetaan aina sanalla `extends` (luku [5.2](05-komponentit-ja-entiteetit.md#52-entiteetit)). Myös komponentit ja vakiot voivat olla ryhmissä. Kenttäpolussa (`target.UNDEAD.HP.now`) isot osat sovitetaan pisimpään olemassa olevaan komponentin nimeen, ja loput osat ovat kenttiä (luku [4.3](04-nimiavaruudet.md#43-nimiavaruuteen-kuuluvat-nimet)).
- **Jokeri.** Jokeri osoittaa koko ryhmän: `LOOT.*` vastaa yhden tason jäseniä ja `LOOT.**` kaikkia tasoja, kuten `include`-riveillä. Jokeri vastaa ryhmän kaikkia täysiä nimiä lajista riippumatta, myös tekstiavaimia. Jos jäsenet ovat eri lajeja eikä käyttöpaikka rajaa lajia, jokeri on latausvirhe, joka luettelee lajit (RogueScript, luku [2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)). Tekstiavainten jokeri vastaa kaikkien ladattujen kielitiedostojen avainten yhdistettä. Jokeri, joka ei vastaa mitään, antaa latausvaroituksen. Jokeri ei kata tilen lukuindeksejä, vaan ne valitaan välillä (`GLYPH.NKI.1..20`, luku [7.1](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#71-tilen-nimi-muunnelma-ja-indeksi)). Ryhmät kannattaa siksi pitää yhden lajin ryhminä: kuvaukset `LORE.*` eivät jaa ryhmää entiteetin `LORE` tekstiavainten kanssa.

## 2.2 Rivit, lohkot ja luettelot

Tämän luvun säännöt koskevat datarivejä. RogueScript-koodilohkot ja `.rogue`-tiedostot noudattavat RogueScriptin sääntöjä, joissa sisennys merkitsee lohkoja (luku [1.2](01-rajaus-ja-periaatteet.md#12-rajapinnat-pelilogiikkaan)).

- Rivillä on yksi määrittely.
- **Otsikkorivi** aloittaa lohkon ja kokoaa sen ominaisuudet: `menu PAUSE size 24x*`.
- **Luettelokohta** alkaa merkillä `-` ja välilyönnillä. Sisennys on ulkoasua eikä merkitse mitään.
- **Lohko päättyy** tyhjään riviin.
- **Alilohko** alkaa kaksoispisteeseen päättyvällä otsikolla (`legend:`, `vars:`). Se jatkaa edellistä määrittelyä tyhjän rivin jälkeenkin, kunnes tulee uusi otsikko. Itsenäisten osioiden otsikoissa (`game`, `profile`, `constants`, `settings`, `autosave`, `terrain`, `tiles`, `tiles.glyphs`, `colors`, `language`) ei ole kaksoispistettä.
- **Tiedostotason rivi** `namespace` (luku [4.2](04-nimiavaruudet.md#42-määrittely)) kirjoitetaan tiedoston alkuun ennen ensimmäistä määrittelyä, ja se koskee koko tiedostoa.
- **Piirretyt lohkot** (kartta, prefab, asettelu ja paneeli) kirjoitetaan tiedostossa markdownin koodilohkoon, jotta tasaus säilyy eikä `#`-rivejä tulkita otsikoiksi. Piirros kuuluu edeltävään otsikkoon tai alilohkoon tyhjän rivin yli. Tämän dokumentin esimerkeissä koodilohkoa ei näytetä erikseen.
- **Tiedostopolut** kirjoitetaan lainausmerkkeihin (`"tiles16.png"`). Poikkeus on `include`, jonka polku kirjoitetaan ilman lainausmerkkejä (luku [3](03-tiedostot-ja-include.md)).
- **Arvoparit:** kenttien, muuttujien ja ylikirjoitusten arvot annetaan kaikkialla pareina `NIMI arvo` ilman avainsanaa: entiteetin rivillä (`- HP now 8`), pakan kortissa ja taulun rivissä (`RING_MAIL ENCHANT -1..3`), legendin `with`-osassa (`with RING_MAIL ENCHANT 2`) ja kartan muuttujissa (`- DANGER 2`).
- **Kerrosparit:** legendin rivillä ensimmäinen tile menee oletuskerrokseen, ja muut maastokerrokset annetaan pareina `KERROS TILE` (`- ~ FLOOR.moss COVER WATER.2`, luku [8.3](08-kartat-ja-prefabit.md#83-kerrokset-legendissä)).
- **Rivin etumerkinnät:** `override`, `new` ja `drop` kirjoitetaan rivin alkuun `-`-merkin jälkeen, ja `override`, `new` ja `patch` lohkon otsikon eteen (luvut [4.5](04-nimiavaruudet.md#45-korvaaminen-ja-override) ja [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).

```
menu PAUSE size 24x*
- RESUME
- SAVE
- OPTIONS open OPTIONS
- QUIT
```

Kielitiedosto (luku [10](10-tekstit.md)) on poikkeus: otsikkorivin `language KIELI` jälkeen se on pelkkä `AVAIN teksti` -lista ilman `-`-merkkejä.

## 2.3 Pistenotaatio

Piste ryhmittelee tai tarkentaa:

- ryhmitelty nimi: `LOOT.RARE`, `GLYPH.GOBLIN` (luku [2.1](#21-nimet-ja-sanasto))
- tilen muunnelma ja indeksi: `DOOR.open`, `WATER.3` (luku [7.1](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#71-tilen-nimi-muunnelma-ja-indeksi))
- kenttä tai mitta: `HP.now`, `size.width 20`
- juuriviittaus: `.LOOT` viittaa nimiavaruuden ulkopuoliseen nimeen (luku [4.4](04-nimiavaruudet.md#44-nimenhaku))

## 2.4 Arvot

- **Luku:** kokonaisluku (`8`, `-3`).
- **Koko:** `size 20x10`, tai yksi mitta `size.width 20` tai `size.height 3`. Luku on kiinteä solumäärä, `25%` venyy ikkunan mukana, ja `*` tai puuttuva mitta täyttää jäljelle jäävän tilan.
- **Noppa:** `3d6+2`, väli `2..5` ja vakioarvo `4`. Avoin yläraja kirjoitetaan `15..`.
- **Prosentti:** `50%` tai `-25%`.
- **Aika:** esityksen ajat kirjoitetaan `500ms` tai `1s`, ja etunollat ovat sallittuja (`0500ms`), jotta aikasarakkeet tasautuvat. Pelin aika on datassa aina kierroksia (`duration 5`). Kierros on 120 mikroaskelta. Mikroaskelia käytetään vain RogueScriptin komennon hinnassa ja `wait`-kutsussa (moottoridokumentti, luku [6.1.1](../moottori/06-ajo.md#611-mikroaskel-ja-kierros)).
- **Suunta:** yksi kahdeksasta (`north`, `northeast`, `east`, `southeast`, `south`, `southwest`, `west`, `northwest`) tai `none`.
- **Suhteellinen suunta:** `front`, `frontright`, `right`, `backright`, `back`, `backleft`, `left`, `frontleft` tai `none`.
- **Totuusarvo:** `true` tai `false`.
- **Sijainti:** kartan ruutu muodossa `X,Y` (`5,3`) ilman välilyöntejä. X on sarake ja Y rivi, ja origo `0,0` on vasen yläkulma, kuten nimiöryhmien numeroinnissa (luku [8.5](08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)). Sijainti on aina absoluuttinen, joten sen luvut eivät ole negatiivisia. RogueScriptissä sijainti kirjoitetaan `pos(5, 3)`.
- **Siirtymä:** sijainnin muotoinen arvo `X,Y`, joka luetaan suhteessa ankkuriin (`2,-1`). Siirtymä esiintyy vain `from`-osan kanssa (luku [8.8](08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)), ja vain siinä luvut saavat olla negatiivisia. Sijainti ja siirtymä erotetaan paikasta, eivät kirjoitusasusta.
- **Nimi:** viittaus nimettyyn määrittelyyn (`LOOT.COMMON`, `GLYPH.GOBLIN`) tai `none`.
- **Lista:** listakentän arvot välilyönnein erotettuina järjestyksessä (`- BESTIARY order MONSTER.KESTREL MONSTER.EMU MONSTER.BAT`, luku [5.1](05-komponentit-ja-entiteetit.md#51-komponentit)).

Pilkku erottaa sijainnin muista arvoista: koko kirjoitetaan `x`-merkillä (`20x10`) ja väli kahdella pisteellä (`2..5`).

**Vakiot.** `constants`-osio nimeää arvoja, ja nimeä voi käyttää missä tahansa arvon paikalla:

```
constants
- BURN_DAMAGE 1d4
- SWORD_DAMAGE 1d8+2
- BUS_SFX 1
- FADE_TIME 500ms
```

- Arvo on luku, prosentti, noppa, aika, totuusarvo tai sijainti, ja se on muuttumaton ajon aikana.
- **Korvaaminen:** myöhempi määrittely samalla nimellä korvaa aiemman latauksessa, esimerkiksi pelaajan `overrides/`-tiedostossa tai rivillä `- override BURN_DAMAGE 1d6` (luvut [4.5](04-nimiavaruudet.md#45-korvaaminen-ja-override) ja [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- **Nimiavaruus:** vakiot kuuluvat nimiavaruuksiin kuten muut yleiset nimet (luku [4.3](04-nimiavaruudet.md#43-nimiavaruuteen-kuuluvat-nimet)). Kirjaston vakio on esimerkiksi `FIRE.BURN_DAMAGE`.
- Muuttuva arvo kirjoitetaan kartan muuttujaan (`vars:`, luku [8.4](08-kartat-ja-prefabit.md#84-kartan-muuttujat-vars)), `game`-lohkon komponenttiin (luku [5.5](05-komponentit-ja-entiteetit.md#55-pelikerran-tila-game)), profiiliin (luku [3.5](03-tiedostot-ja-include.md#35-profiili-profile)) tai entiteetin komponenttiin.

## 2.5 Kommentit

Kommentteja on kahta lajia, ja molemmat näkyvät markdown-katselimessa luontevasti.

- **Kokorivi:** rivi, jonka ensimmäinen merkki on `>` ja jota seuraa välilyönti tai rivin loppu, on kommentti. Peräkkäiset `>`-rivit muodostavat yhden lainauksen, ja kommentissa voi käyttää markdownia. Luettelokohdan voi kommentoida pois: `> - GOBLIN`.
- **Korostus:** `**…**`, `***…***` ja `~~…~~` rivin lopussa tai omalla rivillään. Avaavaa merkkiä seuraa heti teksti ja sulkevaa edeltää heti teksti (`**teksti**`), jotta markdown esittää kommentin korostuksena. Kommentti on rivin viimeinen asia. Samasta syystä jokerit (`include content/**`, `LOOT.**`) eivät ole kommentteja.
- **Otsikot:** markdown-otsikot (`#`-rivit) ovat vapaata tekstiä, jonka lataus ohittaa. Muu vapaa teksti on latausvirhe.
- **Rajaukset:** piirretyissä lohkoissa (kartat, prefabit, asettelun ja paneelin piirros) ei ole kommentteja, koska mikä tahansa merkki voi olla sisältöä. Kielitiedostossa toimivat vain `>`-rivit, koska teksti voi sisältää tarkoituksellista korostusta. RogueScript-koodilohkoissa kommentit kirjoitetaan `//`.
- Yksittäinen `*` ja `~~~` eivät ole kommenttimerkkejä, koska ne sekoittuisivat muihin käyttöihin (`size *x*`) ja markdownin koodilohkoon.

```
entity GOBLIN **perusvihollinen**
- HP now 8 max 8 ~~vanha arvo 6~~
> - MELEE 1d6
- MELEE 1d8
```

---

← [1. Rajaus ja periaatteet](01-rajaus-ja-periaatteet.md) · [Sisällys](README.md) · [3. Tiedostot ja include](03-tiedostot-ja-include.md) →
