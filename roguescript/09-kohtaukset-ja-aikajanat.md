[RLML](../README.md) › [RogueScript](README.md)

# 9. Kohtaukset ja aikajanat

Lyhyt opas esimerkkeineen: [Kohtausopas](kohtausopas.md).

Kohtaus kirjoitetaan joko korutiinina (`scene NIMI():`) tai aikajanana (`scene NIMI() timeline:`). Haarautuva tarina ja dialogi kuuluvat korutiiniin ja ajoitettu esitys aikajanaan.

Kohtauksen käynnistys:

- `wait scene X()` suorittaa kohtauksen ja jatkaa sen jälkeen. Se toimii vain korutiinissa eli komennossa ja kohtauksessa sekä aikajanan rivillä. Komennossa odotus päättää käynnissä olevan toiminnon, ja komento jatkuu kohtauksen jälkeen uutena toimintona (moottoridokumentti, luku [6.1.6](../moottori/06-ajo.md#616-monivaiheinen-komento)).
- `start(X())` laittaa etualan kohtauksen jonoon. Kohtaus alkaa, kun käynnissä oleva toiminto (herätys, käsittelijäketju, kierros tai ajastin, moottoridokumentti luku [6.1.7](../moottori/06-ajo.md#617-toiminto)) on päättynyt, ja jonon kohtaukset ajetaan aloitusjärjestyksessä. `start` toimii kaikkialla, myös käsittelijässä. Taustakohtaus käynnistetään samalla kutsulla (luku [9.9](#99-taustakohtaukset)).
- **Kohtauksen kartta.** Etualan kohtauksen `map` ratkaistaan, kun kohtaus alkaa: se on pelaajan kartta, tai ilman pelaajaa käynnistäjän kartta `start`-hetkellä. Moninpelissä pelaaja on kohtauksen avaajan istuimen ohjattava, ja jos avaaja ei ole jäsen, kartta on kartta-alueen kartta. Toiminnon lopun siirrot tehdään ennen kohtauksia, joten käsittelijä voi kutsua `travel` ja `start` peräkkäin, ja kohtaus pelataan kohdekartalla (luku [6.4](06-ruudut-ja-paikkahaku.md#64-siirtyminen-kartalta-toiselle)). `wait scene` -alikohtaus perii kutsujansa kartan. Taustakohtauksen kartta on käynnistäjän kartta `start`-hetkellä, ja `scope: map` sitoo sen (luku [9.9](#99-taustakohtaukset)).

Kun käsittelijä tarvitsee esityksen ja sen jälkeen valikon, se käynnistää korutiinikohtauksen, joka tekee molemmat:

```roguescript
scene ENDING(robot: entity, kitten: entity):
    wait scene FOUND_KITTEN(robot, kitten)
    open VICTORY()
```

## 9.1 Rivi

Aikajanan rivillä on alku, valinnainen kesto ja toiminto:

```
ALKU [KESTO] [always] [when ehto:] toiminto
```

```roguescript
scene GATE_OPENS(lever: entity) timeline:
    0000ms        play(SFX.LEVER, at: lever)
    0500ms 1000ms shake(camera, strength: 2)
    1000ms 2000ms tile(label(GATE_A), GATE.open, layer: GROUND)
    2000ms        mark OPEN
    OPEN          when game.KING_QUEST == 1: say(GATE.FOR_THE_KING)
    OPEN+1s       say(GATE.OPENED)
    +200ms 0500ms pan(camera, to: player)
```

- **Alku** on aika kohtauksen alusta (`1s`, `1500ms`), aika edellisen rivin alusta (`+200ms`) tai aika merkistä (`OPEN+1s`, `OPEN-500ms`).
- **Kesto** on toinen aikaliteraali. Jos rivillä on kesto, kutsu on tweeni, joka etenee annetun ajan. Ilman kestoa kutsu tapahtuu hetkessä.
- **Yksiköt** ovat `ms` ja `s`. Desimaalit eivät ole sallittuja, mutta etunollat ovat (`0500ms`), jotta sarakkeet tasautuvat ilman erillistä tasausta.
- **Sisennys on rakennetta.** Jokainen rivi alkaa lohkonsa sarakkeesta, ja `track`- ja `when`-lohkojen sisältö on neljä välilyöntiä syvemmällä. Rivin sisällä sanojen väli on vapaa, ja kesto tunnistetaan siitä, että toinen sana on aikaliteraali.
- **Saman hetken rivit** suoritetaan rivijärjestyksessä. Suositus on kirjoittaa sama aika uudelleen. Rivi ilman aikaa (rivi alkaa suoraan toiminnolla lohkon sarakkeesta) perii edellisen rivin ajan.
- **Järjestys.** Lataus järjestää rivit ajan mukaan, ja saman ajan rivit säilyttävät kirjoitusjärjestyksensä.

**Tilaa muuttava tweeni.** Tila on aina kokonaisia ruutuja, joten tilaa muuttava kutsu kestolla jaetaan askeliksi. Jokainen askel on tavallinen tilan muutos, ja sen tapahtumat laukeavat sillä hetkellä, joten logiikka ja ruudulla näkyvä kuva pysyvät samassa tahdissa.

- `move` kestolla laskee rivin alussa kerran reitin kohteeseen. `via: path` (oletus) noudattaa liikkujan estoja, ja `via: line` kulkee suoraan esteistä välittämättä. Esityksen sijainti etenee reittiä pitkin tasaisesti ruudun osina, ja kun pyöristetty ruutu vaihtuu, moottori siirtää liikkujan siihen tavallisella `move`-kutsulla. Moottori laskee osat kokonaisluvuilla, joten tulos on sama kaikilla alustoilla.
- **Askeleen hetki.** Askel osuu hetkeen, jolloin tasaisesti etenevä sijainti pyöristyy seuraavaan ruutuun: kun reitillä on N ruutua ja kesto on D, askel i (1…N) tehdään hetkellä alku + ⌈(2i − 1)·D / 2N⌉. Joukon tilet vaihtuvat samoin hetkin.
- **Este matkalla.** Jos seuraava ruutu on askeleen hetkellä estetty, tweeni pysähtyy viimeiseen saavutettuun ruutuun ja rivi päättyy. Kiertäminen on logiikan asia.
- Joukon tilen vaihto (`tile` tai `carve` joukolle) vaihtaa ruudut yksi kerrallaan tasaisesti keston aikana lukujärjestyksessä, tai `from: ankkuri` -argumentilla etäisyyden mukaan lähimmästä alkaen.
- Yhden ruudun `tile`, `spawn` ja `destroy` muuttuvat rivin alussa, ja kesto häivyttää muutoksen esiin tai pois.
- Kesto kutsulle, jolla ei ole esitystä (`emit`, `GIVE`, kentän sijoitus), on latausvirhe.

```roguescript
scene BRIDGE_RISES(guard: entity) timeline:
    0s    2000ms fade(light(label(GATE)), to: 100%)
    500ms 1500ms move(guard, to: label(DOOR))
    1s    3s     carve(label(BRIDGE), BRIDGE.built, from: label(GATE))
```

## 9.2 Merkit

Rivi `AIKA mark NIMI` nimeää hetken, ja muut rivit voivat laskea aikansa siitä. Merkin avulla koko jakson voi siirtää muuttamalla yhden rivin aikaa. Merkki on kohtauksen sisäinen nimi, ja se näkyy kaikille raidoille, koska raidoilla on yhteinen kello. Silmukkaraidan merkki on latausvirhe.

## 9.3 Raidat ja silmukat

Raidat kulkevat rinnakkain yhteisellä kellolla, ja jokainen alkaa ajasta `0s`. Raidattomat rivit kuuluvat oletusraitaan.

```roguescript
scene AUDIENCE() timeline:
    track TORCHES loop 2s:
        0000ms 1000ms fade(light(label(TORCH)), to: 80%)
        1000ms 1000ms fade(light(label(TORCH)), to: 100%)
    track ACTION:
        0000ms        say(KING.HELLO)
        1000ms 2000ms move(entities(in: label(GUARD)).first, to: label(DOOR))
```

- `loop PITUUS` toistaa raitaa. Pituus on pakollinen. Rivin alku on oltava pituutta pienempi, ja kestoineen rivi saa päättyä täsmälleen rajalle. Rajan ylittävä rivi on latausvirhe.
- Silmukka päättyy, kun kohtaus päättyy tai kun kutsutaan `stop(track: NIMI)`.
- Kohtaus päättyy, kun kaikki sen silmukattomat raidat ovat valmiita. Kohtaus, jossa on pelkkiä silmukoita, jatkuu, kunnes se pysäytetään.

## 9.4 Ehdot

`when ehto:` toiminnon paikalla koskee yhtä toimintoa, tai se aloittaa lohkon. Ehto arvioidaan, kun kello saavuttaa rivin. Lohkon rivit perivät sen ajan, `+`-ajat lasketaan lohkon alusta, ja `else:` on sallittu samalla sisennyksellä kuin rivi, joka aloitti lohkon.

```roguescript
2000ms when game.KING_QUEST == 1:
    say(KING.THANKS)
    GIVE(player, QUEST.REWARD)
    +1s 2s pan(camera, to: label(TREASURY))
else:
    say(KING.WELCOME)
```

Jos ehto ei päde, rivi jätetään väliin, mutta aika kuluu silti.

## 9.5 Kello

Kohtauksen kello laskee esitysaikaa. Pelin aika on pysähdyksissä etualan kohtauksen ajan.

- Modaalinen `say` pysäyttää kellon, kunnes pelaaja kuittaa. Saman hetken `say`-rivit näytetään peräkkäin, ja kello jatkaa samasta kohdasta.
- `wait confirm` pysäyttää kellon ilman tekstiä, ja `wait scene X()` suorittaa alikohtauksen.
- Tweenin loppu ei pysäytä kelloa. Tweenin loppuun odotetaan kirjoittamalla seuraavan rivin alkuun tweenin loppuaika.

## 9.6 say ja log

`say` käsittelijässä, funktiossa tai reaaliaikaisessa taustakohtauksessa on latausvirhe. `log` käyttää `messages`-osiossa ilmoitettua viestiä, joka antaa parametrit. Loki valitsee viestit avaimen ryhmällä tai avaimella (dataformaatit, luku [11.4](../dataformaatit/11-kayttoliittyman-data.md#114-viestit-ja-loki)), ja viestin teksti voi viitata toiseen tekstiavaimeen muodossa {@AVAIN} (dataformaatit, luku [10](../dataformaatit/10-tekstit.md)).

| Kutsu | Näkyy | Missä sallittu |
| --- | --- | --- |
| `say(AVAIN, …)` | modaalinen dialogi-ikkuna, odottaa kuittausta | korutiini, etualan aikajana, pelin ajan taustakohtaus |
| `log(AVAIN, …)` | rivi tapahtumalokissa | kaikkialla |

`speaker:` on molempien valinnainen moottorin parametri, joka näyttää puhujan nimen. Kumpikaan ei käytä kestosaraketta.

**Nimi pelin tilasta.** `game`-lohkon kenttä `naming KYSELY` (dataformaatit, luku [3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)) nimeää kyselyn, jota moottori kutsuu aina, kun se näyttää entiteetin nimen: paikkamerkissä, listavalikossa ja paneelissa. Kyselyn muoto on `query Q(e: entity) -> name|text`. Paluuarvo `name` näyttää avaimen `<NIMI>.NAME` tekstin, ja `{E.DESC}` hakee saman nimen kuvauksen. `text` näytetään sellaisenaan. `none` käyttää tavallista nimeä. Yhdistetty paluutyyppi `name|text` on sallittu vain tälle kyselylle, koska avain taipuu kielen mukaan ja teksti on pelaajan antamille nimille.

```roguescript
query ITEM_NAME(e: entity) -> name|text:
    k = e.kind
    if not k in game.IDENT.looks or game.IDENT.known[k] == true:
        return none
    if k in game.IDENT.called:
        return game.IDENT.called[k]
    return game.IDENT.looks[k]
```

Moottori laskee kyselyn kuolleelle kahvalle tuhohetkellä kerran ja säilyttää tuloksen kahvan mukana, joten juotu tai varastettu esine näkyy viestissä samalla nimellä kuin eläessään (moottoridokumentti, luku [7.3](../moottori/07-esitys.md#73-tekstit)).

## 9.7 Dialogit

Dialogi on korutiinikohtaus, jossa on `choice`. Valinta näytetään valikkona kontekstissa `DIALOGUE`. Haarat sisennetään, joten pitkä keskustelu luetaan yhtenä tarinana.

```roguescript
scene KING.TALK(speaker: entity):
    say(KING.HELLO, speaker: speaker)
    choice:
        QUEST hide when game.KING_QUEST != 0:
            game.KING_QUEST = 1
            GIVE(player, QUEST.SEAL)
            say(KING.THANKS, speaker: speaker)
        SHOP disabled when speaker.WARES.items.count == 0:
            open SHOP.BUY(wares: speaker.WARES.items)
        BYE:
            return
```

- Kohdan nimi on tekstiavain kuten valikon kohta (`KING.TALK.QUEST`).
- `hide when ehto` piilottaa kohdan, kun ehto pätee.
- `disabled when ehto` himmentää kohdan, kun ehto pätee: kohta näkyy, mutta sitä ei voi valita. Jos kohdassa on molemmat, `hide when` tulee ensin, ja piilotettu kohta ei ole himmennetty vaan poissa. Himmennetyn kohdan väri tulee tyylitiedostosta (dataformaatit, luku [9.3](../dataformaatit/09-ulkoasu.md#93-paneelit-lokit-ja-esitys)).
- **Arviointihetki.** Kaikkien kohtien ehdot arvioidaan kerralla, kun `choice` alkaa. Ehdot näkevät kohtauksen luonnoksen (luku [9.8](#98-luonnos-ohitus-ja-alusta-aloitus)), joten saman kohtauksen aiemmat valinnat vaikuttavat myöhempiin valikkoihin. Valitun kohdan lohko ei muuta jo näytettyä valikkoa. Toistossa (`repeat`) jokainen uusi `choice` arvioi ehdot uudelleen.
- **Valikot toimivat samoin.** Datan valikon kohtien `hide when` ja `disabled when` arvioidaan kerralla, kun valikko avautuu (dataformaatit, luku [11.2](../dataformaatit/11-kayttoliittyman-data.md#112-valikot)).
- **Käynnistys** on sama kuin muilla kohtauksilla: `start(KING.TALK(king))` tai `wait scene KING.TALK(king)`.
- `open VALIKKO(…)` avaa valikon ja odottaa, kunnes se suljetaan. `open PANEELI(…)` näyttää parametrillisen paneelin modaalisena, kunnes pelaaja kuittaa sen (dataformaatit, luku [11.5](../dataformaatit/11-kayttoliittyman-data.md#115-paneelit)). `open` toimii vain korutiinissa. Komennossa se päättää käynnissä olevan toiminnon kuten `wait scene`.
- **Luonnos.** Dialogi on etualan kohtaus, joten sen tilan muutokset kirjataan vasta lopussa (luku [9.8](#98-luonnos-ohitus-ja-alusta-aloitus)).

Aiempi valinta voi avata myöhemmän kohdan:

```roguescript
scene GNOME.RIDDLE(speaker: entity):
    choice:
        RIDDLE:
            speaker.GNOME_MOOD.hinted = true
        BYE:
            return
    choice:
        KITTEN hide when not speaker.GNOME_MOOD.hinted:
            say(KITTEN_HINT(speaker), speaker: speaker)
        BYE:
            return
```

**Kohdan parametrit.** `choice`-kohta voi saada tekstiavaimensa parametrit `say`-kutsun muodossa: `BUY_SWORD(PRICE: PRICE_OF(SWORD)) disabled when PRICE_OF(SWORD) > player.PURSE.coin:`. Parametrit lasketaan samalla kertaa kuin `hide when` ja `disabled when`.

**Säädettävä kohta.** `NIMI adjust KENTTÄ a..b [pool NIMI]` ilman runkoa muuttaa kentän arvoa vasemmalla ja oikealla eikä päätä valintaa. `pool` on paikallinen nimi tai kenttä, josta korotukset vähennetään ja johon laskut palautetaan. Valinta päättyy tavalliseen kohtaan. Säätö ei päätä valintaa, joten kohtien ehdot arvioidaan säädön jälkeen uudelleen: alla `DONE` aukeaa, kun pisteet on jaettu.

```roguescript
scene CREATE(hero: entity):
    hero_name = wait text(16)
    add(hero, name(text: hero_name))
    points = 30
    choice:
        STRENGTH adjust hero.STATS.strength 10..25 pool points
        AGILITY adjust hero.STATS.agility 10..25 pool points
        DONE disabled when points > 0:
            return
```

**Syöte korutiinissa.** `x = wait number(a..b)` ja `x = wait text(N)` kysyvät luvun tai tekstin ja palauttavat arvon tai `none`, kun pelaaja peruu. Komennossa ne päättävät toiminnon kuten `wait confirm` ja tallentuvat odotuskohtina (luku [11.1](11-tallennus-ja-korvaaminen.md#111-tallennus)).

**Valikon paluuarvo.** `x = open VALIKKO(…)` palauttaa listavalikon viimeksi valitun jäsenen tai `none` (dataformaatit, luku [11.2](../dataformaatit/11-kayttoliittyman-data.md#112-valikot)). Tavallinen valikko ja paneeli palauttavat `none`.

## 9.8 Luonnos, ohitus ja alusta aloitus

Etualan kohtaus muuttaa maailman tilaa pysyvästi vasta päättyessään. Siihen asti tilan muutokset tehdään kohtauksen omaan luonnokseen, joka näkyy ruudulla kuten oikea tila. Kun kohtaus päättyy, luonnos kirjataan maailmaan yhdellä kertaa.

Esityksen satunnaisvirta alustetaan kohtauksen alussa pelin siemenestä, kohtauksen nimestä ja käynnistyksen järjestysnumerosta. Esitys toistuu siksi samanlaisena alusta aloitettaessa, eikä se vaikuta pelin virtaan.

Pelaajalla on kohtauksessa (konteksti `SCENE`) kolme moottorin toimintoa:

- `skip` suorittaa loput rivit ohitussäännöin (taulukko alla) ja kirjaa luonnoksen.
- `restart` hylkää luonnoksen ja aloittaa kohtauksen alusta. Valinnat kysytään uudelleen.
- `rewind` palaa edelliseen modaaliseen `say`-kohtaan tai valintaan. Moottori tekee hiljaisen restartin ja kelaa kohtaan toistamalla tehdyt valinnat. Kelaus onnistuu, koska kulku on deterministinen.

| Kutsu | Ohitettaessa |
| --- | --- |
| tilaa muuttava (`fn`, `tile`, `move`, kentät, pakkafunktiot) | suoritetaan heti rivijärjestyksessä |
| tilaa muuttava tweeni | jäljellä olevat askeleet tehdään heti järjestyksessä tapahtumineen (luku [9.1](#91-rivi)) |
| `say` | kirjataan lokiin ilman ikkunaa |
| `log` | kirjoitetaan lokiin |
| esitys (`present fn`, `play`, `pan`, `shake`, `fade`, `fly`) | jätetään väliin |
| `always`-rivi | suoritetaan, ja sen esitys näytetään |
| `wait confirm` | ohitetaan |
| `wait scene X()` | alikohtaus ohitetaan samoin säännöin |
| `choice`, `open` | ohitus pysähtyy, koska ne vaativat syötteen |
| silmukkaraita | ei suoriteta |

Moottorin funktion laji (muuttaa, kysely, esitys) ratkaisee, kumpaan riviin kutsu kuuluu (moottoridokumentti, luku [9.2](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#92-funktion-kuvaus)). Ohitettu kohtaus päätyy samoilla valinnoilla aina samaan tilaan kuin loppuun katsottu, koska esitys käyttää omaa satunnaisvirtaansa. Myös loki on sama: ennen ohitusta näytetyt repliikit ovat lokissa jo, ja ohitus kirjaa loput samassa järjestyksessä, joten pelaaja voi lukea ohitetun keskustelun lokista, ja loki ei riipu siitä, missä kohdassa pelaaja ohitti. Lataus varoittaa, jos silmukkaraidassa on tilaa muuttava kutsu.

**Tallennus.** Etualan kohtauksen, dialogin ja `open`-valikon aikana pelaaja ei voi tallentaa. Moottori voi tallentaa kohtausta edeltävän tilan automaattisesti, ja ladattaessa kohtaus ajetaan alusta (moottoridokumentti, luku [5.4](../moottori/05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)).

## 9.9 Taustakohtaukset

Taustakohtaus kulkee pelivuorojen rinnalla eikä ota syötteitä itselleen. Kellon voi valita kahdesta:

```roguescript
scene PATROL(guard: entity) timeline background:
    track ROUTE loop 12:
        00 02 move(guard, to: label(POST_A))
        05 02 move(guard, to: label(POST_B))
        10    log(GUARD.ALL_CLEAR, speaker: guard)

scene CANDLES() timeline background realtime:
    track FLICKER loop 600ms:
        000ms 300ms fade(light(label(CANDLE)), to: 70%)
        300ms 300ms fade(light(label(CANDLE)), to: 100%)
```

|  | `background` | `background realtime` |
| --- | --- | --- |
| Kello | pelin aika, kierroksina (paljas luku) | esitysaika (`ms`, `s`) |
| Kulkee | pelivuorojen mukana | myös kun peli odottaa syötettä |
| Tilan muutokset | sallittu, kirjataan heti | latausvirhe, myös `log` |
| `say` | sallittu, näytetään heti kierroksen rajalla | latausvirhe |
| Etualan kohtauksen aikana | pysähtyy | jatkaa |

- Rivit suoritetaan kierroksen rajalla, kierroksen tapahtumien ja ajastimien jälkeen ja ennen toimijoiden vuoroja (moottoridokumentti, luku [6.1.4](../moottori/06-ajo.md#614-mikroaskeleen-vaiheet)). Pelin aikaa käyttävän tweenin askeleet jaetaan keston mikroaskelille, ja moottori ajaa ne kohtauksen herätyksinä toimijoiden välissä. Graafisessa esitystavassa `move` liukuu sujuvasti.
- **`say` taustakohtauksessa** näyttää ikkunan heti kierroksen rajalla, ja peli odottaa kuittausta. Kuittauksen jälkeen saman hetken loput rivit ja kierroksen muut taustakohtaukset jatkuvat aloitusjärjestyksessä.
- **Käynnistys:** `start(SCENE(…), scope: map, bind: guard)` palauttaa kahvan, ja `stop(kahva)` pysäyttää kohtauksen. Kohtauksia voi olla käynnissä useita yhtä aikaa.
- **Kello.** Pelin ajan kohtauksen kello alkaa ensimmäiseltä kierroksen rajalta `start`-kutsun jälkeen, joten rivi `00` ajetaan sillä rajalla. `realtime`-kohtauksen kello on esitysaikaa eikä kuulu pelin tilaan: tallennus säilyttää kohtauksen, ja ladattaessa sen kello alkaa alusta (moottoridokumentti, luku [5.2](../moottori/05-aloitus-tallennus-ja-checkpointit.md#52-tallennus)). Siksi `realtime`-kohtauksessa ei saa olla myöskään `log`-kutsua, koska loki tallentuu.
- `scope: map` päättää kohtauksen, kun pelaaja poistuu kartalta. Ilman sitä kohtaus on yleinen.
- `bind: X` päättää kohtauksen, kun entiteetti X tuhoutuu (`removed`). Arvo voi olla yksi entiteetti tai kokoelma (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)), ja kokoelmasta kohtaus päättyy, kun yksikin sen jäsen tuhoutuu.
- Taustakohtaus päättyy itsestään, kun sen silmukattomat raidat ovat valmiita.

```roguescript
on arrive(who, map, p) when who == player and map == THRONE_ROOM:
    g = entities(in: label(GUARD)).choose()
    start(PATROL(g), scope: map, bind: g)
```

---

← [8. Generointi](08-generointi.md) · [Sisällys](README.md) · [10. Aloitus ja checkpointit](10-aloitus-ja-checkpointit.md) →
