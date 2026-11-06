[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 6. Tekoäly

STD:n tekoäly `HUNTER` on tavallinen `ai`-funktio, joka palauttaa komennon (RogueScript, luku [7.2](../roguescript/07-komennot-ja-tekoaly.md#72-tekoäly)). Entiteetti saa sen rivillä `- ai behavior HUNTER`. Pelin oma tekoäly voi kerrostaa sen kutsumalla `HUNTER()`, joka palauttaa metsästäjän komennon samalle `self`-arvolle (RogueScript, luku [7.2](../roguescript/07-komennot-ja-tekoaly.md#72-tekoäly)).

```roguescript
namespace STD

query HOSTILES(of: entity) -> collection:
    if of has no sight:
        return []
    return entities(in: near(of, of.sight.radius)) where MAY_ATTACK(of, it) and sees(of, it)

command APPROACH(target: area) cost 120:
    route = path(actor, target, for: actor)
    if route.count == 0:
        return false
    return STEP(route.first)

command FLEE(threat: area) cost 120:
    spots = adjacent(actor) where distance(it, threat) > distance(actor, threat) and not blocks(it, move, for: actor)
    if spots.count == 0:
        return false
    return STEP(spots.choose())

command WANDER() cost 120:
    spots = adjacent(actor) where not blocks(it, move, for: actor)
    if spots.count == 0:
        return false
    return STEP(spots.choose())

ai HUNTER:
    foe = nearest(HOSTILES(self), self)
    if foe != none:
        if self has HP and self.HP.now < self.HP.max / 4:
            return FLEE(foe)
        near_foe = foe.at in adjacent(self)
        if near_foe and self has MELEE:
            return ATTACK(foe)
        if CAN_SHOOT(self, foe):
            return SHOOT(foe.at)
        if near_foe:
            return FLEE(foe)
        return APPROACH(foe)
    noises = heard(self) where it.source != none and MAY_ATTACK(self, it.source)
    if noises.count > 0:
        return APPROACH(noises.first.at)
    return WANDER()
```

- **Näkö ensin, sitten kuulo.** Metsästäjä hyökkää näkemäänsä vihamieliseen. Jos se ei näe ketään, se lähtee kovimman kuulemansa vihamielisen äänen lähteelle (moottoridokumentti, luku [6.8](../moottori/06-ajo.md#68-äänet-ja-kuuleminen)). Ilman `sight`-komponenttia se toimii pelkän kuulon varassa.
- **Pako.** Alle neljänneksen elinvoimalla metsästäjä pakenee näkemäänsä vihollista.
- **Liikkeet** ovat jatkokomentoja, jotka palauttavat `STEP`-komennon, joten niiden hinta on askeleen hinta ja törmäys toimii samoin kuin pelaajalla.
- **Reitti.** `path` ei sisällä lähtöruutua (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)), joten `route.first` on seuraava askel. Reitti noudattaa liikkujan suuntia ja kulmasääntöä. **Naapurusto** on kaikissa liikkeissä `adjacent`, joten lyönti, pako ja vaellus käyttävät samoja suuntia kuin askel.

## 6.1 Seuraaja

Seuraaja kulkee johtajansa perässä ja puolustautuu viereiseltä viholliselta. Se sopii saatettavalle, kuten pelastetulle prinsessalle, ja seuralaiselle. Entiteetti saa sen rivillä `- ai behavior FOLLOWER` tai ajossa `add(e, ai(behavior: FOLLOWER))`.

| Komponentti | Kentät | Merkitys |
| --- | --- | --- |
| `FOLLOW` | `target: entity = none`, `distance: number = 1` | johtaja ja etäisyys, jolla seuraaja pysähtyy; ilman johtajaa johtaja on lähin istuimen jäsen |

```roguescript
namespace STD

component FOLLOW(target: entity = none, distance: number = 1)

query LEADER_OF(e: entity) -> entity:
    if e has FOLLOW and e.FOLLOW.target != none:
        return e.FOLLOW.target
    return nearest(players where it != e, e)

ai FOLLOWER:
    leader = LEADER_OF(self)
    if leader == none or leader.at == none or leader.map != self.map:
        return WANDER()
    foe = nearest(HOSTILES(self), self)
    if foe != none and self has MELEE and foe.at in adjacent(self):
        return ATTACK(foe)
    stop = 1
    if self has FOLLOW:
        stop = self.FOLLOW.distance
    if distance(self, leader) <= stop:
        return WAIT()
    return APPROACH(leader)
```

- **Lähin jäsen.** Ilman `FOLLOW.target`-kenttää johtaja on lähin muu istuimen jäsen (`players`, moottoridokumentti luku [6.1.10](../moottori/06-ajo.md#6110-istuimet-ja-ryhmä)), joten ryhmän jäsen seuraa toista jäsentä eikä itseään. Yksinpelissä se on pelaaja. Moninpelissä `player` vaihtuisi joka vuoro.
- **Poistuva istuin.** `PART_PLAYER` tekee poistuvan istuimen jäsenistä seuraajia, joten hahmot jäävät ryhmään:

```roguescript
on part(seat) as PART_PLAYER:
    for e in members(seat):
        add(e, ai(behavior: FOLLOWER))
        if e has no FOLLOW:
            add(e, FOLLOW())
```

- **Johtaja poissa.** Kuollut johtaja tai johtaja toisella kartalla jättää seuraajan vaeltamaan. Kartalta toiselle seuraaja siirtyy logiikan `travel`-kutsulla tai jäsenen mukana, kun jäsenet käyttävät uloskäyntiä ryhmänä (`TAKE_EXIT`, luku [5](05-komennot.md)).
- **Paikanvaihto.** Pelaajan ja seuraajan suhde on tyypillisesti ystävällinen, joten `STEP` vaihtaa niiden paikkaa törmäyksessä (luku [5](05-komennot.md)).

---

← [5. Komennot](05-komennot.md) · [Sisällys](README.md) · [7. Suhteet ja vihamielisyys](07-suhteet-ja-vihamielisyys.md) →
