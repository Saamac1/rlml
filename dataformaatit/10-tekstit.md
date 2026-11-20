[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 10. Tekstit

Kielitiedosto (`text/fi.rlml`, `text/en.rlml`) alkaa otsikkorivillä `language KIELI`, ja sen jälkeen tulee yksi lista ilman `-`-merkkejä:

```
language fi
PAUSE.TITLE Tauko
PAUSE.RESUME Jatka
GOBLIN.NAME peikko
GOBLIN.DESC Pieni ja ilkeä luolien asukas.
MESSAGE.COMBAT.HIT Osuit kohteeseen {TARGET}, vahinko {AMOUNT}.
MESSAGE.LOOK {ITEM.DESC}
KING.TALK.TEXT Tervetuloa, matkalainen. Mitä haluat?
SHOP.BUY.ROW {ITEM} … {PRICE} kultaa
INTRO.START Aloita painamalla {^confirm}.
```

**Otsikko:** `language KIELI` on tiedoston ensimmäinen määrittely, ja se tekee koko tiedostosta kielitiedoston. Kieli on pieni nimi, sama kuin asetuksen `- language fi` arvo (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)). Tiedoston nimellä ja kansiolla ei ole merkitystä, kuten muillakaan tiedostoilla (luku [3](03-tiedostot-ja-include.md)). Saman kielen tiedostot yhdistyvät riveittäin, joten kirjasto ja peli voivat kumpikin tuoda omat `fi`-tekstinsä. Ennen otsikkoa saa olla vain `namespace`-rivi ja kommentteja (luku [4.2](04-nimiavaruudet.md#42-määrittely)). Muu määrittely kielitiedostossa tai `language`-otsikko muualla kuin tiedoston alussa on latausvirhe.

**Rivi:** avain on isoilla kirjaimilla, ja teksti alkaa ensimmäisen välilyönnin jälkeen ja jatkuu rivin loppuun. Teksti on aina yksi rivi; useampikappaleinen teksti, kuten ohjeruutu, kirjoitetaan paneelin kenttiin kappale kenttää kohti (luku [11.5](11-kayttoliittyman-data.md#115-paneelit)).

Avaimet tulevat määrittelyistä:

| Avain | Lähde |
| --- | --- |
| `<ENTITEETTI>.NAME`, `<ENTITEETTI>.DESC` | entiteetti (luku [5.2](05-komponentit-ja-entiteetit.md#52-entiteetit)) |
| `<VALIKKO>.TITLE`, `<VALIKKO>.TEXT`, `<VALIKKO>.<KOHTA>` | valikko (luku [11.2](11-kayttoliittyman-data.md#112-valikot)) |
| `<VALIKKO>.ROW`, `<VALIKKO>.EMPTY` | listavalikon rivi ja tyhjä lista |
| `<KOHTAUS>.<KOHTA>` | dialogin valinta (RogueScript, luku [9.7](../roguescript/09-kohtaukset-ja-aikajanat.md#97-dialogit)) |
| `<KOMENTO>.SENTENCE`, `<KOMENTO>.NO_TARGET` | verbin lauserivi, paikkamerkkeinä komennon parametrit, ja kelpaamattoman kohteen ilmoitus (luku [11.2](11-kayttoliittyman-data.md#112-valikot)) |
| `ENGINE.NEW_GAME`, `ENGINE.EXIT` | moottorin valikko, kun pelillä ei ole omaa alkuvalikkoa (moottoridokumentti, luku [8](../moottori/08-syote.md)) |
| `<LOKI>.MORE` | lokin sivutuksen kehote (luku [11.4](11-kayttoliittyman-data.md#114-viestit-ja-loki)) |
| `MESSAGE.*` tai muu viestin avain | viesti (luku [11.4](11-kayttoliittyman-data.md#114-viestit-ja-loki)) |
| `<KARTTA>.NAME` | kartan näkyvä nimi (luku [8.1](08-kartat-ja-prefabit.md#81-karttatiedosto)) |
| `<ATLAS>.NAME`, `<ATLAS>.<ALUE>.NAME` | atlaksen ja sen alueen näkyvä nimi; lehden alue on lehden avaimella `<ATLAS>.<LEHTI>.<ALUE>.NAME` (luku [8.12](08-kartat-ja-prefabit.md#812-atlas-karttalehdet)) |
| `<KARTTA>.<ALUE>.NAME`, `<KARTTA>.<NIMIÖ>.NAME` | alueen ja nimiön näkyvä nimi kartalla (luvut [8.11](08-kartat-ja-prefabit.md#811-nimetyt-alueet-areas) ja [9.6](09-ulkoasu.md#96-paperikartta-paper)) |

- **Paikkamerkit:** `{NIMI}` tuo tekstiin viestin parametrin, valikon sidotun nimen tai listavalikon rivin arvon. Entiteetin tekstiavain periytyy määrittelyssä: jos entiteetillä ei ole omaa `NAME`- tai `DESC`-tekstiä, käytetään lähimmän vanhemman avainta. Entiteetti näytetään nimenä avaimella `<ENTITEETTI>.NAME`, ellei `game`-lohkon `naming`-kysely anna sille muuta nimeä (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)). Istuimen jäsen haetaan jokaisella perimystasolla ensin avaimella `<ENTITEETTI>.SEATn.NAME` ja sitten avaimella `<ENTITEETTI>.NAME` (`HERO.SEAT2.NAME Eero`). Istuimilla on siis omat nimensä, ja lapsen oma nimi voittaa vanhemman istuinnimen. Sama koskee muita päätteitä, kuten `DESC`. `{@AVAIN}` tuo toisen tekstiavaimen tekstin samasta kielestä näyttöhetkellä (`MESSAGE.LOOT.RICH Löysit kasan {@GOLD.NAME}!`), joten käännökset ja ylikirjoitukset vaikuttavat kaikkialla. Lataus tarkistaa, että avain on olemassa ja ettei viitteistä synny kehää. `{NIMI:PÄÄTE}` näyttää parametrina annetun nimen tai entiteetin tekstin päätteellä `PÄÄTE` (`{SOUND:DESC}` näyttää kuvauksen `<LAJI>.DESC`); ilman päätettä pääte on `NAME`. `{#TILE}` näyttää tilen merkin valitussa esitystavassa (luku [9.2](09-ulkoasu.md#92-tilejen-ulkoasu-tilesglyphs)), myös muunnelman (`{#WALL.goldvein}`). Paneelissa merkki piirretään tyylin väreissä, joten selitteen rivi on `GLYPH.KEY {#GLYPH.KEY}  avain`, ja muualla, esimerkiksi lokissa, se on pelkkä merkki.
- **Näppäin:** `{^TOIMINTO}` näyttää toiminnon tai komennon ensimmäisen näppäimen sellaisena kuin se on nykyisessä kontekstipinossa sidottu, pelaajan `overrides/`-sidonnat mukaan lukien (`{^confirm}`, `{^QUAFF}`). Peliohjaimella näytetään painikkeen nimi. Pieni nimi on moottorin toiminto ja iso nimi komento kuten `keys`-rivillä (luku [11.3](11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)). Sitomaton toiminto näkyy nimenään, ja tuntematon nimi on latausvirhe. Verbivalikon rivien pikanäppäimet noudattavat samaa sääntöä, mutta moninpelissä ne näyttävät vuorossa olevan istuimen laitteen sidonnan, ja sitomaton jää tyhjäksi (luku [11.2](11-kayttoliittyman-data.md#112-valikot)).
- **Entiteetin kuvaus:** `{NIMI.DESC}` näyttää entiteettiarvon kuvauksen avaimella `<ENTITEETTI>.DESC`. Muita kenttiä paikkamerkissä ei sallita, ja muu arvo kuin entiteetti on latausvirhe.
- **Kirjain ja määrä:** `{NIMI:SLOT}` näyttää entiteetin kirjaimen kokoelmassaan (`a`, `b`, …), ja kartalla oleva entiteetti näkyy tyhjänä. `{NIMI:COUNT}` näyttää pinon koon (luku [5.4](05-komponentit-ja-entiteetit.md#54-kokoelmat)): `MESSAGE.LOOT.NOW Sinulla on nyt {ITEM:COUNT} × {ITEM} ({ITEM:SLOT}).`
- **Nimiarvo:** paikkamerkki, jonka arvo on nimi (esimerkiksi kuolinsyy `CAUSE.STARVATION` tai tekstiavain), näyttää nimen tekstin avaimella `<NIMI>.NAME`, jos se on entiteetin nimi, ja muuten avaimen tekstin. Tekstiarvo (`text`) näkyy sellaisenaan.
- **Tarkistus:** lataus tarkistaa, että viestin paikkamerkit vastaavat sen parametreja.
- **Nimiavaruus:** kielitiedosto voi aloittaa `namespace`-rivillä, ja sen avaimet ovat silloin suhteellisia (luku [4.2](04-nimiavaruudet.md#42-määrittely)).
- **Kieli** valitaan asetuksella `- language fi` (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)).
- **Puuttuva käännös** näkyy avaimena, jolloin puute näkyy heti pelissä. Oletuskieltä ei ole. Valikon puuttuva `<VALIKKO>.TEXT` tarkoittaa, ettei valikossa ole leipätekstiä.
- **Kartan avaimet.** Kartan nimessä voi olla pisteitä (`CAVE.DEEP`), joten avain `CAVE.DEEP.HALL.NAME` luetaan pisimmän kartan nimen mukaan: kartta `CAVE.DEEP` ja alue `HALL`. Alueen ja nimiön nimet eivät voi olla samoja (luku [8.11](08-kartat-ja-prefabit.md#811-nimetyt-alueet-areas)). Lisäksi kartan nimen ja alueen tai nimiön nimen yhdistelmä `<KARTTA>.<NIMI>` ei saa olla sama kuin yleinen nimi, esimerkiksi toisen kartan nimi tai entiteetti `CAVE.BAT`, kun kartalla `CAVE` on nimiö `BAT`. Muuten avaimen `CAVE.BAT.NAME` voisi lukea kahdella tavalla, joten yhdistelmä on latausvirhe kartassa, jossa alue tai nimiö on, ja avain on aina yksiselitteinen. Markdown-otsikko kartan edessä (`## Luola`) on vapaata tekstiä eikä nimeä karttaa.

```
language fi
MERIKARTTA.NAME Kapteeni Aaltolan kartta
MERIKARTTA.NORSUSAARI.NAME Norsusaari
MERIKARTTA.SYNKKA_SYVA.NAME Synkkä syvä
CAVE.DEEP.NAME Luolan alin kerros
CAVE.TO_DEEP.NAME Alas syvyyksiin
```

- **Kommentit:** kielitiedostossa toimivat vain `>`-rivit (luku [2.5](02-yleinen-syntaksi.md#25-kommentit)).

---

← [9. Ulkoasu](09-ulkoasu.md) · [Sisällys](README.md) · [11. Käyttöliittymän data](11-kayttoliittyman-data.md) →
