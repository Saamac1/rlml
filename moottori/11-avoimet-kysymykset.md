[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 11. Avoimet kysymykset

Moottoria koskevat avoimet kysymykset molemmista dokumenteista sekä tämän dokumentin kokoamisessa löytyneet:

- **Pelaajan herätys, kun pelaajalla on tekoäly.** STD:n oletuspelaajalla on `ai behavior HUNTER` (vakiokirjasto, luku [3](../vakiokirjasto/03-komponentit-ja-oletuspelaaja.md)), joten syntyessään kartalle se saa herätyksen hetkeen nyt + viive (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)), ja `control` säilyttää jo olemassa olevan herätyksen. Luvut [5.1](05-aloitus-tallennus-ja-checkpointit.md#51-uusi-peli) ja [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin) lupaavat kuitenkin, että `begin`-käsittelijän luoma pelaaja toimii hetkellä 0. Ajo (31.10.2026) siirtää aloituksen jälkeen pelaajan herätyksen hetkeen 0. Pitäisikö sääntö olla yleinen: `control(e)` antaa herätyksen hetkeen nyt, kun `e` syntyi samassa toiminnossa? Moninpelissä sama koskee jokaista `begin`-käsittelijän luomaa jäsentä (luku [6.1.10](06-ajo.md#6110-istuimet-ja-ryhmä)).
- **Taukonäppäin peitetyn valikon päällä.** `ALWAYS`-sidonta sulkee avoimen valikon (luku [8](08-syote.md)), mutta ei kerro, mitä tapahtuu, kun valikko on peitetty (`PAUSE` → `OPTIONS`, ja pelaaja painaa `esc`). Päätteen ajo (31.10.2026) sulkee valikon ja kaikki sen päällä olevat, eli sama näppäin palaa aina peliin. Vaihtoehto olisi paluu peitettyyn valikkoon (luku [7.5](07-esitys.md#75-valikot)).
- **Komento valikon kohtana.** Kun kohta ajaa komennon (`- RESUME` ilman toimintoa, kun peli määrittelee komennon `RESUME`), sulkeutuuko valikko? Päätteen ajo sulkee pinon `final`-valikkoon asti ja ajaa komennon pelaajan komentona, jos pelaajalla on vuoro; `final`-valikon päällä komentoa ei ajeta.
- **Moottorin valikon tekstit.** Ilman `title`-kenttää `quit` palaa moottorin valikkoon (luku [8](08-syote.md)), jonka kohtien tekstejä ei ole pelin kielitiedostossa. Päätteen ajo näyttää ne englanniksi (`New game`, `Exit`). Tuoko moottori omat käännöksensä, vai käyttääkö se avaimia, jotka peli voi kääntää (esimerkiksi `ENGINE.NEW_GAME`)?
- **Generoinnin nopeus.** Raskaan generoinnin voi siirtää ulkoiseen generaattoriin (luku [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)), mutta se ei toimi kaikilla alustoilla, joten RogueScript-varaversion on oltava riittävän nopea. Jos vakiokirjaston generointi on suurilla kartoilla hidasta, mitkä funktiot siirretään moottoriin primitiiveiksi?
- **Moottorin komponenttien merkitys.** `movement`-kentän `mode`, `timer`-rivien tapahtumat ja kentät sekä `interior` ja `shape` on nimetty mutta ei kuvattu.
- **Keskeytys.** Pitkä komento jatkuu, vaikka vihollinen tulee näkyviin. Ratkaisu voisi olla `interrupt(e)`, joka päättää odotuksen heti. Se vaatisi, että `wait` palauttaa arvon (`if not wait 120: return false`), ja todennäköisesti moottorin näkyvyystapahtuman (luku [9.7](09-moottorin-funktiot-ja-tapahtumat.md#97-raja-vakiokirjastoon)). Toistuva komento voi tarkistaa näkyvyyden kierrosten välissä itse (luku [8](08-syote.md)).
- **Vaihtuva hinta.** `cost` on vakio. Riittääkö `wait` tilanteesta riippuvaan hintaan, vai sallitaanko `cost`-lauseke (esimerkiksi kantamuksen painosta)?
- **Ajastimen tarkkuus.** `timer` laskee kierroksia. Tarvitaanko mikroaskeltarkka ajastin, ja millä kirjoitusasulla se erotetaan kierroksista?
- **Vinoliikkeen hinta.** Chebyshev-metriikalla vinoliike maksaa saman kuin suora. Peli voi kirjoittaa eri hinnan komentoon, mutta moottorin `path` ei tiedä siitä.
- **Kokoelman muut jäsenet.** Saako repussa palava soihtu `turn`-tapahtuman, vaikka reppu ei ole `modifies`-kokoelma?
- **Siirto kokoelmasta toiseen** ei lähetä tapahtumia (luku [9.6](09-moottorin-funktiot-ja-tapahtumat.md#96-tapahtumat)). Riittääkö, että vakiokirjaston komento lähettää omansa?
- **Useat ilmentymät ja aktiiviset kartat.** Voiko samaa karttaa olla useana ilmentymänä (`interior`, syvyystasot)? Ratkaistu 6.11.2026 (moninpeli): aktiivisia karttoja on useita, kun jäsenet ovat eri kartoilla, ja niiden kellot etenevät yhdessä (luku [6.1.10](06-ajo.md#6110-istuimet-ja-ryhmä)). Avoinna on, ajetaanko jäädytetty aika vaihtoehtoisesti nopeasti kiinni. Syvyystasot hoituvat nyt `transient`-kartalla ja `travel`-kutsulla samaan karttaan (luku [6.7](06-ajo.md#67-siirtyminen-kartalta-toiselle)).
- **Esitysaikainen ulkoasu.** Hallusinaatio ja vastaavat vaikutukset muuttaisivat entiteettien piirtoa ja nimeä esityksessä muuttamatta tilaa. `naming` kattaa nimen, mutta piirrolle ei ole vastaavaa kyselyä ([RS 13](../roguescript/13-avoimet-kysymykset.md)).
- **Esto ja liike.** Törmäys on vakiokirjastoa, ja `move` ei tarkista estoa (luvut [9.7](09-moottorin-funktiot-ja-tapahtumat.md#97-raja-vakiokirjastoon) ja [10](10-mita-moottori-ei-tee.md)). [D 7.5](../dataformaatit/07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles) kertoo, että moottori käyttää `blocks`-estoa näössä, valossa ja reitinhaussa. Ratkaistu 24.10.2026: `sight`, `light` ja `path` käyttävät tilejen ja entiteettien estoa, paitsi `through`-joukon tilejä (luvut [3.4](03-pelin-tila.md#34-moottorin-komponentit) ja [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).
- **Ison nimen poikkeukset.** Moottori tuntee isoista nimistä ainakin nimiön `START`, kontekstit `MENU`, `DIALOGUE`, `SCENE`, `AIM`, `DIRECTION`, `LOOK`, `JOIN` ja `ALWAYS`, viestiryhmän `SPEECH`, nimiavaruuden `STD` ja tekstiavainten päätteet `NAME`, `DESC`, `TITLE`, `TEXT`, `ROW`, `EMPTY`, `MORE`, `SENTENCE` ja `NO_TARGET` sekä paikkamerkkien päätteet `SLOT` ja `COUNT`. Pelin peruskonteksti nimetään datassa ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)). Luvun [1.1](01-rajaus-ja-periaatteet.md#11-vastuunjako) ja [D 2.1](../dataformaatit/02-yleinen-syntaksi.md#21-nimet-ja-sanasto):n sääntö on muotoiltava niin, että poikkeukset luetellaan.
- **`cancel` ja `after`.** Estääkö `cancel` myös `after`-käsittelijät ([RS 13](../roguescript/13-avoimet-kysymykset.md))? Moottorin tapahtumia kysymys ei koske, koska niissä ei ole `cancel`-mahdollisuutta.
- **Checkpointit.** Säilytetäänkö vain viimeisin checkpoint vai nimetty lista ([RS 13](../roguescript/13-avoimet-kysymykset.md))?
- **Tyylien rajaus** vaikuttaa siihen, miten moottori kokoaa ladatut tyyliosiot ([D 13](../dataformaatit/13-avoimet-kysymykset.md)).

* **Kuuleminen.** Ratkaistu 24.10.2026 (luku [6.8](06-ajo.md#68-äänet-ja-kuuleminen)).

Suljettu 8.11.2026: usean kohteen verbit. Klikkaus täyttää ensimmäisen vapaan parametrin, jolle kohde kelpaa, joten kohteet voi klikata missä järjestyksessä tahansa, ja riippuva parametri täyttyy vasta riippuvuutensa jälkeen (luku [8.1](08-syote.md#81-hiiri)).

Suljettu 8.11.2026: ennakkokäskyn vanheneminen. Käsky perutaan ennen toista askelta, jos näkyviin on tullut toimija, jota käskyä annettaessa ei näkynyt (luku [8.1](08-syote.md#81-hiiri)).

Suljettu 8.11.2026: verbikolikon ehdokkaat. Komennon määre `noverb` piilottaa sen kolikosta, kun verbivalikkoa ei ole ([RS 7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)).

Suljettu 26.10.2026 (hiiri): klikkausmatkan keskeytys. Moottorin ehtojen lisäksi peli voi antaa oman ehtonsa kentällä `walk_stop` (luku [8.1](08-syote.md#81-hiiri)).

Suljettu 25.10.2026 (robotfindskitten-toteutustesti): toiminnon lopun järjestys ja etualan kohtauksen kartta (luvut [6.1.7](06-ajo.md#617-toiminto) ja [6.2](06-ajo.md#62-tapahtumat)), logiikan kortti (luku [3.2](03-pelin-tila.md#32-ilmentymän-syntyminen)), generaattorin oikeudet (luku [6.5](06-ajo.md#65-generointi)), asettelun valinta (luku [7.1](07-esitys.md#71-asettelu)), näppäin tekstissä ja tekstiavainten jokeri (luku [7.3](07-esitys.md#73-tekstit)), lokin pitkä viesti (luku [7.4](07-esitys.md#74-loki-ja-dialogit)), `final`-valikko ja `new_game` (luvut [7.5](07-esitys.md#75-valikot) ja [8](08-syote.md)), modaalisen paneelin konteksti (luku [7.7](07-esitys.md#77-paneelit)) sekä ruutujoukon `pick`, tyhjä väli ja `scatter` (luku [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)).

Suljettu 25.10.2026 (Rogue-toteutustesti): pelikertojen yli säilyvä tila on profiili (luku [5.6](05-aloitus-tallennus-ja-checkpointit.md#56-profiili)), toistolaskuri on toiminto `repeat` (luku [8](08-syote.md)), valaistus ja muisti ovat tilen ominaisuuksia `lit` ja `forget` (luku [7.2](07-esitys.md#72-kartan-piirtäminen)), ja pinot ovat erillisiä ilmentymiä (luku [3.3](03-pelin-tila.md#33-kokoelmat)). Moottorin kontekstien oletussidonnat on kuvattu luvussa [8](08-syote.md).

Suljettu 24.10.2026: esto näössä, valossa ja reitinhaussa (luvut [3.4](03-pelin-tila.md#34-moottorin-komponentit) ja [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)), pelin peruskonteksti (luku [8](08-syote.md)).

Suljettu 23.10.2026 (ilta): komentojen `wait` tallennuksessa. Suorituskohta on rakenteellinen polku (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)). Rekursion raja: kutsusyvyys `game.call_depth` (luvut [2.6](02-lataus.md#26-virheiden-lajit) ja [6.2](06-ajo.md#62-tapahtumat)).

---

← [10. Mitä moottori ei tee](10-mita-moottori-ei-tee.md) · [Sisällys](README.md)
