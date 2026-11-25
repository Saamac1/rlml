[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 8. Käsittelijät

| Nimi | Tapahtuma ja vaihe | Prioriteetti | Tekee |
| --- | --- | --- | --- |
| `SPAWN_PLAYER` | `on begin` | 0 | luo jokaiselle istuimelle hahmon: istuimelle 1 kohtaan `game.at`, muille vapaisiin ruutuihin sen ympärille, ja antaa ne hallintaan |
| `JOIN_PLAYER` | `on join` | 0 | luo kesken pelin liittyvälle istuimelle hahmon `player`-hahmon viereen |
| `PART_PLAYER` | `on part` | 0 | tekee poistuvan istuimen jäsenistä seuraajia (luku [6.1](06-tekoaly.md#61-seuraaja)) |
| `EXIT_STEP` | `after enter` | 0 | istuimen jäsen, joka astuu `step`-uloskäynnin ruutuun, käyttää sitä (`USE_EXIT`, luku [5](05-komennot.md)) |
| `GROUP_ARRIVAL` | `on arrive` | 0 | siirtää saapuvan jäsenen tai seuraajan vapaaseen ruutuun, jos saapumisruudussa on jo estävä entiteetti (luku [5](05-komennot.md)) |
| `APPLY_DAMAGE` | `on HIT` | 0 | vähentää elinvoimaa ja lähettää `DIED` osuman syyn kanssa |
| `HIT_MESSAGE` | `after HIT` | -10 | kirjaa osuman lokiin, jos pelaaja näki sen |
| `DEATH` | `on DIED` | -10 | synnyttää saaliin ja tuhoaa kuolleen, myös pelaajan |
| `DEATH_MESSAGE` | `after DIED` | -10 | kirjaa kuoleman lokiin, jos pelaaja näki sen |
| `HEAR_MESSAGE` | `on hear` | -10 | kirjaa jäsenen kuuleman äänen kuvauksen kovimman äänen, kun lähde ei ole yhteisessä näkökentässä, enintään kerran ajanhetkessä koko ryhmälle |
| `PROVOKE_HIT`, `PROVOKE_MISS` | `on HIT`, `after MISSED` | 1, 0 | hyökkäys synnyttää vihaa (luku [7](07-suhteet-ja-vihamielisyys.md)) |
| `PICKUP_MESSAGE`, `DROP_MESSAGE` | `after PICKED_UP`, `after DROPPED` | -10 | kirjaavat jäsenten esineet |
| `ADJACENCY_MOVE`, `_ARRIVE`, `_DEPART`, `_CREATED`, `_REMOVED` | `after enter`, `arrive`, `depart`, `created`, `removed` | 0 | vierekkäisyys (RogueScript, luku [4.2](../roguescript/04-tapahtumat-ja-kasittelijat.md#42-vakiokirjaston-tapahtumat)) |

```roguescript
namespace STD

query WITNESSED(e: entity) -> bool:
    return seat_of(e) != none or in_view(e)

query SOLO() -> bool:
    return seats().count < 2 and members(seats().first).count < 2

query OPEN(a: area, who: entity) -> area:
    return a where entities(in: it).count == 0 and not blocks(it, move, for: who)

fn SPAWN_MEMBER(n: number, at: area) -> entity:
    if game has PLAYER:
        e = spawn(game.PLAYER.class, at: at).first
    else:
        e = spawn(DEFAULT.PLAYER, at: at)
    if e != none:
        control(e, seat: n)
    return e

on begin() as SPAWN_PLAYER:
    for n in seats():
        if n == seats().first:
            SPAWN_MEMBER(n, game.at)
        else:
            SPAWN_MEMBER(n, OPEN(near(game.at, 0..2), none))

on join(seat) as JOIN_PLAYER:
    if player == none or player.at == none:
        return
    SPAWN_MEMBER(seat, OPEN(near(player, 1..2), none))

on HIT(target, attacker, amount, cause) as APPLY_DAMAGE:
    if target has no HP:
        return
    target.HP.now -= amount
    if target.HP.now <= 0:
        emit DIED(target, attacker, cause)

after HIT(target, attacker, amount) as HIT_MESSAGE priority -10 when WITNESSED(target):
    if target == player and SOLO():
        log(MESSAGE.COMBAT.HURT, AMOUNT: amount)
    else:
        log(MESSAGE.COMBAT.HIT, TARGET: target, AMOUNT: amount)

on DIED(victim, killer) as DEATH priority -10:
    if victim has DROPS:
        spawn(victim.DROPS.loot, at: victim)
    destroy(victim)

after DIED(victim, killer) as DEATH_MESSAGE priority -10 when WITNESSED(victim):
    if victim == player and SOLO():
        log(MESSAGE.COMBAT.YOU_DIED)
    else:
        log(MESSAGE.COMBAT.DIED, VICTIM: victim)

on hear(who, s) as HEAR_MESSAGE priority -10 when seat_of(who) != none and (s.source == none or not in_view(s.source)):
    if game.LISTENER.last == game.time:
        return
    game.LISTENER.last = game.time
    log(MESSAGE.SENSE.HEARD, SOUND: s.kind)

after PICKED_UP(who, item) as PICKUP_MESSAGE priority -10 when seat_of(who) != none:
    if who == player and SOLO():
        log(MESSAGE.LOOT.PICKUP, ITEM: item)
    else:
        log(MESSAGE.LOOT.ALLY_PICKUP, WHO: who, ITEM: item)

after DROPPED(who, item) as DROP_MESSAGE priority -10 when seat_of(who) != none:
    if who == player and SOLO():
        log(MESSAGE.LOOT.DROP, ITEM: item)
    else:
        log(MESSAGE.LOOT.ALLY_DROP, WHO: who, ITEM: item)
```

- **Kuolema ei päätä peliä.** `DEATH` tuhoaa myös pelaajan, ja maailma jatkaa ilman sitä (RogueScript, luku [10.2](../roguescript/10-aloitus-ja-checkpointit.md#102-checkpointit)). Peli, joka haluaa checkpointin tai pelin lopun, lisää oman `DIED`-käsittelijänsä.
- **Tiedot ennen tuhoa.** Pelin oma kuolinkäsittelijä, joka tarvitsee kuolleen kentät (kulta, nimi hautakiveen), ajetaan prioriteetilla väliltä -10…0, esimerkiksi -5, jolloin se ehtii ennen `DEATH`-käsittelijää.
- **Kuollut kahva.** `DEATH` tuhoaa kohteen kesken `HIT`-jakelun, ja `HIT_MESSAGE` näkee kuolleen kahvan, jonka nimi näkyy edelleen (RogueScript, luku [4](../roguescript/04-tapahtumat-ja-kasittelijat.md)).
- **Istuimet.** `SPAWN_PLAYER` luo hahmon jokaiselle liitetylle istuimelle (moottoridokumentti, luku [6.1.10](../moottori/06-ajo.md#6110-istuimet-ja-ryhmä)). Yhden ruudun `START` riittää, koska istuimen 1 jälkeiset hahmot saavat vapaan ruudun kahden ruudun säteeltä. `JOIN_PLAYER` ei luo hahmoa, jos elävää jäsentä tai vapaata ruutua ei ole, joten istuin jää tyhjäksi.
- **Yksin vai ryhmässä.** `SOLO` on tosi, kun istuimia on yksi ja jäseniä enintään yksi. Silloin pelaajaa koskevat viestit ovat sinä-muodossa (`HURT`, `YOU_DIED`, `PICKUP`, `DROP`). Muuten ne nimeävät jäsenen (`HIT`, `DIED`, `ALLY_PICKUP`, `ALLY_DROP`), koska vuoro kiertää ja sinä olisi epäselvä.
- **Todistaja.** Viestit kirjataan vain, kun jäsen on osapuoli tai kohde on yhteisessä näkökentässä (`in_view`, moottoridokumentti luku [7.2](../moottori/07-esitys.md#72-kartan-piirtäminen)). Viimeisen jäsenen kuolema kirjautuu, koska yhteinen näkökenttä säilyy, vaikka `DEATH` tuhoaa jäsenen ennen `DEATH_MESSAGE`-käsittelijää. Kuuloviesti kirjataan kerran ajanhetkessä koko ryhmälle, ja hetki on `game`-lohkon komponentissa `LISTENER`.

---

← [7. Suhteet ja vihamielisyys](07-suhteet-ja-vihamielisyys.md) · [Sisällys](README.md) · [9. Äänet, viestit ja tekstit](09-aanet-viestit-ja-tekstit.md) →
