[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 3. Tiedostot ja include

Pelin päätiedosto nimeää pelin `game`-lohkossa, kertoo aloituksen (luku [3.3](#33-aloitus)) ja ottaa mukaan moduulit:

```
game CRYPT
- start TOWN

include lib/std.rlml
include content/**
include maps/*
```

- **Pelin laajuiset osiot:** `game` kokoaa aloituksen ja pelikerran tilan (luvut [3.3](#33-aloitus) ja [5.5](05-komponentit-ja-entiteetit.md#55-pelikerran-tila-game)), `profile` pelikertojen yli säilyvän tilan (luku [3.5](#35-profiili-profile)), ja `terrain` julistaa ruudun maastokerrokset (luku [7.2](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#72-maastokerrokset-terrain)). Nimetty `game`-lohko, `profile`-lohko ja `terrain`-osio saavat olla pelissä vain kerran.
- Sisäkkäiset includet toimivat: kirjasto voi itse ottaa mukaan toisia kirjastoja.
- Sama tiedosto otetaan mukaan vain kerran. Mukaan otetun tiedoston sisältö latautuu `include`-rivin kohdalle syvyys ensin, ja vakiokirjasto tulee mukaan vain `include`-rivillä. Sen kannattaa olla pelin ensimmäinen include, jotta vakiokirjaston käsittelijät ovat latausjärjestyksessä ensin.
- Myöhempi määrittely samalla täydellä nimellä korvaa aiemman, ja lataus listaa korvaukset tiedoksi.
- Korvaus toisesta tiedostosta antaa latausvaroituksen, ellei määrittelyä ole merkitty sanalla `override` (luku [4.5](04-nimiavaruudet.md#45-korvaaminen-ja-override)). Nimetyn lohkon yksittäisiä rivejä muutetaan sanalla `patch` (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- `namespace`-rivi tiedoston alussa antaa tiedoston nimille etuliitteen. Se koskee vain omaa tiedostoaan eikä periydy includeissa (luku [4.2](04-nimiavaruudet.md#42-määrittely)).
- **Paketti** on tiedosto, joka vain ottaa mukaan muita (`include lib/std.rlml`).
- **Päätteet:** datatiedostot ovat markdownia, ja niiden pääte on `.rlml`, `.md` tai `.rlml.md`. Kaikki kolme ovat samanarvoisia. `.rlml.md` kertoo tiedoston lajin ja avautuu silti suoraan markdown-katselimessa. `.rogue`-tiedosto on kokonaan RogueScriptiä (RogueScript, luku [2.1](../roguescript/02-yleinen-syntaksi.md#21-tiedostot)).
- **Polku** kirjoitetaan päätteen kanssa ilman lainausmerkkejä (`include lib/hp.rlml`).
- **Haku** etenee kolmessa vaiheessa: ensin sisältävän tiedoston kansio, sitten pelin juuri ja viimeiseksi moottorin vakiokirjasto. Pelin oma `lib/hp.rlml` korvaa siis vakiokirjaston version ilman `override`-merkintää, koska vakiokirjaston tiedostoa ei silloin ladata. Korvaava tiedosto kirjoitetaan nimiavaruuteen `STD` (luku [4.7](04-nimiavaruudet.md#47-kirjastot)).
- **Jokeri:** `include maps/*` ottaa mukaan kansion `.rlml`-, `.md`-, `.rlml.md`- ja `.rogue`-tiedostot, ja `include content/**` myös alikansioiden tiedostot. Pääte rajaa jokerin (`include logic/*.rogue`). Tiedostot luetaan polun mukaisessa aakkosjärjestyksessä, ja myöhempi määrittely voittaa. Jokeri ohittaa tiedostot, joiden nimi alkaa merkillä `_` tai `.`, sekä tiedostot `README.md`, `README.rlml` ja `README.rlml.md`.
- **Vain mukaan otetut tiedostot ladataan.** Karttaan viitataan sen nimellä, joka tulee määrittelystä `map TOWN`, joten tiedoston nimellä ei ole merkitystä.

## 3.1 Kansiorakenne

Projektin sisältö on ajon aikana vain luettavaa. Kansioiden nimet ovat suositus, eikä moottori vaadi niitä, koska rakenne syntyy `include`-riveistä:

```
game.rlml
lib/          omat kirjastot (undead/, ...); vakiokirjasto std.rlml ja std/ tulevat moottorin hakupolusta
content/      components/, entities/, decks/, tables/
logic/        RogueScript-tiedostot (.rogue)
maps/         kartat
prefabs/      prefabit
legends/      jaetut legendit
style/        tyylitiedostot (ascii.rlml, tiles16.rlml, ...)
ui/           asettelu, valikot, näppäimet, viestit, paneelit
text/         kielitiedostot (fi.rlml, en.rlml)
audio/        äänten ilmoitukset ja äänitiedostot
```

```
game CRYPT
- start TOWN

include lib/std.rlml
include content/**
include logic/*
include maps/*
include prefabs/*
include style/*
include ui/*
include text/*
include audio/*
```

## 3.2 Pelaajan oma data

Pelaajan oma data kirjoitetaan käyttöjärjestelmän käyttäjädatakansioon eikä projektin sisälle:

```
<käyttäjädata>/CRYPT/
    overrides/
        keys.rlml
        settings.rlml
    saves/
    profile
```

`overrides/` sisältää RLML-tiedostoja, jotka moottori ottaa mukaan automaattisesti viimeisinä. Niiden lohkot ovat paikkoja: ne käsitellään kuin niissä olisi `patch` (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)), joten pelaaja kirjoittaa vain muuttamansa rivit eikä merkintää.

```
game
- start BOSS_LAIR
- at 5,3

keys GAME
- QUAFF d
- new LOOK l
```

- Olemassa oleva rivi korvautuu ilman merkintää. Yllä `game`-lohkon `start` ja `at` vaihtuvat, ja `QUAFF` saa uuden näppäimen, mutta pelin muut sidonnat säilyvät.
- Uusi rivi merkitään sanalla `new` ja uusi lohko otsikon edessä sanalla `new` (`new keys MY_MACROS`). Ilman merkintää tuntematon avain tai lohko on latausvirhe, joten kirjoitusvirhe (`keys GAEM`) jää kiinni.
- Koko lohkon korvaus merkitään tavalliseen tapaan sanalla `override` (`override keys GAME`).
- `settings` on moottorin osio, jonka avaimet ovat moottorin sanastoa. Ne ovat aina olemassa, joten `new`-sanaa ei tarvita.
- **Profiili ei ole paikattavissa.** `profile`-lohko `overrides/`-tiedostossa on latausvirhe. Profiilin arvot ovat moottorin omassa tiedostossa (luku [3.5](#35-profiili-profile)).

Asetukset kirjoitetaan `settings`-osioon:

```
settings
- language fi
- style ascii
- round 500ms
- idle 4s ai STD.WAIT
- external ask
- external_timeout 10s
- mouse on
```

- `round` on yhden kierroksen kesto silloin, kun pelaajalla ei ole vuoroa: pelaaja on lamaantunut, kuollut tai puuttuu. Ilman riviä kierros etenee vain toiminnolla `pass` (luku [11.3](11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad); moottoridokumentti, luku [6.1.2](../moottori/06-ajo.md#612-toimijat-ja-ajoitin)).
- `idle AIKA [ai] [KOMENTO]` päättää vuoron, jota pelaaja ei käytä. Kun aika kuluu ilman painallusta, `ai` ajaa pelaajan tekoälyn valitseman komennon ja `KOMENTO` nimetyn parametrittoman komennon; molemmat yhdessä tarkoittavat, että komento on vara, jos pelaajalla ei ole tekoälyä. Ilman riviä odotus on rajaton (moottoridokumentti, luku [6.1.2](../moottori/06-ajo.md#612-toimijat-ja-ajoitin)).
- `external ask|all|none` päättää, saako peli ajaa ulkoisia generaattoriohjelmia (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)). `ask` (oletus) kysyy luvan ohjelmakohtaisesti ensimmäisellä ajolla, `all` sallii kaikki ja `none` kieltää kaikki. `external_timeout` on yhden ajon aikaraja, oletus 10 sekuntia.
  - `style` ja `language` ovat pakollisia: puuttuva asetus on latausvirhe, ja moottori ei arvaa. Puuttuva käännös näkyy avaimena, jotta puute huomataan heti (luku [10](10-tekstit.md)).
    - `targeting cursor|list|auto` (oletus `cursor`) valitsee, miten pelaaja valitsee kohteen kohdistettavalle parametrille: kohdistimella, listasta vai automaattisesti lähimmän (moottoridokumentti, luku [8](../moottori/08-syote.md)).
- `mouse on|off` (oletus `on`) ottaa hiiren käyttöön tai pois, kun pääte tai alusta tukee sitä. Hiiri on aina lisäsyöte: kaiken voi tehdä myös näppäimistöllä tai peliohjaimella (luku [11.3](11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad); moottoridokumentti, luku [8.1](../moottori/08-syote.md#81-hiiri)).

`saves/` ei ole RLML:ää vaan moottorin omaa dataa, eikä sen sisältöä kuvata tässä dokumentissa. Sen merkitys kuitenkin kuuluu tähän:

- **Tallennus on tilannekuva.** Se sisältää koko pelin tilan: ilmentymät, niiden sijainnit, pelaajan, `game`-lohkon kentät ja komponentit, pakkojen tilan (luku [6.2](06-pakat-ja-taulut.md#62-tila-sekoitus-ja-nosto)) ja satunnaisuuden tilan. Taulut ovat tilattomia, joten niistä ei tallenneta mitään (luku [6.5](06-pakat-ja-taulut.md#65-taulut)). Ladattu peli käyttää aloitushetken `game`-arvoja, vaikka data tai `overrides/` olisi sen jälkeen muuttunut.
- **Checkpoint on tallennus.** Checkpoint on tilannekuva, jonka logiikka ottaa sovitussa kohdassa ja jonka se voi myöhemmin ladata, esimerkiksi kuoleman jälkeen. Milloin checkpoint otetaan ja ladataanko se, kuuluu pelilogiikkaan (RogueScript, luku [10](../roguescript/10-aloitus-ja-checkpointit.md)). Kuolema ei päätä peliä: ilman sääntöä maailma jatkaa kulkuaan, ja pelistä poistutaan taukovalikosta. Viimeisin checkpoint kuuluu tallennukseen, joten se toimii myös ladatussa pelissä.
- **Checkpoint 0 on laskettu.** Uuden pelin aloitustila (luku [3.3](#33-aloitus)) on pelin ensimmäinen checkpoint. Sitä ei tallenneta etukäteen, vaan moottori laskee sen datasta ja siemenestä. Kun peli on alkanut, checkpoint 0 tallentuu pelin mukana.

## 3.3 Aloitus

Uusi peli ja ladattu peli alkavat samalla tavalla: moottori lataa tilan ja jatkaa siitä. Uuden pelin tila on laskettu oletustallennus: se lasketaan datasta, siemenestä ja profiilista, eikä sitä tallenneta etukäteen. Datan muutokset näkyvät siksi heti uudessa pelissä, ja sama siemen tuottaa saman aloituksen.

Aloitus kirjoitetaan pelin päätiedoston `game`-lohkoon:

```
game CRYPT
- start START.MAP
- at SHRINES
- DIFFICULTY 2
- PERMADEATH
- KING_QUEST 0
```

- **Pienet rivit** ovat moottorin kenttiä: `start`, `at`, `seed`, `call_depth`, `loop_steps`, `seats`, `party` ja `leash`. Moottori ylläpitää lisäksi kenttiä `turn` (kierrosluku) ja `time` (kulunut aika mikroaskelina, moottoridokumentti luku [6.1.1](../moottori/06-ajo.md#611-mikroaskel-ja-kierros)). Kentät ovat logiikalle vain luettavia. Lisäksi `keys KONTEKSTI` nimeää pelin peruskontekstin (luku [11.3](11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)), `title VALIKKO` pelin oman alkuvalikon, joka avautuu moottorin pelivalikon jälkeen (moottoridokumentti, luku [8](../moottori/08-syote.md)), `layout ASETTELU` käytettävän asettelun (luku [11.1](11-kayttoliittyman-data.md#111-asettelu)), `naming KYSELY` kyselyn, joka antaa entiteetin näytettävän nimen pelin tilasta (alla), sekä `walk KOMENTO` ja `walk_stop KYSELY` klikkausmatkan kävelykomennon ja pelin oman pysähtymisehdon ja `leader KYSELY`, `rest KOMENTO`, `follow_distance N` ja `follow_ai TEKOÄLY` seuraamisen kentät (alla). Ne luetaan datasta, eivätkä ne lukitu pelikerralle. `masking N` on äänten peittoraja (luku [12.3](12-aanten-ilmoitukset.md#123-pelimaailman-äänet)).
- **Isot rivit** ovat komponentteja kuten entiteetin rivillä (luku [5.2](05-komponentit-ja-entiteetit.md#52-entiteetit)). Ne ovat pelikerran tilaa, ja logiikka voi muuttaa niitä (luku [5.5](05-komponentit-ja-entiteetit.md#55-pelikerran-tila-game)).

**Nimi pelin tilasta.** `naming KYSELY` nimeää RogueScriptin kyselyn `query Q(e: entity) -> name|text`, jota moottori kutsuu aina, kun se näyttää entiteetin nimen (RogueScript, luku [9.6](../roguescript/09-kohtaukset-ja-aikajanat.md#96-say-ja-log)). Kysely voi esimerkiksi palauttaa tunnistamattoman juoman ulkonäön nimen (`LOOK.POTION.BLUE`) tai pelaajan antaman nimen. Paluuarvo `none` käyttää tavallista nimeä (luku [10](10-tekstit.md)).

**Klikkausmatka.** `walk KOMENTO` nimeää komennon, jolla moottori kulkee hiirellä klikatun tai katselukursorilla valitun ruudun reitin askel kerrallaan (`- walk STD.MOVE`). Komennolla on oltava täsmälleen yksi `dir`-parametri, ja jokainen askel on tavallinen pelaajan komento hintoineen ja tapahtumineen. `walk_stop KYSELY` nimeää kyselyn `query Q(who: entity, next: pos) -> bool`, jota moottori kutsuu ennen jokaista askelta; tosi pysäyttää matkan. Pelin ehto tulee moottorin omien pysähtymisehtojen lisäksi, esimerkiksi kun näkyviin tulee uusi toimija (moottoridokumentti, luku [8.1](../moottori/08-syote.md#81-hiiri)). Ilman `walk`-kenttää klikkausmatkaa ei ole.

**Seuraaminen.** Moottorin toiminto `follow` (luku [11.3](11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)) kuljettaa ohjattavaa johtajan perässä omilla vuoroillaan, kunnes pelaaja painaa jotakin tai matkan pysähtymisehto täyttyy (moottoridokumentti, luku [8.1](../moottori/08-syote.md#81-hiiri)).

- `leader KYSELY` nimeää kyselyn `query Q(who: entity) -> entity`, joka antaa seurattavan, kun `follow` painetaan ilman katselukursoria (`- leader STD.LEADER_OF`).
- `rest KOMENTO` nimeää parametrittoman komennon, jonka seuraaja ajaa, kun se on perillä (`- rest STD.WAIT`). Askeleet kulkevat `walk`-komennolla.
- `follow_distance N` (oletus 1) on etäisyys, jolla seuraaja pysähtyy odottamaan.
- `follow_ai TEKOÄLY` antaa askeleet tekoälylle (`- follow_ai STD.FOLLOWER`). Silloin uusi toimija näkökentässä ei pysäytä seuraamista, joten seuraaja puolustautuu kuin lemmikki.

```
game KNIGHTS
- seats 1..4
- walk STD.MOVE
- leader STD.LEADER_OF
- rest STD.WAIT
- follow_ai STD.FOLLOWER
```

```
game CRYPT
- start TOWN
- walk STD.MOVE
- walk_stop HUNGRY_OR_NOISY
```

**Lyhyt muoto.** Moottorin kentät voi kirjoittaa otsikkoriville kuten valikon ominaisuudet:

```
game CRYPT start TOWN
game CRYPT start TOWN at START
game CRYPT start DUNGEON at STAIR.up
game CRYPT start START.MAP at SHRINES
game CRYPT start ARENA at 5,3
```

`start` nimeää aloituskartan tai noston pakasta tai taulusta, jonka kohteet ovat karttoja (luvut [6.2](06-pakat-ja-taulut.md#62-tila-sekoitus-ja-nosto) ja [6.5](06-pakat-ja-taulut.md#65-taulut)). Nosto on tavallinen nosto, joten `start START.MAP` heittää taulua kerran. Kenttä `game.start` on nostettu kartta, esimerkiksi `TOWN` tai `CAMP`:

```
table START.MAP 1d10
- 1..7 TOWN
- 8..10 CAMP
```

- `at` kertoo aloituskohdan paikkahaulla (luku [8.8](08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)): nimiö, hakujoukko (tile, tilen väli tai tilejoukko) tai sijainti, ja tarvittaessa `near` tai `from`. Jos `at` puuttuu, aloituskohta on aloituskartan nimiö `START`. Moottori laskee haun, kun aloituskartta on ladattu, ja tallentaa tuloksen ruutujoukkona kenttään `game.at`.
- `seed` kiinnittää pelin siemenen. Jos kenttä puuttuu, moottori arpoo uudelle pelille siemenen, ja `game.seed` kertoo sen. Kiinteä siemen kuuluu yleensä pelaajan `overrides/`-tiedostoon (luku [3.2](#32-pelaajan-oma-data)), esimerkiksi testaukseen.
- **Rajat.** `call_depth N` on sisäkkäisten kutsujen ja tapahtumien syvyysraja (oletus 256, sallittu 32–4096), ja `loop_steps N` on yhden silmukan askelraja (oletus 1 000 000, vähintään 10 000). Rajat vaikuttavat pelin kulkuun, joten ne ovat pelin dataa eivätkä asetuksia. Ne luetaan uuden pelin alussa ja tallentuvat pelikerran mukana kuten `autosave`-osion `slots` (luku [3.4](#34-automaattinen-tallennus-autosave)), joten `overrides/` tai modi ei muuta niitä kesken pelin. Rajan ylitys ajossa on käyttöhetken virhe (moottoridokumentti, luvut [2.6](../moottori/02-lataus.md#26-virheiden-lajit) ja [6.2](../moottori/06-ajo.md#62-tapahtumat)).
- **Istuimet.** `seats a..b` on liitettävien istuinten eli ihmispelaajien vähimmäis- ja enimmäismäärä paikallisessa yhteistyöpelissä (oletus `1..1`, moottorin raja 8). Kun yläraja on yli 1, uusi peli kysyy liittyjät liittymisruudussa (moottoridokumentti, luku [8](../moottori/08-syote.md)). Määrä lukitaan pelikerralle kuten rajat. `party switch|each` kertoo, odottavatko istuimen muut kuin ohjattavat jäsenet syötettä: `switch` (oletus) antaa niille tekoälyn vuoron, ja pelaaja vaihtaa ohjattavaa, kun taas `each` kysyy syötteen jokaiselta jäseneltä vuorollaan. `leash on|off` kytkee vakiokirjaston talutushihnan, joka estää ryhmää hajaantumasta kartta-alueen ulkopuolelle (oletus `on`, kun yläraja on yli 1). `party` ja `leash` luetaan datasta eivätkä lukitu. Istuimen jäsen on entiteetti, jolla on moottorin komponentti `seat` (luku [5.3](05-komponentit-ja-entiteetit.md#53-moottorin-komponentit)). Ilman `seats`-kenttää peli on yhden istuimen peli, ja kaikki säännöt ovat yksinpelin sääntöjä.
- **Pelaaja.** Kartat eivät sisällä pelaajaa (luku [8.1](08-kartat-ja-prefabit.md#81-karttatiedosto)). Kun aloitustila on laskettu, logiikka saa tapahtuman `begin`, ja sen käsittelijä luo jokaiselle liitetylle istuimelle hahmon aloituskohtaan `game.at` (RogueScript, luku [10.1](../roguescript/10-aloitus-ja-checkpointit.md#101-aloitus)). Vakiokirjasto tekee tämän oletuksena, joten aloitus toimii ilman pelin omaa logiikkaa. Pelaajan entiteetin valitsee vakiokirjaston komponentti `PLAYER`, jonka kenttä `class` on kortti (`- PLAYER FIGHTER`). Ilman riviä peli saa oletuspelaajan `STD.DEFAULT.PLAYER`, joten pelin pelaajaentiteetit nimetään muulla nimellä kuin `PLAYER`.
- **Arvonta.** Jos `game.at` sisältää monta ruutua, `begin`-käsittelijä arpoo ruudun pelin virrasta. Monen ruudun `START`-ryhmä on siis satunnainen aloituspiste. Vakiokirjasto sijoittaa istuimen 1 hahmon kenttään `game.at` ja muut vapaisiin ruutuihin sen ympärille kahden ruudun säteelle, joten yhden ruudun `START` riittää myös moninpeliin.
- **Liittyminen ja poistuminen.** Kesken pelin liittyvä istuin saa tapahtuman `join(seat)` ja poistuva tapahtuman `part(seat)` (RogueScript, luku [4.1](../roguescript/04-tapahtumat-ja-kasittelijat.md#41-moottorin-tapahtumat)). Vakiokirjasto antaa liittyjälle hahmon ja tekee poistuvan istuimen jäsenistä seuraajia.
- **Laskentajärjestys.** Moottori kiinnittää siemenen, sekoittaa `shuffled`-pakat täysien nimien aakkosjärjestyksessä, antaa `game`-lohkon alkuarvot, valitsee ja lataa aloituskartan (legend tai generaattori, `with`-entiteetit, kokoelmien täyttö, kerrosten `fill`), laskee kentän `game.at` ja lähettää tapahtuman `begin`. Nopat heitetään tässä järjestyksessä, joten tulos on toistettava. Laskettu aloitustila on tila `begin`-käsittelijöiden jälkeen.
- **Kirjaston `game`-lohko.** Kirjasto voi kirjoittaa nimettömän `game`-lohkon, joka lisää komponentteja (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)). Pelin nimi on vain päätiedostossa, ja kaksi nimettyä `game`-lohkoa on latausvirhe.
- **Virheet.** `start` ilman olemassa olevaa karttaa, pakkaa tai taulua on latausvirhe. Jos `start` puuttuu, peliä ei voi aloittaa uutena, mikä on virhe pelin käynnistyessä mutta ei kirjaston latauksessa. Tyhjä hakutulos on virhe aloitushetkellä, ja se kertoo kartan ja haun. Sijoitus moottorin kenttään (`game.start = …`) on latausvirhe. `call_depth` tai `loop_steps` rajojensa ulkopuolella on latausvirhe. `naming` ilman kyselyä tai kysely väärällä muodolla on latausvirhe. `layout` ilman olemassa olevaa asettelua on latausvirhe. `walk` ilman komentoa tai komento, jolla on muu kuin yksi `dir`-parametri, sekä `walk_stop` ilman kyselyä tai kysely väärällä muodolla ovat latausvirheitä. `seats` välin 1–8 ulkopuolella tai alaraja yli ylärajan on latausvirhe, ja samoin `party` tai `leash` muulla arvolla. `leash on` yhden istuimen pelissä saa varoituksen.
- **Kirjoitusasu on dataa.** `start`, `at` ja `seed` ovat `game`-lohkon kenttiä, joten aloituskohdan voi vaihtaa koskematta logiikkaan, esimerkiksi pelaajan `overrides/`-tiedostossa.

## 3.4 Automaattinen tallennus: autosave

Peli kertoo `autosave`-osiossa, montako automaattista tallennusta pidetään ja missä kohdissa moottori tallentaa. Ilman osiota automaattista tallennusta ei ole, ja omat tallennukset ja lopetus noudattavat oletuksia.

```
autosave
- slots 3
- on input every 20
- on scene
- on checkpoint
- on arrive
```

- `slots N` on paikkojen määrä. `0` ei tallenna automaattisesti. `1` on permadeath: yksi paikka, jonka jokainen tallennus korvaa, ja pelaajan oma tallennus menee samaan paikkaan ja lopettaa pelin. Suurempi luku on pino, jossa uusin on päällimmäisenä ja vanhin putoaa pois; pelaajan omat tallennukset ovat silloin omissa paikoissaan pinon ulkopuolella.
  - `manual N` on pelaajan omien paikkojen määrä (oletus 3, `0` = ei omaa tallennusta).
  - `quit save|none` päättää, tallentaako lopetus. `save` (oletus) tallentaa moodissa 0 omaan lopetuspaikkaansa ja moodissa N pinoon. Moodissa 1 oma tallennus ja lopetus käyttävät aina ainoaa paikkaa.
- `on KOHTA` valitsee tallennuskohdan: `input` (moottori odottaa syötettä, `every N` joka N:nnellä kerralla, ja moninpelissä odotukset lasketaan kaikilta istuimilta yhteen), `scene` (ennen etualan kohtausta, dialogia tai `open`-valikkoa), `checkpoint` (kun `checkpoint()` toteutuu) tai `arrive` (pelaaja saapuu kartalle, myös `travel`-kutsulla nykyiseen karttaan; ryhmän siirto tallentaa kerran toiminnossa). Logiikka voi lisäksi pyytää tallennuksen funktiolla `autosave()` (RogueScript, luku [10.2](../roguescript/10-aloitus-ja-checkpointit.md#102-checkpointit)).
- **Lukitus.** `slots` luetaan uuden pelin alussa ja tallentuu pelikerran mukana, joten `overrides/` ei muuta sitä kesken pelin. `on`-rivit luetaan aina datasta.
- **Yhdistyminen.** `autosave` on nimetön osio, ja sen rivit yhdistyvät kuten `settings`-osiossa (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).

Tallennuksen ajoitus, kirjoitus ja haarautuminen kuvataan moottoridokumentin luvussa [5.4](../moottori/05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus).

## 3.5 Profiili: profile

Profiili on pelin pysyvä tila, joka elää pelikertojen yli, kuten `game` elää yhden pelikerran: pistelista, avatut hahmoluokat, saavutukset ja tilastot. Se kirjoitetaan päätiedoston `profile`-lohkoon, jonka nimi on sama kuin pelin nimi:

```
profile ROGUE
- HALL
- UNLOCKS

component HALL
- scores list SCORE 10 sort score desc

component SCORE
- name text
- score number
- depth number
- cause name

component UNLOCKS
- wizard bool
```

- **Rakenne.** Lohkon rivit ovat komponentteja alkuarvoineen kuten `game`-lohkossa (luku [5.5](05-komponentit-ja-entiteetit.md#55-pelikerran-tila-game)). Pistelista on komponenttiarvojen lista (luku [5.1](05-komponentit-ja-entiteetit.md#51-komponentit)), jonka `sort` ja koko pitävät järjestyksessä ja leikattuna. Logiikka näkee profiilin kontekstinimellä `profile` (RogueScript, luku [10.4](../roguescript/10-aloitus-ja-checkpointit.md#104-profiili-profile)).
- **Alkuarvot.** Lohkon arvot ovat profiilin alkutila, kun peliä ei ole pelattu tällä koneella. Uusi komponentti tai kenttä lisätään olemassa olevaan profiiliin oletusarvoineen, ja datasta poistettu komponentti jää profiiliin näkymättömänä, kunnes se palaa.
- **Tallennus on moottorin.** Moottori kirjoittaa profiilin käyttäjädatakansioon asetusten viereen (luku [3.2](#32-pelaajan-oma-data)) omassa muodossaan tarkistussummalla. Se ei ole RLML:ää, joten `overrides/` ja modit eivät paikkaa sitä, ja käsin muokattu tiedosto huomataan (moottoridokumentti, luku [5.6](../moottori/05-aloitus-tallennus-ja-checkpointit.md#56-profiili)).
- **Ei pelikerran tilaa.** Profiili ei kuulu tallennukseen, checkpointiin, `rollback`-kutsuun eikä `delete_save`-kutsuun. Pelikerta näkee kuitenkin profiilin aloitushetken tilannekuvan ja omat kirjoituksensa, ja ne tallentuvat pelikerran mukana, joten pelin kulku on toistettava.
- **Nollaus** tehdään moottorin valikosta, ei logiikasta.
- **Virheet.** Toinen nimetty `profile`-lohko, `profile`-lohko kirjastossa nimettynä tai `profile`-lohko `overrides/`-tiedostossa on latausvirhe. Kirjasto voi kirjoittaa nimettömän `profile`-lohkon, joka lisää komponentteja.

---

← [2. Yleinen syntaksi](02-yleinen-syntaksi.md) · [Sisällys](README.md) · [4. Nimiavaruudet](04-nimiavaruudet.md) →
