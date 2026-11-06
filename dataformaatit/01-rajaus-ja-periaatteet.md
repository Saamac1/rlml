[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 1. Rajaus ja periaatteet

Roguelike Markup Language (RLML) kuvaa roguelike-pelin sisällön ja säännöt tekstitiedostoina. Tämä dokumentti kuvaa kielen dataosan: tiedostot, joiden sisältö on kuvausta eikä toimintaa. Dataformaatteja ovat

- pelin päätiedosto, pelikerran tila `game`, profiili `profile`, includet ja kansiorakenne,
- nimet, nimiavaruudet, korvaaminen ja paikkaaminen,
- komponentit, entiteetit ja kokoelmat,
- vakiot, pakat ja taulut,
- tilet, maastokerrokset, tilejoukot ja tilen ominaisuudet,
- kartat, prefabit, nimiöt ja paikkahaku,
- tyylitiedostot,
- kielitiedostot,
- käyttöliittymän data (asettelu, valikot, näppäimet, viestit, loki ja paneelit) ja
- äänten ilmoitukset.

## 1.1 Periaatteet

- **Merkitys, tekstit ja ulkoasu ovat erillään.** Kartat ja entiteetit kertovat, mitä jokin on. Tekstit ovat kielitiedostoissa ja ulkoasu tyylitiedostoissa, joten ASCII-versio ja graafinen versio tarvitsevat vain eri tyylitiedoston.
- **Yhteinen syntaksi.** Kaikki dataformaatit noudattavat samoja nimeämis-, rivi- ja luettelosääntöjä (luku [2](02-yleinen-syntaksi.md)).
- **Yksi sana, yksi merkitys.** Sanan merkitys ei riipu seuraavasta sanasta, kirjainkoosta tai paikasta. Arvot annetaan aina `NIMI arvo` -pareina, ja perintä kirjoitetaan aina sanalla `extends`.
- **Moduulit includella.** Peli ottaa mukaan vain tarvitsemansa tiedostot (luku [3](03-tiedostot-ja-include.md)).
- **Ominaisuudet ovat tarkoituksellisia.** Järjestelmä on olemassa vain, jos sen osio on kirjoitettu. Tyhjä peli ei sisällä mitään.
- **Tiedostot ovat markdownia.** Jokainen datatiedosto näyttää markdown-katselimessa luettavalta: otsikot ja lainaukset ovat kommentteja, ja piirrokset ja logiikka ovat koodilohkoissa.
- **Toistettavuus.** Satunnaisuus tulee pelin siemenestä, joten sama siemen tuottaa samat nopanheitot, pakkojen nostot, kokoelmien sisällöt ja aloitustilan.

## 1.2 Rajapinnat pelilogiikkaan

Pelilogiikan kieli on RogueScript, joka kuvataan omassa dokumentissaan. Se kirjoitetaan datatiedoston koodilohkoon, jonka kielitunniste on `roguescript`, tai omaan `.rogue`-tiedostoon. RogueScriptillä on oma syntaksinsa: sisennys merkitsee lohkoja, ja kommentit alkavat merkeillä `//`. Tämän dokumentin rivi-, lohko- ja kommenttisäännöt (luku [2](02-yleinen-syntaksi.md)) eivät siksi koske sitä. Nimet (luku [2.1](02-yleinen-syntaksi.md#21-nimet-ja-sanasto)), nimiavaruudet (luku [4](04-nimiavaruudet.md)) ja korvaaminen (luku [4.5](04-nimiavaruudet.md#45-korvaaminen-ja-override)) ovat yhteiset.

Datatiedostot viittaavat pelilogiikkaan muutamassa kohdassa. Näiden kohtien paikka kuuluu dataformaattiin, mutta niiden sisältö kuvataan RogueScript-dokumentissa:

| Kohta | Missä | Sisältö |
| --- | --- | --- |
| `hide when` ja `disabled when` | valikon kohta | ehto: piilotetaanko tai himmennetäänkö kohta |
| `where`-suodatin | konsolin kerros | ehto, jossa `it` on suodatettava jäsen |
| `zone VYÖHYKKEET` | kartan otsikko | vyöhykkeiden nimet |
| prefabin parametrit | prefabin otsikko | parametrien nimet |
| valikon `then`, `with` ja listavalikko | valikko | askel ja sidotut nimet |
| paneelin `value` ja parametrit | paneeli | kyselylauseke |
| `naming` | `game`-lohko | kysely, joka antaa entiteetin nimen pelin tilasta |
| `walk` ja `walk_stop` | `game`-lohko | klikkausmatkan kävelykomento ja pelin oma pysähtymisehto |
| paikalliset osiot | karttatiedosto | säännöt ja muut kartan vyöhykkeen osiot |
| `collection ... modifies` | kokoelmakenttä | jäsenten muunnokset kantajaan |
| checkpoint | tallennus (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)) | milloin tilannekuva otetaan ja milloin se ladataan |
| `begin` | aloitus (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)) | pelaajan luonti ja sijoitus jokaiselle istuimelle |
| `join` ja `part` | istuimet (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)) | liittyvän istuimen hahmo ja poistuvan istuimen jäsenet |
| siirtyminen | nimiöt (luku [8.5](08-kartat-ja-prefabit.md#85-nimiöt-ryhmät-ja-johtimet)) | milloin ja mille kartalle siirrytään |

- Datassa `when` esiintyy vain valikon kohdan ehdoissa `hide when` ja `disabled when`. Suodatin kirjoitetaan `where`, jossa `it` on jäsen (RogueScript, luku [5.3](../roguescript/05-funktiot-ja-lausekkeet.md#53-lausekkeet)), ja molemmat tulevat rivin loppuun. Pakan sisältöä ei rajata ehdoilla, vaan logiikka kokoaa pakan (luku [6.1](06-pakat-ja-taulut.md#61-pakka)).
- Kaksi datan lajia voidaan määritellä myös RogueScriptissä, ja muodot ovat sama laji: komponentti (luku [5.1](05-komponentit-ja-entiteetit.md#51-komponentit)) ja kartta (generoitu kartta, luku [8.1](08-kartat-ja-prefabit.md#81-karttatiedosto)).

Toiseen suuntaan logiikka käyttää datan arvoja ja rakenteita. Luvun [2.4](02-yleinen-syntaksi.md#24-arvot) arvojen kirjoitusasu on yhteinen, paitsi sijainti, joka kirjoitetaan RogueScriptissä `pos(5, 3)`. Luvun [8.8](08-kartat-ja-prefabit.md#88-paikkahaku-at-near-ja-from) paikkahaku vastaa RogueScriptin ruutujoukkoja (`area`), ja `game`-lohkon tila näkyy logiikalle kontekstinimellä `game` (luku [5.5](05-komponentit-ja-entiteetit.md#55-pelikerran-tila-game)) ja profiili kontekstinimellä `profile` (luku [3.5](03-tiedostot-ja-include.md#35-profiili-profile)).

---

[Sisällys](README.md) · [2. Yleinen syntaksi](02-yleinen-syntaksi.md) →
