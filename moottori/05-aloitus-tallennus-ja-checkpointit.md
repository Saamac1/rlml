[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 5. Aloitus, tallennus ja checkpointit

## 5.1 Uusi peli

Uuden pelin tila on laskettu oletustallennus, checkpoint 0 ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus), [RS 10](../roguescript/10-aloitus-ja-checkpointit.md)). Moottori laskee sen järjestyksessä:

1. siemen: `game`-lohkon `seed`, tai uusi arvottu siemen, jos kenttä puuttuu,
2. profiilin tilannekuva: moottorin profiilitiedosto tai, jos sitä ei ole, `profile`-lohkon alkuarvot (luku [5.6](#56-profiili)),
3. `shuffled`-pakkojen sekoitus täysien nimien aakkosjärjestyksessä,
4. `game`-lohkon alkuarvot ja kokoelmien täyttö,
5. aloituskartan valinta (`start` kartta, pakka tai taulu) kenttään `game.start`,
6. aloituskartan lataus: legend tai generaattori, `with`-entiteetit, kokoelmien täyttö ja kerrosten `fill`,
7. aloituskohdan haku (`at` tai nimiö `START`) ruutujoukoksi kenttään `game.at`,
8. tapahtuma `begin`, jonka käsittelijät luovat pelaajan ja kutsuvat `control` ([RS 10.1](../roguescript/10-aloitus-ja-checkpointit.md#101-aloitus)). Moninpelissä liitetyt istuimet ovat tiedossa ennen tätä vaihetta, ja liitettyjen istuinten määrä kuuluu aloituksen syötteisiin kuten profiili (luku [8.2](08-syote.md#82-istuimet-ja-laitteet)).

Aloituksessa `game.time` on 0. Kartan latauksessa syntyneet toimijat saavat ensimmäisen herätyksensä hetkeen 0 + viive(oletushinta), ja `control` antaa pelaajalle herätyksen hetkeen 0 (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)).

Checkpoint 0 on tila `begin`-käsittelijöiden jälkeen. Jonoon laitetut etualan kohtaukset kuuluvat siihen nimenä ja argumentteina. Uutta peliä varten checkpointia 0 ei tallenneta etukäteen, vaan moottori laskee sen datasta, siemenestä ja profiilista. Kun peli on alkanut, checkpoint 0 tallentuu pelin mukana (luku [5.3](#53-checkpointit)). Datan muutokset näkyvät siksi heti uudessa pelissä.

## 5.2 Tallennus

- **Tilannekuva.** Tallennus sisältää koko tilan (luku [3.1](03-pelin-tila.md#31-tilan-osat)), myös `game`-kentät, pelaajan ja istuimet, ajoituksen ja profiilin näkymän. Laitteet eivät kuulu tallennukseen (luku [8.2](08-syote.md#82-istuimet-ja-laitteet)). Ladattu peli käyttää siksi aloitushetken `game`-arvoja, vaikka data tai pelaajan `overrides/` olisi sen jälkeen muuttunut. Muoto on moottorin oma, ja tiedostot ovat käyttäjädatakansion `saves/`-kansiossa.
- **Tallennuskohta.** Pelaaja voi tallentaa vain toimintojen välissä, käytännössä silloin, kun moottori odottaa pelaajan syötettä (luku [6.1.7](06-ajo.md#617-toiminto)). Etualan kohtauksen, dialogin ja `open`-valikon aikana tallennus on estetty. Moottori voi tallentaa kohtausta edeltävän tilan automaattisesti (luku [5.4](#54-automaattinen-tallennus)).
- **Kohtauksista kuvaus.** Pysyvä tila muuttuu vasta kokonaisen toiminnon lopussa, joten moottori tallentaa kohtauksista vain kuvauksen ([RS 11.1](../roguescript/11-tallennus-ja-korvaaminen.md#111-tallennus)). Ainoa tallennettava suorituskohta on odottavan komennon polku (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)):

| Rakenne | Tallennetaan |
| --- | --- |
| etualan kohtaus, dialogi ja open-valikko | ei mitään: tallennus on estetty, ja automaattinen tallennus ottaa kohtausta edeltävän tilan (luku [5.4](#54-automaattinen-tallennus)) |
| jonossa oleva etualan kohtaus | nimi ja argumentit |
| taustakohtaus | nimi, argumentit, `scope`, `bind`, kellon arvo ja silmukoiden vaiheet |
| pakka | nostopinon järjestys ja poistopakka |
| taulu | ei mitään |
| ajoitus | `game.time`, herätysjono järjestysnumeroineen, seuraava järjestysnumero ja karttojen kellot (luku [6.1.8](06-ajo.md#618-aktiiviset-kartat-ja-jäädytys)) |
| odottava komento (`wait N`, `wait scene`, `wait confirm` tai `open`) | nimi, rungon tiiviste, argumentit, paikalliset nimet, suorituskohta rakenteellisena polkuna ja `wait N` -odotuksessa jatkoherätys (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)) |
| checkpoint | viimeisin checkpoint tilannekuvana (luku [5.3](#53-checkpointit)) |
| keskeneräinen tweeni pelin ajan taustakohtauksessa | jäljellä oleva reitti tai ruudut ([RS 9.1](../roguescript/09-kohtaukset-ja-aikajanat.md#91-rivi)) |
| profiili | aloitushetken tilannekuva ja pelikerran omat kirjoitukset (luku [5.6](#56-profiili)) |
| kuollut kahva | laji ja tuhohetken nimi (luku [9.5](09-moottorin-funktiot-ja-tapahtumat.md#95-ilmentymän-elinkaari-ja-komponentit)) |

- **Lataus.** Ladattu peli jatkaa tilannekuvasta. Tapahtuma `begin` ei laukea. Odottava komento jatkaa suorituskohdastaan, tai se hylätään, jos komennon runko on muuttunut tallennuksen jälkeen (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)).

**Tallennuksen rajaus.** Kartan otsikon määre `nosave` ([D 8.1](../dataformaatit/08-kartat-ja-prefabit.md#81-karttatiedosto)) estää pelaajan oman tallennuksen ja `quit`-tallennuksen, kun pelaaja on kartalla, moninpelissä kun yksikin jäsen on kartalla, ja valikon `save`-kohta näkyy himmennettynä. Automaattinen tallennus ja checkpoint noudattavat kohtiaan normaalisti.

## 5.3 Checkpointit

- `checkpoint()` ottaa tilannekuvan, mukaan lukien pelin virran tilan ja ajoituksen. Kuva otetaan sen toiminnon lopussa, jossa kutsu tehtiin (luku [6.1.7](06-ajo.md#617-toiminto)). Milloin sitä kutsutaan, on logiikan päätös. Viimeisin checkpoint kuuluu tallennukseen, joten `rollback()` toimii myös ladatussa pelissä. Checkpoint 0 tallentuu samoin, kun peli on alkanut.
- `rollback()` lataa viimeisimmän checkpointin sellaisena kuin se tallennettiin. Istuintaulu palautuu checkpointin mukaiseksi. Istuin, joka on liitetty checkpointin jälkeen, säilyy liitettynä tyhjänä, eivätkä laitteet muutu. Lataus tehdään toiminnon lopussa, joten käynnissä oleva käsittelijäketju ajetaan loppuun. Jos sama toiminto kutsuu molempia, viimeinen kutsu ratkaisee. Profiilin kirjoitukset eivät peruunnu (luku [5.6](#56-profiili)).
- Moottori ei päätä, mitä kuolemassa tapahtuu, eikä kuolema päätä peliä. Kun pelaajaa ei ole, maailma jatkaa kulkuaan (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)), ja pelistä poistutaan taukovalikosta (luku [8](08-syote.md)).

## 5.4 Automaattinen tallennus

Moottori tallentaa pelin automaattisesti kohdissa, jotka peli valitsee. Paikkamäärä ja kohdat ovat dataa ([D 3.4](../dataformaatit/03-tiedostot-ja-include.md#34-automaattinen-tallennus-autosave)), ja logiikka voi pyytää tallennuksen funktiolla `autosave()`.

- **Paikat.** `slots 0` ei tallenna automaattisesti. `slots 1` on permadeath: yksi paikka, jonka jokainen tallennus korvaa, ja pelaajan oma `save` tallentaa samaan paikkaan ja lopettaa pelin, moninpelissä kaikilta istuimilta. `slots N` on pino: uusin menee päälle, ja vanhin putoaa pois, kun paikat ovat täynnä. Pelaajan omat tallennukset ovat silloin omissa paikoissaan pinon ulkopuolella. Omien paikkojen määrä on autosave-osion manual N (oletus 3, 0 = ei omaa tallennusta). quit noudattaa riviä quit: save (oletus) tallentaa moodissa 0 omaan lopetuspaikkaansa ja moodissa N pinoon, none ei tallenna. Moodissa 1 oma tallennus ja quit käyttävät aina ainoaa paikkaa. Ilman autosave-osiota pätevät slots 0 ja oletukset.
- **Lukitus.** Paikkamäärä luetaan uuden pelin alussa ja tallentuu pelikerran mukana, joten `overrides/` ei muuta sitä kesken pelin. Tallennuskohdat luetaan aina datasta.
- **Rajalla.** Automaattinen tallennus tehdään aina toimintojen rajalla, joten tila on eheä. Kohdat ovat:

| Kohta | Milloin |
| --- | --- |
| `input` | moottori odottaa pelaajan syötettä; `every N` tallentaa joka N:nnellä kerralla, ja moninpelissä kaikkien istuinten odotukset lasketaan yhteen |
| `scene` | ennen etualan kohtausta, dialogia tai `open`-valikkoa |
| `checkpoint` | kun `checkpoint()` toteutuu toiminnon lopussa |
| `arrive` | pelaaja saapuu kartalle, myös `travel`-kutsulla nykyiseen karttaan (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)); ryhmän siirto tallentaa kerran toiminnossa |

- **Logiikka.** `autosave()` tehdään toiminnon lopussa kuten `checkpoint()`. Monta pyyntöä samassa toiminnossa tuottaa yhden tallennuksen.
- **Haarautuminen.** Kun pelaaja lataa pinosta vanhemman tallennuksen ja jatkaa, uudemmat tallennukset jäävät pinoon. Pino järjestyy kirjoitusajan mukaan.
- **Kirjoitus.** Moottori ottaa tilannekuvan rajalla ja kirjoittaa sen taustalla. Jos uusi tallennus pyydetään ennen kuin edellinen on kirjoitettu, välissä oleva jätetään kirjoittamatta. Tiedosto kirjoitetaan ensin väliaikaisena ja nimetään lopuksi oikeaksi, joten kaatuminen ei jätä rikkinäistä tallennusta.
- **Kohtaus ladattaessa.** Kohtausta edeltävä tallennus sisältää jonossa olevan kohtauksen tai sitä odottavan komennon, joten ladattaessa kohtaus ajetaan alusta. Odottava komento jatkaa `wait scene`-, `wait confirm`- tai `open`-kohdastaan (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)).

## 5.5 Tallennuksen poisto

`delete_save(kohde)` poistaa tallennuksia. Se tehdään toiminnon lopussa kuten `checkpoint()`.

| Kohde | Mitä poistetaan |
| --- | --- |
| `checkpoint` | viimeisin checkpoint; sen jälkeen `rollback()` ei tee mitään ja kirjaa ajon varoituksen |
| `auto` | pelikerran automaattiset tallennukset (moodissa 1 ainoa paikka); automaattinen tallennus loppuu tältä pelikerralta, eikä `quit` moodissa 1 tallenna |
| `all` | molemmat |

Pelaajan omat tallennukset (moodi N) jäävät aina, samoin profiili (luku [5.6](#56-profiili)). Funktiota kutsuu logiikka, esimerkiksi permadeath-pelin kuolemassa (`delete_save(all)`) tai valikon kohdassa, jolla pelaaja poistaa oman checkpointinsa (`delete_save(checkpoint)`).

## 5.6 Profiili

Profiili on pelin pysyvä tila pelikertojen yli ([D 3.5](../dataformaatit/03-tiedostot-ja-include.md#35-profiili-profile), [RS 10.4](../roguescript/10-aloitus-ja-checkpointit.md#104-profiili-profile)). Moottori omistaa sen tallennuksen kokonaan.

- **Tiedosto.** Moottori kirjoittaa profiilin käyttäjädatakansioon asetusten viereen (`<käyttäjädata>/PELI/profile`, [D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)) omassa muodossaan. Profiili ei ole RLML:ää, joten `overrides/` ja modit eivät paikkaa sitä.
- **Suojaus.** Tiedostossa on tarkistussumma, joka lasketaan sisällöstä ja pelin nimestä. Jos summa ei täsmää, moottori kirjaa ajon varoituksen, nimeää tiedoston varmuuskopioksi ja aloittaa profiilin alkuarvoista. Suojaus estää helpon muokkauksen, ei määrätietoista.
- **Alkuarvot ja rakenne.** Ilman tiedostoa profiili saa `profile`-lohkon alkuarvot. Kun data lisää komponentin tai kentän, moottori lisää sen olemassa olevaan profiiliin oletusarvoineen. Datasta poistettu komponentti säilyy tiedostossa näkymättömänä, kunnes se palaa tai profiili nollataan.
- **Näkymä pelikerrassa.** Uuden pelin alussa moottori ottaa profiilista tilannekuvan (luku [5.1](#51-uusi-peli)). Pelikerta lukee tilannekuvaa ja omia kirjoituksiaan, eikä se näe muiden pelikertojen myöhempiä muutoksia. Näkymä tallentuu pelikerran mukana (luku [5.2](#52-tallennus)), joten ladattu peli ja sen toisto toimivat samoin toisella koneella.
- **Kirjoitus.** Logiikan kirjoitus profiiliin näkyy pelikerran näkymässä heti, ja moottori kirjoittaa sen profiilitiedostoon toiminnon lopussa kuten `autosave()` (luku [6.1.7](06-ajo.md#617-toiminto)). Etualan kohtauksessa kirjoitus tehdään luonnoksen kirjauksessa. Tiedosto kirjoitetaan väliaikaisena ja nimetään lopuksi oikeaksi kuten tallennus (luku [5.4](#54-automaattinen-tallennus)).
- **Pysyvyys.** `rollback`, `delete_save` ja pelikerran lataus eivät peru profiilin kirjoituksia tiedostosta. Vanhan tallennuksen lataus palauttaa pelikerran oman näkymän, mutta profiilitiedosto säilyy.
- **Nollaus.** Profiilin voi nollata vain moottorin valikosta, ei logiikasta.

---

← [4. Satunnaisuus ja toistettavuus](04-satunnaisuus-ja-toistettavuus.md) · [Sisällys](README.md) · [6. Ajo](06-ajo.md) →
