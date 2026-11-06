[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 9. Äänet, viestit ja tekstit

**Äänilajit** (dataformaatit, luku [12.1](../dataformaatit/12-aanten-ilmoitukset.md#121-ilmoitukset)):

```
sounds
- SOUND.STEP level 3
- SOUND.SNEAK level 1
- SOUND.FIGHT level 6
- SOUND.SHOT level 4
```

STD:n äänillä ei ole efektiä. Peli antaa sen korvaamalla rivin: `- override STD.SOUND.STEP level 3 effect SFX.STEP`.

**Viestit** (dataformaatit, luku [11.4](../dataformaatit/11-kayttoliittyman-data.md#114-viestit-ja-loki)). Nimiavaruudessa `STD` avaimet ovat muotoa `STD.MESSAGE.…`, ja ryhmät toimivat luokkina:

```
messages
- MESSAGE.COMBAT.HIT TARGET AMOUNT
- MESSAGE.COMBAT.HURT AMOUNT
- MESSAGE.COMBAT.DIED VICTIM
- MESSAGE.COMBAT.YOU_DIED
- MESSAGE.COMBAT.MISS TARGET
- MESSAGE.LOOT.PICKUP ITEM
- MESSAGE.LOOT.DROP ITEM
- MESSAGE.LOOT.ALLY_PICKUP WHO ITEM
- MESSAGE.LOOT.ALLY_DROP WHO ITEM
- MESSAGE.SENSE.HEARD SOUND
```

Peli valitsee ne lokiin ryhmittäin, esimerkiksi taistelu ja aistit päälokiin ja tavarat omaansa:

```
log LOG history 100 merge
- STD.MESSAGE.COMBAT
- STD.MESSAGE.SENSE

log LOOTLOG
- STD.MESSAGE.LOOT
```

**Tekstit.** STD toimittaa kielitiedostot suomeksi ja englanniksi (`lib/std/text/fi.rlml`, `en.rlml`). Suomenkielinen:

```
language fi
STD.DEFAULT.PLAYER.NAME sinä
STD.MESSAGE.COMBAT.HIT {TARGET} ottaa {AMOUNT} vahinkoa.
STD.MESSAGE.COMBAT.HURT Otat {AMOUNT} vahinkoa.
STD.MESSAGE.COMBAT.DIED {VICTIM} kuolee.
STD.MESSAGE.COMBAT.YOU_DIED Kuolet.
STD.MESSAGE.COMBAT.MISS {TARGET} väistää.
STD.MESSAGE.LOOT.PICKUP Poimit: {ITEM}.
STD.MESSAGE.LOOT.DROP Pudotit: {ITEM}.
STD.MESSAGE.LOOT.ALLY_PICKUP {WHO} poimii: {ITEM}.
STD.MESSAGE.LOOT.ALLY_DROP {WHO} pudottaa: {ITEM}.
STD.EXIT.LEAVE_OTHERS.ASK Muut jäävät jälkeen. Lähdetäänkö koko ryhmänä?
STD.EXIT.LEAVE_OTHERS.YES Kyllä
STD.EXIT.LEAVE_OTHERS.NO Ei
STD.MESSAGE.SENSE.HEARD {SOUND:DESC}
STD.SOUND.STEP.DESC Kuulet askelia.
STD.SOUND.SNEAK.DESC Kuulet hiipiviä askelia.
STD.SOUND.FIGHT.DESC Kuulet taistelun ääniä.
STD.SOUND.SHOT.DESC Kuulet jänteen napsahduksen.
```

- Pelaajalla on omat viestinsä (`HURT`, `YOU_DIED`), koska suomen verbi taipuu persoonassa: “sinä ottaa” ei kelpaa.
- **Ryhmä.** Kun istuimia tai jäseniä on useampi, viestit nimeävät jäsenen: taistelussa `HIT` ja `DIED` ja esineissä `ALLY_PICKUP` ja `ALLY_DROP` (luku [8](08-kasittelijat.md)). Oletuspelaajan nimi on silloinkin “sinä”, joten moninpeli antaa hahmoluokilleen omat nimet (avoin kysymys, luku [14](14-avoimet-kysymykset.md)).
- `{SOUND:DESC}` näyttää parametrina annetun nimen `DESC`-tekstin `NAME`-tekstin sijaan. Pääte valitsee parametrina annetun nimen tekstiavaimen (dataformaatit, luku [10](../dataformaatit/10-tekstit.md)).
- Peli korvaa tekstin omassa kielitiedostossaan samalla avaimella.

---

← [8. Käsittelijät](08-kasittelijat.md) · [Sisällys](README.md) · [10. Generointi](10-generointi.md) →
