[RLML](../README.md) › [RogueScript](README.md)

# 8. Generointi

## 8.1 Generaattorit

Generaattori on funktio, joka rakentaa kartan moottorin primitiiveillä ja vakiokirjaston algoritmeilla (luku [8.3](#83-vakiokirjaston-generointi)). Se käyttää pelin satunnaisvirtaa, joten sama siemen tuottaa saman kartan.

```roguescript
generator CAVE(depth: number) size 80x40:
    map.DEPTH = depth
    carve(tiles(), WALL)
    SCATTER_TILES(FLOOR, 45%)
    repeat 4:
        SMOOTH(floor: FLOOR, wall: WALL)
    for room in ROOMS(count: 6..10, width: 4..9, height: 4..7):
        carve(room, FLOOR)
    if depth >= 3 and chance(30%):
        PLACE(VAULT, area: at(WALL))
    CONNECT(floor: FLOOR)
    entry = at(FLOOR).choose()
    tile(entry, STAIR.up, label: START)
    stairs = (at(FLOOR) - near(entry, 10)).choose()
    tile(stairs, STAIR.down, label: STAIRS_DOWN)
    scatter(SPAWN.CAVE, count: 5..8, in: at(FLOOR) - near(label(START), 10))
```

- `tile(paikka, TILE, layer: KERROS, label: NIMIÖ)` asettaa ruudun tilen. Ilman `layer`-argumenttia `tile` menee oletuskerrokseen, ja `none` tyhjentää kerroksen.
- **Saapumiskohdat.** Generoidulla kartalla ei ole legendiä, joten generaattori nimeää saapumiskohdat itse: `START` ja muut nimiöt, joihin `travel` vie (`tile(…, label:)` tai `tag`, luku [6.4](06-ruudut-ja-paikkahaku.md#64-siirtyminen-kartalta-toiselle)).
- `PLACE(PREFAB, area: …)` (vakiokirjasto, luku [10](../vakiokirjasto/10-generointi.md)) painaa prefabin karttaan kohtaan, jossa sen kaikki piirretyt ruudut mahtuvat joukkoon. Sijoitusehto on generaattorin logiikkaa: prefab kuvaa datassa vain muotonsa, ja generaattori päättää, sijoitetaanko se (`if depth >= 3 and chance(30%):` yllä). `PLACE` tulee ennen `CONNECT`-kutsua, jotta prefab liittyy käytäviin.
- Kartan muuttujat (`map.DEPTH`) asetetaan generaattorissa. Huoneluettelon voi tallentaa kartan muuttujaan listana (`map.ROOMS = ROOMS(…)`), jolloin logiikka löytää huoneet myöhemmin.
- **Entiteetit.** Generaattori voi synnyttää entiteettejä generoitavalle kartalle (`spawn`, `scatter`) ja kirjoittaa niiden kenttiä, koska ne kuuluvat karttaan. Muiden karttojen entiteetit ja `game`-tila ovat sille vain luettavia (moottoridokumentti, luku [6.5](../moottori/06-ajo.md#65-generointi)).

## 8.2 Generoitu kartta

Generoitu kartta määritellään RogueScriptissä yhdellä rivillä. Se on samaa lajia kuin datan piirretty kartta, joten siihen viitataan samoin (`game`-lohkon `start`, `travel`).

```roguescript
map LEVEL.CAVE generate CAVE(depth: 3)
```

**Pysyvyys.** Kartta säilyy vierailujen välillä: paluu löytää sen sellaisena kuin se jäi. Sana `transient` ennen `generate`-sanaa tekee kartasta kertakäyttöisen, joka hylätään pelaajan lähtiessä ja generoidaan uudelleen seuraavalla saapumisella, esimerkiksi satunnaiselle kohtaamiskartalle:

```roguescript
map ENCOUNTER transient generate FIELD(depth: 1)
```

Argumentit ovat vakiolausekkeita, jotka lataus laskee. Prefabin parametrit käyttävät samaa lausekekieltä (avoin kysymys, luku [13](13-avoimet-kysymykset.md)).

**Pelin tila generaattorissa.** Generaattori näkee `game`-tilan sillä hetkellä, kun kartta luodaan. Tilasta riippuva taso luetaan siksi generaattorin rungossa, ja argumentit pysyvät vakioina. Rogue-tyyppinen luolasto on yksi `transient`-kartta, jonka generaattori lukee syvyyden `game`-tilasta ja jonka `travel` luo uudelleen jokaisella siirtymällä (luku [6.4](06-ruudut-ja-paikkahaku.md#64-siirtyminen-kartalta-toiselle)):

```roguescript
map DUNGEON transient generate ROGUE_LEVEL()

generator ROGUE_LEVEL() size 80x21:
    depth = game.DEPTH.now
    carve(tiles(), ROCK)
    // …
```

## 8.3 Vakiokirjaston generointi

Generointialgoritmit on kirjoitettu RogueScriptillä vakiokirjastoon. Moottori tarjoaa vain primitiivit: `carve`, `tile`, ruutujoukkofunktiot (luku [6](06-ruudut-ja-paikkahaku.md)), `region`, `footprint`, `stamp` ja `external` (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)). Koska kirjaston lähdekoodi määrää algoritmin ja satunnaisvirran käytön, sama siemen tuottaa saman kartan kaikissa toteutuksissa. Peli voi korvata algoritmin (`override`, luku [11.2](11-tallennus-ja-korvaaminen.md#112-korvaaminen)) tai kirjoittaa oman.

| Funktio | Merkitys |
| --- | --- |
| `SCATTER_TILES(t, share, area: area = tiles())` | jokainen ruutu muuttuu `t`:ksi todennäköisyydellä `share`, lukujärjestyksessä pelin virrasta |
| `SMOOTH(floor, wall, area: area = tiles())` | yksi solukkoautomaatin kierros: ruutu on `wall`, jos 8 naapurista enintään 3 on `floor`, ja `floor`, jos vähintään 5; muuten se pysyy. Kartan ulkopuoli ja muut tilet lasketaan seinäksi. |
| `ROOMS(count, width, height, area: area = tiles())` | suorakaiteet, jotka eivät mene päällekkäin ja joiden välissä on vähintään yhden ruudun rako; palauttaa listan `list area` mutta ei kaiva |
| `CONNECT(floor, with: name = floor)` | yhdistää `floor`-alueet L-muotoisilla käytävillä, jotka kaivetaan `with`-tilellä; alueet käsitellään lukujärjestyksessä |
| `PLACE(p, area: area = tiles()) -> area` | arpoo kohdan, jossa prefabin piirretyt ruudut mahtuvat joukkoon, ja painaa sen; palauttaa peitetyn alueen tai `none` |

Joukon parametri on nimeltään `area`, koska `in` on varattu sana eikä kelpaa paikalliseksi nimeksi (luku [2.4](02-yleinen-syntaksi.md#24-varatut-sanat-ja-paikalliset-nimet)).

Funktioiden koodi on vakiokirjaston luvussa [10](../vakiokirjasto/10-generointi.md), joka on niiden normatiivinen lähde. Sama koodi määrää satunnaisvirran käytön, joten sama siemen tuottaa saman kartan kaikissa toteutuksissa.

**Ulkoinen generaattori.** `external("tools/wfc", depth: depth)` ajaa pelin kansiossa olevan ohjelman ja painaa sen tuottaman kartan karttaan kuten `stamp`. Ohjelma tulostaa samanlaisen karttatiedoston kuin piirretty kartta (otsikko `map NIMI`, piirros, `legend:` ja `vars:`, dataformaatit luku [8.1](../dataformaatit/08-kartat-ja-prefabit.md#81-karttatiedosto)), joten tulosteen voi tallentaa ja tarkistaa kuten minkä tahansa kartan. `external` palauttaa `false`, jos ohjelmaa ei voi ajaa, joten generaattori voi siirtyä RogueScript-varaversioon:

```roguescript
generator CAVE(depth: number) size 80x40:
    if not external("tools/wfc", depth: depth):
        carve(tiles(), WALL)
        SCATTER_TILES(FLOOR, 45%)
        repeat 4:
            SMOOTH(floor: FLOOR, wall: WALL)
        CONNECT(floor: FLOOR)
```

Ohjelman kutsu, lupa ja virheet kuvataan moottoridokumentin luvussa [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot).

---

← [7. Komennot ja tekoäly](07-komennot-ja-tekoaly.md) · [Sisällys](README.md) · [9. Kohtaukset ja aikajanat](09-kohtaukset-ja-aikajanat.md) →
