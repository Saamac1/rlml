[RLML](../README.md) › [RogueScript](README.md)

# 3. Komponentit

## 3.1 Määrittely

Komponentti on luokka. Sen kentät ovat konstruktorin parametreja, ja sen käsittelijät reagoivat tapahtumiin. Komponentti ilman käsittelijöitä mahtuu yhdelle riville.

```roguescript
component HP(now: number, max: number = now)

component ARMOR(value: number):
    before HIT(attacker, amount):
        amount = max(0, amount - value)

component REGENERATES(rate: number = 1):
    on turn():
        self.HP.now = min(self.HP.now + rate, self.HP.max)
```

- **Nimet käsittelijässä.** Komponentin omat kentät näkyvät paljaina niminä (`value`, `rate`). `self` on entiteetti, jolla komponentti on.
- **Subjekti jää pois.** Komponentin käsittelijä saa tapahtumat, joiden subjekti on `self` (kokoelman jäsenellä kantaja `holder`, luku [3.4](#34-kokoelman-jäsenet-ja-tilat)), joten subjektia ei kirjoiteta parametriksi. Sääntö on sama moottorin ja pelin tapahtumille: `before HIT(attacker, amount)` saa tapahtuman `HIT(target, attacker, amount, cause)` parametrit subjektin jälkeen, ja `on enter(p, prev)` tapahtuman `enter(who, p, prev)` parametrit. Yleinen käsittelijä kirjoittaa subjektin aina (luku [4](04-tapahtumat-ja-kasittelijat.md)).
- Entiteetin määrittely pysyy datassa ennallaan (`- ARMOR 2`). Entiteetillä ei ole muuta tyyppiä kuin komponenttinsa.
- **Lisäys ja poisto ajossa.** `add(e, ARMOR(value: 2))` lisää komponentin, ja `remove(e, ARMOR)` poistaa sen (moottoridokumentti, luku [9.5](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)). Kumpikaan ei lähetä tapahtumaa.

## 3.2 Datassa ja skriptissä

Komponentin voi määritellä datassa tai skriptissä, ja muodot ovat sama laji:

| Data | RogueScript |
| --- | --- |
| `component HP` | `component HP(…)` |
| `- now number` | `now: number` |
| `- max number default now` | `max: number = now` |
| `- deck deck copy` | `deck: deck copy` |
| `- loot table` | `loot: table` |
| `- order list name` | `order: list name` |
| `component BOSS` | `component BOSS` |

- **Yksi määrittely.** Täydellä nimellä on yksi määrittely. Toinen määrittely samalla nimellä on korvaus (luku [11.2](11-tallennus-ja-korvaaminen.md#112-korvaaminen)), kirjoitettiin se kummassa muodossa tahansa.
- Käsittelijät ovat mahdollisia vain skriptimuodossa.
- **Ei yhdistämistä.** Kenttiä ei yhdistetä kahdesta määrittelystä, koska `override` korvaa aina koko määrittelyn. Datan `patch` koskee entiteettien ja muiden lohkojen rivejä, ei komponentin kenttämäärittelyjä (dataformaatit, luku [4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).

## 3.3 Nimiavaruudet

Komponentit ja vakiot kuuluvat nimiavaruuksiin kuten muut yleiset nimet: tiedostossa `namespace UNDEAD` komponentti `HP` on `UNDEAD.HP`, ja lyhyt nimi löytyy tavallisella nimenhaulla.

Kenttäpolku `target.UNDEAD.HP.now` ratkaistaan latauksen jälkeen: polun isot osat sovitetaan pisimpään olemassa olevaan komponentin nimeen, ja loput osat ovat kenttiä. Pieni osa, jota seuraa sulku, on ketjukutsu (luku [2.7](02-yleinen-syntaksi.md#27-ketjukutsu)), ja polku päättyy sitä ennen.

**Törmäys.** Nimellä indeksoitava kenttä voi näyttää samalta kuin ryhmitelty komponentti (`RESIST.FIRE` kenttänä ja komponenttina `FIRE` nimiavaruudessa `RESIST`). Tällainen törmäys on latausvirhe, joka kertoo molemmat paikat.

## 3.4 Kokoelman jäsenet ja tilat

`modifies`-kokoelmien jäsenet (tilat ja puetut varusteet) saavat kantajansa tapahtumat. Niissä `holder` on kantaja ja `self` jäsen itse. Kantaja on tapahtuman subjekti, joten sitä ei kirjoiteta parametriksi (luku [3.1](#31-määrittely)).

Tila on tavallinen komponentti tai entiteetti `modifies`-kokoelmassa. Se poistaa itsensä, kun sen aika loppuu.

```roguescript
component FIRE_WARD(percent: percent = 50%):
    before HIT(attacker, amount) when attacker has FIRE:
        amount = amount * (100% - percent)
```

Kun koko reppu on `modifies`-kokoelma, jokainen sen esine saa kantajan tapahtumat. Vaikutus, joka koskee vain puettua esinettä, rajataan vartijalla, esimerkiksi merkillä `WORN`: `on turn() when self has WORN:`.

---

← [2. Yleinen syntaksi](02-yleinen-syntaksi.md) · [Sisällys](README.md) · [4. Tapahtumat ja käsittelijät](04-tapahtumat-ja-kasittelijat.md) →
