[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 6. Pakat ja taulut

Pakka ja taulu tuottavat entiteettejä kokoelmiin, karttaan sekä toisiin pakkoihin ja tauluihin. Ne eroavat tilassa:

- **Pakka** on nimetty luettelo kortteja. Sen järjestys ja poistopakka ovat pelin tilaa, joten nostettu kortti palaa vain pakan säännön mukaan. Pakasta, josta nostetaan kaikki kortit kirjoitusjärjestyksessä, tulee lista.
- **Taulu** on tilaton noppa. Jokainen nosto heittää nopan uudestaan, ja tulos valitsee rivin. Taulu ei ehdy eikä tallennu.

Pakka sopii, kun nostettu ei saa toistua (artefaktit, aloitusvarusteet, liikesarjat). Taulu sopii, kun jokainen arvonta on itsenäinen (saalis, kohtaamiset, aloituskartan valinta).

## 6.1 Pakka

```
deck LOOT.ARTIFACT
- SWORD.VORPAL
- AMULET.YENDOR
- RING.SPEED

deck KIT.FIGHTER
- LEATHER_ARMOR
- DAGGER
- RATION 3
- LOOT.POTIONS 1d2
```

- **Otsikko:** `deck NIMI [shuffled]`. Sääntö kuvataan luvussa [6.2](#62-tila-sekoitus-ja-nosto).
- **Kortti** on rivi `- KOHDE [MÄÄRÄ] [KOMPONENTTI arvo ...]`.
- **Ei painoa.** Kortilla ei ole painoa, ja rivin ensimmäinen sana on aina kohde. Monta samaa korttia kirjoitetaan omille riveilleen. Todennäköisyydet kirjoitetaan tauluun (luku [6.5](#65-taulut)).
- **Kohde** on entiteetti, nosto toisesta pakasta tai taulusta (luvut [6.2](#62-tila-sekoitus-ja-nosto) ja [6.5](#65-taulut)), `none` (tyhjä kortti) tai pakan käyttöpaikan mukainen nimi, kuten kartta (`game`-lohkon `start`, luku [3.3](03-tiedostot-ja-include.md#33-aloitus)). Lajia ei kirjoiteta, koska nimi on yksikäsitteinen (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). Tilejoukko ei kelpaa kohteeksi, koska siitä ei voi tuottaa yhtä tileä (luku [7.3](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#73-tilejoukot-tileset)).
- **Määrä** on noppa kohteen jälkeen. Entiteettikortti synnyttää niin monta samanlaista ilmentymää (`RATION 3` synnyttää kolme annosta), ja `merge`-kokoelmassa ne näkyvät yhtenä pinona (luku [5.4](05-komponentit-ja-entiteetit.md#54-kokoelmat)). Ilman `merge`-sääntöä ne ovat erillisiä jäseniä, ja kartalla ne ovat samassa ruudussa. Pakka- tai taulukortissa sama paikka on noston määrä (`LOOT.POTIONS 1d2`).
- **Kenttien ylikirjoitus:** `KOMPONENTTI arvo` asettaa syntyvän ilmentymän kentän (luku [6.4](#64-kenttien-ylikirjoitus)). Kortin kaikki ilmentymät saavat saman arvon, ja noppa heitetään kerran korttia kohti, jotta ne ovat samanlaisia.
- **Ei ehtoja.** Kortilla ei ole ehtoa. Jos pakan sisällön pitää riippua tilanteesta, esimerkiksi tason syvyydestä, logiikka kokoaa pakan ennen nostoja: `reset` palauttaa pakan määrittelyn mukaiseksi, ja `add` yhdistää siihen toisen pakan kortit (RogueScript, luku [5.5](../roguescript/05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)).
- **Tyhjä pakka** on sallittu määrittely (`deck SPAWN.MONSTER` ilman kortteja), kun logiikka täyttää sen. Nosto tyhjästä pakasta ei tuota mitään.
- Kortteja lisää toisesta tiedostosta `patch deck` (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- Kehä pakkojen ja taulujen välillä on latausvirhe.

## 6.2 Tila, sekoitus ja nosto

- **Jaettu tila.** Jokaisella pakalla on pelissä yksi tila: nostopino järjestyksineen ja poistopakka. Kaikki nostot samasta pakasta käyttävät samaa tilaa, ellei nosto tee kopiota (luku [6.3](#63-kopiot)). Tila tallentuu tilannekuvaan (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)).
- **Alkutila.** Nostopinossa ovat kaikki kortit kirjoitusjärjestyksessä, ja poistopakka on tyhjä. `shuffled`-pakka sekoitetaan aloitustilassa (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)).

**Säännöt.** Pakan sääntö kertoo, miten nostetut kortit palaavat:

| Sääntö | Nostettu kortti | Sekoitus |
| --- | --- | --- |
| ei sääntöä | menee poistopakkaan eikä palaa | ei koskaan datassa |
| `shuffled` | menee poistopakkaan ja palaa, kun nostopino loppuu | aloitustilassa ja aina, kun poistopakka palaa |

Itsenäinen arvonta, jossa sama tulos voi toistua ja jota ei tallenneta, kirjoitetaan tauluna (luku [6.5](#65-taulut)). Pakalla ei ole sääntöä, joka palauttaisi kortin heti.

**Sekoitus** järjestää nostopinon tasaisesti satunnaiseen järjestykseen. Sekoitus käyttää pelin satunnaisvirtaa. Datassa pakan sekoittaa vain sen sääntö `shuffled`, ja logiikka sekoittaa pakan funktiolla `shuffle` (RogueScript, luku [5.5](../roguescript/05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)).

**Nosto** kirjoitetaan `PAKKA [N | all] [copy]`. Se ottaa nostopinosta N korttia päältä, ja nostetut kortit siirtyvät poistopakkaan.

- N on luku tai noppa, ja sen oletus on 1. `all` nostaa koko nostopinon.
- **Järjestys:** nostetut kortit tuottavat entiteettinsä nostojärjestyksessä, joten kokoelma antaa niille kirjaimet samassa järjestyksessä.
- `copy` nostaa kertakopiosta (luku [6.3](#63-kopiot)).
- **Nostopaikat** ovat kokoelmakentän arvo ja oletus (luku [5.4](05-komponentit-ja-entiteetit.md#54-kokoelmat)), legendin `with` (luku [8.2](08-kartat-ja-prefabit.md#82-legend)), pakan kortti, taulun rivi ja `game`-lohkon `start` (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)). Samat paikat hyväksyvät taulun (luku [6.5](#65-taulut)). Logiikassa pakasta ja taulusta nostetaan funktioilla `draw` ja `spawn`, ja vakiokirjaston `GIVE` antaa nostetun kortin entiteetin reppuun (RogueScript, luvut [4.2](../roguescript/04-tapahtumat-ja-kasittelijat.md#42-vakiokirjaston-tapahtumat) ja [5.5](../roguescript/05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)).
- **Ehtyminen.** Jos nostopino loppuu kesken noston, `shuffled`-pakka palauttaa poistopakan nostopinoon, sekoittaa sen ja jatkaa. Muu pakka antaa vain jäljellä olevat kortit, ja tyhjästä pakasta nosto ei tuota mitään. `LOOT.ARTIFACT` jakaa siis jokaisen artefaktin enintään kerran pelissä. `all` nostaa nostopinon sellaisenaan eikä käynnistä uudelleensekoitusta.
- **Sisäkkäinen nosto.** Kortti, jonka kohde on pakka tai taulu, on itse nosto. Kortin N ja `copy` koskevat sisempää nostoa.
- `none`-kortti ei tuota mitään, mutta se nostetaan ja siirretään poistopakkaan kuten muutkin kortit.
- **Nimetty nosto.** Usein toistuvan noston voi nimetä yhden rivin tauluksi. Taulun `table CHEST_LOOT 1` rivi `- 1 LOOT.CHEST 1d3 copy` on itse nosto, joten `CHEST items CHEST_LOOT` tekee saman noston joka kerta.
- **Toistettavuus.** Sekoitukset ja nopat käyttävät pelin virtaa nostojärjestyksessä, joten sama siemen tuottaa samat nostot.

## 6.3 Kopiot

Kopio on uusi pakka, jonka tila alkaa pakan määrittelystä: kaikki kortit ovat nostopinossa kirjoitusjärjestyksessä, ja `shuffled`-pakan kopio sekoitetaan syntyessään. Kopio noudattaa pakan sääntöä. Jaetun pakan nykytila ei vaikuta kopioon, eikä kopio muuta jaettua pakkaa.

```
entity CHEST.OLD
- render GLYPH.CHEST
- CHEST items LOOT.CHEST 1d3 copy

entity PLAYER
- PACK items KIT.FIGHTER all copy

component MOVES
- deck deck copy

entity GOBLIN
- MOVES deck MOVES.GOBLIN
```

- **Kertakopio.** Nosto, jossa on `copy`, tekee pakasta tilapäisen kopion, nostaa siitä ja hylkää kopion. Jokainen arkku arpoo näin itsenäisesti, ja samat aloitusvarusteet voi jakaa monelle pelaajalle.
- **Omistettu kopio.** Kentän tyyppi `deck copy` antaa jokaiselle ilmentymälle oman kopion, kun ilmentymä syntyy (luku [5.1](05-komponentit-ja-entiteetit.md#51-komponentit)). Kopio säilyy ilmentymän mukana ja tallentuu sen mukana. Kenttä ilman `copy`-sanaa on viittaus jaettuun pakkaan.
- **Kopio logiikassa.** RogueScriptin `copy(PAKKA)` palauttaa uuden pakan (RogueScript, luku [5.5](../roguescript/05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)).
- **Sisäkkäiset pakat.** Kopion kortti, joka nimeää toisen pakan, nostaa jaetusta pakasta, ellei kortissa ole `copy`-sanaa. Sama koskee taulun riviä. Tavallisen saaliin taulussa voi siis olla jaettu artefaktipakka (`- 20 LOOT.ARTIFACT`), joka ehtyy koko pelin osalta.
- **Taululla ei ole kopiota.** Taulu on tilaton, joten `copy` taulun nostossa on latausvirhe (luku [6.5](#65-taulut)).
- **Satunnaisuus.** Kopion sekoitus käyttää pelin virtaa kopion syntyhetkellä.

## 6.4 Kenttien ylikirjoitus

Syntyvän ilmentymän kenttiä voi ylikirjoittaa kaikkialla, missä data synnyttää entiteetin. Noppa heitetään syntyhetkellä, ja tulos tallentuu ilmentymään:

```
table LOOT.ARMOR 1d10
- 1..2 RING_MAIL ENCHANT -1..3
- 3 PLATE_MAIL ENCHANT -2..2
- 4..6 ARROW 3d4 ENCHANT 0
- 7..10 none
```

- Pakan kortissa ja taulun rivissä `KOMPONENTTI arvo` (iso nimi) ylikirjoittaa kentän, ja noppa ilman nimeä on määrä. Molemmat saa kirjoittaa samalle riville.
- Kartan legendissä sama toimii `with`-osassa: `- ! FLOOR with RING_MAIL ENCHANT 2` (luku [8.2](08-kartat-ja-prefabit.md#82-legend)). Myös moottorin komponentit käyvät (`with LIFT label GATE_A`).
- `card`-tyyppinen kenttä (luku [5.1](05-komponentit-ja-entiteetit.md#51-komponentit)) tallentaa saman kirjoitusasun myöhempää synnytystä varten.
- Kirjoitusasu on sama kuin entiteetin määrittelyssä, joten yhden kentän komponentilla kentän nimi jää pois (`ENCHANT -1..3`).
- RogueScriptissä sama kortti rakennetaan entiteetin nimen kutsulla: `RING_MAIL(ENCHANT: 2)` (RogueScript, luku [2.8](../roguescript/02-yleinen-syntaksi.md#28-ison-nimen-kutsu)).

## 6.5 Taulut

```
table FRUITS 1d20
- 1..13 APPLE 1d6
- 14 TREASURE
- 15..20 none

table LOOT.COMMON 2d6
- 2..3 POTION.HEAL
- 4..10 COIN 2d6
- 11 LOOT.RARE copy
- 12 LOOT.ARTIFACT
```

- **Otsikko:** `table NIMI NOPPA`. Noppa on mikä tahansa luvun [2.4](02-yleinen-syntaksi.md#24-arvot) noppa tai vakio (`1d20`, `2d6`, `1d6+2`, `1`).
- **Rivi** on `- TULOS KOHDE [MÄÄRÄ] [KOMPONENTTI arvo ...]`. Tulos on luku tai väli (`14`, `1..13`), ja rivin loppu on kuten pakan kortissa (luku [6.1](#61-pakka)).
- Rivin alun luku on aina heiton tulos, koska nimet alkavat kirjaimella (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)). Avoin väli (`15..`) ulottuu nopan suurimpaan tulokseen.
- **Kattavuus.** Välien on katettava nopan koko alue ilman aukkoja ja päällekkäisyyksiä: taulun `2d6` rivit kattavat tulokset 2–12. Aukko, päällekkäisyys tai alueen ulkopuolinen tulos on latausvirhe, joka kertoo rivit. Taulua ei siksi voi paikata riveittäin (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- **Todennäköisyys** tulee välin leveydestä ja nopan jakaumasta. Taulun `FRUITS` rivi `1..13` osuu 13 kertaa 20:stä, ja `2d6` painottaa keskimmäisiä tuloksia.
- **Kohde** on entiteetti, nosto pakasta tai toisesta taulusta, `none` tai taulun käyttöpaikan mukainen nimi, kuten kartta (`game`-lohkon `start`, luku [3.3](03-tiedostot-ja-include.md#33-aloitus)). Tilejoukko ei kelpaa kohteeksi (luku [7.3](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#73-tilejoukot-tileset)).
- **Tilaton.** Jokainen nosto heittää nopan uudestaan, ja sama tulos voi toistua. Taululla ei ole nostopinoa eikä poistopakkaa, joten se ei ehdy eikä tallennu (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)).
- **Nosto** kirjoitetaan `TAULU [N]` kuten pakasta. N on luku tai noppa, ja sen oletus on 1. Jokainen nosto heittää nopan erikseen, joten `LOOT.COMMON 3` voi tuottaa kolme kolikkopinoa. `all` ja `copy` ovat taululle latausvirheitä, koska taululla ei ole nostopinoa eikä tilaa, josta tehdä kopio.
- **Nostopaikat** ovat samat kuin pakalla (luku [6.2](#62-tila-sekoitus-ja-nosto)), ja laji ratkeaa nimenhaun jälkeen (luku [4.4](04-nimiavaruudet.md#44-nimenhaku)). Komponentin kenttä on poikkeus: kenttätyyppi `table` hyväksyy vain taulun ja `deck` vain pakan (luku [5.1](05-komponentit-ja-entiteetit.md#51-komponentit)).
- **Sisäkkäinen nosto.** Rivi, jonka kohde on pakka tai taulu, on itse nosto. Pakkaan osoittava rivi nostaa jaetusta pakasta, ellei rivissä ole `copy`-sanaa (`- 11 LOOT.RARE copy`).
- `none`-rivi ei tuota mitään.
- **Ei ehtoja.** Taulun rivillä ei ole ehtoa. Tilanteesta riippuva arvonta valitaan logiikassa (RogueScript, luku [5.5](../roguescript/05-funktiot-ja-lausekkeet.md#55-pakat-ja-taulut)).
- **Toistettavuus.** Heitot käyttävät pelin virtaa nostojärjestyksessä, joten sama siemen tuottaa samat tulokset.

---

← [5. Komponentit ja entiteetit](05-komponentit-ja-entiteetit.md) · [Sisällys](README.md) · [7. Tilet, maastokerrokset, tilejoukot ja ominaisuudet](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md) →
