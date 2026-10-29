[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 4. Tapahtumat

STD:n tapahtumat tulkitsevat moottorin muutoksia pelin säännöillä (RogueScript, luku [4.2](../roguescript/04-tapahtumat-ja-kasittelijat.md#42-vakiokirjaston-tapahtumat)). Ensimmäinen parametri on subjekti.

| Tapahtuma | Lähettäjä | Merkitys |
| --- | --- | --- |
| `BUMP(who, other)` | `STEP` | `who` yritti astua ruutuun, jossa `other` estää; ei siirrä |
| `HIT(target, attacker, amount, cause)` | `STRIKE`, `HURT` | `target` ottaa vahinkoa `amount`; `attacker` voi olla `none`. `cause` on syyn nimi tai `none` (alla). `before`-vaihe voi muuttaa määrää tai perua osuman |
| `DIED(victim, killer, cause)` | `APPLY_DAMAGE` | elinvoima loppui; `killer` ja `cause` voivat olla `none` |
| `ADJACENT(who, other)`, `APART(who, other)` | vierekkäisyyden käsittelijät | kaksi entiteettiä tuli viereen tai erkani (RogueScript, luku [4.2](../roguescript/04-tapahtumat-ja-kasittelijat.md#42-vakiokirjaston-tapahtumat)) |
| `PICKED_UP(who, item)` | `PICK_UP` | esine siirtyi maasta reppuun |
| `DROPPED(who, item)` | `DROP` | esine siirtyi repusta maahan |

Lisäksi `MISSED(target, attacker)` lähetetään, kun ammus menee kohteen ohi (luku [5.1](05-komennot.md#51-kaukoaseet)). Funktiot, joilla logiikka laukaisee samat säännöt:

```roguescript
namespace STD

event HIT(target: entity, attacker: entity, amount: number, cause: name = none)
event DIED(victim: entity, killer: entity, cause: name = none)
event BUMP(who: entity, other: entity)
event PICKED_UP(who: entity, item: entity)
event DROPPED(who: entity, item: entity)

fn HURT(target: entity, amount: number, source: entity = none, cause: name = none):
    emit HIT(target, source, amount, cause)

fn GIVE(to: entity, card: card):
    spawn(card, into: to.PACK.items)
```

`GIVE` ei lähetä `PICKED_UP`-tapahtumaa, koska esine ei tule maasta.

**Kuoleman syy.** `cause` kertoo, mihin kohde kuoli, kun tappaja ei kerro sitä: nälkä, ansa, myrkky tai pelaajan lopetus. Arvo on nimi, tyypillisesti pelin entiteetti ilman komponentteja (`CAUSE.STARVATION`), jolloin sen teksti tulee kielitiedostosta avaimella `CAUSE.STARVATION.NAME`. `APPLY_DAMAGE` välittää syyn osumasta kuolemaan. Käsittelijä saa jättää oletusarvoiset loppuparametrit pois (RogueScript, luku [4](../roguescript/04-tapahtumat-ja-kasittelijat.md)), joten `on HIT(target, attacker, amount)` ja `on DIED(victim, killer)` toimivat ennallaan.

---

← [3. Komponentit ja oletuspelaaja](03-komponentit-ja-oletuspelaaja.md) · [Sisällys](README.md) · [5. Komennot](05-komennot.md) →
