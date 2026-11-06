[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 7. Suhteet ja vihamielisyys

Suhde on luku −100…100, joka kertoo, miten `a` suhtautuu `b`:hen. Suhde ei ole symmetrinen: `a`:n suhde `b`:hen ei muuta `b`:n suhdetta `a`:han. Vihamielinen on se, jonka suhde on alle nollan. Suhde tulee ensimmäiseltä tasolta, jolla on arvo:

1. **Yksilö:** `a`:n oma suhde `b`:hen komponentissa `BONDS` (ruokittu tai lumottu rotta).
2. **Ryhmä:** `a`:n ryhmän suhde `b`:n ryhmään (rottien suhde pelaajiin).
3. **Alignment:** `TOLERANCE - |a - b|` alignmenttien arvoista, oletus `TOLERANCE` = 50. Puuttuva `ALIGNMENT` on 0.

Oletuksena kaikki ovat siis rauhallisia: kaksi entiteettiä ilman suhteita ja alignmenttia saa suhteen 50. Vihamielisyys kirjoitetaan peliin alignmenttina, ryhmäsuhteena tai yksilön suhteena.

**Ryhmät ovat entiteettejä.** Ryhmä määritellään entiteettinä, jonka komponentti `STANCE` kertoo sen suhteet muihin ryhmiin. Peli luettelee ryhmänsä pakassa, ja `game`-lohkon komponentti `FACTIONS` nostaa ne pelin alussa kokoelmaan, jossa ryhmien suhteet voivat muuttua pelin aikana. Olento viittaa ryhmäänsä nimellä, joten lataus tarkistaa sen:

```
entity GROUP.RAT
- STANCE toward GROUP.PLAYER -20

deck GROUPS
- GROUP.RAT
- GROUP.PLAYER

game MINI
- FACTIONS GROUPS all

entity RAT
- FACTION GROUP.RAT
```

```roguescript
namespace STD

query GROUP_OF(e: entity) -> entity:
    if e has no FACTION or game has no FACTIONS:
        return none
    return (game.FACTIONS.items where it is e.FACTION.group).first

query GROUP_STANCE(a: entity, b: entity) -> number:
    ga = GROUP_OF(a)
    if ga == none or ga has no STANCE or b has no FACTION:
        return none
    if not b.FACTION.group in ga.STANCE.toward:
        return none
    return ga.STANCE.toward[b.FACTION.group]

query BASE(a: entity, b: entity) -> number:
    s = GROUP_STANCE(a, b)
    if s != none:
        return s
    d = 0
    if a has ALIGNMENT:
        d += a.ALIGNMENT.value
    if b has ALIGNMENT:
        d -= b.ALIGNMENT.value
    return TOLERANCE - max(d, -d)

query ATTITUDE(a: entity, b: entity) -> number:
    if a has BONDS and b in a.BONDS.toward:
        return a.BONDS.toward[b]
    return BASE(a, b)

fn BEFRIEND(a: entity, b: entity, points: number, group: bool = false):
    if group:
        ga = GROUP_OF(a)
        if ga != none and b has FACTION:
            if ga has no STANCE:
                add(ga, STANCE(toward: {}))
            ga.STANCE.toward[b.FACTION.group] = min(100, max(-100, BASE(a, b) + points))
            return
    if a has no BONDS:
        add(a, BONDS(toward: {}))
    a.BONDS.toward[b] = min(100, max(-100, ATTITUDE(a, b) + points))
```

- **`BEFRIEND`** muuttaa `a`:n suhdetta `b`:hen. Negatiivinen `points` suututtaa. `group: true` muuttaa `a`:n ryhmän suhdetta `b`:n ryhmään, joten monen rotan ruokkiminen voi kääntää lajin suhteen. Jos `a`:lla ei ole ryhmää, muutos kohdistuu yksilöön.
- **Ryhmän tunnistus.** `GROUP_OF` vertaa ilmentymää ryhmän nimeen muodolla `it is e.FACTION.group`, joten `is` hyväksyy myös nimiarvon (RogueScript, luku [5.3](../roguescript/05-funktiot-ja-lausekkeet.md#53-lausekkeet)).
- **Ryhmä ilman ilmentymää.** Jos ryhmä puuttuu `FACTIONS`-kokoelmasta, ryhmätasoa ei ole, ja suhde tulee alignmentista.
- **Kuollut kohde.** Kuollut kahva jää `BONDS`-sanakirjaan avaimeksi, kunnes se poistetaan. Se ei haittaa, koska kuollutta ei kohdata.
- **Esimerkki.** Rotta, jolla on `ALIGNMENT -100`, saa pelaajaan (alignment 0) suhteen −50. Ruokinnan `BEFRIEND(rat, player, 60)` jälkeen suhde on +10.

## 7.1 Viholliset ja ystävät

Suhde ratkaisee, kuka saa lyödä ketä. Viholliset ovat vastavuoroisia: jos jompikumpi suhtautuu toiseen negatiivisesti, ne ovat vihollisia. Ystävyys vaatii molemmilta vähintään `FRIENDLY`. Alignment muuttaa sääntöä:

- **Hyvä** (`ALIGNMENT` ≥ `GOOD_AT`) lyö vain vihollista, eikä `FIGHT` muuta sitä.
- **Neutraali** lyö vihollista, ja `FIGHT` sallii tarkoituksellisen lyönnin myös neutraaliin. Neutraali asettuu lajinsa puolelle: kun lajitoveria lyödään, näkevät lajitoverit suuttuvat lyöjään.
- **Paha** (`ALIGNMENT` ≤ `EVIL_AT`) lyö kaikkia, joihin sen suhde on alle `FRIENDLY`, eikä välitä lajitovereistaan.

```roguescript
namespace STD

query ALIGNMENT_OF(e: entity) -> number:
    if e has ALIGNMENT:
        return e.ALIGNMENT.value
    return 0

query ENEMIES(a: entity, b: entity) -> bool:
    return ATTITUDE(a, b) < 0 or ATTITUDE(b, a) < 0

query FRIENDS(a: entity, b: entity) -> bool:
    return ATTITUDE(a, b) >= FRIENDLY and ATTITUDE(b, a) >= FRIENDLY

query MAY_ATTACK(a: entity, b: entity) -> bool:
    if a == b or b has no HP:
        return false
    if ALIGNMENT_OF(a) <= EVIL_AT:
        return ATTITUDE(a, b) < FRIENDLY
    return ENEMIES(a, b)

query NEUTRAL(e: entity) -> bool:
    return ALIGNMENT_OF(e) > EVIL_AT and ALIGNMENT_OF(e) < GOOD_AT

fn GRUDGE(victim: entity, attacker: entity):
    if victim has no BONDS:
        add(victim, BONDS(toward: {}))
    victim.BONDS.toward[attacker] = min(ATTITUDE(victim, attacker), PROVOKED)

fn PROVOKE(victim: entity, attacker: entity):
    if attacker == none or attacker == victim:
        return
    if seat_of(victim) != none and seat_of(attacker) != none:
        return
    GRUDGE(victim, attacker)
    if not NEUTRAL(victim) or victim has no FACTION:
        return
    kin = entities(in: near(victim, KIN_RANGE)) where it != victim and it != attacker and it has FACTION and it.FACTION.group == victim.FACTION.group and NEUTRAL(it) and sees(it, victim)
    for k in kin:
        GRUDGE(k, attacker)

on HIT(target, attacker, amount) as PROVOKE_HIT priority 1:
    PROVOKE(target, attacker)

after MISSED(target, attacker) as PROVOKE_MISS:
    PROVOKE(target, attacker)
```

- **Vakiot** ovat korvattavia lukuja kuten `TOLERANCE`: `FRIENDLY` = 50, `PROVOKED` = −50, `GOOD_AT` = 34, `EVIL_AT` = −34 ja `KIN_RANGE` = 8.
- **Hyökkäys synnyttää vihaa.** Osuma ja huti laskevat uhrin suhteen lyöjään enintään arvoon `PROVOKED`. Rotta, jota pelaaja on ruokkinut, kääntyy siis pelaajaa vastaan, jos pelaaja lyö sitä. `PROVOKE_HIT` ajetaan ennen vahinkoa (prioriteetti 1), jotta kuoleva uhri ehtii suuttua.
- **Ystävällinen tuli.** Istuinten jäsenet eivät suutu toisiinsa. Kaukoaseen harhalaukaus voi osua jäseneen (luku [5.1](05-komennot.md#51-kaukoaseet)) ja vahingoittaa sitä, mutta `PROVOKE` palaa heti, kun uhri ja hyökkääjä ovat molemmat jäseniä. Ilman poikkeusta yksi harhalaukaus tekisi jäsenistä toistensa vihollisia, koska `ENEMIES` on molemminpuolinen.
- **Ruokinta.** `FEED` kutsuu `BEFRIEND`-funktiota ruoan `FOOD.points`-arvolla, ja `FOOD group true` muuttaa koko lajin suhdetta (luku [5](05-komennot.md)).

---

← [6. Tekoäly](06-tekoaly.md) · [Sisällys](README.md) · [8. Käsittelijät](08-kasittelijat.md) →
