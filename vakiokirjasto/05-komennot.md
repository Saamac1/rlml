[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 5. Komennot

| Komento | Hinta | Tekee |
| --- | --- | --- |
| `STEP(to: pos)` | 120 | astuu viereiseen ruutuun tai törmää (alla) |
| `MOVE(dir: dir)` | `STEP`-komennon | `STEP(actor.at + dir)` |
| `MOVE.NORTH` … `MOVE.NORTHWEST` | `STEP`-komennon | sidotut suunnat näppäimille, kaikki kahdeksan |
| `WAIT()` | 120 | ei tee mitään |
| `SNEAK()` | 0 | hiipiminen päälle tai pois: askel on hiljainen (`SOUND.SNEAK`) mutta maksaa 240 |
| `FIGHT(dir: dir)`, `FIGHT.NORTH` … | 120 | lyö suunnassa olevaa olentoa tarkoituksella (luku [7](07-suhteet-ja-vihamielisyys.md)) |
| `FEED(item, target: pos)` | 120 | antaa ruokaa viereiselle olennolle ja parantaa sen suhdetta (luku [7](07-suhteet-ja-vihamielisyys.md)) |
| `ATTACK(target: entity)` | 120 | lyö viereistä kohdetta (`STRIKE`) |
| `TAKE_EXIT()` | 120 | käyttää ruudun uloskäyntiä |
| `PICK_UP(item)` | 120 | poimii esineen ruudusta reppuun |
| `DROP(item)` | 120 | pudottaa esineen repusta ruutuun |
| `APPROACH(target: area)`, `FLEE(threat: area)`, `WANDER()` | `STEP`-komennon | tekoälyn liikkeet (luku [6](06-tekoaly.md)) |

**Ytimet.** Komennon voi ajaa vain jatkokomentona rungon lopussa, joten liikkeen ja iskun säännöt ovat funktioissa, joita komennot ja pelin omat silmukat (juoksu, taistelu kuolemaan asti) kutsuvat. Ytimet eivät kuluta aikaa: hinta ja `wait` kuuluvat kutsuvalle komennolle.

| Ydin | Laji | Tekee |
| --- | --- | --- |
| `CAN_STEP(who: entity, to: pos) -> bool` | `query` | `to` on askeleen päässä (`adjacent`, suunnat ja kulmasääntö), eikä talutushihna estä sitä |
| `LEASHED(who: entity, to: pos) -> bool` | `query` | hihna estää ohjattavan askeleen, jos saman kartan jäsenten rajaava suorakulmio kasvaisi kartta-aluetta kahta ruutua pienempää suuremmaksi |
| `BLOCKER(who: entity, to: pos) -> entity` | `query` | ensimmäinen entiteetti, jonka oma `blocks` estää liikkeen ruudussa `to`, tai `none` |
| `DO_STEP(who: entity, to: pos)` | `fn` | siirtää `who`:n ja pitää askeleen äänen; ei tarkista estoa |
| `STRIKE(attacker: entity, target: entity) -> bool` | `fn` | lyö viereistä kohdetta ja pitää taistelun äänen; `false`, jos lyönti ei ole mahdollinen |

Peli, joka muuttaa iskua, korvaa ytimen `STD.STRIKE` eikä komentoa, jolloin muutos koskee myös törmäystä ja pelin omia silmukoita.

```roguescript
namespace STD

query STEP_SOUND(e: entity) -> name:
    if e has SNEAKING:
        return SOUND.SNEAK
    return SOUND.STEP

query CAN_STEP(who: entity, to: pos) -> bool:
    return to in adjacent(who) and not LEASHED(who, to)

query LEASHED(who: entity, to: pos) -> bool:
    if not game.leash:
        return false
    n = seat_of(who)
    if n == none or active(n) != who:
        return false
    group = players where it != who and it.map == who.map
    if group.count == 0:
        return false
    left = group.first.at.x
    right = left
    top = group.first.at.y
    bottom = top
    for e in group:
        left = min(left, e.at.x)
        right = max(right, e.at.x)
        top = min(top, e.at.y)
        bottom = max(bottom, e.at.y)
    // the box with the step and without it: only a step that widens a box
    // past the limit is refused, so a spread group can close up
    wide = max(right, to.x) - min(left, to.x) + 1
    high = max(bottom, to.y) - min(top, to.y) + 1
    was_wide = max(right, who.at.x) - min(left, who.at.x) + 1
    was_high = max(bottom, who.at.y) - min(top, who.at.y) + 1
    return (wide > view_width() - 2 and wide > was_wide) or (high > view_height() - 2 and high > was_high)

query BLOCKER(who: entity, to: pos) -> entity:
    return (entities(in: to) where it != who and blocks(it, move, tiles: false)).first

fn DO_STEP(who: entity, to: pos):
    move(who, to: to)
    noise(to, STEP_SOUND(who), source: who)

fn STRIKE(attacker: entity, target: entity) -> bool:
    if attacker has no MELEE or target has no HP or not target.at in adjacent(attacker):
        return false
    emit HIT(target, attacker, attacker.MELEE.damage)
    noise(attacker.at, SOUND.FIGHT, source: attacker)
    return true

command STEP(to: pos) cost 120:
    if actor has SNEAKING:
        wait 120
    if not CAN_STEP(actor, to):
        return false
    other = BLOCKER(actor, to)
    if other != none:
        emit BUMP(actor, other)
        if other has HP and MAY_ATTACK(actor, other):
            return ATTACK(other)
        if FRIENDS(actor, other) and (other has ai or seat_of(other) != none) and not blocks(actor.at, move, for: other, except: actor):
            move(other, to: actor.at)
            DO_STEP(actor, to)
            return
        return false
    if blocks(to, move, for: actor):
        return false
    DO_STEP(actor, to)

command MOVE(dir: dir) cost 120:
    return STEP(actor.at + dir)

command MOVE.NORTH = MOVE(north)
command MOVE.NORTHEAST = MOVE(northeast)
// … kaikki kahdeksan suuntaa

command WAIT() cost 120:
    return

command SNEAK() cost 0:
    if actor has SNEAKING:
        remove(actor, SNEAKING)
    else:
        add(actor, SNEAKING())

command FIGHT(dir: dir) cost 120:
    others = entities(in: actor.at + dir) where it != actor and it has HP
    if others.count == 0:
        return false
    other = others.first
    if ALIGNMENT_OF(actor) >= GOOD_AT and not ENEMIES(actor, other):
        return false
    return ATTACK(other)

command FIGHT.NORTH = FIGHT(north)
// … kaikki kahdeksan suuntaa

command FEED(item: entity from actor.PACK.items where it has FOOD, target: pos aim 1 at entities(in: adjacent(actor)) where it has HP) cost 120:
    eaters = entities(in: target) where it != actor and it has HP
    if eaters.count == 0 or not target in adjacent(actor):
        return false
    BEFRIEND(eaters.first, actor, item.FOOD.points, group: item.FOOD.group)
    destroy(item)

command ATTACK(target: entity) cost 120:
    return STRIKE(actor, target)

fn TRAVEL_GROUP(who: entity, door: entity):
    group = players where it.map == who.map
    tag = entities(in: near(door.at, 1)) where it has FOLLOW and seat_of(it) == none and LEADER_OF(it) in group
    for e in group:
        travel(e, door.EXIT.to, at: door.EXIT.at)
    for e in tag:
        travel(e, door.EXIT.to, at: door.EXIT.at)

scene EXIT.LEAVE_OTHERS(who: entity, door: entity):
    say(EXIT.LEAVE_OTHERS.ASK)
    choice:
        YES:
            TRAVEL_GROUP(who, door)
        NO:
            return

command TAKE_EXIT() cost 120:
    exit = (entities(in: actor.at) where it has EXIT).first
    if exit == none:
        return false
    if seat_of(actor) == none:
        travel(actor, exit.EXIT.to, at: exit.EXIT.at)
        return
    away = players where it.map == actor.map and distance(it, exit) > 1
    if away.count > 0:
        start(EXIT.LEAVE_OTHERS(actor, exit))
        return false
    TRAVEL_GROUP(actor, exit)

on arrive(who, map, p) as GROUP_ARRIVAL when seat_of(who) != none or who has FOLLOW:
    if (entities(in: p) where it != who and blocks(it, move, for: who, tiles: false)).count == 0:
        return
    spot = OPEN(near(p, 1..2), who)
    if spot.count > 0:
        move(who, to: spot)

fn SWITCH_BY(step: number) -> bool:
    n = seat_of(actor)
    if n == none:
        return false
    group = members(n)
    if group.count < 2:
        return false
    i = 0
    here = 0
    for e in group:
        if e == actor:
            here = i
        i = i + 1
    control(group[(here + step) % group.count])
    return true

command SWITCH() cost 0:
    return SWITCH_BY(1)

command SWITCH.PREV() cost 0:
    return SWITCH_BY(-1)

command PICK_UP(item: entity from entities(in: actor.at) where it has ITEM) cost 120:
    if actor has no PACK:
        return false
    if item has AMMO:
        same = (actor.PACK.items where it is item and it has AMMO).first
        if same != none:
            same.AMMO.count += item.AMMO.count
            destroy(item)
            emit PICKED_UP(actor, same)
            return
    move(item, to: actor.PACK.items)
    emit PICKED_UP(actor, item)

command DROP(item: entity from actor.PACK.items) cost 120:
    move(item, to: actor.at)
    emit DROPPED(actor, item)
```

- **Törmäys.** Estävä entiteetti on entiteetti, jonka oma `blocks`-komponentti estää liikkeen; `BLOCKER` kysyy sen kyselyllä `blocks(it, move, tiles: false)`, joka ei katso ruudun tilejä, joten pelkkä näköeste (savu) ei aiheuta törmäystä. Olento tarvitsee siksi rivin `- blocks move`, jotta siihen voi törmätä ja lyödä (moottoridokumentti, luvut [3.4](../moottori/03-pelin-tila.md#34-moottorin-komponentit) ja [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)). `BUMP` lähetetään aina. Kohteeseen, jota toimija saa lyödä (`MAY_ATTACK`, luku [7.1](07-suhteet-ja-vihamielisyys.md#71-viholliset-ja-ystävät)) ja jolla on `HP`, törmäys palauttaa jatkokomennon `ATTACK`, joten se maksaa hyökkäyksen hinnan. Muuten `STEP` palauttaa `false`, eikä törmäys vie aikaa. Ystävän (`FRIENDS`) kanssa, jolla on tekoäly tai joka on pelaaja, toimija vaihtaa paikkaa. Neutraaliin ei lyödä vahingossa: tarkoituksellinen lyönti on `FIGHT`, eikä hyvä toimija (`ALIGNMENT` ≥ `GOOD_AT`) lyö sillä muuta kuin vihollista.
- **Oletussuhde ja paikanvaihto.** Kahden entiteetin suhde ilman yksilö-, ryhmä- ja alignment-eroja on `TOLERANCE` = 50, ja `FRIENDLY` on myös 50, joten ne ovat ystäviä (luku [7.1](07-suhteet-ja-vihamielisyys.md#71-viholliset-ja-ystävät)). Paikanvaihdon rajaa siksi vain ehto, että toisella on tekoäly tai se on pelaaja: estävä esine tai muu tekoälytön entiteetti ei vaihda paikkaa, vaan törmäys lähettää `BUMP`-tapahtuman ja palauttaa `false`. Peli, joka käyttää törmäystä kosketuksena (robotfindskitten), nojaa tähän ehtoon (avoin kysymys, luku [14](14-avoimet-kysymykset.md)).
- **Seinä.** Tilen tai entiteetin esto ruudussa (`blocks(to, move, for: actor)`) estää liikkeen ilman tapahtumaa, ja `through`-joukon tilet eivät estä. Pelaajalta se ei vie vuoroa.
- **Ääni.** Askel pitää ääntä `SOUND.STEP` ja lyönti `SOUND.FIGHT`, joten paikallaan oleva ei kuulu (luku [9](09-aanet-viestit-ja-tekstit.md)). Äänet ovat ytimissä, joten pelin oma juoksu kuuluu samoin kuin askel.
- **Suunnat.** `CAN_STEP` hyväksyy vain ruudut, jotka moottorin `adjacent(who)` antaa. Oletus on kahdeksan suuntaa ilman seinän kulman ohitusta: vinoaskel on kielletty, jos jompikumpi sen sivuruuduista estää liikkeen, jotta vinot seinät eivät vuoda. Neljä suuntaa saa rivillä `- movement dirs 4` ja kulman ohituksen rivillä `- movement corners true` (moottoridokumentti, luku [3.4](../moottori/03-pelin-tila.md#34-moottorin-komponentit)). Reitinhaku noudattaa samoja kenttiä, joten tekoäly ei suunnittele askelta, jota `STEP` ei hyväksy.
- **Hiipiminen** maksaa lisäodotuksen `STEP`-komennossa eikä ytimessä. Pelin oma silmukka, joka kutsuu `DO_STEP`-funktiota, päättää hiipijän tahdin itse.
- **Talutushihna.** Kun `game`-lohkon `leash` on päällä (oletus, kun istuimia voi olla useampi, dataformaatit luku [3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)), `CAN_STEP` epää ohjattavan askeleen, joka hajottaisi ryhmän kartta-alueen ulkopuolelle. Kartta-alueen koko on asettelussa kirjoitettu solukoko (`- m map size 40x15`, moottoridokumentti luku [7.1](../moottori/07-esitys.md#71-asettelu)), joten peli, joka haluaa hihnan, antaa kartta-alueelle koon. Hihna epää vain askeleen, joka levittää suorakulmion rajan yli, joten jo hajonnut ryhmä voi aina palata yhteen. Epäonnistunut askel ei vie aikaa. Hihna koskee vain istuimen ohjattavaa: tekoälyn ohjaamat jäsenet, seuraajat ja hirviöt kulkevat vapaasti, joten tekoäly ei suunnittele askelta, jota `STEP` ei hyväksy. Kiertävää karttaa hihna ei vielä huomioi.
- **Paikanvaihdon esto.** `blocks(actor.at, move, for: other, except: actor)` kysyy, voiko ystävä astua törmääjän ruutuun. `except` jättää törmääjän itsensä pois, koska se on lähdössä ruudusta. Ilman sitä törmääjän oma `blocks move` estäisi jokaisen vaihdon.
- **Paikanvaihto jäsenen kanssa.** `STEP` vaihtaa paikkaa ystävän kanssa, jolla on tekoäly tai joka on istuimen jäsen (`seat_of(other) != none`), eikä vain `player`-hahmon kanssa.
- **Ryhmä uloskäynnillä.** Kun saman kartan jäsenet ovat uloskäynnin ruudussa tai sen vieressä, `TAKE_EXIT` siirtää koko ryhmän ja jäsenten seuraajat, jotka ovat uloskäynnin vieressä. Muuten komento epäonnistuu ilman ajan kulumista ja käynnistää dialogin `EXIT.LEAVE_OTHERS`, jonka `YES` siirtää ryhmän. Vahvistettu siirto ei maksa komennon hintaa, koska dialogi alkaa vasta toiminnon lopussa. `GROUP_ARRIVAL` siirtää kohdekartalla saapujan vapaaseen ruutuun, jos saapumisruudussa on jo estävä entiteetti.
- **Ryhmän vaihto.** `SWITCH` ja `SWITCH.PREV` (hinta 0) tekevät istuimen seuraavasta tai edellisestä elävästä jäsenestä ohjattavan, ja ne epäonnistuvat, jos muita jäseniä ei ole. Vaihto ei anna ylimääräisiä vuoroja (moottoridokumentti, luku [6.1.10](../moottori/06-ajo.md#6110-istuimet-ja-ryhmä)). STD ei sido näppäimiä, joten peli sitoo komennot omaan peruskontekstiinsa (esimerkiksi `- SWITCH tab` ja `- SWITCH.PREV shift+tab`).
- **`TAKE_EXIT`** käyttää ruudun ensimmäistä uloskäyntiä. Siirto tehdään toiminnon lopussa, ja saapumisruudussa ei lähetetä `enter`-tapahtumaa (moottoridokumentti, luku [6.7](../moottori/06-ajo.md#67-siirtyminen-kartalta-toiselle)).
- **`PICK_UP`** siirtää esineen kokoelmaan kutsulla `move(item, to: kokoelma)` (moottoridokumentti, luku [9.3](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).

Esimerkki pelin omasta silmukasta, joka noudattaa STD:n sääntöjä (Rogue-toteutustesti):

```roguescript
command RUN(dir: dir) cost 0:
    steps = 0
    while true:
        to = actor.at + dir
        if not STD.CAN_STEP(actor, to) or STD.BLOCKER(actor, to) != none or blocks(to, move, for: actor):
            break
        STD.DO_STEP(actor, to)
        steps += 1
        wait 120
        if RUN_STOPS(actor):
            break
    if steps == 0:
        return false
```

Komento on itse maksuton (`cost 0`), ja jokainen askel kuluttaa aikansa `wait`-kutsulla, joten epäonnistunut ensimmäinen askel ei vie aikaa.

## 5.1 Kaukoaseet

Kaukoase on ampujan komponentti `RANGED`, kuten lähitaistelun `MELEE`. Ammuksella on kaksi lentotapaa:

- **Suora** (oletus): ammus lentää linjaa pitkin kantaman loppuun. Jokainen olento matkalla on mahdollinen osuma, myös oma, ja ensimmäinen osuma pysäyttää ammuksen. Ohi mennyt ammus jatkaa ja putoaa linjan loppuun.
- **Kaari** (`lob true`): ammus lentää olentojen yli kohderuutuun, ja vain kohderuudun olento voi tulla osutuksi.

Seinä ja muu este pysäyttävät molemmat. Ammuksen esto on liikkumistapa `MISSILE`: `- WINDOW blocks move MISSILE` pysäyttää kävelijän ja nuolen mutta ei näköä.

**Osuma.** Ampuja heittää `1d20 + MARKSMANSHIP` ja kohde pelastusheiton `1d20 + EVASION`, tässä järjestyksessä. Ammus osuu, jos ampujan tulos on suurempi; tasapeli menee ohi. Lähitaistelu osuu toistaiseksi aina.

| Komponentti | Kentät | Merkitys |
| --- | --- | --- |
| `RANGED` | `range: number`, `damage: dice`, `lob: bool = false`, `ammo: name = none`, `projectile: name = GLYPH.MISSILE` | ampujan kaukoase; `ammo` on ammusentiteetin nimi |
| `MARKSMANSHIP` | `value: number = 0` | ampujan taito |
| `EVASION` | `value: number = 0` | kohteen pelastusheiton lisä |
| `THROWN` | `damage: dice` | heitetyn esineen vahinko |
| `AMMO` | `count: number = 1` | ammuspino yhtenä entiteettinä; `SHOOT` vähentää määrää, ja laskeutunut ammus yhdistyy ruudun pinoon |

**Ammukset.** Nuolipino on yksi entiteetti, jolla on `AMMO count`. Aloitusvarusteen kortti `- ARROWS AMMO 5` antaa viisi nuolta. `SHOOT` vähentää määrää yhdellä ja tuhoaa pinon nollassa. Laskeutunut nuoli kasvattaa ruudun pinoa tai luo uuden pinon. `PICK_UP` yhdistää poimitun pinon reppuun jo olevaan samaan ammukseen, joten erillistä yhdistämissääntöä ei tarvita. Peli voi käyttää ammuksille myös kokoelman pinoja, joissa jokainen nuoli on oma ilmentymänsä (dataformaatit, luku [5.4](../dataformaatit/05-komponentit-ja-entiteetit.md#54-kokoelmat)); STD:n oletus on `AMMO`, koska se pitää pinon yhtenä ilmentymänä.

```roguescript
namespace STD

event MISSED(target: entity, attacker: entity)

fn HITS(shooter: entity, target: entity) -> bool:
    attack = 1d20
    if shooter has MARKSMANSHIP:
        attack += shooter.MARKSMANSHIP.value
    save = 1d20
    if target has EVASION:
        save += target.EVASION.value
    return attack > save

query ROUTE(shooter: entity, target: pos, range: number, lob: bool) -> area:
    if lob:
        return ray(shooter.at, target, for: shooter, mode: MISSILE)
    return ray(shooter.at, target, for: shooter, range: range, mode: MISSILE)

fn STRIKE_LINE(shooter: entity, route: area, lob: bool) -> entity:
    candidates = route
    if lob and route.count > 0:
        candidates = route where it == route.last
    for p in candidates:
        victim = (entities(in: p) where it has HP and it != shooter).first
        if victim != none:
            if HITS(shooter, victim):
                return victim
            emit MISSED(victim, shooter)
    return none

query LANDING(shooter: entity, route: area, victim: entity) -> pos:
    if victim != none:
        return victim.at
    if route.count > 0:
        return route.last
    return shooter.at

command SHOOT(target: pos aim actor.RANGED.range at HOSTILES(actor)) cost 120:
    if actor has no RANGED:
        return false
    w = actor.RANGED
    ammo = none
    if w.ammo != none:
        if actor has no PACK:
            return false
        ammo = (actor.PACK.items where it is w.ammo and it has AMMO and it.AMMO.count > 0).first
        if ammo == none:
            return false
    noise(actor.at, SOUND.SHOT, source: actor)
    route = ROUTE(actor, target, w.range, w.lob)
    victim = STRIKE_LINE(actor, route, w.lob)
    land = LANDING(actor, route, victim)
    fly(w.projectile, line(actor.at, land) - actor.at, speed: 30ms)
    if ammo != none:
        ammo.AMMO.count -= 1
        pile = (entities(in: land) where it is w.ammo and it has AMMO).first
        if pile != none:
            pile.AMMO.count += 1
        else:
            spawn(w.ammo, at: land)
        if ammo.AMMO.count <= 0:
            destroy(ammo)
    if victim != none:
        emit HIT(victim, actor, w.damage)

command THROW(item: entity from actor.PACK.items, target: pos aim 6 at HOSTILES(actor)) cost 120:
    route = ROUTE(actor, target, 6, true)
    victim = none
    if item has THROWN:
        victim = STRIKE_LINE(actor, route, true)
    land = LANDING(actor, route, victim)
    fly(item.render.tile, line(actor.at, land) - actor.at, speed: 30ms)
    move(item, to: land)
    if victim != none:
        emit HIT(victim, actor, item.THROWN.damage)

query CAN_SHOOT(shooter: entity, foe: entity) -> bool:
    if shooter has no RANGED or distance(shooter, foe) > shooter.RANGED.range:
        return false
    w = shooter.RANGED
    if w.ammo != none and (shooter has no PACK or (shooter.PACK.items where it is w.ammo and it has AMMO and it.AMMO.count > 0).count == 0):
        return false
    return foe.at in ROUTE(shooter, foe.at, w.range, w.lob)

after MISSED(target, attacker) as MISS_MESSAGE priority -10 when WITNESSED(target):
    log(MESSAGE.COMBAT.MISS, TARGET: target)
```

- **Kohteen valinta.** `aim N at JOUKKO` antaa kantaman ja ehdokkaat. Pelaaja valitsee kohteen asetuksensa `targeting` mukaan kohdistimella, listasta tai automaattisesti lähimmän (moottoridokumentti, luku [8](../moottori/08-syote.md)). Tekoäly antaa ruudun suoraan.
- **Laskeutuminen.** Ammus putoaa osuttuun olentoon, ohi mentyään linjan loppuun ja esteeseen törmätessään viimeiseen vapaaseen ruutuun. Laskeutumispaikka lasketaan ennen vahinkoa, koska osuma voi tappaa kohteen.
- **Esitys.** `fly` piirtää ammuksen reitille ennen vahinkoa. Se on esitystä, joten ohitettu kohtaus ei näytä sitä.
- **Nimi.** Linjan osuman etsivä funktio on `STRIKE_LINE`, jotta se ei sekoitu lähitaistelun ytimeen `STRIKE` (luku [5](#5-komennot)).
- **Tekoäly.** `HUNTER` lyö vieressä olevaa vain, jos sillä on `MELEE`. Muuten se ampuu, kun `CAN_SHOOT` on tosi. Suora laukaus voi osua omaan, eikä metsästäjä vielä varo sitä. Jos kohde on vieressä eikä metsästäjä voi ampua, se perääntyy (`FLEE`).

---

← [4. Tapahtumat](04-tapahtumat.md) · [Sisällys](README.md) · [6. Tekoäly](06-tekoaly.md) →
