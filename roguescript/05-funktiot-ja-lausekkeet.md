[RLML](../README.md) › [RogueScript](README.md)

# 5. Funktiot ja lausekkeet

## 5.1 Funktioiden lajit

Funktioita on kolmea lajia, ja laji kertoo, mitä funktio saa tehdä:

| Laji | Saa muuttaa pelin tilaa | Satunnaisuus | Käyttö |
| --- | --- | --- | --- |
| `fn` | kyllä | pelin virta | efektit ja toiminnot |
| `query` | ei (lataus tarkistaa) | kopio pelin virrasta | kyselyt ja laskenta |
| `present fn` | ei | esityksen virta | esitys: ääni, kamera, valot |

```roguescript
fn HURT(target: entity, amount: number, source: entity = none, cause: name = none):
    emit HIT(target, source, amount, cause)

fn EXPLODE(center: pos, radius: number, damage: dice):
    for e in entities(in: near(center, radius)) where it has HP:
        HURT(e, damage)
    for p in near(center, radius) where it.COVER in WET:
        tile(p, none, layer: COVER)
    play(SFX.EXPLOSION, at: center)

query WOUNDED(of: entity) -> collection:
    return entities(in: near(of, 5)) where it has HP and it.HP.now < it.HP.max
```

Vakiokirjaston kysely `HOSTILES` on vakiokirjaston luvussa [6](../vakiokirjasto/06-tekoaly.md).

Moottorin funktioilla on sama jako (muuttaa, kysely, esitys), ja lataus tarkistaa, ettei `query` tai `present fn` kutsu tilaa muuttavaa funktiota (moottoridokumentti, luku [9.2](../moottori/09-moottorin-funktiot-ja-tapahtumat.md#92-funktion-kuvaus)).

**Paluutyyppi.** `fn`-funktion paluutyyppi on valinnainen: `fn NEAREST_FOE(of: entity) -> entity:`. Ilman paluutyyppiä funktio ei palauta arvoa, ja `return lauseke` on latausvirhe. `query`-funktion paluutyyppi on pakollinen.

**Funktio ei odota.** `wait` ja `open` ovat latausvirheitä kaikissa funktioissa (`fn`, `query`, `present fn`), kuten käsittelijöissäkin. Odottava komento on siksi aina yksi kehys, ja sen kohta tallentuu ilman kutsupinoa (luku [11.1](11-tallennus-ja-korvaaminen.md#111-tallennus)). Odotusta tarvitseva toiminto kirjoitetaan komennoksi tai korutiinikohtaukseksi.

## 5.2 Ohjausrakenteet ja muuttujat

Ohjausrakenteet: `if` / `elif` / `else`, `for x in …`, `repeat N`, `while`, `break`, `match` ja `return`.

Muuttujat: `x = …` sitoo paikallisen nimen, ja `+=` sekä `-=` muuttavat arvoa. Listaan `+=` lisää jäsenen (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)). Noppa heitetään, kun se luetaan tai sidotaan.

`match` vertaa arvoa kohtiin järjestyksessä. Kohta on arvo, väli tai `else`:

```roguescript
match self.MOOD.value:
    0:
        return REST()
    1..3:
        return WANDER()
    else:
        return APPROACH(player)
```

**Silmukka ja muuttuva joukko.** `for` käy läpi joukon sellaisena kuin se oli silmukan alussa. Jäsenet, jotka ovat sillä välin tuhoutuneet, ohitetaan, eivätkä silmukan aikana syntyneet tule mukaan. `where` arvioi lähteensä samoin kerran.

**`while` ja `break`.** `while ehto:` toistaa runkoa, kunnes ehto on epätosi. `break` päättää sisimmän `for`-, `repeat`- tai `while`-silmukan. Moottori keskeyttää silmukan, joka ylittää askelrajan `game.loop_steps` (oletus 1 000 000 kierrosta), käyttöhetken virheellä, jotta virheellinen ehto ei jumita peliä.

```roguescript
tries = 0
while rooms.count < want:
    tries += 1
    if tries > 200:
        break
```

**Kutsusyvyys.** Sisäkkäiset funktiokutsut ja tapahtumat kasvattavat samaa syvyyttä: `fn`- ja `query`-kutsut, `emit` sekä moottorin tapahtumat, jotka moottorin funktio lähettää käsittelijän sisällä. Syvyyden raja on `game.call_depth` (oletus 256). Rekursiivinen funktio ja toisiaan laukaisevat tapahtumat pysähtyvät siis samaan rajaan.

**Käyttöhetken virhe** päättää käynnissä olevan toiminnon. Jo tehdyt muutokset jäävät voimaan, ja komento tulkitaan epäonnistuneeksi (luku [7.1](07-komennot-ja-tekoaly.md#71-komennot)). Sama koskee askelrajan ja kutsusyvyyden ylitystä (moottoridokumentti, luvut [2.6](../moottori/02-lataus.md#26-virheiden-lajit) ja [6.2](../moottori/06-ajo.md#62-tapahtumat)).

## 5.3 Lausekkeet

- vertailu `==` `!=` `<` `<=` `>` `>=` ja logiikka `and`, `or`, `not`
- `has KOMP` ja `has no KOMP`
- `is ENTITEETTI` vertaa entiteetin lajia perintä mukaan lukien. Oikea puoli voi olla myös entiteettiarvo, jolloin verrataan lajeja (`it is item` on tosi samalle lajille tai sen perijälle), tai nimiarvo, esimerkiksi kentästä: `it is e.FACTION.group`. Muut nimet, esimerkiksi kartat, verrataan operaattorilla `==` (`map == FIELD`). Tarkka laji ilman perintää on `e.kind == POTION.HEALING`.
- `in`: väli (`d in 2..4`), kokoelma, lista, ruutujoukko, hakujoukko tai sanakirjan avaimet. `pos in FLOOR` on tosi, jos jonkin ruudun kerroksen tile on hakujoukon jäsen (dataformaatit, luku [7.4](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#74-hakeva-ja-tuottava-paikka)).
- aritmetiikka `+ - * / %` kokonaisluvuilla. Jakolasku pyöristää alaspäin, ja jakojäännös `a % b` on `a - (a / b) * b`, joten se on positiivisella jakajalla välillä 0…b − 1 (`-1 % 3` on 2). Jako ja jakojäännös nollalla ovat käyttöhetken virheitä. Prosentti kertoo luvun ja pyöristää alaspäin (`amount * 50%`), ja prosenttien summa ja erotus on prosentti (`100% - percent`).
- **Suunnat.** `-d` on suunnan vastasuunta (`-north` on `south`, `-front` on `back`), ja `-none` on `none`.
- **Tekstit.** `a + b` yhdistää kaksi `text`-arvoa. Nimen tai luvun voi muuttaa tekstiksi vain `text_of`-funktiolla (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)).
- kenttäpolut (`target.HP.now`, `map.DEPTH`, `game.KING_QUEST`, `profile.HALL.scores`), entiteetin moottorikentät (`e.kind`, `e.slot`, `e.stack`, luku [2.6](02-yleinen-syntaksi.md#26-tyypit)) ja sijainnit (`self.at`, `self.at + north`, koordinaatit `p.x` ja `p.y`)
- kokoelman, listan, joukon ja pakan ominaisuudet `.count`, `.first` ja `.last`. Moottorin palauttamat joukot ovat määrätyssä järjestyksessä (entiteetit luontijärjestyksessä, ruudut lukujärjestyksessä). Pakalla on vain `.count`, joka on nostopinon korttien määrä, eikä taululla ole ominaisuuksia (luku [5.5](#55-pakat-ja-taulut)).
- funktiot `min()`, `max()`, `distance()`, `nearest()`, `sees()` ja `chance()` sekä joukkofunktiot (luku [5.4](#54-satunnaisuus)) ja pakkafunktiot (luku [5.5](#55-pakat-ja-taulut)), moottorin funktiot myös ketjumuodossa (luku [2.7](02-yleinen-syntaksi.md#27-ketjukutsu))
- suodatin `joukko where ehto` (alla)

**Suodatin `where`.** `joukko where ehto` käy jäsenet läpi, arvioi ehdon jokaiselle ja palauttaa uuden joukon, jossa ovat ne jäsenet, joille ehto pätee. Ehdossa `it` on käsiteltävä jäsen.

```roguescript
entities() where it has HP
near(self, 1) where it in FLOOR and entities(in: it).count == 0
```

- Lähde voi olla mikä tahansa joukko: kokoelma, lista, ruutujoukko, `entities()`-tulos, jokeri (`ITEM.*`) tai tilen indeksiväli.
- Tulos on samaa lajia kuin lähde, ja jäsenten järjestys säilyy. Suodatin ei muuta lähdettä.
- **Sitovuus.** `where` sitoo löyhimmin, joten koko sen oikea puoli on ehtoa, eikä `and`- tai `or`-osia tarvitse sulkea.
- **Ketjutus.** `a where x where y` suodattaa ensin ehdolla `x` ja sitten tuloksen ehdolla `y`. Vastaanottajana suodatettu joukko tarvitsee sulut: `(near(self, 1) where it in FLOOR).choose()` (luku [2.7](02-yleinen-syntaksi.md#27-ketjukutsu)).
- `for`-otsikossa sama suodatin rajaa silmukan: `for e in entities() where it has HP:`.
- **Sisäkkäinen `it`.** Sisemmän suodattimen `it` peittää ulomman: lausekkeessa `entities() where (it.PACK.items where it is GOLD).count > 0` sisempi `it` on kokoelman jäsen. Jos sisemmässä ehdossa tarvitaan ulompaa jäsentä, se sidotaan ensin nimeen `for`-silmukalla.

**`when` ja `where`.** `when` päättää, tapahtuuko jotain, eikä siinä ole `it`-nimeä. `where` valitsee jäsenet joukosta ja tuottaa arvon. `when` on logiikan sana, ja se esiintyy kahdessa paikassa:

- Käsittelijän otsikossa (`on`, `before`, `after`) ennen kaksoispistettä. Se rajaa, milloin käsittelijä ajetaan.
- Aikajanan rivillä toiminnon paikalla (`when ehto:`, luku [9.4](09-kohtaukset-ja-aikajanat.md#94-ehdot)).

Datassa `when` esiintyy vain valikon muodoissa `hide when` ja `disabled when` (alla). Datan suodattimet, kuten konsolin kerros, käyttävät `where`-sanaa, ja pakan kortit valitaan kokoamalla pakka logiikassa (luku [5.5](#55-pakat-ja-taulut)).

**`hide when` ja `disabled when`** ovat `choice`-kohdan ja valikon kohdan ehtoja. Kun ehto pätee, `hide when` piilottaa kohdan ja `disabled when` himmentää sen: kohta näkyy, mutta sitä ei voi valita (luku [9.7](09-kohtaukset-ja-aikajanat.md#97-dialogit)). Molemmat kertovat, mitä tapahtuu ehdon päteessä, joten kohta ilman ehtoja on aina näkyvissä ja valittavissa.

**Puuttuva komponentti.** Kentän luku entiteetiltä, jolla ei ole komponenttia (`target.HP.now` ilman HP:tä), antaa `none`, ja moottori kirjaa ajon varoituksen, joka kertoo polun. Vertailu `== none` ja `!= none` toimii. Laskutoimitus tai suuruusvertailu `none`-arvolla on käyttöhetken virhe, samoin kirjoitus puuttuvaan komponenttiin: komponentti lisätään `add`-kutsulla. Kun komponentin puute on odotettu, `has` tarkistaa sen ilman varoitusta.

## 5.4 Satunnaisuus

Satunnaisuutta on kolme virtaa, ja kutsuja ratkaisee, mitä virtaa käytetään:

- **Pelin virta** alkaa pelin siemenestä, ja sitä käyttävät `fn`, komento, käsittelijä, tekoäly ja generaattori. Virran tila kuuluu tallennukseen.
- **Kyselyn kopio.** `query` käyttää kopiota pelin virran nykytilasta. Kopio sekoitetaan kutsukontekstiin (kutsuva entiteetti, jos sellainen on, ja kyselyn nimi), joten eri kutsujat saavat eri tuloksen. Kysely ei siirrä pelin virtaa eteenpäin, joten sama kysely samassa tilassa antaa saman tuloksen.
- **Esityksen virta** on erillinen, ja sitä käyttävät `present fn` ja kohtausten esitysrivit (luku [9.8](09-kohtaukset-ja-aikajanat.md#98-luonnos-ohitus-ja-alusta-aloitus)). Ohitettu ja katsottu kohtaus kuluttavat siksi pelin virtaa yhtä paljon.

**Heitto kuuluu `fn`-funktioon.** Osuma, vahinko ja muu heitto, joka ratkaisee pelin kulun, kirjoitetaan `fn`-funktioksi paluutyypin kanssa. `query` heittäisi kopiosta, joten kaksi iskua samassa tilassa osuisi aina samoin:

```roguescript
// Oikein: pelin virta etenee jokaisella iskulla
fn SWING(att: entity, def: entity) -> bool:
    return choose(1..20) + HIT_BONUS(att) >= 20 - LEVEL_OF(att) - ARMOR_CLASS(def)

// Väärin: sama tulos niin kauan kuin tila ei muutu
query SWING_WRONG(att: entity, def: entity) -> bool:
    return choose(1..20) + HIT_BONUS(att) >= 20 - LEVEL_OF(att) - ARMOR_CLASS(def)
```

`query` sopii satunnaisuuteen, joka ei ratkaise mitään, kuten tekoälyn harkintaan tai paneelin vaihtelevaan vihjeeseen.

Joukkofunktiot käyttävät kutsujansa virtaa. Taulukossa ne ovat ketjumuodossa, ja funktiomuoto (`choose(joukko)`) on sama kutsu (luku [2.7](02-yleinen-syntaksi.md#27-ketjukutsu)):

| Funktio | Palauttaa |
| --- | --- |
| `joukko.choose()` | yhden satunnaisen jäsenen, tyhjästä joukosta `none` |
| `joukko.shuffled() -> collection` | uuden joukon, jossa on kaikki jäsenet satunnaisessa järjestyksessä; ruutujoukosta lista `list pos` |
| `joukko.pick(n) -> collection` | `n` eri jäsentä, eli sekoitetun joukon `n` ensimmäistä; ruutujoukosta lista `list pos` |
| `chance(p)` | `true` todennäköisyydellä `p` |

- Joukko on kokoelma, lista, ruutujoukko, `where`-suodatettu joukko, jokeri (`ITEM.*`) tai tilen indeksiväli (`GLYPH.NKI.1..20`). Jokeri ja indeksiväli tuottavat nimiä, eivät ilmentymiä. Listan `shuffled` ja `pick` palauttavat listan.
- **Ruutujoukko.** Ruutujoukolla ei ole omaa järjestystä lukujärjestyksen lisäksi, joten `shuffled` ja `pick` palauttavat siitä listan ruutuja (`list pos`) arvontajärjestyksessä. Listasta saa takaisin ruutujoukon sijoittamalla sen `area`-paikkaan, esimerkiksi `near(spots[0], 1)`; joukko-operaatiot hyväksyvät listan ruuduista.
- **`shuffled` ja `shuffle`.** `joukko.shuffled()` palauttaa uuden joukon eikä muuta mitään. `pakka.shuffle()` sekoittaa pakan tilan (luku [5.5](#55-pakat-ja-taulut)). Muoto kertoo, kumpi on kyseessä.
- **Jokeri ei kata indeksejä.** `GLYPH.NKI.*` vastaa ryhmän nimettyjä jäseniä. Indeksit valitaan välillä, joten tulos ei riipu tyylitiedostosta.
- **Liian pieni joukko:** jos jäseniä on vähemmän kuin `n`, `pick` palauttaa kaikki jäsenet ja moottori kirjaa varoituksen.
- **Tasainen arvonta ilman taulua.** Kun kaikilla vaihtoehdoilla on sama todennäköisyys eikä arvonnalla ole tilaa, jokeri ja `pick` tai `choose` korvaavat taulun.
- **Luku välistä.** `choose(4..9)` arpoo kokonaisluvun väliltä, päät mukaan lukien. Väli toimii joukkona myös muille joukkofunktioille, kun sillä on molemmat päät (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)).

```roguescript
for item in ITEM.*.pick(18):
    scatter(item, count: 1, in: tiles() - near(home, 1))

component DISGUISE:
    on created():
        // Väli vastaanottajana tarvitsisi sulut, joten funktiomuoto on selvempi
        self.render.tile = choose(GLYPH.NKI.1..20)
```

## 5.5 Pakat ja taulut

Pakka (dataformaatit, luku [6](../dataformaatit/06-pakat-ja-taulut.md)) on logiikassa arvo, jonka tyyppi on `deck`, ja taulu (dataformaatit, luku [6.5](../dataformaatit/06-pakat-ja-taulut.md#65-taulut)) arvo, jonka tyyppi on `table`. Pakan nimi (`LOOT.ARTIFACT`) viittaa jaettuun pakkaan, `deck copy`-kenttä ilmentymän omaan kopioon ja `.copy()` palauttaa uuden kopion. Pakkafunktiot ovat tavallisia moottorin funktioita. Taulukossa ne ovat ketjumuodossa, ja `pakka.draw(3)` on sama kutsu kuin `draw(pakka, 3)` (luku [2.7](02-yleinen-syntaksi.md#27-ketjukutsu)).

| Funktio | Tekee |
| --- | --- |
| `pakka.shuffle() -> deck` | sekoittaa nostopinon tasaisesti satunnaiseen järjestykseen |
| `pakka.draw(n = 1) -> collection card` | nostaa `n` korttia päältä ja siirtää ne poistopakkaan; `n` on luku, noppa tai `all` |
| `taulu.draw(n = 1) -> collection card` | heittää taulun nopan `n` kertaa ja palauttaa osuneiden rivien kortit; `n` on luku tai noppa |
| `pakka.peek(n = 1) -> collection card` | palauttaa `n` päällintä korttia nostamatta niitä |
| `pakka.put(kortti, bottom: false, shuffle: false) -> deck` | lisää kortin nostopinon päälle, pohjalle (`bottom: true`) tai satunnaiseen kohtaan (`shuffle: true`) |
| `pakka.add(toinen, shuffle: false) -> deck` | lisää toisen pakan kaikki kortit nostopinon päälle; `shuffle: true` sekoittaa pakan lisäyksen jälkeen |
| `pakka.reset() -> deck` | palauttaa poistopakan nostopinoon ja kortit kirjoitusjärjestykseen |
| `pakka.copy() -> deck` | uuden pakan määrittelyn alkutilasta (dataformaatit, luku [6.3](../dataformaatit/06-pakat-ja-taulut.md#63-kopiot)) |
| `pakka.count` | nostopinon korttien määrä (kenttä, ei kutsu) |

```roguescript
on DIED(victim, killer) when victim has BOSS:
    spawn(LOOT.ARTIFACT, at: victim)

component MOVES(deck: deck copy):
    on turn():
        if deck.count == 0:
            deck.reset().shuffle()
        for card in deck.draw():
            emit USE_MOVE(self, card)

fn DEAL_HAND(to: entity):
    for card in LOOT.RARE.copy().shuffle().draw(3):
        GIVE(to, card)
```

- **Kortit.** `draw` palauttaa nostetut kortit nostojärjestyksessä. Taulun rivi tuottaa samanlaisen kortin kuin pakan kortti. Kortti on `card`-arvo: kohteen nimi, määrä ja kenttien ylikirjoitukset. Se ei ole vielä ilmentymä, vaan `spawn(kortti, at: …)`, `spawn(kortti, into: …)`, vakiokirjaston `GIVE(entiteetti, kortti)` tai generaattorin `scatter(kortti, …)` synnyttää sen. Kortin määrä synnyttää niin monta ilmentymää (dataformaatit, luku [6.1](../dataformaatit/06-pakat-ja-taulut.md#61-pakka)). Kartan kortti on pelkkä nimi. Logiikka rakentaa kortin entiteetin nimen kutsulla (`RING(ENCHANT: 2)`, luku [2.8](02-yleinen-syntaksi.md#28-ison-nimen-kutsu)).
- **Sisäkkäiset nostot ja tyhjät kortit.** Pakka- tai taulukortti nostetaan heti sisemmästä pakasta tai taulusta, joten `draw` palauttaa vain lopullisia kortteja. `none`-kortti nostetaan ja siirretään poistopakkaan, ja `none`-rivi heitetään, mutta kumpaakaan ei palauteta.
- `peek` ei muuta pakkaa, joten se on sallittu myös `query`-funktiossa. Se näkee vain nostopinon.
- `put` ottaa kortin (`card`-arvo, esimerkiksi `draw`- tai `peek`-kutsun tulos tai entiteetin nimen kutsu) tai entiteetin nimen. Kortin ei tarvitse olla pakan määrittelyssä, joten logiikka voi lisätä pakkaan uusia kortteja, kuten kirouksen. `bottom` ja `shuffle` yhdessä on virhe.
- `add` litistää toisen pakan: sen kortit tulevat yksitellen, eivät yhtenä sisäkkäisenä korttina. Kortit otetaan toisen pakan määrittelystä samaan tapaan kuin `copy`, joten toisen pakan nykytila ei vaikuta niihin eikä `add` muuta sitä. Sisäkkäinen kortti `- LOOT.RARE` on eri asia: se on yksi kortti, joka nostaa sisemmästä pakasta.
- `reset` palauttaa pakan määrittelyn mukaiseksi, joten se poistaa `put`- ja `add`-kutsuilla lisätyt määrittelyn ulkopuoliset kortit.
- **Ketju.** `shuffle`, `put`, `add` ja `reset` palauttavat pakan, joten ne voi ketjuttaa: `deck.reset().shuffle()`.
- **Pakan kokoaminen.** Pakan korteilla ei ole ehtoja. Tilanteesta riippuva pakka kootaan logiikassa, tyypillisesti tason alussa: `reset` tyhjentää edellisen kokoonpanon ja `add` tai `put` lisää tähän tilanteeseen kuuluvat kortit. Määrittelyssä tyhjä pakka on sallittu, kun logiikka täyttää sen.

```roguescript
fn BUILD_LOOT(depth: number):
    LOOT.LEVEL.reset()
    if depth > 13:
        LOOT.LEVEL.add(LOOT.DEEP)
    if depth >= 20:
        LOOT.LEVEL.add(LOOT.ARTIFACT_HINTS)
```

- **Nosto ja synnytys yhdellä kutsulla.** `spawn(pakka, at: …)` ja `spawn(pakka, into: …)` nostavat yhden kortin ja synnyttävät sen, kuten datan nosto. Sama toimii taululle.
- Säännöt toimivat kuten datassa: `shuffled`-pakka sekoittaa poistopakkansa takaisin ehtyessään, ja muu pakka palauttaa vain jäljellä olevat kortit (dataformaatit, luku [6.2](../dataformaatit/06-pakat-ja-taulut.md#62-tila-sekoitus-ja-nosto)).
- **Taulut.** Taulu on tilaton, joten siihen käyvät vain `draw` ja `spawn` (sekä `scatter` generaattorissa). `shuffle`, `peek`, `put`, `add`, `reset`, `copy` ja `.count` ovat taululle latausvirheitä. Funktiot hyväksyvät pakan tai taulun ensimmäisen parametrin tyypin mukaan. Yhteistä tyyppiä ei ole, joten parametri tai kenttä on `deck` tai `table`, ja logiikka valitsee lähteen itse:

```roguescript
fn DROP_LOOT(victim: entity, depth: number):
    if depth > 13:
        spawn(LOOT.DEEP, at: victim)
    else:
        spawn(victim.DROPS.loot, at: victim)
```

- **Tila.** Jaetun pakan ja `deck copy`-kentän kopion muutokset kuuluvat pelin tilaan ja tallentuvat. Paikallinen kopio (`hand`) on olemassa vain kutsun ajan, paitsi odottavan komennon paikallisena nimenä, jolloin se tallentuu komennon mukana (luku [11.1](11-tallennus-ja-korvaaminen.md#111-tallennus)). Taulusta ei tallennu mitään.
- **Funktion laji.** `shuffle`, `draw`, `put`, `add` ja `reset` jaetulle pakalle tai ilmentymän kopiolle muuttavat pelin tilaa, joten ne ovat latausvirheitä `query`- ja `present fn`-funktioissa. `query` saa käsitellä paikallista kopiota, koska se ei muuta pelin tilaa. Taulusta nosto ei muuta tilaa, joten se on sallittu `query`-funktiossa, jossa se käyttää kyselyn virtaa.
- **Satunnaisuus.** `shuffle`, `copy` ja taulun `draw` käyttävät kutsujan virtaa (luku [5.4](#54-satunnaisuus)).
- **Luonnos.** Etualan kohtauksessa pakkojen muutokset tehdään luonnokseen kuten muut tilan muutokset (luku [9.8](09-kohtaukset-ja-aikajanat.md#98-luonnos-ohitus-ja-alusta-aloitus)).

---

← [4. Tapahtumat ja käsittelijät](04-tapahtumat-ja-kasittelijat.md) · [Sisällys](README.md) · [6. Ruudut ja paikkahaku](06-ruudut-ja-paikkahaku.md) →
