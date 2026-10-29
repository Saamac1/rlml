[RLML](../README.md) › [RogueScript](README.md)

# 10. Aloitus ja checkpointit

## 10.1 Aloitus

Aloitus on dataa (`game`-lohko, dataformaatit luku [3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)), eikä se tarvitse pelin omaa logiikkaa. Kun laskettu aloitustila on valmis ja aloituskartta on ladattu, moottori lähettää tapahtuman `begin`. Kartoilla ei ole pelaajaa, vaan `begin`-käsittelijä luo sen ja antaa sen pelaajan hallintaan:

```roguescript
on begin() as STD.SPAWN_PLAYER:
    if game has PLAYER:
        control(spawn(game.PLAYER.class, at: game.at))
    else:
        control(spawn(DEFAULT.PLAYER, at: game.at))

on begin():
    start(INTRO())
```

- `control(entiteetti)` tekee ilmentymästä pelaajan: `player` viittaa siihen, ja moottori odottaa sen vuorolla syötettä. Kutsu toimii myös myöhemmin, esimerkiksi ruumiin vaihdossa (`control(golem)`).
- **Vakiokirjasto tekee oletuksen.** `STD.SPAWN_PLAYER` on vakiokirjaston nimetty käsittelijä, joten peli ilman omaa logiikkaa saa pelaajan. Entiteetti valitaan `game`-lohkon vakiokirjaston komponentilla `PLAYER`, jonka kenttä `class` on kortti ja oletus `STD.DEFAULT.PLAYER` (`- PLAYER FIGHTER`). Pelin pelaajaentiteetit nimetään siksi muulla nimellä kuin `PLAYER`. Peli korvaa sen tavalliseen tapaan (`override on begin() as STD.SPAWN_PLAYER:`).
- **Aloituskohta.** `game.at` on `game`-lohkon `at`-haun tulos ruutujoukkona, ja oletus on aloituskartan nimiö `START`. `spawn` arpoo joukosta ruudun pelin virrasta (luku [6.2](06-ruudut-ja-paikkahaku.md#62-operaatiot)), joten monen ruudun `START` on satunnainen aloituspiste.
- Ennen ensimmäistä `control`-kutsua `player` on `none`. Kartan latauksen `created`-käsittelijät eivät siis näe pelaajaa.
- Ladatussa pelissä `begin` ei laukea, koska peli on jo kerran alkanut.

## 10.2 Checkpointit

- **Checkpoint** on tilannekuva: koko pelin tila, mukaan lukien `game`, pakkojen tila ja pelin satunnaisvirran tila (dataformaatit, luku [3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)).
- **Checkpoint 0** on laskettu aloitustila `begin`-käsittelijöiden jälkeen. Uutta peliä varten sitä ei tallenneta etukäteen, vaan se lasketaan datasta ja siemenestä. Kun peli on alkanut, checkpoint 0 tallentuu pelin mukana.
- `checkpoint()` on moottorin toiminto, joka ottaa tilannekuvan sen toiminnon lopussa, jossa kutsu tehtiin (moottoridokumentti, luku [6.1.7](../moottori/06-ajo.md#617-toiminto)). Sitä kutsutaan käsittelijässä, esimerkiksi kun pelaaja astuu `SHRINES`-joukon ruutuun tai saapuu uudelle kartalle.
- `rollback()` lataa viimeisimmän checkpointin sellaisena kuin se tallennettiin. Lataus tehdään toiminnon lopussa, joten käynnissä oleva käsittelijäketju ajetaan loppuun. Jos sama toiminto kutsuu molempia, viimeinen kutsu ratkaisee.
- Viimeisin checkpoint kuuluu tallennukseen, joten `rollback()` toimii myös ladatussa pelissä. `autosave()` pyytää automaattisen tallennuksen, joka tehdään toiminnon lopussa. Peli voi myös asettaa checkpointin automaattisen tallennuksen kohdaksi (dataformaatit, luku [3.4](../dataformaatit/03-tiedostot-ja-include.md#34-automaattinen-tallennus-autosave); moottoridokumentti, luku [5.4](../moottori/05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)).

Kuoleman käsittely on logiikkaa: vakiokirjaston tai pelin sääntö päättää, kutsutaanko `rollback()` vai poistetaanko tallennus. Vakiokirjaston `STD.DEATH` vain tuhoaa kuolleen, myös pelaajan. Moottori ei päätä peliä kuolemaan. Ilman sääntöä maailma jatkaa kulkuaan ilman pelaajaa, ja pelistä poistutaan taukovalikosta (moottoridokumentti, luvut [6.1.2](../moottori/06-ajo.md#612-toimijat-ja-ajoitin) ja [8](../moottori/08-syote.md)). Permadeath-peli ei käytä checkpointeja vaan poistaa tallennuksen:

```roguescript
on DIED(victim, killer) when victim == player and game has no PERMADEATH:
    rollback()

on DIED(victim, killer) when victim == player and game has PERMADEATH:
    delete_save(all)
    start(GAMEOVER())
```

`delete_save(kohde)` poistaa viimeisimmän checkpointin (`checkpoint`), pelikerran automaattiset tallennukset (`auto`) tai molemmat (`all`). Se tehdään toiminnon lopussa, ja pelaajan omat tallennukset jäävät (moottoridokumentti, luku [5.5](../moottori/05-aloitus-tallennus-ja-checkpointit.md#55-tallennuksen-poisto)). Pelaaja voi poistaa oman checkpointinsa valikon kohdasta, joka kutsuu `delete_save(checkpoint)`. Profiiliin (luku [10.4](#104-profiili-profile)) `delete_save` ei koske.

## 10.3 Pelikerran tila: game

`game` on kontekstinimi, joka viittaa pelikerran tilaan (dataformaatit, luku [5.5](../dataformaatit/05-komponentit-ja-entiteetit.md#55-pelikerran-tila-game)). Se näkyy kaikkialla.

- **Pienet kentät** ovat moottorin: `game.start` (nostettu aloituskartta), `game.at` (aloituskohta ruutujoukkona), `game.seed` (pelin siemen), `game.turn` (kierrosluku) ja `game.time` (kulunut aika mikroaskelina, moottoridokumentti luku [6.1.1](../moottori/06-ajo.md#611-mikroaskel-ja-kierros)) sekä rajat `game.call_depth` (kutsusyvyys, oletus 256, sallittu 32–4096) ja `game.loop_steps` (silmukan askelraja, oletus 1 000 000, vähintään 10 000). Rajat asetetaan datan `game`-lohkossa (luku [5.2](05-funktiot-ja-lausekkeet.md#52-ohjausrakenteet-ja-muuttujat)). Ne ovat vain luettavia, ja sijoitus niihin on latausvirhe.
- **Isot osat** ovat komponentteja, joita logiikka lukee ja muuttaa kuten entiteetin komponentteja: `game.KING_QUEST = 1`, `game has PERMADEATH`.
- **Tallentuu.** `game` kuuluu tilannekuvaan. Ladattu peli käyttää aloitushetken arvoja, vaikka data olisi sen jälkeen muuttunut.

## 10.4 Profiili: profile

`profile` on kontekstinimi, joka viittaa pelin profiiliin: tilaan, joka elää pelikertojen yli, kuten pistelista, avatut hahmoluokat ja saavutukset (dataformaatit, luku [3.5](../dataformaatit/03-tiedostot-ja-include.md#35-profiili-profile)). Profiilin isot osat ovat komponentteja kuten `game`-tilassa.

```roguescript
on DIED(victim, killer, cause) as ROGUE_SCORE priority -5 when victim == player:
    profile.HALL.scores += SCORE(name: victim.name.text, score: victim.PURSE.gold, depth: game.DEPTH.now, cause: cause)

on begin() when profile has UNLOCKS and profile.UNLOCKS.wizard:
    game.CLASSES.items += CLASS.WIZARD
```

- **Kirjoitus** tehdään toiminnon lopussa kuten `autosave()`, ja etualan kohtauksessa vasta luonnoksen kirjauksessa. `rollback`, `delete_save` ja pelikerran lataus eivät peru profiilin muutoksia.
- **Luku pelikerrassa.** Pelikerta näkee profiilista aloitushetken tilannekuvan ja omat kirjoituksensa. Tilannekuva tallentuu pelikerran mukana, joten sama tallennus ja samat syötteet tuottavat saman kulun myös koneella, jonka profiili on toinen.
- Profiilin tallennus ja suojaus ovat moottorin asia (moottoridokumentti, luku [5.6](../moottori/05-aloitus-tallennus-ja-checkpointit.md#56-profiili)).

---

← [9. Kohtaukset ja aikajanat](09-kohtaukset-ja-aikajanat.md) · [Sisällys](README.md) · [11. Tallennus ja korvaaminen](11-tallennus-ja-korvaaminen.md) →
