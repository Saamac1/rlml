[RLML](../README.md) › [RogueScript](README.md)

# Kohtausopas

Tämä opas kertoo lyhyesti, miten kohtaukset kirjoitetaan. Speksi on luvussa [9. Kohtaukset ja aikajanat](09-kohtaukset-ja-aikajanat.md), ja se ratkaisee, jos opas ja speksi eroavat.

## 1. Kolme lajia

| Laji | Syntaksi | Käyttö |
| --- | --- | --- |
| Korutiini | `scene NIMI(param: tyyppi):` | haarautuva tarina, dialogi, valikot, syötteen kysely |
| Aikajana | `scene NIMI() timeline:` | ajoitettu esitys: liike, häivytys, kamera |
| Taustakohtaus | `scene NIMI() timeline background [realtime]:` | kulkee pelivuorojen rinnalla, esim. vartijan partio |

## 2. Käynnistys

- **`start(X())`** laittaa kohtauksen jonoon. Se toimii kaikkialla, myös käsittelijässä. Kohtaus alkaa, kun käynnissä oleva toiminto on päättynyt, ja jonossa olevat kohtaukset ajetaan aloitusjärjestyksessä.
- **`wait scene X()`** ajaa kohtauksen loppuun ja jatkaa sitten. Se toimii vain korutiinissa (komento tai kohtaus) ja aikajanan rivillä.
- Kohtaus pelataan pelaajan kartalla. Jos käsittelijä kutsuu ensin `travel` ja sitten `start`, kohtaus pelataan kohdekartalla.

```roguescript
on BUMP(who, other) as SPEAK_TO_OZ when who == player and other has OZ_MARK:
    start(OZ.TALK(other))

scene ENDING(robot: entity, kitten: entity):
    wait scene FOUND_KITTEN(robot, kitten)   // ensin esitys
    open VICTORY()                           // sitten valikko
```

Lisää: [RS 9](09-kohtaukset-ja-aikajanat.md).

## 3. Korutiini ja dialogi

Korutiini on tavallista koodia, joka pysähtyy odottamaan pelaajaa:

- `say(AVAIN, speaker: e)` avaa modaalisen ikkunan ja odottaa kuittausta.
- `wait confirm` odottaa kuittausta ilman tekstiä.
- `x = wait text(16)` ja `x = wait number(1..10)` kysyvät tekstin tai luvun. Jos pelaaja peruu, arvo on `none`.
- `open VALIKKO(…)` tai `open PANEELI(…)` avaa valikon tai paneelin ja odottaa sen sulkemista. `x = open …` palauttaa listavalikosta valitun jäsenen.
- `choice:` näyttää valikon (konteksti `DIALOGUE`):

```roguescript
scene KING.TALK(speaker: entity):
    say(KING.HELLO, speaker: speaker)
    choice:
        QUEST hide when game.KING_QUEST != 0:               // piilossa, kun ehto pätee
            game.KING_QUEST = 1
            say(KING.THANKS, speaker: speaker)
        SHOP disabled when speaker.WARES.items.count == 0:  // himmennetty
            open SHOP.BUY(wares: speaker.WARES.items)
        BYE:
            return
```

- Kohdan nimi on tekstiavain, esimerkiksi `KING.TALK.QUEST`.
- Kaikkien kohtien ehdot arvioidaan kerran, kun `choice` alkaa. Aiemman valinnan tulos voi siksi avata kohdan seuraavassa `choice`-lohkossa.
- Kohdalle voi antaa parametreja: `BUY_SWORD(PRICE: PRICE_OF(SWORD)) disabled when …:`
- Säädettävä kohta `STRENGTH adjust hero.STATS.strength 10..25 pool points` muuttaa arvoa nuolilla eikä päätä valintaa.

Lisää: [RS 9.7](09-kohtaukset-ja-aikajanat.md#97-dialogit).

## 4. Aikajana

Jokainen rivi on muotoa `ALKU [KESTO] [always] [when ehto:] toiminto`:

```roguescript
scene GATE_OPENS(lever: entity) timeline:
    0000ms        play(SFX.LEVER, at: lever)
    0500ms 1000ms shake(camera, strength: 2)         // kesto, joten tweeni
    2000ms        mark OPEN                          // nimetty hetki
    OPEN          when game.KING_QUEST == 1: say(GATE.FOR_THE_KING)
    OPEN+1s       say(GATE.OPENED)
    +200ms 0500ms pan(camera, to: player)            // edellisen rivin alusta
```

- **Aika** voi olla absoluuttinen (`1s`, `1500ms`), suhteellinen edelliseen riviin (`+200ms`) tai merkkiin (`OPEN-500ms`). Yksiköt ovat `ms` ja `s`, eikä desimaaleja sallita. Etunollat (`0500ms`) ovat sallittuja, jotta sarakkeet tasautuvat.
- **Kesto** tekee kutsusta tweenin. Tilaa muuttava tweeni (`move`, joukon `tile`/`carve`) etenee kokonaisina ruutuina, ja jokainen askel laukaisee tapahtumansa. Kesto kutsulle, jolla ei ole esitystä (esim. `emit`), on latausvirhe.
- **Raidat** kulkevat rinnakkain yhteisellä kellolla:

  ```roguescript
  scene HALL_LIGHTS() timeline:
      track TORCHES loop 2s:
          0000ms 1000ms fade(light(label(TORCH)), to: 80%)
          1000ms 1000ms fade(light(label(TORCH)), to: 100%)
  ```

  `loop` vaatii pituuden. Silmukan voi pysäyttää kutsulla `stop(track: NIMI)`. Kohtaus päättyy, kun kaikki silmukattomat raidat ovat valmiita.
- **`when … :` / `else:`** koskee yhtä riviä tai lohkoa. Jos ehto ei päde, rivi ohitetaan, mutta aika kuluu silti.
- **Kello:** pelin aika on pysähdyksissä etualan kohtauksen ajan. `say` ja `wait confirm` pysäyttävät kohtauksen kellon. Tweenin loppu ei pysäytä sitä, joten jos haluat odottaa tweenin loppuun, kirjoita seuraavan rivin alkuajaksi tweenin loppuaika.
- **Sisennys on rakennetta:** `track`- ja `when`-lohkojen sisältö on neljä välilyöntiä syvemmällä.

Lisää: [RS 9.1–9.5](09-kohtaukset-ja-aikajanat.md#91-rivi).

## 5. Luonnos, ohitus ja tallennus

- Etualan kohtaus kirjoittaa tilan muutokset **luonnokseen**, joka näkyy ruudulla. Luonnos kirjataan maailmaan vasta, kun kohtaus päättyy.
- Pelaajalla on kohtauksen aikana (konteksti `SCENE`) kolme toimintoa:
  - `skip` tekee loput tilan muutokset heti, kirjaa `say`-repliikit lokiin ja jättää esityksen (`pan`, `shake`, `fade`…) väliin. Ohitus pysähtyy `choice`- ja `open`-kohtiin. `always`-rivi suoritetaan ohitettaessakin.
  - `restart` aloittaa kohtauksen alusta.
  - `rewind` palaa edelliseen `say`-kohtaan tai valintaan.
- Ohitettu kohtaus päätyy samaan tilaan kuin loppuun katsottu, koska esityksellä on oma satunnaisvirtansa.
- Kohtauksen, dialogin tai `open`-valikon aikana pelaaja ei voi tallentaa. Ladattaessa kohtaus alkaa alusta.

Lisää: [RS 9.8](09-kohtaukset-ja-aikajanat.md#98-luonnos-ohitus-ja-alusta-aloitus).

## 6. Taustakohtaukset

```roguescript
scene PATROL(guard: entity) timeline background:
    track ROUTE loop 12:                 // paljas luku = kierroksia
        00 02 move(guard, to: label(POST_A))
        05 02 move(guard, to: label(POST_B))
        10    log(GUARD.ALL_CLEAR, speaker: guard)

after begin() as START_PATROL:
    s = entities(in: label(SOLDIER_POST)).first
    start(PATROL(s), scope: map, bind: s)
```

|  | `background` | `background realtime` |
| --- | --- | --- |
| Kello | kierrokset | `ms`/`s` |
| Tilan muutokset ja `log` | sallittu | latausvirhe |
| `say` | sallittu | latausvirhe |
| Etualan kohtauksen aikana | pysähtyy | jatkaa |

- `start` palauttaa kahvan, ja `stop(kahva)` pysäyttää kohtauksen.
- `scope: map` päättää kohtauksen, kun pelaaja poistuu kartalta.
- `bind: e` päättää kohtauksen, kun entiteetti `e` tuhoutuu. Jos arvo on kokoelma, riittää että yksi sen jäsen tuhoutuu.

Lisää: [RS 9.9](09-kohtaukset-ja-aikajanat.md#99-taustakohtaukset).

## 7. `say` vai `log`

|  | Näkyy | Sallittu |
| --- | --- | --- |
| `say` | modaalinen ikkuna, odottaa kuittausta | korutiini, etualan aikajana, pelin ajan taustakohtaus |
| `log` | rivi tapahtumalokissa | kaikkialla paitsi `realtime`-kohtauksessa |

`say` käsittelijässä tai funktiossa on latausvirhe. Jos käsittelijän pitää näyttää ikkuna, se käynnistää kohtauksen kutsulla `start`.

## 8. Valmis esimerkki

[`esimerkit/kohtaus/`](../esimerkit/kohtaus/) on Ozin tarina, ja siinä on mukana lähes kaikki yllä kuvattu:

- tekstin kysely (`GUARDIAN`)
- dialogi ehdoilla (`OZ.TALK`)
- aiemman valinnan avaama kohta (`SCARECROW.TALK`)
- aikajana merkkeineen (`SCREEN_FALLS`)
- taustapartio (`PATROL`)

Koodi on tiedostossa [`esimerkit/kohtaus/logic/kohtaukset.rlml`](../esimerkit/kohtaus/logic/kohtaukset.rlml). Siitä puuttuvat `adjust`-kohta, kauppa ja äänet.

Kaksi asiaa on vielä tekemättä: sujuva liuku graafisessa esitystavassa (`smooth`) ja äänitiedostot. Lisäksi `rewind` aikajanalla voidaan siirtää omaksi vaiheekseen, jos se osoittautuu raskaaksi ([toteutusohje](../RLML-kohtaukset.md#ei-tässä-vaiheessa)).

---

[Sisällys](README.md) · [9. Kohtaukset ja aikajanat](09-kohtaukset-ja-aikajanat.md)
