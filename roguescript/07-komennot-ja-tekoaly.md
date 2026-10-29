[RLML](../README.md) › [RogueScript](README.md)

# 7. Komennot ja tekoäly

## 7.1 Komennot

Komento on toiminto, jonka pelaaja tai tekoäly valitsee. Komennon nimi on sama, johon näppäimet sidotaan datan `keys`-osiossa.

```roguescript
command QUAFF(potion: entity from actor.PACK.items where it has DRINKABLE) cost 120:
    emit DRINK(actor, potion)

command THROW(item: entity from actor.PACK.items, to: pos aim 6) cost 120:
    move(item, to: path(actor, to).last)

command DIG(dir: dir):
    repeat 5:
        wait 120
        tile(actor.at + dir, FLOOR, layer: GROUND)

command MOVE.NORTH = MOVE(north)
```

- Parametrin lähde (`from`, `aim`) kertoo, mistä moottori kysyy arvon pelaajalta: listavalikosta tai kohdistimella. Tekoäly antaa saman arvon suoraan, jolloin kysymys ohitetaan. `aim N at JOUKKO` antaa kohdistimelle kantaman ja ehdokkaat, ja pelaajan asetus `targeting` päättää, valitaanko kohde kohdistimella, listasta vai automaattisesti (moottoridokumentti, luku [8](../moottori/08-syote.md)). `N` voi olla lauseke (`aim actor.RANGED.range`); jos se on `none`, komento epäonnistuu ennen kysymistä. Samoin tyhjä `from`-joukko: komento epäonnistuu ennen kysymistä eikä vie aikaa (`FEED` ilman ruokaa).
- `cost N` on toiminnon hinta mikroaskelina normaalinopeudella 10. Suorittajan nopeus skaalaa sen viiveeksi, joka kertoo, milloin suorittaja saa seuraavan vuoron. Oletushinta on 120 eli yksi kierros (moottoridokumentti, luku [6.1.3](../moottori/06-ajo.md#613-hinta-ja-nopeus)).
- `actor` on komennon suorittaja. Nimi on käytössä vain komennoissa.
- `wait N` kuluttaa N mikroaskelta nopeudella skaalattuna ja jatkaa komentoa sen jälkeen. Suorittaja ei saa välissä uusia vuoroja. Monivuoroinen toiminto kirjoitetaan siksi suoraan ilman ajastimia (moottoridokumentti, luku [6.1.6](../moottori/06-ajo.md#616-monivaiheinen-komento)).
- **Odotus rungossa.** `wait`, `wait scene`, `wait confirm` ja `open` voivat olla missä tahansa komennon rungossa, myös silmukoiden ja `if`-haarojen sisällä, mutta eivät komennon kutsumissa funktioissa (luku [5.1](05-funktiot-ja-lausekkeet.md#51-funktioiden-lajit)). Tallennus säilyttää odottavan komennon kohdan rakenteellisena polkuna, ja ladattu peli jatkaa siitä (luku [11.1](11-tallennus-ja-korvaaminen.md#111-tallennus)).
- **Monivaiheisen komennon hinta.** Komento, joka kuluttaa aikansa silmukan `wait`-kutsuilla (juoksu, etsintä, taistelu kuolemaan asti), on itse maksuton (`cost 0`). Muuten `cost 120` ja silmukan `wait 120` laskuttaisivat yhden askeleen liikaa, ja epäonnistunut ensimmäinen askel veisi aikaa.
- **Epäonnistuminen.** Komento epäonnistuu, kun sen runko palauttaa `false`. Muuten se onnistuu, myös ilman `return`-lausetta. Pelaajan epäonnistunut komento ei kuluta aikaa, ja moottori odottaa uutta syötettä. Jo tehdyt tilan muutokset jäävät voimaan, joten komento tarkistaa ehtonsa ennen muutoksia (moottoridokumentti, luku [6.1.5](../moottori/06-ajo.md#615-vuoron-kulku)).
- Sidottu komento `MOVE.NORTH = MOVE(north)` antaa parametrilliselle komennolle nimen näppäimiä varten.
  - **Jatkokomento.** Runko voi palauttaa komennon (`return ATTACK(other)`). Moottori ajaa sen samassa toiminnossa kutsujan tilalla, ja hinta ja epäonnistuminen ovat palautetun komennon. Ketju on sallittu, ja kutsusyvyyden raja pysäyttää kehän (moottoridokumentti, luku [6.1.5](../moottori/06-ajo.md#615-vuoron-kulku)).
- **Liike ja törmäys.** Moottorin `move` ei tarkista estoa. Vakiokirjaston liikekomento tarkistaa kohderuudun ja lähettää `STD.BUMP`-tapahtuman, kun ruudussa on estävä entiteetti (luku [4.2](04-tapahtumat-ja-kasittelijat.md#42-vakiokirjaston-tapahtumat)). Pelin oma silmukka käyttää samoja sääntöjä vakiokirjaston ytimillä `STD.CAN_STEP`, `STD.BLOCKER`, `STD.DO_STEP` ja `STD.STRIKE` (vakiokirjasto, luku [5](../vakiokirjasto/05-komennot.md)).

**Parametrin lähteet.** Lähde kertoo, miten moottori kysyy arvon pelaajalta. Tekoäly antaa arvon aina suoraan, ja pelaajan `back` peruu komennon ilman ajan kulumista.

| Lähde | Tyyppi | Kysymys |
| --- | --- | --- |
| `from JOUKKO` | jäsenen tyyppi | listavalikko (moottoridokumentti, luku [7.5](../moottori/07-esitys.md#75-valikot)) |
| `aim N [at JOUKKO]` | `pos` | kohdistus kantamalla ja ehdokkailla |
| `ask` | `dir` | yksi suuntanäppäin kontekstissa `DIRECTION` |
| `ask` | `pos` | viereinen ruutu `actor.at + suunta` yhdellä suuntanäppäimellä |
| `ask a..b` | `number` | lukukenttä välin rajoin |
| `ask N` | `text` | tekstikenttä, enintään N koodipistettä |
| `count` | `number` | toistolaskuri: luku, jonka pelaaja kirjoitti toiminnolla `repeat` ennen komennon näppäintä; oletus 1 |

```roguescript
command FIRE(dir: dir ask) cost 120:
    return SHOOT(from(actor, actor.RANGED.range, dir).first)

command UNLOCK(target: pos ask) cost 120:
    if not target in DOOR.locked:
        return false
    tile(target, DOOR.open)

command OFFER(amount: number ask 1..actor.PURSE.coin) cost 120:
    actor.PURSE.coin -= amount

command SEARCH(times: number count) cost 0:
    repeat times:
        for p in near(actor, 1) where it in DOOR.secret:
            if chance(20%):
                tile(p, DOOR)
        wait 120
        if HOSTILES(actor).count > 0:
            break
```

- **Toistolaskuri.** Moottorin toiminto `repeat` avaa lukukentän, ja ensimmäinen ei-numeronäppäin päättää sen ja tulkitaan komennon näppäimeksi: `#20s` etsii 20 kertaa (moottoridokumentti, luku [8](../moottori/08-syote.md)). Luku menee komennon `count`-parametriin, joten komento itse päättää, milloin toisto keskeytyy. Komennolla on enintään yksi `count`-parametri. Komento ilman sitä ei ota toistoa vastaan, ja moottori kirjaa ajon varoituksen.

## 7.2 Tekoäly

Tekoäly on funktio, joka palauttaa komennon. Se ei siis voi tehdä mitään, mihin pelaaja ei pysty.

```roguescript
ai GUARD:
    foe = nearest(HOSTILES(self), self)
    if foe == none:
        return WAIT()
    if foe.at in adjacent(self):
        return ATTACK(foe)
    return APPROACH(foe)
```

Vakiokirjaston `HUNTER` lisää tähän pakenemisen, kaukoaseet ja kuulon (vakiokirjasto, luku [6](../vakiokirjasto/06-tekoaly.md)).

- Moottorin komponentti `ai behavior HUNTER` valitsee funktion. Otsikossa ei ole parametreja eikä paluutyyppiä: `self` on tekoälyn ilmentymä, ja paluuarvo on aina komento.
- Jos palautettu komento epäonnistuu, entiteetti menettää vuoronsa: se saa seuraavan vuoron oletushinnan päästä.
- Paluuarvo `none` on lepo, joka kuluttaa oletushinnan. Vihollisuus lasketaan vakiokirjaston suhteista (`STD.ATTITUDE`) eikä `ai`-komponentista, joten pelaaja ilman tekoälyä on kelvollinen kohde.
- **Hyötypohjainen tekoäly** kirjoitetaan tavallisena `ai`-funktiona: se laskee vaihtoehdoille pisteet ja palauttaa parhaan komennon. Moottori ei valitse komentoja.
- **Tekoälyn kerrostus.** Tekoäly voi kutsua toista tekoälyä: `HUNTER()` toisen tekoälyn rungossa ajaa metsästäjän samalla `self`-arvolla ja palauttaa sen komennon. Kutsu on sallittu vain tekoälyn rungossa (luku [2.8](02-yleinen-syntaksi.md#28-ison-nimen-kutsu)), ja se kasvattaa kutsusyvyyttä kuten funktiokutsu.

```roguescript
ai ROGUE_MONSTER:
    if self has ASLEEP:
        return WAIT()
    if self has ERRATIC and chance(50%):
        return WANDER()
    return HUNTER()
```

- **Pelaajan tekoäly.** Pelaajan `ai`-komponenttia moottori ei kutsu pelaajan vuorolla, paitsi kun asetus `idle` laukeaa: silloin tekoälyn palauttama komento ajetaan pelaajan komentona (moottoridokumentti, luku [6.1.2](../moottori/06-ajo.md#612-toimijat-ja-ajoitin)). Logiikka voi vaihtaa pelaajan tekoälyn, esimerkiksi sekavuudessa (`CONFUSED`) tai automaattisessa tutkimisessa.

---

← [6. Ruudut ja paikkahaku](06-ruudut-ja-paikkahaku.md) · [Sisällys](README.md) · [8. Generointi](08-generointi.md) →
