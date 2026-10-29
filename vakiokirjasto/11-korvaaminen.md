[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 11. Korvaaminen

Peli muuttaa STD:tä kolmella tavalla. Kaikki korvaukset kirjoitetaan täydellä nimellä, koska STD:n oma koodi löytää ensin `STD`-nimen (luku [1.2](01-rajaus-periaatteet-ja-lataus.md#12-nimeäminen)).

- **Lisäys.** Oma käsittelijä samalle tapahtumalle STD:n rinnalle, vyöhykkeen mukaan ennen tai jälkeen (luku [2](02-prioriteettivyohykkeet.md)).
- **Korvaus.** `override` korvaa koko nimetyn käsittelijän, funktion, komennon, komponentin tai tekoälyn (RogueScript, luku [11.2](../roguescript/11-tallennus-ja-korvaaminen.md#112-korvaaminen)). Käsittelijän voi poistaa käytöstä korvaamalla sen rungolla, joka vain palaa.
- **Datan paikkaus.** `patch` muuttaa STD:n entiteetin rivejä, ja nimettömän osion rivi korvataan `override`-merkinnällä (dataformaatit, luvut [4.5](../dataformaatit/04-nimiavaruudet.md#45-korvaaminen-ja-override) ja [4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).

```roguescript
// Checkpoint pelaajan kuollessa: lisäys STD:n rinnalle
on DIED(victim, killer) priority -20 when victim == player:
    rollback()

// Vahinko on aina vähintään 1: korvaus
override on HIT(target, attacker, amount, cause) as STD.APPLY_DAMAGE:
    if target has no HP:
        return
    target.HP.now -= max(1, amount)
    if target.HP.now <= 0:
        emit STD.DIED(target, attacker, cause)

// Osumatarkkuus lähitaisteluun: ytimen korvaus koskee myös törmäystä
override fn STD.STRIKE(attacker: entity, target: entity) -> bool:
    if attacker has no MELEE or target has no HP or not target.at in adjacent(attacker):
        return false
    if SWING(attacker, target):
        emit STD.HIT(target, attacker, attacker.MELEE.damage)
    else:
        emit STD.MISSED(target, attacker)
    noise(attacker.at, STD.SOUND.FIGHT, source: attacker)
    return true

// Ei kuolinviestejä: käsittelijä pois käytöstä
override after DIED(victim, killer) as STD.DEATH_MESSAGE:
    return
```

```
patch entity STD.DEFAULT.PLAYER
- HP now 15

sounds
- override STD.SOUND.STEP level 2 effect SFX.STEP
```

---

← [10. Generointi](10-generointi.md) · [Sisällys](README.md) · [12. Minipeli STD:llä](12-minipeli-std-lla.md) →
