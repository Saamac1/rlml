[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 12. Minipeli STD:llä

Valmiustestin minipeli kirjoitettuna tätä sopimusta vasten. Pelissä ei ole yhtään RogueScript-riviä. Alla on pelin data yhtenä lohkona. Koko peli karttoineen on kansiossa [`esimerkit/mini/`](../esimerkit/mini/), jossa sama data on jaettu tiedostoihin [`game.rlml`](../esimerkit/mini/game.rlml), [`maps/caves.rlml`](../esimerkit/mini/maps/caves.rlml), [`ui/main.rlml`](../esimerkit/mini/ui/main.rlml), [`style/ascii.rlml`](../esimerkit/mini/style/ascii.rlml) ja [`text/fi.rlml`](../esimerkit/mini/text/fi.rlml).

```
game MINI
- start CAVE_A
- at SPAWN
- keys GAME
- layout MAIN
- layout PHONE when portrait and width < 70
- walk STD.MOVE
- PLAYER HERO

include lib/std.rlml

settings
- style ascii
- language fi
- round 250ms

tiles
- WALL blocks move sight sound
- default blocks none lit

entity HERO extends STD.DEFAULT.PLAYER
- render GLYPH.HERO

entity RAT
- HP now 3
- MELEE 1d2
- blocks move
- ALIGNMENT -100
- sight radius 6
- hearing
- ai behavior HUNTER
- render GLYPH.RAT

entity STAIRS_DOWN
- EXIT to CAVE_B

entity STAIRS_UP
- EXIT to CAVE_A

keys ALWAYS
- open PAUSE esc

keys GAME
- MOVE.NORTH k up
- MOVE.SOUTH j down
- MOVE.WEST h left
- MOVE.EAST l right
- WAIT s
- TAKE_EXIT > <
- pass z

menu PAUSE
- RESUME
- SAVE save
- QUIT

log LOG history 50
- STD.MESSAGE.COMBAT
- STD.MESSAGE.SENSE

layers
- GROUND map
- THINGS entities where it has render
- FOG fog
```

Asettelut, kosketusnapit (`panel PAD`), kartat `CAVE_A` ja `CAVE_B` sekä `tiles.glyphs ascii` ovat esimerkkikansiossa, ja kielitiedostossa on `HERO.NAME`, `RAT.NAME`, tauon ja nappien tekstit. Peli alkaa nimiöstä `SPAWN`, ja kummankin luolan `START` on vastakkaisten portaiden ruudussa, joten paluu tuo pelaajan portaille. Kohdat, jotka testissä jäivät auki, kulkevat nyt näin:

- **Liike:** `MOVE.EAST` palauttaa `STEP`-komennon, joka kysyy eston ytimillä `CAN_STEP` ja `BLOCKER` ja siirtää ytimellä `DO_STEP` (luku [5](05-komennot.md)).
- **Lyönti:** rotan suhde pelaajaan on `50 - |-100 - 0|` = −50, joten törmäys palauttaa `ATTACK`-komennon ja `HUNTER` hyökkää näkemäänsä tai kuulemaansa pelaajaan (luvut [6](06-tekoaly.md) ja [7](07-suhteet-ja-vihamielisyys.md)).
- **Kuolema:** `APPLY_DAMAGE` ja `DEATH` tuhoavat kuolleen, myös pelaajan, ja maailma jatkaa `round 250ms` -tahdilla (luku [8](08-kasittelijat.md)).
- **Valaistus:** `- default blocks none lit` valaisee koko luolan, joten pelaaja ja rotat näkevät koko näkösäteensä (`sight radius`). Ilman sitä mikään ruutu ei ole valaistu, ja jokainen näkee vain viereiset ruudut (moottoridokumentti, luku [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen)).
- **Kosketusnapit:** pystyssä olevassa puhelimessa (`PHONE`) lokin alla ovat napit `button`-kenttinä (dataformaatit, luku [11.5](../dataformaatit/11-kayttoliittyman-data.md#115-paneelit)). Ne tekevät samat toiminnot kuin näppäimet: suunnat, `WAIT`, `pass`, `open PAUSE` ja `TAKE_EXIT`. Portaat-nappi näkyy aina, koska `TAKE_EXIT` epäonnistuu muualla eikä vie aikaa. Kartta on keskellä nappien yläpuolella (`- m map size 20x8 center`), ja sitä napauttamalla kävellään ruutuun (`walk STD.MOVE`).
- **Kartanvaihto:** `TAKE_EXIT` kutsuu `travel`-funktiota, siirto tehdään toiminnon lopussa ja pelaaja saapuu kohdekartan `START`-nimiöön (luku [5](05-komennot.md)).

---

← [11. Korvaaminen](11-korvaaminen.md) · [Sisällys](README.md) · [13. Muutokset muihin dokumentteihin](13-muutokset-muihin-dokumentteihin.md) →
