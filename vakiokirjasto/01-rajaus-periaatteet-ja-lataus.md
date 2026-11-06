[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 1. Rajaus, periaatteet ja lataus

## 1.1 Mikä STD on

- **Tavallista koodia.** STD on nimiavaruudessa `STD` olevaa RLML:ää ja RogueScriptiä. Moottori tuntee siitä vain nimiavaruuden nimen (moottoridokumentti, luku [11](../moottori/11-avoimet-kysymykset.md)), eikä mikään STD:n sääntö ole moottorin sääntö.
- **Tämä dokumentti on sopimus.** Nimet, kentät, parametrit, prioriteetit ja käytös ovat normatiivisia. Toinen toteutus saa kirjoittaa koodin toisin, mutta sen on käytettävä satunnaisvirtaa samassa järjestyksessä, jotta sama siemen tuottaa saman pelin (RogueScript, luku [5.4](../roguescript/05-funktiot-ja-lausekkeet.md#54-satunnaisuus)).
- **Pienet nimetyt osat.** Jokainen sääntö on oma nimetty käsittelijänsä, funktionsa tai komentonsa, joten peli voi korvata yhden kohdan koskematta muihin (luku [11](11-korvaaminen.md)).
- **Tyhjä peli ei sisällä STD:tä.** STD tulee mukaan vain `include`-rivillä (dataformaatit, luku [3](../dataformaatit/03-tiedostot-ja-include.md)).

## 1.2 Nimeäminen

- **Komennot ovat verbejä** (`MOVE`, `ATTACK`, `TAKE_EXIT`), **komponentit substantiiveja** (`HP`, `MELEE`, `EXIT`) ja **tekoälyt toimijan nimiä** (`HUNTER`). Tapahtumat ovat substantiiveja tai partisiippeja (`HIT`, `DIED`).
- **Ryhmät:** äänilajit ovat ryhmässä `SOUND`, viestit ryhmässä `MESSAGE` alaryhmineen ja oletusentiteetit ryhmässä `DEFAULT`.
- Pelin koodi viittaa STD:n nimiin lyhyesti (`HP`, `MOVE`), koska nimenhaku kokeilee lopuksi nimeä `STD.N` (dataformaatit, luku [4.4](../dataformaatit/04-nimiavaruudet.md#44-nimenhaku)). Pelin oma samanniminen määrittely peittää STD:n nimen pelin koodissa mutta ei STD:n omassa koodissa, jossa haku löytää ensin nimen `STD.N`. STD:n osa korvataan siksi aina täydellä nimellä (`override … STD.HP`).

## 1.3 Lataus ja moduulit

Peli ottaa koko STD:n mukaan rivillä `include lib/std.rlml`. Rivin kannattaa olla pelin ensimmäinen include: mukaan otettu sisältö latautuu rivin kohdalle, joten STD:n käsittelijät ovat silloin latausjärjestyksessä ensin ja vyöhykkeet toimivat kuvatusti (luku [2](02-prioriteettivyohykkeet.md)). Moduulit voi ottaa mukaan myös yksitellen:

| Moduuli | Sisältö | Tarvitsee |
| --- | --- | --- |
| `lib/std/core.rlml` | `PLAYER`, `DEFAULT.PLAYER`, `SPAWN_PLAYER`, `SPAWN_MEMBER`, `JOIN_PLAYER`, `OPEN`, `SOLO`, `WITNESSED`, `SWITCH`, `SWITCH.PREV` | – |
| `lib/std/combat.rlml` | `HP`, `MELEE`, `DROPS`, `HIT`, `DIED`, `HURT`, `STRIKE`, `ATTACK`, vahinko ja kuolema sekä kaukoaseet (`RANGED`, `MARKSMANSHIP`, `EVASION`, `THROWN`, `MISSED`, `SHOOT`, `THROW`) | core |
| `lib/std/move.rlml` | `CAN_STEP`, `LEASHED`, `BLOCKER`, `DO_STEP`, `MOVE`, `STEP`, suuntakomennot, `WAIT`, `BUMP` | relations, combat, sound |
| `lib/std/relations.rlml` | `ALIGNMENT`, `FACTION`, `BONDS`, `STANCE`, `FACTIONS`, `GROUP.PLAYER`, `ATTITUDE`, `BEFRIEND` | – |
| `lib/std/ai.rlml` | `HUNTER`, `HOSTILES`, `APPROACH`, `FLEE`, `WANDER`, `FOLLOWER`, `FOLLOW`, `LEADER_OF`, `PART_PLAYER` | move, combat, relations |
| `lib/std/items.rlml` | `PACK`, `ITEM`, `GIVE`, `PICK_UP`, `DROP` | – |
| `lib/std/exits.rlml` | `EXIT`, `TAKE_EXIT`, `TRAVEL_GROUP`, `EXIT.LEAVE_OTHERS`, `GROUP_ARRIVAL` | core, ai |
| `lib/std/adjacency.rlml` | `ADJACENT`, `APART` ja niiden käsittelijät | – |
| `lib/std/sound.rlml` | äänilajit ja kuulemisen viestit | – |
| `lib/std/gen.rlml` | generointialgoritmit (luku [10](10-generointi.md)) | – |
| `lib/std/text/fi.rlml`, `en.rlml` | STD:n tekstit (luku [9](09-aanet-viestit-ja-tekstit.md)) | – |

Moduuli ottaa tarvitsemansa moduulit itse mukaan, ja sama tiedosto latautuu vain kerran. `move` tarvitsee combat-moduulin, koska törmäys palauttaa jatkokomennon `ATTACK`, ja sound-moduulin, koska askel pitää ääntä `SOUND.STEP` tai `SOUND.SNEAK`. STD:n omien tilejen ASCII-tyylirivit ovat tiedostossa `lib/std/ascii.rlml`, jonka `lib/std.rlml` ottaa mukaan.

---

[Sisällys](README.md) · [2. Prioriteettivyöhykkeet](02-prioriteettivyohykkeet.md) →
