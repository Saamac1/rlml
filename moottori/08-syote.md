[RLML](../README.md) › [RLML-moottorin vastuut](README.md)

# 8. Syöte

- **Kontekstipino.** Moottori pinoaa näppäinkontekstit. `modal` antaa syötteet vain päällimmäiselle, `passthrough` päästää sitomattomat läpi ([D 11.3](../dataformaatit/11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)).
- Moottorin kontekstit ovat `MENU`, `DIALOGUE`, `SCENE`, `AIM`, `DIRECTION`, `LOOK`, `JOIN` ja `ALWAYS`. `JOIN` on moninpelin liittymisruutu (luku [8.2](#82-istuimet-ja-laitteet)). Valikko ja modaalinen paneeli käyttävät kontekstia `MENU`. Pelin peruskonteksti nimetään game-lohkon kentällä keys ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)). Se on pinon pohjalla koko ajon, myös pelin omassa alkuvalikossa. **Kohdistus.** Parametrin lähde `aim N at JOUKKO` (RogueScript, luku [7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)) kysyy kohteen pelaajan asetuksen `targeting` mukaan ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)): `cursor` avaa kohdistimen kontekstiin `AIM`, ja se alkaa lähimmästä ehdokkaasta; `list` näyttää ehdokkaat valikkona ja korostaa valitun kartalla; `auto` valitsee lähimmän kysymättä. Kontekstin `AIM` toiminnot ovat `cursor.north` … `cursor.northwest`, `next` ja `prev` (ehdokkaat etäisyysjärjestyksessä), `confirm` ja `back`. Kohdistin liikkuu kantaman sisällä myös ehdokkaiden ulkopuolelle, ja `back` peruu komennon ilman ajan kulumista. Jos `list`-asetuksella ehdokkaita ei ole, moottori avaa kohdistimen. **Tyhjä lähde.** Jos parametrin `from`-joukko on tyhjä, komento epäonnistuu ennen kysymistä eikä vie aikaa (RogueScript, luku [7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)).
- **Sidonnat.** Iso nimi ajaa komennon, pieni nimi moottorin toiminnon. Moottori tulkitsee muuntonäppäimet, näppäinten nimet ja gamepadin paikkapohjaiset painikkeet sekä `deadzone`-arvon.
- **Oletussidonnat.** Moottori sitoo omien kontekstiensa toiminnot. Jos peli sitoo saman toiminnon samassa kontekstissa, pelin rivi korvaa oletuksen, ja pelaajan `overrides/keys.rlml` korvaa molemmat.

  | Konteksti | Toiminto | Näppäimet | Peliohjain |
  | --- | --- | --- | --- |
  | `MENU`, `DIALOGUE` | `cursor.north`, `cursor.south` | `up` `k`, `down` `j` | `dpad` |
  | `MENU`, `DIALOGUE` | `confirm`, `back` | `enter` `space`, `esc` `backspace` | `south`, `east` |
  | `SCENE` | `confirm`, `skip`, `restart`, `rewind` | `enter` `space`, `tab`, `r`, `backspace` | `south`, `start`, –, `east` |
  | `AIM` | `cursor.north` … `cursor.northwest` | nuolet, `hjklyubn`, numeronäppäimistö | `dpad`, vasen tikku |
  | `AIM` | `next`, `prev` | `tab`, `shift+tab` | `rb`, `lb` |
  | `AIM` | `confirm`, `back` | `enter` `space`, `esc` | `south`, `east` |
  | `LOOK` | `cursor.north` … `cursor.northwest`, `next`, `prev` | kuten `AIM` | kuten `AIM` |
  | `LOOK` | `confirm`, `back` | `enter` `space`, `esc` | `south`, `east` |
  | pelin peruskonteksti | `travel_to` | `click` | – |

  Näppäinnimet `tab`, `enter`, `space`, `esc`, `backspace` ja muunnin `shift+` kuuluvat moottorin nimistöön ([D 11.3](../dataformaatit/11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)). Hiiren nimet ja niiden oletuskäytös valikoissa ovat luvussa [8.1](#81-hiiri). Toiminnolle `look` moottori ei anna oletusnäppäintä, koska peruskonteksti on pelin oma; peli sitoo sen (`- look x`, peliohjaimella esimerkiksi `north`).
- Pelaajan sidonnat tulevat `overrides/keys.rlml`-tiedostosta, ja ne paikkaavat pelin sidonnat riveittäin ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data), [D 4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)). Valikot ennen peliä: moottorin oma valikko valitsee pelin. Jos game-lohkossa on kenttä title, moottori avaa sitten pelin oman alkuvalikon, jonka kohdat käyttävät moottorin toimintoja new\_game, load\_game ja exit. Muuten moottorin valikko tarjoaa uuden pelin, tallennukset ja profiilin nollauksen (luku [5.6](05-aloitus-tallennus-ja-checkpointit.md#56-profiili)). quit palaa pelin alkuvalikkoon, jos sellainen on, ja muuten moottorin valikkoon.
- **Uusi peli pelin aikana.** `new_game` on sallittu kaikissa valikoissa. Pelin aikana moottori päättää nykyisen pelikerran kuten `quit`, eli tallentaa `quit`-rivin mukaan (luku [5.4](05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)), ja laskee sitten uuden pelin uudella siemenellä (luku [5.1](05-aloitus-tallennus-ja-checkpointit.md#51-uusi-peli)) ohittaen alkuvalikot. Kiinteä `seed` ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)) antaa saman aloituksen uudelleen.

**Kysyvät lähteet.** Parametrin lähde `ask` ([RS 7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)) avaa moottorin kontekstin `DIRECTION`: suuntanäppäin vastaa heti, ja `back` peruu komennon ilman ajan kulumista. Tyypillä `dir` tulos on suunta ja tyypillä `pos` ruutu `actor.at + suunta`. `DIRECTION` käyttää kontekstin `AIM` suuntasidontoja.

**Luku- ja tekstisyöte.** `number ask a..b` avaa lukukentän ja `text ask N` tekstikentän, johon mahtuu enintään N koodipistettä. Korutiinin `wait number(a..b)` ja `wait text(N)` käyttävät samoja kenttiä ja palauttavat arvon tai `none`, kun pelaaja peruu. Ne päättävät käynnissä olevan toiminnon kuten `wait confirm` (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)), ja `idle` ei laukea niiden aikana.

**Toistolaskuri: `repeat`.** Moottorin toiminto `repeat` ([D 11.3](../dataformaatit/11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)) on sallittu pelaajan vuorolla pelin peruskontekstissa.

- **Kenttä.** `repeat` avaa lukukentän. Numeronäppäimet kirjoittavat kenttään, `backspace` poistaa numeron, ja `back` peruu koko toiston ilman ajan kulumista. Tyhjä kenttä tai 0 peruu samoin.
- **Komento.** Ensimmäinen muu näppäin päättää kentän ja tulkitaan peruskontekstissa kuten tavallinen painallus. Numeronäppäimistön suuntanäppäimet ovat siis kentässä numeroita, mutta nuolet ja kirjaimet ovat komentoja.
- **`count`-lähde.** Jos komennolla on parametri, jonka lähde on `count` ([RS 7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)), moottori antaa sille kentän luvun. Muut parametrit kysytään tavalliseen tapaan sen jälkeen. Ilman toistolaskuria `count` saa oletusarvonsa 1.
- **Ilman `count`-parametria.** Komento ajetaan kerran, ja moottori kirjaa ajon varoituksen, jotta kirjoittaja huomaa puuttuvan tuen. Moottorin toiminto ei toistu.
- **Toisto on komennon asia.** Moottori ei toista komentoa itse, vaan komento käyttää lukua silmukassaan ja päättää, milloin toisto keskeytyy (esimerkiksi vihollisen tullessa näkyviin). Hinta ja odotukset ovat komennon omat (luku [6.1.6](06-ajo.md#616-monivaiheinen-komento)).

**Aina kuunneltava konteksti.** `keys ALWAYS` ja `pad ALWAYS` ([D 11.3](../dataformaatit/11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)) ovat voimassa kaikissa muissa konteksteissa ja myös silloin, kun peli etenee ilman pelaajan vuoroa. Ne voittavat muut kontekstit. Tyypillisesti niihin sidotaan taukovalikko: `- open PAUSE esc` ja `- open PAUSE select`.

- Sama sidonta sulkee avoimen valikon, joten tauko toimii vaihtokytkimenä. `final`-valikkoa se ei sulje (luku [7.5](07-esitys.md#75-valikot)).
- Valikko avautuu heti, jos moottori odottaa syötettä, ja muuten seuraavalla toimintojen rajalla. Pelin aika ja reaaliaikaiset taustakohtaukset pysähtyvät valikon ajaksi.
- Kohtauksen aikana valikko avautuu kohtauksen päälle. `save` on silloin estetty, mutta `quit` toimii.

**Moottorin toiminnot:**

| Toiminto | Merkitys |
| --- | --- |
| `save` | tallentaa pelin; ei toimi etualan kohtauksen, dialogin eikä `open`-valikon aikana (luku [5.2](05-aloitus-tallennus-ja-checkpointit.md#52-tallennus)) |
| `quit` | tallentaa tallennusmoodin mukaan (luku [5.4](05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)) ja palaa alkuvalikkoon |
| `new_game` | aloittaa uuden pelin; pelin aikana päättää ensin pelikerran kuten `quit` (yllä) |
| `pass` | ajaa seuraavan kierroksen heti, kun pelaajalla ei ole vuoroa (luku [6.1.2](06-ajo.md#612-toimijat-ja-ajoitin)) |
| `repeat` | avaa toistolaskurin seuraavalle komennolle (yllä) |
| `help` | näyttää ohjeen |
| `confirm` | kuittaa tai valitsee |
| `back`, `close`, `resume` | valikoiden ohjaus; eivät sulje `final`-valikkoa |
| `skip`, `restart`, `rewind` | kohtauksen ohjaus (luku [6.4](06-ajo.md#64-kohtaukset)) |
| `travel_to` | kulkee osoittimen ruutuun klikkausmatkana (luku [8.1](#81-hiiri)) |
| `look` | avaa katselukursorin, kontekstin `LOOK` (luku [8.1](#81-hiiri)) |
| `join` | liittää laitteen uudeksi istuimeksi (luku [8.2](#82-istuimet-ja-laitteet)) |
| `leave_seat` | poistaa laitteen istuimen; piilossa, kun istuimia on enintään alaraja (luku [8.2](#82-istuimet-ja-laitteet)) |

`toggle PANEELI` näyttää tai piilottaa nimetyn paneelin (luku [7.7](07-esitys.md#77-paneelit)). Sidonta kirjoitetaan toiminnon ja paneelin nimellä: `- toggle DEBUG f12`.

## 8.1 Hiiri

Hiiri on valinnainen syöte: kaiken, minkä hiirellä voi tehdä, voi tehdä myös näppäimistöllä tai peliohjaimella. Moottori lukee hiiren, kun pääte tai alusta tukee sitä, ja pelaajan asetus `- mouse off` ([D 3.2](../dataformaatit/03-tiedostot-ja-include.md#32-pelaajan-oma-data)) poistaa sen käytöstä. Hiiren syöte on syötettä kuten painallus: se kulkee samojen kontekstien ja toimintojen kautta, joten tila, tallennukset ja toisto pysyvät eheinä.

- **Nimet ja sidonnat.** Hiiren tapahtumat ovat näppäinnimiä: `click` (vasen), `rclick` (oikea), `mclick` (keski), `wheel.up` ja `wheel.down`, ja niihin käyvät muuntimet (`shift+click`). Ne sidotaan `keys`-riveillä kuten näppäimet (`- confirm enter click`), ja pelaajan `overrides/keys.rlml` paikkaa niitä samoin. Tuntematon hiiren nimi on latausvirhe (luku [2.5](02-lataus.md#25-tarkistukset)).
- **Osoitin.** Moottori pitää kirjaa osoittimen ruudusta: kartta-alueella se on kartan ruutu kameran mukaan, muualla asettelun alue ja sen rivi. Osoitin ei kuulu pelin tilaan eikä tallennukseen. Kun katselukursori tai tähtäyksen kohdistin on auki, hiiren osoitin ja kursori ovat sama: hiiren liike siirtää kursoria, ja näppäimet siirtävät sitä edelleen.
- **Valikot ja paneelit.** Osoitin korostaa valikon tai listavalikon rivin, jonka päällä se on, ja `click` valitsee sen kuten `confirm`. `wheel.up` ja `wheel.down` vierittävät valikkoa, lokia ja monirivistä paneelin kenttää. `click` valikon ulkopuolella on `back`. Modaalisessa paneelissa `click` on `confirm`. Säädettävällä kohdalla (luku [7.5](07-esitys.md#75-valikot)) rulla muuttaa arvoa.
- **Kysyvät lähteet.** Kontekstissa `AIM` osoitin siirtää kohdistimen ruutuunsa kantaman sisällä ja `click` vahvistaa sen. Kontekstissa `DIRECTION` `click` antaa suunnan pelaajasta klikattuun ruutuun päin (lähin kahdeksasta suunnasta) ja tyypillä `pos` sen ruudun, joka on pelaajan vieressä tuohon suuntaan. Luku- ja tekstikenttään hiiri ei kirjoita.

**Hover.** Kun osoitin on kartta-alueen ruudun päällä, moottori kirjoittaa asettelun `hover`-alueeseen (luku [7.1](07-esitys.md#71-asettelu)) kuvauksen siitä, mitä ruudussa näkyy:

- Ylin piirretty entiteetti, ja ilman sitä ruudun ylimmän kerroksen tile. Teksti on avaimen `<NIMI>.DESC` teksti, ja nimi ratkaistaan kuten paikkamerkissä (luku [7.3](07-esitys.md#73-tekstit)), joten `naming` ja komponentti `name` vaikuttavat siihen.
- Kuvaus näytetään vain ruudusta, joka on pelaajan näkökentässä, muistissa (`remember`) tai jossa on `revealed`-entiteetti. Muistetusta ruudusta näytetään muistettu kohde. Näkymätön ruutu jättää alueen tyhjäksi, joten hover ei paljasta mitään, mitä pelaaja ei näe.
- Hover on esitystä: se ei kuluta aikaa, ei muuta tilaa eikä nollaa `idle`-ajastinta. Peli voi kirjoittaa saman tiedon itse paneeliin, ja `hover` on vain valmis lähde.
- **Tähtäyksessä** `hover`-alue näyttää kohdistimen ruudun kuvauksen samoin säännöin, olipa kohdistinta siirretty näppäimillä, peliohjaimella tai hiirellä. Pelaaja näkee siis, mihin on tähtäämässä, ennen kuin vahvistaa.

**Katselukursori: `look`.** Moottorin toiminto `look` avaa kontekstin `LOOK`, jossa näppäimin tai peliohjaimella ohjattava kursori tekee saman kuin hiiren osoitin:

- Kursori alkaa pelaajan ruudusta, tai ilman pelaajaa kameran keskeltä. `cursor.<suunta>` siirtää sitä, ja `next` ja `prev` hyppäävät näkyvien entiteettien välillä lähimmästä alkaen. Kontekstin oletussidonnat ovat samat kuin kontekstilla `AIM` (luku [8](#)).
- `hover`-alue näyttää kursorin ruudun kuvauksen hoverin säännöin, ja kursori piirretään kartalle kuten kohdistin. Kamera seuraa kursoria, kun se lähestyy kartta-alueen reunaa.
- `confirm` aloittaa matkan kursorin ruutuun kuten `travel_to` ja sulkee kontekstin. `back` sulkee kontekstin.
- Katselu ei kuluta aikaa eikä muuta tilaa, ja `idle` ei laukea sen aikana.

**Matka klikkaamalla.** Moottorin toiminto `travel_to` (oletuksena `click` pelin peruskontekstissa) kuljettaa pelaajan osoittimen ruutuun:

1. **Reitti.** Moottori laskee reitin `path(player.at, ruutu, for: player)` (luku [9.3](09-moottorin-funktiot-ja-tapahtumat.md#93-funktiot)). Reitti saa kulkea vain ruutujen kautta, jotka pelaaja näkee tai muistaa. Jos reittiä ei ole tai ruutu on pelaajan oma, mitään ei tapahdu eikä aikaa kulu.
2. **Kävelykomennot.** Moottori muuttaa reitin askeliksi ja ajaa jokaisen pelaajan komentona `game`-lohkon kentän `walk` nimeämällä komennolla (`- walk STD.MOVE`, [D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)) ja askeleen suunnalla. Komennolla on oltava täsmälleen yksi `dir`-parametri (luku [2.5](02-lataus.md#25-tarkistukset)). Jokainen askel on tavallinen vuoro: hinta, nopeus, tapahtumat, törmäys ja välikehykset menevät kuten painalluksella, joten liikkeen säännöt pysyvät vakiokirjastossa (`STD.CAN_STEP`, `STD.BUMP`).
3. **Pysähtyminen.** Matka päättyy, kun pelaaja on perillä, kävelykomento epäonnistuu, pelaaja painaa mitä tahansa näppäintä tai klikkaa, pelaaja menettää vuoronsa (lamaantuminen, `control` toiselle), avautuu valikko, dialogi tai etualan kohtaus, tai pelaajan näkökenttään tulee toimija, jota edellisellä askeleella ei näkynyt. Seuraavan askeleen reitti lasketaan uudelleen, jos edellinen askel ei vienyt suunniteltuun ruutuun.
4. **Pelin oma ehto.** `game`-lohkon kenttä `walk_stop KYSELY` ([D 3.3](../dataformaatit/03-tiedostot-ja-include.md#33-aloitus)) nimeää kyselyn `query Q(who: entity, next: pos) -> bool`, jonka moottori kutsuu ennen jokaista askelta. Tosi pysäyttää matkan ennen askelta. Ehto tulee moottorin ehtojen lisäksi eikä korvaa niitä, joten peli voi pysäyttää matkan esimerkiksi kuultuun ääneen, nälkään tai vaaralliseen maastoon. Kysely käyttää kyselyn virtaa (luku [4.1](04-satunnaisuus-ja-toistettavuus.md#41-satunnaisvirrat)), joten se ei muuta tilaa eikä toistettavuutta.
5. **Ilman `walk`-kenttää** `travel_to` ja katselukursorin `confirm` eivät tee mitään, ja `click` kartalla on sitomaton.

Matka ei ole moottorin komento eikä kuulu tallennukseen: tallennettava tila on askelten välissä tavallinen syötteen odotus. Automaattisen tallennuksen kohta `input` (luku [5.4](05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)) ei laukea matkan askelten välissä, ja `idle` ei laukea matkan aikana. Toistossa matka näkyy askelina samoin kuin näppäimillä kuljettu reitti.

## 8.2 Istuimet ja laitteet

Moninpelissä (luku [6.1.10](06-ajo.md#6110-istuimet-ja-ryhmä)) laite on näppäimistöosa ([D 11.3](../dataformaatit/11-kayttoliittyman-data.md#113-näppäimet-hiiri-ja-gamepad)), peliohjain tai hiiri. Reititin kuvaa laitteen istuimeksi. Ydin saa syötteen parina (istuin, toiminto) eikä tiedä laitteista mitään.

**Liittymisruutu: konteksti `JOIN`.** Järjestys on moottorin pelivalitsin, pelin alkuvalikko, `new_game`, liittymisruutu ja laskettu aloitus. Ruutu näytetään vain, kun `seats`-yläraja on yli 1, ja `new_game` pelin aikana ei ohita sitä.

- Laite, joka valitsi `new_game`, on istuin 1. Muut laitteet liittyvät toiminnolla `join` ja saavat seuraavan vapaan numeron.
- `back` liitetyltä laitteelta poistaa sen istuimen. `confirm` miltä tahansa liitetyltä laitteelta aloittaa pelin, kun istuimia on vähintään alaraja.
- **Oletussidonnat:** `join` on `enter` ja peliohjaimella `start`, `confirm` on `enter`, ja `back` on `esc`. Liittämättömältä laitteelta `enter` on `join` ja liitetyltä `confirm`.
- **Aloitus.** Liitettyjen istuinten määrä kuuluu aloituksen syötteisiin (luku [5.1](05-aloitus-tallennus-ja-checkpointit.md#51-uusi-peli)), ja `begin` näkee istuimet funktiolla `seats()`.

**Liittyminen kesken pelin.** `join` pelin peruskontekstissa vapaalta laitteelta liittää uuden istuimen, kun istuimia on alle ylärajan. Peli sitoo `join`in tähän kontekstiin itse. Etualan kohtauksen, dialogin tai valikon aikana painallus hylätään. Liittyminen on oma toimintonsa, ja moottori lähettää sen lopussa tapahtuman `join(seat)`, jonka subjekti on `game` (luku [6.1.7](06-ajo.md#617-toiminto)). Jos mikään käsittelijä ei anna istuimelle hahmoa, istuin jää tyhjäksi.

**Poistuminen.** `leave_seat` kuuluu pelin taukovalikkoon, ja moottori piilottaa sen, kun istuimia on enintään alaraja. Viimeinen istuin poistuu `quit`-toiminnolla. Moottori lähettää toiminnon lopussa tapahtuman `part(seat)`, ja käsittelijöiden jälkeen istuimen jäsenet menettävät `seat`-komponentin ja istuin ja laite vapautuvat. Muiden istuinten numerot eivät muutu, ja vapautunut numero annetaan seuraavalle liittyjälle.

**Reititys.**

- **Vuorolla** vain vuorossa olevan istuimen laite ohjaa, myös parametrikyselyt (`from`, `aim`, `ask`), toistolaskuri ja `look`. `pass` toimii miltä tahansa istuimelta.
- **Valikkoa, dialogia ja kohtausta** ohjaa avaajan istuin, ja jos avaaja ei ole jäsen, mikä tahansa istuin. Sidonnasta avattu valikko ajaa pelaajan komennon avaajan istuimen ohjattavalle (luku [7.5](07-esitys.md#75-valikot)). `more`-sivutusta (luku [7.4](07-esitys.md#74-loki-ja-dialogit)) kuittaa mikä tahansa istuin.
- **`ALWAYS`** toimii kaikilta laitteilta, ja taukovalikkoa ohjaa sen avannut laite.
- **Muiden istuinten painallukset** hylätään eivätkä jonoudu. `join` toimii vapailta laitteilta.
- **Näppäimistöosat.** Konteksti ilman `keyboard N` -osioita käyttää osan 1 sidontoja kaikille näppäimistön istuimille, ja reititys rajaa ohjaajan. `{^TOIMINTO}` (luku [7.3](07-esitys.md#73-tekstit)) näyttää osan 1 näppäimen.
- **Hiiri** kuuluu osan 1 istuimelle. Hover toimii aina, ja hiiren toiminnot toimivat vain hiiren istuimen vuorolla.
- **Klikkausmatka** (luku [8.1](#81-hiiri)) etenee vain matkaajan vuoroilla, ja muiden istuinten vuorot kulkevat välissä. Vain matkaajan istuimen painallus pysäyttää sen. Muiden istuinten odotukset ovat tavallisia syötteen odotuksia, ja `on input` laskee ne.

**Tallennus ja lataus.** Tallennukseen kuuluvat istuinten numerot, jäsenyydet, ohjattavat ja `player`, mutta eivät laitteet (luku [5.2](05-aloitus-tallennus-ja-checkpointit.md#52-tallennus)). Latauksessa liittymisruutu kysyy laitteen jokaiselle tallennetulle istuimelle numerojärjestyksessä. Istuin, jolle ei tule laitetta (`skip` tai `confirm` ennen sitä), poistuu kuten `part` ennen ensimmäistä syötettä. Uusia istuimia voi liittää ylärajaan asti.

**Syöteloki.** Toisto ja `rlml run` -transkriptit kirjaavat syötteet pareina (istuin, toiminto), ja `join` ja `leave_seat` ovat rivejä muiden joukossa (luku [4](04-satunnaisuus-ja-toistettavuus.md)).

---

← [7. Esitys](07-esitys.md) · [Sisällys](README.md) · [9. Moottorin funktiot ja tapahtumat](09-moottorin-funktiot-ja-tapahtumat.md) →
