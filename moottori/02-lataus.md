[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 2. Lataus

## 2.1 Tiedostojen haku

- **Aloitus.** Moottori lukee pelin päätiedoston ja seuraa sen `include`-rivejä, myös sisäkkäisiä ([D 3](../dataformaatit/03-tiedostot-ja-include.md)).
- **Hakupolku.** Polku etsitään järjestyksessä sisältävän tiedoston kansiosta, pelin juuresta ja moottorin vakiokirjastosta. Ensimmäinen osuma ladataan, joten pelin oma `lib/hp.rlml` estää vakiokirjaston version lataamisen.
- **Jokerit.** `*` ja `**` laajennetaan polun mukaiseen aakkosjärjestykseen. Moottori ohittaa tiedostot, joiden nimi alkaa merkillä `_` tai `.`, sekä tiedostot `README.md`, `README.rlml` ja `README.rlml.md`. Hyväksytyt päätteet ovat `.rlml`, `.md`, `.rlml.md` ja `.rogue`.
- **Kerran.** Sama tiedosto otetaan mukaan vain kerran. Mukaan otetun tiedoston sisältö latautuu include-rivin kohdalle syvyys ensin, joten latausjärjestys on tiedostojen lukujärjestys. Vakiokirjasto tulee mukaan vain include-rivillä; peli ilman sitä ei sisällä vakiokirjastoa.
- **Pelaajan korvaukset.** Moottori lataa käyttäjädatakansion `overrides/`-tiedostot viimeisinä ja käsittelee niiden lohkot kuin niissä olisi `patch` ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data), [D 4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)). `profile`-lohko niissä on latausvirhe ([D 3.5](../dataformaatit/03-tiedostot-ja-include.md#35-profiili-profile)).

## 2.2 Jäsennys

- **Markdown.** Otsikot (`#`) ja `>`-rivit ohitetaan. Muu vapaa teksti datan seassa on latausvirhe ([D 2.5](../dataformaatit/02-yleinen-syntaksi.md#25-kommentit)).
- **Koodilohkot.** Koodilohko on joko piirros (kartta, prefab, asettelu, paneeli), joka kuuluu edeltävään otsikkoon, tai RogueScriptiä, kun sen kielitunniste on `roguescript` ([D 2.2](../dataformaatit/02-yleinen-syntaksi.md#22-rivit-lohkot-ja-luettelot), [RS 2.1](../roguescript/02-yleinen-syntaksi.md#21-tiedostot)).
- **Kaksi syntaksia.** Datariveihin moottori soveltaa datan rivi- ja kommenttisääntöjä, RogueScriptiin sisennyssääntöjä: neljä välilyöntiä, sarkain on virhe ([RS 2.2](../roguescript/02-yleinen-syntaksi.md#22-lohkot-ja-kommentit)).
- **Merkistö.** Tiedostot ovat UTF-8:aa ja rivinvaihto on LF. Nimissä hyväksytään vain ASCII-merkit ([D 2.1](../dataformaatit/02-yleinen-syntaksi.md#21-nimet-ja-sanasto)).
- **Tiedoston nimiavaruus.** `namespace`-rivi koskee omaa tiedostoaan ja sen koodilohkoja, ei includeja ([D 4.2](../dataformaatit/04-nimiavaruudet.md#42-määrittely)).

## 2.3 Nimet

- Nimet ratkaistaan vasta, kun kaikki tiedostot on ladattu, joten määrittelyjen järjestys ei vaikuta hakuun ([D 4.4](../dataformaatit/04-nimiavaruudet.md#44-nimenhaku)).
- Nimenhaku kokeilee oman nimiavaruuden, ulommat nimiavaruudet, juuren ja lopuksi `STD`:n. Pisteellä alkava nimi viittaa suoraan juureen.
- Lajin tarkistus tehdään haun jälkeen. Haku ei ohita väärän lajista nimeä.
- Kenttäpolut (`target.UNDEAD.HP.now`) ratkaistaan sovittamalla isot osat pisimpään olemassa olevaan komponentin nimeen ([D 4.3](../dataformaatit/04-nimiavaruudet.md#43-nimiavaruuteen-kuuluvat-nimet), [RS 3.3](../roguescript/03-komponentit.md#33-nimiavaruudet)).
- Tilen indeksi ei ole osa nimeä haussa: `WATER.3` etsii tileä `WATER` ([D 4.4](../dataformaatit/04-nimiavaruudet.md#44-nimenhaku)).
- **Paikalliset nimet.** Kartan nimiöt ratkaistaan kartan sisällä, ja moottori tarkistaa, ettei nimiö ole sama kuin yleinen nimi, jonka haku löytäisi ([D 2.1](../dataformaatit/02-yleinen-syntaksi.md#21-nimet-ja-sanasto)).
- **Jokerit.** Jokeri laajennetaan ryhmän kaikkiin täysiin nimiin lajista riippumatta, ja tekstiavainten jokeri kaikkien ladattujen kielitiedostojen avainten yhdisteeseen ([D 2.1](../dataformaatit/02-yleinen-syntaksi.md#21-nimet-ja-sanasto), [RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)).

## 2.4 Korvaaminen ja paikkaaminen

- Myöhempi saman lajin määrittely samalla täydellä nimellä korvaa aiemman. Datan ja RogueScriptin komponentit ovat sama laji, samoin piirretyt ja generoidut kartat ([D 4.5](../dataformaatit/04-nimiavaruudet.md#45-korvaaminen-ja-override), [RS 3.2](../roguescript/03-komponentit.md#32-datassa-ja-skriptissä)).
- `override`-merkintä tarkistetaan koko latauksen jälkeen. Merkitty määrittely voittaa merkitsemättömän järjestyksestä riippumatta.
- Nimettömät osiot (`constants`, `tiles`, nimetön `game`- ja `profile`-lohko …) yhdistyvät riveittäin rivin avaimen mukaan ([D 4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- **Paikat.** `patch`-lohkot sovelletaan koko latauksen jälkeen latausjärjestyksessä ja vasta, kun `override` on ratkaistu, joten paikka koskee voittanutta määrittelyä. Pelaajan `overrides/`-tiedostot sovelletaan viimeisinä ([D 4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- Moottori listaa korvaukset ja paikat tiedoksi täysin nimin.

## 2.5 Tarkistukset

Moottori tarkistaa latauksessa kaiken, minkä voi tietää ilman pelin tilaa. Tärkeimmät:

| Alue | Tarkistus |
| --- | --- |
| nimet | puuttuva nimi, väärä laji, eri lajien törmäys, väärä kirjainkoko, nimen muoto, jokeri, jonka jäsenet ovat eri lajeja eikä käyttöpaikka rajaa lajia ([D 2.1](../dataformaatit/02-yleinen-syntaksi.md#21-nimet-ja-sanasto)) |
| korvaaminen | `override` ilman korvattavaa, kaksi `override`-määrittelyä samalle nimelle |
| paikat | `patch` ilman kohdetta, `patch` taululle, `terrain`-osiolle tai piirrokselle, uusi avain ilman `new`-merkintää, `new` olemassa olevalle avaimelle tai lohkolle, `drop` puuttuvalle avaimelle ([D 4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)) |
| aloitus | kaksi nimettyä `game`-lohkoa, sijoitus moottorin `game`-kenttään ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)), `call_depth` tai `loop_steps` rajojensa ulkopuolella (luku [2.6](#26-virheiden-lajit)), puuttuva style- tai language-asetus (luvut [7.2](07-esitys.md#72-kartan-piirtäminen) ja [7.3](07-esitys.md#73-tekstit)), `naming` ilman kyselyä tai väärän muotoinen kysely (luku [7.3](07-esitys.md#73-tekstit)), `layout` ilman asettelua tai monta asettelua ilman `layout`-kenttää (luku [7.1](07-esitys.md#71-asettelu)), `walk`-komento puuttuu tai sillä on muu kuin yksi `dir`-parametri, `walk_stop` ilman kyselyä tai väärän muotoinen kysely (luku [8.1](08-syote.md#81-hiiri)) |
| profiili | kaksi nimettyä `profile`-lohkoa, `profile`-lohko `overrides/`-tiedostossa ([D 3.5](../dataformaatit/03-tiedostot-ja-include.md#35-profiili-profile)) |
| kehät | pakat ja taulut ([D 6.1](../dataformaatit/06-pakat-ja-taulut.md#61-pakka)), tilejoukot ([D 7.3](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#73-tilejoukot-tileset)) |
| taulut | välit kattavat nopan alueen ilman aukkoja ja päällekkäisyyksiä, `all` tai `copy` taulun nostossa ([D 6.5](../dataformaatit/06-pakat-ja-taulut.md#65-taulut)) |
| tilet | joukko tai väli tuottavassa paikassa ([D 7.4](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#74-hakeva-ja-tuottava-paikka)), etunollat indeksissä |
| tyyli | sarjarivin avoin väli, graafisen sarjan välien eri pituudet, tyylin merkki, joka on yhdistävä merkki, liitosmerkki tai variaatiovalitsin ([D 9.2](../dataformaatit/09-ulkoasu.md#92-tilejen-ulkoasu-tilesglyphs)) |
| kartat | tuntematon merkki piirroksessa (rivi ja sarake), liian pitkä rivi, sijainti kartan ulkopuolella, puuttuva saapumisnimiö piirretyllä kohdekartalla (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)) |
| maasto | toinen `terrain`-osio, useampi `default`-kerros |
| komponentit | moottorin komponentin `component`-lohko, suuntatyyppien sekaannus, vanhentunut yhden kentän lyhenne |
| komennot | negatiivinen `cost`- tai `wait`-arvo (luku [6.1.3](06-ajo.md#613-hinta-ja-nopeus)), useampi kuin yksi `count`-parametri tai `count` muulle kuin `number`-parametrille (luku [8](08-syote.md)) |
| tekstit | paikkamerkit vastaavat viestin parametreja, `say`-kutsun argumentteja ([D 11.4](../dataformaatit/11-kayttoliittyman-data.md#114-viestit-ja-loki)), `{^TOIMINTO}` tuntemattomalle toiminnolle tai komennolle (luku [7.3](07-esitys.md#73-tekstit)), muu määrittely kielitiedostossa tai `language`-otsikko muualla kuin tiedoston alussa ([D 10](../dataformaatit/10-tekstit.md)) |
| valikot ja näppäimet | kohdan toiminto on olemassa, näppäinristiriita samassa kontekstissa, `final`-valikko ilman kohtaa, joka vie pois valikosta (luku [7.5](07-esitys.md#75-valikot)); varoitus, jos `ALWAYS`-kontekstin näppäin on sidottu myös muualla; tuntematon hiiren nimi ja useampi `hover`-alue asettelussa (luvut [7.1](07-esitys.md#71-asettelu) ja [8.1](08-syote.md#81-hiiri)) |
| paneelit | `param`-kentän nimi on otsikon parametri, asettelun tai `open`-kutsun argumentit vastaavat parametreja, kenttä on yhtenäinen vaakajakso tai suorakulmio (luku [7.7](07-esitys.md#77-paneelit)) |
| RogueScript | varattu sana paikallisena nimenä, `wait` tai `open` käsittelijässä, `fn`- tai `query`-funktiossa (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)), `say` käsittelijässä, `query` muuttaa tilaa, `say` reaaliaikaisessa taustakohtauksessa, tapahtuman kutsu ilman `emit`-sanaa ([RS 2.8](../roguescript/02-yleinen-syntaksi.md#28-ison-nimen-kutsu)), `return`-lauseke `fn`-funktiossa ilman paluutyyppiä ([RS 5.1](../roguescript/05-funktiot-ja-lausekkeet.md#51-funktioiden-lajit)), eri jäsenlajien sekoitus kokoelmassa ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)), kesto aikajanan kutsulle, jolla ei ole esitystä ([RS 9.1](../roguescript/09-kohtaukset-ja-aikajanat.md#91-rivi)), vakio avoin väli tuottavassa käytössä (`choose`, `for`, `pick`, `shuffled`; [RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)), `text_of` tilaa muuttavassa logiikassa (luku [7.3](07-esitys.md#73-tekstit)), paluutyyppi `name\|text` muussa kuin `naming`-kyselyssä, entiteetin nimen kutsu tuntemattomalla komponentilla tai kentällä ([RS 2.8](../roguescript/02-yleinen-syntaksi.md#28-ison-nimen-kutsu)) |
| moottorin funktiot | argumenttien tyypeille ei ole versiota (luku [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)), tilaa muuttava funktio kyselyssä tai esityksessä, esitys- tai tilaa muuttava funktio reaaliaikaisessa taustakohtauksessa, `external` generaattorin ulkopuolella (luku [9.2](09-moottorin-funktiot-ja-tapahtumat.md#92-funktion-kuvaus)), `idle`-asetuksen komento puuttuu tai sillä on parametreja (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)) |
| tapahtumat | `before` tai `cancel` moottorin tapahtumassa, parametrien määrä ei vastaa käsittelijän muotoa (luku [9.6](09-moottorin-funktiot-ja-tapahtumat.md#96-tapahtumat)); komponentin käsittelijä ei kirjoita subjektia, ja yleinen käsittelijä kirjoittaa sen (luku [6.2](06-ajo.md#62-tapahtumat)); käsittelijä saa jättää pois oletusarvoiset loppuparametrit ([RS 4](../roguescript/04-tapahtumat-ja-kasittelijat.md)) |
| aikajanat | aikayksikkö vastaa kelloa, silmukkaraidan rajan ylitys, merkki silmukassa |

**Piirrokset, perityt kartat, paneelit ja listavalikot.** Piirroksen merkit tarkistetaan NFC-normalisoinnin jälkeen: yhdistävä merkki, liitosmerkki (ZWJ) tai variaatiovalitsin on latausvirhe, ja leveä merkki antaa latausvaroituksen. `at:`-rivin sijainti kartan ulkopuolella, `extends` ilman pohjaa tai kehässä ja perivän kartan oma piirros ovat latausvirheitä. Paneelin ja listavalikon lausekkeet ovat kyselyjä, joten tilaa muuttava kutsu niissä on latausvirhe. Jokerista valitun tekstiavaimen kaikilla jäsenillä on oltava samat paikkamerkit (luku [7.3](07-esitys.md#73-tekstit)).

## 2.6 Virheiden lajit

| Laji | Milloin | Seuraus |
| --- | --- | --- |
| latausvirhe | latauksessa | peli ei käynnisty |
| latausvaroitus | latauksessa | peli käynnistyy, varoitus näytetään |
| käyttöhetken virhe | ajossa, esimerkiksi tyhjä paikkahaku datassa ([D 8.8](../dataformaatit/08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from)), puuttuva saapumisnimiö (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)), mutaatio tuhotulle ilmentymälle (luku [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)), kirjoitus puuttuvaan komponenttiin tai laskutoimitus `none`-arvolla ([RS 5.3](../roguescript/05-funktiot-ja-lausekkeet.md#53-lausekkeet)), silmukan askelrajan ylitys (`game.loop_steps`, [RS 5.2](../roguescript/05-funktiot-ja-lausekkeet.md#52-ohjausrakenteet-ja-muuttujat)), kutsusyvyyden ylitys (`game.call_depth`, luku [6.2](06-ajo.md#62-tapahtumat)), avoin väli tuottavassa käytössä ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)), listan kirjoitus alueen ulkopuolelle ([RS 2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)) | virheilmoitus kertoo kartan ja koko haun |
| ajon varoitus | ajossa, esimerkiksi puuttuvan komponentin kentän luku ([RS 5.3](../roguescript/05-funktiot-ja-lausekkeet.md#53-lausekkeet)), täysi kokoelma tai liian pieni joukko `pick`-kutsulle, `scatter`, jolta loppuvat vapaat ruudut (luku [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)), odottava komento hylätään ladattaessa, koska sen runko on muuttunut (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)), toistolaskuri komennolle, jolla ei ole `count`-parametria (luku [8](08-syote.md)), profiilin tarkistussumma ei täsmää (luku [5.6](05-aloitus-tallennus-ja-checkpointit.md#56-profiili)) | moottori kirjaa varoituksen ja jatkaa |

**Käyttöhetken virhe päättää toiminnon** (luku [6.1.7](06-ajo.md#617-toiminto)). Jo tehdyt muutokset jäävät voimaan, ja komento tulkitaan epäonnistuneeksi: pelaajalta ei kulu aikaa, ja tekoäly menettää vuoronsa (luku [6.1.5](06-ajo.md#615-vuoron-kulku)). Ajo jatkuu seuraavasta toiminnosta.

**Rajat.** `game`-lohkon kentät `call_depth` (oletus 256, sallittu 32–4096) ja `loop_steps` (oletus 1 000 000, vähintään 10 000) ovat pelin dataa ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)). Ne luetaan uuden pelin alussa ja tallentuvat pelikerran mukana kuten automaattisen tallennuksen paikkamäärä (luku [5.4](05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)), joten `overrides/` tai modi ei muuta niitä kesken pelin. Moottorin on tuettava kutsusyvyyttä 4096 asti, joten sama peli toimii kaikissa toteutuksissa. Rajojen ulkopuolinen arvo on latausvirhe (luku [2.5](#25-tarkistukset)).

Ilmoitukset kertovat aina täydet nimet ja kaikki asiaan liittyvät paikat: puuttuvasta nimestä kaikki kokeillut nimet, törmäyksestä molemmat määrittelyt, puuttuvasta paikan avaimesta lohkon olemassa olevat avaimet.

---

← [1. Rajaus ja periaatteet](01-rajaus-ja-periaatteet.md) · [Sisällys](README.md) · [3. Pelin tila](03-pelin-tila.md) →
