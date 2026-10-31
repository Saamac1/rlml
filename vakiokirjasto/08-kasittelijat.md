[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 8. Käsittelijät

| Nimi | Tapahtuma ja vaihe | Prioriteetti | Tekee |
| --- | --- | --- | --- |
| `SPAWN_PLAYER` | `on begin` | 0 | luo pelaajan kohtaan `game.at` ja antaa sen hallintaan |
| `APPLY_DAMAGE` | `on HIT` | 0 | vähentää elinvoimaa ja lähettää `DIED` osuman syyn kanssa |
| `HIT_MESSAGE` | `after HIT` | -10 | kirjaa osuman lokiin, jos pelaaja näki sen |
| `DEATH` | `on DIED` | -10 | synnyttää saaliin ja tuhoaa kuolleen, myös pelaajan |
| `DEATH_MESSAGE` | `after DIED` | -10 | kirjaa kuoleman lokiin, jos pelaaja näki sen |
| `HEAR_MESSAGE` | `on hear` | -10 | kirjaa pelaajan kuuleman äänen kuvauksen kovimman äänen, kun lähde ei näy, enintään kerran herätyksellä |
| `PROVOKE_HIT`, `PROVOKE_MISS` | `on HIT`, `after MISSED` | 1, 0 | hyökkäys synnyttää vihaa (luku [7](07-suhteet-ja-vihamielisyys.md)) |
| `PICKUP_MESSAGE`, `DROP_MESSAGE` | `after PICKED_UP`, `after DROPPED` | -10 | kirjaavat pelaajan esineet |
| `ADJACENCY_MOVE`, `_ARRIVE`, `_DEPART`, `_CREATED`, `_REMOVED` | `after enter`, `arrive`, `depart`, `created`, `removed` | 0 | vierekkäisyys (RogueScript, luku [4.2](../roguescript/04-tapahtumat-ja-kasittelijat.md#42-vakiokirjaston-tapahtumat)) |

```roguescript
namespace STD

query WITNESSED(e: entity) -> bool:
    return player != none and (e == player or sees(player, e))

on begin() as SPAWN_PLAYER:
    if game has PLAYER:
        control(spawn(game.PLAYER.class, at: game.at).first)
    else:
        control(spawn(DEFAULT.PLAYER, at: game.at))

on HIT(target, attacker, amount, cause) as APPLY_DAMAGE:
    if target has no HP:
        return
    target.HP.now -= amount
    if target.HP.now <= 0:
        emit DIED(target, attacker, cause)

after HIT(target, attacker, amount) as HIT_MESSAGE priority -10 when WITNESSED(target):
    if target == player:
        log(MESSAGE.COMBAT.HURT, AMOUNT: amount)
    else:
        log(MESSAGE.COMBAT.HIT, TARGET: target, AMOUNT: amount)

on DIED(victim, killer) as DEATH priority -10:
    if victim has DROPS:
        spawn(victim.DROPS.loot, at: victim)
    destroy(victim)

after DIED(victim, killer) as DEATH_MESSAGE priority -10 when WITNESSED(victim):
    if victim == player:
        log(MESSAGE.COMBAT.YOU_DIED)
    else:
        log(MESSAGE.COMBAT.DIED, VICTIM: victim)

on hear(who, s) as HEAR_MESSAGE priority -10 when who == player and (s.source == none or not sees(who, s.source)):
    if who has LISTENER and who.LISTENER.last == game.time:
        return
    if who has no LISTENER:
        add(who, LISTENER())
    who.LISTENER.last = game.time
    log(MESSAGE.SENSE.HEARD, SOUND: s.kind)

after PICKED_UP(who, item) as PICKUP_MESSAGE priority -10 when who == player:
    log(MESSAGE.LOOT.PICKUP, ITEM: item)

after DROPPED(who, item) as DROP_MESSAGE priority -10 when who == player:
    log(MESSAGE.LOOT.DROP, ITEM: item)
```

- **Kuolema ei päätä peliä.** `DEATH` tuhoaa myös pelaajan, ja maailma jatkaa ilman sitä (RogueScript, luku [10.2](../roguescript/10-aloitus-ja-checkpointit.md#102-checkpointit)). Peli, joka haluaa checkpointin tai pelin lopun, lisää oman `DIED`-käsittelijänsä.
- **Tiedot ennen tuhoa.** Pelin oma kuolinkäsittelijä, joka tarvitsee kuolleen kentät (kulta, nimi hautakiveen), ajetaan prioriteetilla väliltä -10…0, esimerkiksi -5, jolloin se ehtii ennen `DEATH`-käsittelijää.
- **Kuollut kahva.** `DEATH` tuhoaa kohteen kesken `HIT`-jakelun, ja `HIT_MESSAGE` näkee kuolleen kahvan, jonka nimi näkyy edelleen (RogueScript, luku [4](../roguescript/04-tapahtumat-ja-kasittelijat.md)).
- **Todistaja.** Viestit kirjataan vain, kun pelaaja on osapuoli tai näkee kohteen, joten kaukainen taistelu ei täytä lokia. Taistelun äänen pelaaja voi silti kuulla. `MISS_MESSAGE` (luku [5.1](05-komennot.md#51-kaukoaseet)) kirjaa ohilaukauksen samalla ehdolla.

---

← [7. Suhteet ja vihamielisyys](07-suhteet-ja-vihamielisyys.md) · [Sisällys](README.md) · [9. Äänet, viestit ja tekstit](09-aanet-viestit-ja-tekstit.md) →
