[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 10. Generointi

Generointialgoritmit ovat STD:tä, ja niiden koodi on tässä luvussa. Ne käyttävät moottorin primitiivejä (`carve`, `tile`, `region`, `footprint`, `stamp`) ja pelin satunnaisvirtaa.

| Funktio | Tekee |
| --- | --- |
| `SCATTER_TILES(t, share, area)` | muuttaa ruudut tileksi `t` todennäköisyydellä `share` |
| `SMOOTH(floor, wall, area)` | yksi solukkoautomaatin kierros |
| `ROOMS(count, width, height, area) -> list area` | huoneet, jotka eivät mene päällekkäin; ei kaiva |
| `CONNECT(floor, with)` | yhdistää lattia-alueet L-käytävillä |
| `PLACE(p, area) -> area` | painaa prefabin kohtaan, johon se mahtuu |
| `FLOOR_AROUND(p, floor)`, `FITS(p, spot, area)` | apukyselyt |

Koodi on normatiivinen: toinen toteutus saa kirjoittaa sen toisin, mutta sen on käytettävä satunnaisvirtaa samassa järjestyksessä (luku [1.1](01-rajaus-periaatteet-ja-lataus.md#11-mikä-std-on)). Entinen `NOISE` on nyt `SCATTER_TILES`, jotta se ei sekoitu moottorin äänifunktioon `noise`. Joukon parametri on nimeltään `area`, koska `in` on varattu sana (RogueScript, luku [2.4](../roguescript/02-yleinen-syntaksi.md#24-varatut-sanat-ja-paikalliset-nimet)).

```roguescript
namespace STD

fn SCATTER_TILES(t: name, share: percent, area: area = tiles()):
    for p in area:
        if chance(share):
            carve(p, t)

query FLOOR_AROUND(p: pos, floor: name) -> number:
    return ((near(p, 1) - p) and at(floor)).count

fn SMOOTH(floor: name, wall: name, area: area = tiles()):
    cells = area and (at(floor) or at(wall))
    grow = cells where FLOOR_AROUND(it, floor) <= 3
    shrink = cells where FLOOR_AROUND(it, floor) >= 5
    carve(grow, wall)
    carve(shrink, floor)

fn ROOMS(count: range, width: range, height: range, area: area = tiles()) -> list area:
    rooms = []
    taken = area - area
    want = choose(count)
    tries = 0
    while rooms.count < want:
        tries += 1
        if tries > want * 20:
            break
        room = rect(area.choose(), choose(width), choose(height))
        if (room - area).count == 0 and (near(room, 1) and taken).count == 0:
            rooms += room
            taken = taken or room
    return rooms

fn CONNECT(floor: name, with: name = floor):
    cells = at(floor)
    if cells.count == 0:
        return
    joined = region(cells.first, of: at(floor) or at(with))
    for p in cells:
        if not p in joined:
            goal = nearest(joined, p)
            bend = pos(goal.x, p.y)
            carve(line(p, bend) or line(bend, goal), with)
            joined = region(goal, of: at(floor) or at(with))

query FITS(p: name, spot: pos, area: area) -> bool:
    shape = footprint(p, at: spot)
    if shape == none:
        return false
    return (shape - area).count == 0

fn PLACE(p: name, area: area = tiles()) -> area:
    spots = area where FITS(p, it, area)
    if spots.count == 0:
        return none
    spot = spots.choose()
    stamp(p, at: spot)
    return footprint(p, at: spot)
```

`SMOOTH` laskee kaikki muutokset ennen kuin kirjoittaa ne, joten ruudut päivittyvät samanaikaisesti. `ROOMS` yrittää enintään 20 kertaa huonetta kohden, joten täysi kartta tuottaa vähemmän huoneita eikä jumita. Se palauttaa listan alueita (RogueScript, luku [2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)), jonka voi tallentaa kartan muuttujaan. `CONNECT` laskee yhdistetyn alueen uudelleen jokaisen käytävän jälkeen, koska käytävä voi liittää samalla muitakin alueita.

---

← [9. Äänet, viestit ja tekstit](09-aanet-viestit-ja-tekstit.md) · [Sisällys](README.md) · [11. Korvaaminen](11-korvaaminen.md) →
