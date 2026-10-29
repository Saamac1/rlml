[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 3. Komponentit ja oletuspelaaja

| Komponentti | Kentät | Merkitys |
| --- | --- | --- |
| `PLAYER` | `class: card = DEFAULT.PLAYER` | `game`-lohkossa: pelaajan entiteetti, jonka `SPAWN_PLAYER` luo |
| `HP` | `now: number`, `max: number = now` | elinvoima; 0 tai alle laukaisee `DIED`-tapahtuman |
| `MELEE` | `damage: dice` | lähitaistelun vahinko, jota `STRIKE` käyttää |
| `DROPS` | `loot: table` | `DEATH` synnyttää taulusta kortin kuolinpaikkaan |
| `PACK` | `items: collection` | reppu, jota `GIVE`, `PICK_UP` ja `DROP` käyttävät |
| `ITEM` | – (merkki) | esine, jonka voi poimia |
| `EXIT` | `to: name`, `at: label = START` | uloskäynti, jonka `TAKE_EXIT` käyttää |
| `FOOD` | `points: number = 30`, `group: bool = false` | ruoka, jonka `FEED` antaa; `group` muuttaa koko lajin suhdetta |
| `SNEAKING` | – (merkki) | hiipiminen, jonka `SNEAK` kytkee |
| `LISTENER` | `last: number = -1` | pelaajan kuuloviestin tahti (luku [8](08-kasittelijat.md)) |
| `ALIGNMENT` | `value: number = 0` | arvoasteikko −100…100 (luku [7](07-suhteet-ja-vihamielisyys.md)) |
| `FACTION` | `group: name` | ryhmäentiteetin nimi, esimerkiksi `GROUP.RAT` (luku [7](07-suhteet-ja-vihamielisyys.md)) |
| `BONDS` | `toward: dict entity number` | yksilön omat suhteet muihin (luku [7](07-suhteet-ja-vihamielisyys.md)) |
| `FACTIONS` | `items: collection` | `game`-lohkossa: pelin ryhmien ilmentymät, nostettuna pakasta (luku [7](07-suhteet-ja-vihamielisyys.md)); ryhmäentiteetin `STANCE` (`toward: dict name number`) kertoo sen suhteet muihin ryhmiin |

Datassa komponentit kirjoitetaan tavalliseen tapaan: `- HP now 10`, `- MELEE 1d4`, `- FACTION GROUP.RAT`, `- ALIGNMENT -100`. Yhden kentän komponentti annetaan ilman kentän nimeä, joten pelaajan valinta on `game`-lohkossa `- PLAYER HERO` ja sallii ylikirjoitukset (`- PLAYER HERO HP 20`). Kaukoaseiden komponentit `RANGED`, `MARKSMANSHIP`, `EVASION` ja `THROWN` kuvataan luvussa [5.1](05-komennot.md#51-kaukoaseet).

**Oletuspelaaja.** Ilman `PLAYER`-riviä `SPAWN_PLAYER` luo oletuspelaajan:

```
entity DEFAULT.PLAYER
- HP now 10
- MELEE 1d4
- PACK
- blocks move
- ALIGNMENT 0
- FACTION GROUP.PLAYER
- sight radius 8
- hearing
- ai behavior HUNTER
- render GLYPH.PLAYER
```

Nimiavaruudessa `STD` tilen nimi on `STD.GLYPH.PLAYER`, ja STD toimittaa sille ASCII-rivin tiedostossa `lib/std/ascii.rlml`. Muissa tyyleissä peli antaa rivin itse tai käyttää `default`-riviä. Ryhmä `GROUP.PLAYER` on STD:n entiteetti ilman suhteita. Pelin pelaajaentiteetit nimetään muulla nimellä kuin `PLAYER`, koska nimi on jo komponentin (RogueScript, luku [10.1](../roguescript/10-aloitus-ja-checkpointit.md#101-aloitus)). Oma pelaaja voi periä oletuksen: `entity HERO extends STD.DEFAULT.PLAYER`.

STD ei kirjoita tyyleihin omaa `default`-riviä, joten pelin oma `default` ei kilpaile sen kanssa.

**Oletuspelaajan tekoäly.** Rivi `- ai behavior HUNTER` ei ohjaa pelaajaa, kun pelaaja on hallinnassa. Moottori kysyy tekoälyä vain joutokäynnissä (`idle`, RogueScript, luku [7.2](../roguescript/07-komennot-ja-tekoaly.md#72-tekoäly)), ja silloin pelaaja noudattaa samoja vihamielisyyssääntöjä kuin muut (luku [7.1](07-suhteet-ja-vihamielisyys.md#71-viholliset-ja-ystävät)).

---

← [2. Prioriteettivyöhykkeet](02-prioriteettivyohykkeet.md) · [Sisällys](README.md) · [4. Tapahtumat](04-tapahtumat.md) →
