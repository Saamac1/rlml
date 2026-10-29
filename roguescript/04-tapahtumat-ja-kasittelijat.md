[RLML](../README.md) › [RogueScript](README.md)

# 4. Tapahtumat ja käsittelijät

Tapahtuma on nimetty viesti, jolla on tyypitetyt parametrit. Ensimmäinen parametri on tapahtuman subjekti.

```roguescript
namespace STD

event HIT(target: entity, attacker: entity, amount: number, cause: name = none)
event DIED(victim: entity, killer: entity, cause: name = none)

on HIT(target, attacker, amount, cause) as APPLY_DAMAGE:
    target.HP.now -= amount
    if target.HP.now <= 0:
        emit DIED(target, attacker, cause)

after HIT(target, attacker, amount) as HIT_MESSAGE priority -10:
    log(MESSAGE.COMBAT.HIT, TARGET: target, AMOUNT: amount)

on DIED(victim, killer) as DEATH priority -10:
    if victim has DROPS:
        spawn(victim.DROPS.loot, at: victim)
    destroy(victim)
```

Esimerkki on vakiokirjaston tapahtumat `STD.HIT` ja `STD.DIED` lyhennettyinä. Normatiivinen koodi on vakiokirjaston luvuissa [4](../vakiokirjasto/04-tapahtumat.md) ja [8](../vakiokirjasto/08-kasittelijat.md).

**Lähetys.** `emit HIT(target, self, 5)` lähettää tapahtuman, ja käsittelijät ajetaan ennen kuin `emit` palaa. Puuttuvat loppuargumentit saavat oletusarvonsa.

**Käsittelijä ei odota.** `wait` ja `say` käsittelijässä (`before`, `on`, `after`) ovat latausvirheitä. Tapahtuma voidaan lähettää mistä tahansa, myös `on turn()`-käsittelijästä, jolloin odotukselle ei olisi omistajaa. Lisäksi saman tapahtuman myöhemmät käsittelijät jäisivät odottamaan. Esitystä tarvitseva käsittelijä laittaa kohtauksen jonoon (`start`, luku [9](09-kohtaukset-ja-aikajanat.md)). Kohtaus alkaa toiminnon lopussa siirtojen jälkeen, joten sama käsittelijä voi siirtää toimijan toiselle kartalle ja käynnistää siellä pelattavan kohtauksen:

```roguescript
component REUNION:
    on TOUCH(robot):
        travel(robot, STAGE, at: LEFT)
        travel(self, STAGE, at: RIGHT)
        start(ENDING(robot, self))
```

- **Vaiheet.** `before` voi muuttaa parametreja tai perua tapahtuman (`cancel`). `on` tekee varsinaisen työn, ja `after` reagoi lopputulokseen.
- **Jakelu.** Tapahtuma menee subjektin komponenteille, subjektin `modifies`-kokoelmien jäsenille ja yleisille käsittelijöille. Järjestyksen ratkaisevat prioriteetti ja latausjärjestys (alla).
- **Subjekti parametreissa.** Yleinen käsittelijä kirjoittaa subjektin ensimmäiseksi parametriksi. Komponentin käsittelijä jättää sen pois, koska subjekti on `self` tai kokoelman jäsenellä `holder` (luku [3.1](03-komponentit.md#31-määrittely)). Sääntö koskee kaikkia tapahtumia, myös moottorin (luku [4.1](#41-moottorin-tapahtumat)).
- **Otsikon ehto.** `when` käsittelijän otsikossa ennen kaksoispistettä rajaa, milloin käsittelijä ajetaan.
- **Nimi.** `as NIMI` nimeää yleisen käsittelijän, jotta sen voi korvata (`override on HIT(...) as STD.APPLY_DAMAGE:`). Komponentin käsittelijä korvataan korvaamalla komponentti.
- **Oletusarvoiset loppuparametrit.** Käsittelijän otsikko saa jättää pois tapahtuman loppuparametrit, joilla on oletusarvo: `on HIT(target, attacker, amount)` käy tapahtumalle `HIT(target, attacker, amount, cause: name = none)`. Parametri, jolla ei ole oletusta, on pakollinen, ja sen puuttuminen on latausvirhe. Tapahtuman laajentaminen uudella oletusarvoisella loppuparametrilla ei siksi riko olemassa olevia käsittelijöitä. Pois jätetty parametri ei näy käsittelijässä, eikä `before`-vaihe voi muuttaa sitä.
- **Prioriteetti ja järjestys.** Käsittelijät järjestetään kunkin vaiheen alussa yhteen listaan. Ensin ratkaisee prioriteetti, suurin ensin: otsikkoon kirjoitetaan `priority N` tai `priority -N`, ja oletus on 0. Sitten ratkaisee latausjärjestys eli käsittelijän määrittelyn paikka tiedostoissa; `override` ja `patch` säilyttävät alkuperäisen paikan. Jos sama käsittelijä osuu useaan kohteeseen, subjekti tulee ennen `modifies`-jäseniä ja jäsenet luontijärjestyksessä. Ilman prioriteetteja järjestys on siis latausjärjestys, eikä se riipu entiteetin perinnästä tai `add`-kutsuista.
- **Muutokset kesken jakelun.** Lista lasketaan kunkin vaiheen (`before`, `on`, `after`) alussa. Jos käsittelijä lisää komponentin, sen käsittelijät tulevat mukaan vasta seuraavasta vaiheesta. Jos käsittelijä poistaa komponentin tai jäsenen ennen kuin sen vuoro tulee, sen käsittelijät ohitetaan.

**Vakiokirjaston vyöhykkeet.** Kun vakiokirjaston include on pelin ensimmäinen, sen käsittelijät ovat latausjärjestyksessä ennen pelin omia, joten samalla prioriteetilla ne ajetaan ensin (vakiokirjasto, luku [2](../vakiokirjasto/02-prioriteettivyohykkeet.md)). Vakiokirjasto käyttää siksi sovittuja tasoja, joiden suhteen peli asettaa omat sääntönsä:

| Prioriteetti | Tehtävä | Esimerkki |
| --- | --- | --- |
| 10 | muokkaa parametreja | suojat, heikkoudet |
| 0 | soveltaa | vahinko, parannus |
| -10 | reagoi | viestit, kuolema |

Tasot ovat dokumentoitu käytäntö, eivät moottorin sääntöjä.

```roguescript
component HOLY_SHIELD:
    before HIT(attacker, amount) priority 10:
        if attacker has UNDEAD:
            cancel
```

**Kuollut subjekti.** `emit`, jonka subjekti on tuhottu, ei jaa tapahtumaa vaan palaa heti ilman virhettä. Jos subjekti tuhoutuu kesken jakelun, jakelu jatkuu loppuun, ja loput käsittelijät näkevät kuolleen kahvan. Kuollut kahva säilyttää lajinsa, joten `is`, `.kind` ja tekstin nimi toimivat, mutta sen kentät ovat `none` (moottoridokumentti, luvut [6.2](../moottori/06-ajo.md#62-tapahtumat) ja [9.5](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)). Siksi `EXPLODE` (luku [5.1](05-funktiot-ja-lausekkeet.md#51-funktioiden-lajit)) ei kaadu, vaikka sama räjähdys tappaa olennon kesken silmukan.

## 4.1 Moottorin tapahtumat

Moottorin tapahtumat ovat `begin` (luku [10](10-aloitus-ja-checkpointit.md)), `turn`, `wake`, `created`, `removed`, `enter`, `leave`, arrive, depart ja hear (`hear(who, s)`, komponentissa `hear(s)`, moottoridokumentti luku [6.8](../moottori/06-ajo.md#68-äänet-ja-kuuleminen)). Niiden nimet ovat pieniä, koska ne ovat moottorin sanastoa. Moottori lähettää ne itse, kun se on tehnyt muutoksen, joten niillä on vain vaiheet `on` ja `after`. `before` ja `cancel` niissä ovat latausvirheitä. Toiminnon estäminen kuuluu komentoon.

Yleinen käsittelijä saa subjektin ensimmäisenä parametrina. Komponentin käsittelijässä subjekti on `self`, eikä sitä kirjoiteta, kuten pelin omissakin tapahtumissa (luku [4](#)). Parametrien nimet ovat vapaat, mutta niiden määrän on vastattava muotoa.

| Tapahtuma | Yleinen | Komponentissa | Milloin |
| --- | --- | --- | --- |
| `begin` | `begin()` | – | kerran uuden pelin alussa (luku [10.1](10-aloitus-ja-checkpointit.md#101-aloitus)) |
| `turn` | `turn()`, subjekti `game` | `turn()` | kierroksen rajalla |
| `wake` | `wake(who)` | `wake()` | toimijan jokaisella herätyksellä |
| `created` | `created(e)` | `created()` | ilmentymä on valmis ja sijainnissaan; kokoelman jäsenet ennen kantajaa |
| `removed` | `removed(e)` | `removed()` | `destroy` alkaa; ilmentymä on vielä ehjä ja paikallaan |
| `enter` | `enter(who, p, prev)` | `enter(p, prev)` | entiteetti astui ruutuun `p`; `prev` on lähtöruutu tai `none` kokoelmasta tultaessa |
| `leave` | `leave(who, p)` | `leave(p)` | entiteetti poistui ruudusta `p` |
| `arrive` | `arrive(who, map, p)` | `arrive(map, p)` | entiteetti saapui kartalle ruutuun `p` |
| `depart` | `depart(who, map, p)` | `depart(map, p)` | entiteetti lähti kartalta ruudusta `p` |

- **Ruutu ja kartta.** `enter` ja `leave` koskevat ruutua, `arrive` ja `depart` karttaa (luku [6.4](06-ruudut-ja-paikkahaku.md#64-siirtyminen-kartalta-toiselle)). Siirto kokoelmaan on lähtö kartalta ja siirto kokoelmasta maahan saapuminen.
- **Siirron jälkeen.** Moottori siirtää entiteetin ensin ja lähettää tapahtumat sitten, joten käsittelijä näkee sen uudessa sijainnissa. Ruutujoukkofunktiot käyttävät kontekstin `map`-karttaa, ja `arrive`- ja `depart`-käsittelijässä se on tapahtuman kartta.
- **Syntymä ja tuho** eivät lähetä ruutu- ja karttatapahtumia: `created` ja `removed` korvaavat ne.
- `turn` lähetetään kierroksen rajalla aktiivisten karttojen ilmentymille ja kerran yleisille käsittelijöille, joissa subjekti on `game`. Sillä ei ole parametreja, ja kierros luetaan kentästä `game.turn` (`on turn():`).
- `wake` lähetetään toimijalle sen jokaisella herätyksellä, myös monivaiheisen komennon jatkossa. Tila, joka vaikuttaa toimijan omaan tahtiin, kirjoitetaan tähän (moottoridokumentti, luvut [6.1.4](../moottori/06-ajo.md#614-mikroaskeleen-vaiheet) ja [6.1.5](../moottori/06-ajo.md#615-vuoron-kulku)).
- **`removed` ja `remove`.** `remove(e, KOMP)` poistaa komponentin eikä lähetä tapahtumaa. `removed` kuuluu ilmentymän tuholle. Sen käsittelijä voi pudottaa tavarat ennen kuin kokoelmien jäsenet tuhotaan.

Tapahtumien järjestys kussakin sijainnin muutoksessa on moottoridokumentin luvussa [9.6](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#96-tapahtumat).

## 4.2 Vakiokirjaston tapahtumat

Moottori lähettää tapahtuman vain, kun se tekee muutoksen itse eikä tapahtumassa ole pelin käsitettä. Törmäys, vierekkäisyys, esineen saaminen ja kuolema tulkitsevat muutosta pelin säännöillä, joten ne ovat vakiokirjastoa (moottoridokumentti, luku [9.7](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#97-raja-vakiokirjastoon)):

| Vakiokirjasto | Mitä tekee |
| --- | --- |
| `STD.BUMP(who, other)` | liikekomento lähettää, kun kohderuudussa on estävä entiteetti, eikä siirrä |
| `STD.ADJACENT(who, other)`, `STD.APART(who, other)` | lähetetään, kun kaksi entiteettiä tulee viereen tai erkanee |
| `STD.GIVE(to, card)` | synnyttää kortin vastaanottajan reppuun |
| `STD.DIED(victim, killer, cause)` | lähetetään, kun vahinko vie hengen (luku [4](#)) |

Koska nämä ovat tavallista RogueScriptiä, peli voi korvata tai poistaa ne. Vierekkäisyys rakennetaan moottorin tapahtumista:

```roguescript
namespace STD

event ADJACENT(who: entity, other: entity)
event APART(who: entity, other: entity)

fn GIVE(to: entity, card: card):
    spawn(card, into: to.PACK.items)

query NEIGHBOURS(who: entity, around: area) -> collection:
    return entities(in: near(around, 1)) where it != who

fn MEET(who: entity, others: collection):
    for e in others:
        emit ADJACENT(who, e)
        emit ADJACENT(e, who)

fn PART(who: entity, others: collection):
    for e in others:
        emit APART(who, e)
        emit APART(e, who)

after enter(who, p, prev) as ADJACENCY_MOVE when prev != none:
    was = NEIGHBOURS(who, prev)
    now = NEIGHBOURS(who, p)
    PART(who, was where not it in now)
    MEET(who, now where not it in was)

after arrive(who, map, p) as ADJACENCY_ARRIVE:
    MEET(who, NEIGHBOURS(who, p))

after depart(who, map, p) as ADJACENCY_DEPART:
    PART(who, NEIGHBOURS(who, p))

after created(e) as ADJACENCY_CREATED:
    MEET(e, NEIGHBOURS(e, e))

after removed(e) as ADJACENCY_REMOVED:
    PART(e, NEIGHBOURS(e, e))
```

Kokoelmassa olevan ilmentymän naapurusto on tyhjä (luku [6.1](06-ruudut-ja-paikkahaku.md#61-joukon-tuottavat-funktiot)), joten repussa syntyvä tai tuhoutuva esine ei lähetä vierekkäisyyttä. Kokoelmasta maahan tuleva entiteetti saa vierekkäisyytensä `arrive`-tapahtumasta, ja sen `enter` ohitetaan, koska `prev` on `none`.

---

← [3. Komponentit](03-komponentit.md) · [Sisällys](README.md) · [5. Funktiot ja lausekkeet](05-funktiot-ja-lausekkeet.md) →
