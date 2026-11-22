[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 11. Käyttöliittymän data

## 11.1 Asettelu

Asettelu jakaa ikkunan alueisiin. Piirros kertoo vain alueiden järjestyksen, ja mitat tulevat legendistä:

```
layout MAIN
mms
mmi
hhh
lll

legend:
- m map
- s STATS(player) size.width 20
- i INVENTORY size 20x10
- h hover size.height 1
- l LOG size.height 3
```

- **Valinta.** `game`-lohkon kenttä `layout ASETTELU [when EHTO]` valitsee käytettävän asettelun (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)). Kenttiä voi olla useita, ja ehto valitsee asettelun ikkunan koon ja suunnan mukaan (**Kokoehto** alla). Jos pelissä on yksi asettelu, se on oletus eikä kenttää tarvita. Useampi asettelu ilman kenttää on latausvirhe, joka luettelee asettelut.
- **Muoto:** jokaisen alueen täytyy olla suorakulmio. Asettelun ei tarvitse olla jaettavissa suorilla viivoilla.
- **Raidat:** asettelu on ruudukko sarakkeita ja rivejä. Sarakkeen tai rivin raja on jokaisessa kohdassa, jossa jonkin alueen reuna on piirroksessa, joten piirroksessa kohdakkain olevat reunat ovat kohdakkain myös ruudulla. Esimerkissä `s` ja `i` ovat samassa sarakkeessa, ja `h` ja `l` ulottuvat kummankin sarakkeen yli.
- **Mitat:** legendin `size.width` ja `size.height` (luku [2.4](02-yleinen-syntaksi.md#24-arvot)) antavat alueen raitojen koon. Kun alue kattaa useamman raidan, mitta jaetaan niille raidoille, joilla ei vielä ole mittaa, niiden piirrosleveyden suhteessa. Ristiriitaiset mitat ja solumäärien ja prosenttien sekoittaminen samoilla raidoilla ovat latausvirheitä.
- **Täyttö:** raita ilman mittaa täyttää jäljelle jäävän tilan muiden tällaisten kanssa niiden piirrosleveyden suhteessa. Piirros on asettelun pienin sallittu koko, ja alueen sisältö (paneelin piirros ja reuna) kasvattaa tarvittaessa sen raitoja. Jos kaikilla raidoilla on mitta, ylimääräinen tila menee viimeiselle.
- **Alueen nimi** on yleinen nimi, johon paneelit (luvut [9.3](09-ulkoasu.md#93-paneelit-lokit-ja-esitys) ja [11.5](#115-paneelit)) ja loki (luku [11.4](#114-viestit-ja-loki)) viittaavat. Pieni nimi `map` on moottorin lähde, johon kartta piirretään.
- **Hover-alue.** Pieni nimi `hover` on moottorin lähde, johon moottori kirjoittaa hiiren osoittimen, katselukursorin tai tähtäyksen kohdistimen alla olevan kohteen kuvauksen, mutta vain ruudusta, jonka pelaaja näkee tai muistaa. Moninpelissä näkökenttä on kaikkien jäsenten yhteinen. Asettelussa saa olla yksi `hover`-alue, ja toinen on latausvirhe (moottoridokumentti, luvut [7.1](../moottori/07-esitys.md#71-asettelu) ja [8.1](../moottori/08-syote.md#81-hiiri)).
- **Lauserivi.** Pieni nimi `sentence` on moottorin lähde, johon moottori kirjoittaa valitun verbin ja kohteen lauseen tekstistä `<KOMENTO>.SENTENCE` (luvut [10](10-tekstit.md) ja [11.2](#112-valikot)). Asettelussa saa olla yksi `sentence`-alue.
- **Telakoitu valikko.** `- v menu NIMI` piirtää valikon alueeseen pysyvästi. Valikko ei ole modaalinen: hiiri valitsee sen rivejä suoraan, ja näppäimistö saa sen fokukseen toiminnolla `focus NIMI` (luku [11.3](#113-näppäimet-hiiri-ja-gamepad)). Rivi näyttää kohdan toiminnon ensimmäisen sidonnan oikeaan reunaan tasattuna, olipa toiminto komento, moottorin toiminto (`look`) tai `open VALIKKO`, joten tavallinen telakoitu valikko toimii klikattavana näppäinohjeena. Telakoitu valikko on tyypillisesti verbivalikko, mutta myös listavalikko kelpaa, esimerkiksi reppu, jonka riviä klikkaamalla valitaan verbin kohde (moottoridokumentti, luku [7.1](../moottori/07-esitys.md#71-asettelu)).
- **Paneelin parametrit.** Kun alueeseen piirretään parametrillinen paneeli (luku [11.5](#115-paneelit)), argumentit annetaan alueen nimen perään RogueScriptin muodossa (`- s STATS(player)`). Argumentit ovat kyselylausekkeita, jotka moottori laskee jokaisessa välikehyksessä.

**Kokoehto.** Peli voi antaa eri asettelun leveälle ja kapealle ikkunalle sekä vaaka- ja pystyasennolle, esimerkiksi puhelimelle:

```
game SEIKKAILU
- layout WIDE
- layout PHONE_TALL when portrait and width < 60
- layout PHONE_WIDE when landscape and height < 24
- layout NARROW when width < 80
```

- **Järjestys.** Ehdolliset rivit kokeillaan kirjoitusjärjestyksessä, ja ensimmäinen, jonka ehto pätee, valitaan. Ehdoton rivi on oletus, joka valitaan, kun mikään ehto ei päde, ja sen paikalla listassa ei ole väliä. Kun `layout`-rivejä on useampi, täsmälleen yhdellä ei saa olla ehtoa. Ehdottoman rivin puuttuminen ja kaksi ehdotonta riviä ovat latausvirheitä.
- **Ehto** on yksi tai useampi osa, jotka yhdistetään sanalla `and`: `width VERTAILU N` ja `height VERTAILU N` vertaavat ikkunan kokoa soluina (`<`, `<=`, `>`, `>=`), ja `portrait` tai `landscape` katsoo ikkunan suunnan. Muu sana on latausvirhe. Ehto ei ole RogueScriptiä, eikä se näe pelin tilaa, joten sama ikkuna antaa aina saman asettelun.
- **Suunta.** `portrait` tarkoittaa, että ikkuna on pikseleinä korkeampi kuin leveä, ja `landscape` muuta, myös neliötä. Pikselikoko tulee alustalta: selain ja graafinen ikkuna tietävät sen, ja pääte kertoo sen, jos osaa. Muuten moottori olettaa solun kaksi kertaa niin korkeaksi kuin leveäksi, jolloin 80×24-pääte on vaaka-asennossa ja 40×40-pääte pystyasennossa.
- **Vaihto.** Kun ikkunan koko tai suunta muuttuu, esimerkiksi puhelin käännetään, moottori valitsee asettelun uudelleen. Alueiden esitystila säilyy alueen nimen mukaan: samanniminen loki jatkaa samasta vierityksestä, välilehtialue samasta välilehdestä ja telakoitu valikko samasta fokuksesta, ja fokus poistuu, jos valikkoa ei uudessa asettelussa ole. Vaihto on esitystä eikä vie pelin aikaa.
- **Pienin koko.** Ehto ei katso asettelun vähimmäiskokoa. Jos valittu asettelu ei mahdu ikkunaan, moottori leikkaa raitoja kuten yhdellä asettelulla (moottoridokumentti, luku [7.1](../moottori/07-esitys.md#71-asettelu)), joten kirjoittaja valitsee rajat niin, että kukin asettelu mahtuu ehtonsa ikkunoihin.
- **Sama peli kaikilla ikkunoilla.** `view_width()` ja `view_height()` luetaan aina oletusasettelusta, ei valitusta, joten logiikka kulkee samoin asettelusta riippumatta (luku [4](../moottori/04-satunnaisuus-ja-toistettavuus.md)).
- **Pelaajan valinta.** Kentät luetaan datasta, joten pelaaja voi `overrides/`-tiedostossa poistaa ehdot ja pitää yhden asettelun (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)).

**Välilehdet.** Legendin rivi `- t tabs NIMI NIMI …` tekee alueesta välilehtialueen. Kapeassa ikkunassa sivupalkin sisällöt mahtuvat näin yhteen alueeseen:

```
layout PHONE_TALL

m
h
t

legend:
- m map
- h hover size.height 1
- t tabs PAD KEYS PACK(player) SIDE size.height 14
```

- **Sisällöt.** Jokainen nimi on paneeli argumentteineen (`PACK(player)`), valikko, joka telakoidaan välilehdelle, tai loki. Moottorin lähteet `map`, `hover` ja `sentence` eivät kelpaa välilehdiksi, koska niitä on asettelussa yksi.
- **Välilehtirivi.** Alueen yläreunassa ovat välilehtien otsikot tekstiavaimista `<NIMI>.TAB` (`PAD.TAB Liiku`) tasaleveinä nappeina, ja valitun välilehden sisältö piirretään niiden alle. Otsikot piirretään nappien ulkoasulla (`panels`-rivi `button`, luku [9.3](09-ulkoasu.md#93-paneelit-lokit-ja-esitys)): reunallisina ne ovat kolme riviä korkeita, ja ilman reunaa rivi on yksi. Valittu otsikko on täytetty reunan värillä tai käänteinen, ja osoittimen alla oleva on lihavoitu. Mahtumaton otsikko katkaistaan.
- **Keskitys.** Jos alue on sisältöään suurempi, välilehtirivi ja sisältö keskitetään pystysuunnassa, kun jokaisella välilehdellä on oma koko (paneeli tai valikko), ja paneeli keskitetään vaakasuunnassa. Esimerkiksi vaaka-asennon sivupalkissa sisältö on näin keskellä eikä yläreunan alla.
- **Valinta.** Alussa valittuna on ensimmäinen välilehti. `click` otsikkoon valitsee välilehden, ja moottorin toiminto `tab NIMI` valitsee välilehden, jonka sisältö on NIMI. `tab.next` ja `tab.prev` kiertävät välilehtiä; asettelussa, jossa välilehtialueita on useampi, ne kiertävät sitä, jonka välilehteä viimeksi vaihdettiin, ja alussa ensimmäistä (luku [11.3](#113-näppäimet-hiiri-ja-gamepad)). Valinta on esitystä, ei pelin tilaa, eikä se tallennu.
- **Mitat.** Välilehtialueen vähimmäiskoko on sen suurimman sisällön koko ja välilehtirivi. Sisältö piirretään välilehtirivin alle kuten omaan alueeseensa, ja `panels`-rivi nimetään sisällön nimellä (luku [9.3](09-ulkoasu.md#93-paneelit-lokit-ja-esitys)).
- **Laskenta.** Moottori laskee vain valitun välilehden `value`-kentät ja listavalikon rivit (moottoridokumentti, luku [7.7](../moottori/07-esitys.md#77-paneelit)).

**Kameran seuranta.** Legendin `map`-rivi voi saada seurantatavan: `- m map follow center` (oletus) tai `- m map follow margin 4`. Kamera seuraa oletuksena pelaajaa, moninpelissä kartta-alueen kartan jäsenten rajaavaa suorakulmiota, ja logiikka vaihtaa kohteen funktiolla `follow` (moottoridokumentti, luku [7.1](../moottori/07-esitys.md#71-asettelu)).

**Vuoron merkki.** Kun jäseniä on useampi, moottori korostaa vuorossa olevan jäsenen ruudun. Kartta-alueen rivin määre `turn TYYLI` valitsee korostuksen, esimerkiksi `- m map follow center turn reverse`, ja ilman määrettä korostus on käänteiset värit.

**Kursorit.** Kartta-alueen rivin määre `cursor TYYLI` valitsee katselukursorien, ennakkokäskyjen ja hiiren osoittimen ruudun merkin (`- m map turn reverse cursor underline`). Tyyli on `underline`, `reverse`, `bold` tai `bg VÄRI`, joka vaihtaa ruudun taustavärin (`cursor bg CURSOR`, väri luvun [9.1](09-ulkoasu.md#91-värit) mukaan). Ilman määrettä merkki on alleviivaus (moottoridokumentti, luku [8.1](../moottori/08-syote.md#81-hiiri)).

**Ruudukkomuoto.** Asettelun voi kirjoittaa myös alueiden nimistä koottuna ruudukkona, kun merkkipiirros on hankala. Tämä tekee saman asettelun kuin esimerkki yllä:

```
layout MAIN grid
- columns * 20
- rows * 10 1 3

map   STATS(player)
map   INVENTORY
hover hover
LOG   LOG
```

- Otsikossa on sana `grid`. Ruudukon rivit ovat välilyönnein erotettuja alueita: alueen nimi argumentteineen (`STATS(player)`) tai moottorin lähde (`map`, `hover`, `sentence`). Telakoitu valikko kirjoitetaan soluun muodossa `menu:NIMI`. Sama nimi viereisissä soluissa venyttää alueen niiden yli, ja alueen täytyy olla suorakulmio. Jokaisella rivillä on yhtä monta solua, ja välilyöntejä saa käyttää tasaamiseen.
- Jokainen solu on oma raitansa. `- columns` antaa mitan jokaiselle sarakkeelle ja `- rows` jokaiselle riville (luku [2.4](02-yleinen-syntaksi.md#24-arvot): solumäärä, prosentti tai `*`). Jos `- columns`- tai `- rows`-rivi puuttuu, kaikki sen raidat ovat `*`. `*`-raidat jakavat jäljelle jäävän tilan tasan. Useamman raidan yli venyvä alue on raitojensa summa.
- Rivit `- columns` ja `- rows` voivat olla ennen ruudukkoa (tyhjä rivi välissä sallitaan) tai heti sen jälkeen. Legendiä ei ole. Alueen määreet, kuten kartan seuranta, annetaan omalla rivillään: `- map follow margin 4`.
- Väärä määrä mittoja suhteessa sarakkeisiin tai riveihin, eri pituiset rivit ja tuntematon rivi ovat latausvirheitä.

## 11.2 Valikot

```
menu PAUSE size 24x*
- RESUME
- SAVE disabled when game has PERMADEATH
- OPTIONS open OPTIONS
- QUIT_TO_MENU quit

menu OPTIONS context POPUP
- SOUND open SOUND
- BACK back
```

- **Otsikko:** `menu NIMI [size LxK] [context KONTEKSTI] [final] [verbs]`. Valikon näppäinkonteksti on oletuksena `MENU` (luku [11.3](#113-näppäimet-hiiri-ja-gamepad)).
- **Kohta** on `- AVAIN [TOIMINTO] [hide when ehto] [disabled when ehto]`. Avain on tekstiavain.
- **Toiminto:** ilman toimintoa kohta ajaa samannimisen komennon tai, jos sellaista ei ole, samannimisen moottorin toiminnon pienillä kirjoitettuna (`- RESUME` → `resume`). Jos kumpaakaan ei ole, kohta on latausvirhe. Poikkeava toiminto kirjoitetaan perään: iso nimi on komento, pieni nimi moottorin toiminto (`quit`), `open VALIKKO` avaa toisen valikon, `back` palaa edelliseen ja `close` sulkee valikon.
- **Avaaminen ja pino:** `open NIMI` palaa valikkoon tai paneeliin, joka on jo pinossa: sen päällä olevat sulkeutuvat, ja se avautuu uudelleen samalle paikalle. Näin vaiheittaisen valikon Takaisin-kohta (`- BACK open EDELLINEN`) ja edellisen vaiheen `open SEURAAVA` eivät kasvata pinoa. `open NIMI stack` avaa aina uuden kerroksen, vaikka sama valikko olisi jo auki, esimerkiksi kun samaa listavalikkoa porataan syvemmälle eri argumenteilla (`- then open DIR(it) stack`, listassa `keep`). Jokainen kerros säilyttää omat argumenttinsa, rivinsä ja kursorinsa. `back` sulkee kummassakin tapauksessa vain päällimmäisen (moottoridokumentti, luku [7.5](../moottori/07-esitys.md#75-valikot)).
- **Uusi peli.** `new_game` on sallittu kaikissa valikoissa. Pelin aikana se päättää nykyisen pelikerran kuten `quit`, eli tallentaa `autosave`-osion `quit`-rivin mukaan (luku [3.4](03-tiedostot-ja-include.md#34-automaattinen-tallennus-autosave)), ja aloittaa sitten uuden pelin uudella siemenellä (moottoridokumentti, luku [8](../moottori/08-syote.md)).
- **Ehdot:** `hide when ehto` piilottaa kohdan, kun ehto pätee. `disabled when ehto` himmentää kohdan, kun ehto pätee: se näkyy, mutta sitä ei voi valita. Jos kohdassa on molemmat, `hide when` tulee ensin. Himmennetyn kohdan väri tulee tyylitiedostosta (luku [9.3](09-ulkoasu.md#93-paneelit-lokit-ja-esitys)).
- **Arviointihetki:** kaikkien kohtien ehdot arvioidaan kerralla, kun valikko avautuu, kuten dialogin `choice` (RogueScript, luku [9.7](../roguescript/09-kohtaukset-ja-aikajanat.md#97-dialogit)). Kohtauksen sisällä avattu valikko näkee kohtauksen luonnoksen.
- **Tekstit:** valikon otsikko haetaan avaimella `<VALIKKO>.TITLE`, leipäteksti avaimella `<VALIKKO>.TEXT` ja kohdan teksti avaimella `<VALIKKO>.<KOHTA>` (`PAUSE.RESUME`, luku [10](10-tekstit.md)). Siksi `TITLE` ja `TEXT` eivät kelpaa kohtien nimiksi.
- **Paikkaaminen:** `patch menu` lisää kohtia valikon loppuun tai muuttaa niitä (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- **Logiikka valikossa:** kohdan ehdot ja `then`-osa, avauksen sidotut nimet ja listavalikot ovat pelilogiikan rajapintaa (luku [1.2](01-rajaus-ja-periaatteet.md#12-rajapinnat-pelilogiikkaan)).
- **Dialogit** ovat RogueScriptin kohtauksia (RogueScript, luku [9.7](../roguescript/09-kohtaukset-ja-aikajanat.md#97-dialogit)).

**Verbivalikko: `verbs`.** Määre `verbs` tekee valikosta seikkailupelien tapaisen komentovalikon. Jokainen kohta on komento, ja muu toiminto kohdassa on latausvirhe.

```
menu VERBS verbs
- LOOK_AT
- TAKE
- OPEN
- THROW
- QUAFF
```

- **Pikanäppäin.** Rivi näyttää kohdan tekstin ja komennon sidonnan oikeaan reunaan tasattuna (`Heitä     t`). Moninpelissä sidonta on vuorossa olevan istuimen laitteen.
- **Valinta.** Parametriton komento ajetaan heti. Muuten komennosta tulee valittu verbi, ja jokainen klikkaus kartalla tai telakoidussa listavalikossa täyttää yhden sen klikattavista parametreista (`from`, `aim` tai suunta `ask`) missä järjestyksessä tahansa: "Avaa" → ovi → avain toimii yhtä hyvin kuin "Avaa" → avain → ovi. `rclick` peruu viimeisen täytön. Näppäimillä parametrit kysytään tavalliseen tapaan järjestyksessä.
- **Lauserivi.** Teksti `<KOMENTO>.SENTENCE` näyttää valinnan asettelun `sentence`-alueessa, esimerkiksi `THROW.SENTENCE Heitä {item} kohteeseen {to}`. Paikkamerkit ovat komennon parametrien nimiä (luku [10](10-tekstit.md)).
- **Kelpaamaton kohde.** Teksti `<KOMENTO>.NO_TARGET` näkyy `hover`-alueessa, kun klikattu kohde ei kelpaa.
- **Verbikolikko.** Sama valikko antaa toiminnon `verbs` ehdokkaat: oikea klikkaus näyttää ne kohdat, joiden jollekin parametrille klikattu ruutu kelpaa (luku [11.3](#113-näppäimet-hiiri-ja-gamepad)). Ilman verbivalikkoa ehdokkaat ovat peruskontekstiin sidotut komennot, ja RogueScriptin määre `noverb` piilottaa komennon niistä.
- Verbivalikko telakoidaan asettelun alueeseen (`- v menu VERBS`, luku [11.1](#111-asettelu)), tai sen voi avata modaalisena (`open VERBS`). Moottorin käytös on moottoridokumentin luvussa [8.1](../moottori/08-syote.md#81-hiiri).

**Lopullinen valikko: `final`.** Määre `final` tekee valikosta sulkeutumattoman: `back` ja `close` eivät sulje sitä, ja se poistuu vain kohdan toiminnolla, joka vie pois pelistä tai valikosta (`quit`, `new_game`, `exit`, `open`). `ALWAYS`-konteksti toimii silti, joten taukovalikko avautuu sen päälle (luku [11.3](#113-näppäimet-hiiri-ja-gamepad)). Valikko sopii pelin loppuun:

```
menu VICTORY final
- NEW_GAME new_game
- QUIT
```

Lataus tarkistaa, että `final`-valikossa on ainakin yksi kohta, joka ei ole himmennettävissä eikä piilotettavissa ja jonka toiminto vie pois valikosta.

**Listavalikko.** Otsikon sana `list` tekee valikosta listan, jonka rivit tulevat joukosta. Otsikko voi saada parametreja RogueScriptin muodossa, ja `open VALIKKO(…)` antaa ne.

```
menu SHOP.BUY(wares: collection, merchant: entity) list size 36x*
- source wares
- row ITEM: it, PRICE: PRICE_OF(it, merchant)
- sort PRICE_OF(it, merchant)
- disabled when PRICE_OF(it, merchant) > player.PURSE.coin
- then BUY(it, merchant)
- keep
```

- `source LAUSEKE` on joukko, ja rivin lausekkeissa `it` on sen jäsen.
- `row PARAMETRIT` antaa tekstiavaimen `<VALIKKO>.ROW` parametrit `say`-kutsun muodossa.
- `sort LAUSEKE [desc]` järjestää rivit; ilman sitä järjestys on lähteen järjestys.
- `hide when` ja `disabled when` ovat rivikohtaisia ehtoja.
- `then KUTSU` ajaa valitulla rivillä funktion tai komennon, ja `keep` pitää valikon auki valinnan jälkeen.
- Tekstiavaimet ovat `<VALIKKO>.TITLE`, `.TEXT`, `.ROW` ja `.EMPTY`. Listavalikossa ei ole tavallisia kohtia.
- **Kirjaimet.** Kun lähteen kaikki jäsenet ovat samassa kokoelmakentässä, rivit saavat jäsentensä kokoelmakirjaimet (luku [5.4](05-komponentit-ja-entiteetit.md#54-kokoelmat)) ja ovat oletuksena kirjainjärjestyksessä, ellei `sort` järjestä niitä toisin. Repun suodatettu lista näyttää siis juomat kirjaimilla `f` ja `h`, jos ne ovat repussa paikoilla `f` ja `h`, ja `q h` juo aina saman juoman. Jos lähde yhdistää eri kokoelmia tai kartan entiteettejä, rivit saavat kirjaimet a–z näkyvässä järjestyksessä.
- **Pinot.** Pinon jäsenet (luku [5.4](05-komponentit-ja-entiteetit.md#54-kokoelmat)) näkyvät yhtenä rivinä, ja `it` on pinon ensimmäinen jäsen. Pinon koko on `it.stack.count` (RogueScript, luku [2.6](../roguescript/02-yleinen-syntaksi.md#26-tyypit)), ja rivin teksti voi näyttää sen paikkamerkillä `{ITEM:COUNT}` (luku [10](10-tekstit.md)).
- Moottorin käytös on moottoridokumentin luvussa [7.5](../moottori/07-esitys.md#75-valikot).

**Säädettävä kohta.** `- AVAIN adjust KENTTÄ a..b [pool NIMI]` muuttaa kentän arvoa vasemmalla ja oikealla eikä sulje valikkoa. `pool` on luku, josta korotukset vähennetään (moottoridokumentti, luku [7.5](../moottori/07-esitys.md#75-valikot)).

## 11.3 Näppäimet, hiiri ja gamepad

```
keys ALWAYS
- open PAUSE esc

pad ALWAYS
- open PAUSE select

keys GAME
- MOVE.NORTH k up w num8
- QUAFF q
- SEARCH s
- repeat #
- pass z
- look x
- cursor c
- follow f
- focus VERBS v
- travel_to click
- save ctrl+s
- help ? f1
- quit shift+q

pad GAME deadzone 40%
- MOVE.NORTH dpad.up lstick.up
- confirm south
- pass west
- look north
- save lb+start

keys POPUP passthrough
- close backspace

keys SCENE
- confirm enter space
- skip tab
- restart r
- rewind backspace
```

- **Otsikko:** `keys KONTEKSTI [modal | passthrough] [keyboard N]` ja `pad KONTEKSTI [deadzone P%]`.
- **Rivi** on `- TOIMINTO näppäimet...`. Iso nimi on komento (`MOVE.NORTH`, `QUAFF`) ja pieni nimi moottorin toiminto (`save`, `quit`, `pass`, `help`, `confirm`, `back`, `close`, `resume`, `skip`, `restart`, `rewind`, `repeat`, `look`, `cursor`, `follow`, `verbs`, `focus`, `travel_to`, `join`, `leave_seat`, `read`, `tab`, `tab.next`, `tab.prev`, `page.up`, `page.down`, `first`, `last`). `focus` saa alueen valikon nimen: `- focus VERBS v`, `read` lokin nimen: `- read LOG o` ja `tab` välilehden sisällön nimen: `- tab PACK i`. Kirjainkoko erottaa ne, joten samannimistä toimintoa ja komentoa ei voi syntyä.
- **Kohtauksen toiminnot** `skip`, `restart` ja `rewind` toimivat kontekstissa `SCENE` (RogueScript, luku [9.8](../roguescript/09-kohtaukset-ja-aikajanat.md#98-luonnos-ohitus-ja-alusta-aloitus)).
- **Vaihtoehdot** erotetaan välilyönnillä. Muuntonäppäimet ovat `ctrl`, `shift` ja `alt`, ja ne yhdistetään `+`-merkillä. Merkit `+` ja `-` kirjoitetaan `plus` ja `minus`. Erikoisnäppäimet ovat muun muassa `tab`, `enter`, `space`, `esc`, `backspace`, nuolet `up` `down` `left` `right` ja numeronäppäimistön `num0`…`num9`; `shift+tab` on tab vaihtonapilla. Vaihdetut merkit kirjoitetaan sellaisenaan (`?`, `#`).
- **Numeronäppäimistö päätteessä.** Pääte ei erota numeronäppäimistöä tavallisista numeroista, joten päätteessä `numN` on sama näppäin kuin `N`: `num8`-sidonta laukeaa myös tavallisesta `8`-näppäimestä. Alusta, joka erottaa ne, käyttää niitä erikseen. Saman kontekstin `num8` ja `8` eri toiminnoille on päätteessä ristiriita, ja lataus varoittaa siitä, koska muualla sidonta on kelvollinen.
- **Hiiren nimet** ovat `click` (vasen painike), `rclick` (oikea), `mclick` (keski), `wheel.up` ja `wheel.down`, ja niihin käyvät muuntimet (`shift+click`). Ne kirjoitetaan `keys`-riveille kuten näppäimet (`- travel_to click`, `- confirm enter click`), ja pelaaja paikkaa ne samoin. Hiiri on lisäsyöte: jokaisella hiiren toiminnolla on myös näppäin- tai peliohjainsidonta. Pelaaja voi poistaa hiiren käytöstä asetuksella `mouse` (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)). Hiiren nimet kuuluvat osan 1 osioon, ja hiiren nimi `keyboard N` -osiossa on latausvirhe, koska hiiri ei kuulu millekään osalle.
- **Gamepadin painikkeet** ovat paikkapohjaisia: `south`, `east`, `west`, `north`, `lb`, `rb`, `lt`, `rt`, `start`, `select`, `lstick`, `rstick` ja `dpad.up` jne. Tatit ovat suuntina (`lstick.up`).
- **Ristiriita** samassa kontekstissa on virhe. Eri konteksteissa sama näppäin saa olla, mutta jos `ALWAYS`-kontekstin näppäin on sidottu myös muualla, lataus varoittaa.
- **Kontekstit pinoutuvat** ajon aikana. Oletus on `modal`, jolloin vain päällimmäinen konteksti saa syötteet. `passthrough` päästää läpi syötteet, joita konteksti ei sido. `pad`-osio seuraa vastaavan `keys`-osion asetusta.
- **Moottorin kontekstit** ovat `MENU`, `DIALOGUE`, `SCENE`, `AIM`, `DIRECTION`, `LOOK`, `JOIN` ja `ALWAYS`. Pelin peruskonteksti nimetään `game`-lohkon kentällä `keys` (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)), ja se on pinon pohjalla myös pelin alkuvalikossa. Moottorin konteksteilla on oletussidonnat: valikossa ja dialogissa `cursor.north` `up k`, `cursor.south` `down j`, `confirm` `enter space`, `back` `esc backspace`, `page.up` `pageup`, `page.down` `pagedown`, `first` `home` ja `last` `end`; kohtauksessa `confirm` `enter space`, `skip` `tab`, `restart` `r` ja `rewind` `backspace`; kohdistuksessa ja katselussa (`AIM`, `LOOK`) `cursor.*` nuolet, `hjklyubn` ja numeronäppäimistö, `next` `tab`, `prev` `shift+tab`, `confirm` `enter space` ja `back` `esc`. Peliohjaimella `dpad` liikuttaa kohdistinta, `south` vahvistaa ja `east` palaa. Pelin peruskontekstissa `travel_to` on oletuksena `click` ja `verbs` `rclick`. Valikoissa hiiri toimii ilman sidontoja: osoitin korostaa rivin, `click` valitsee, rulla vierittää ja `click` valikon ulkopuolella on `back`. Modaalinen paneeli käyttää kontekstin `MENU` sidontoja (luku [11.5](#115-paneelit)). Pelin oma rivi samalle toiminnolle samassa kontekstissa korvaa oletuksen (moottoridokumentti, luvut [8](../moottori/08-syote.md) ja [8.1](../moottori/08-syote.md#81-hiiri)).
- **Pelaajan sidonnat** kirjoitetaan `overrides/keys.rlml`-tiedostoon, ja ne paikkaavat pelin sidonnat riveittäin (luvut [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data) ja [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)). Kirjasto tai modi lisää sidontoja samoin `patch keys`-lohkolla.
- `ALWAYS` on konteksti, jota moottori kuuntelee kaikkien muiden kontekstien päällä ja myös silloin, kun peli etenee ilman pelaajan vuoroa. Siihen sidotaan taukovalikko, jotta pelistä pääsee aina pois. Sama sidonta sulkee avoimen valikon; `open NIMI stack` -sidonta avaa sen sijaan aina uuden kerroksen. Lataus varoittaa, jos `ALWAYS`-kontekstin näppäin on sidottu myös muualla (moottoridokumentti, luku [8](../moottori/08-syote.md)).
- `pass` ajaa seuraavan kierroksen heti, kun pelaajalla ei ole vuoroa (luku [3.2](03-tiedostot-ja-include.md#32-pelaajan-oma-data)).
- **Toistolaskuri.** `repeat` avaa lukukentän. Numerot kirjoittavat kenttään sidonnoista riippumatta, myös numeronäppäimistön numerot, ja ensimmäinen muu näppäin päättää sen ja tulkitaan komennon näppäimeksi: `#20s` etsii 20 kertaa. Luku menee komennon `count`-parametriin (RogueScript, luku [7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)), ja `back` peruu (moottoridokumentti, luku [8](../moottori/08-syote.md)).

**Näppäimistöosat.** Paikallisessa moninpelissä (`seats`, luku [3.3](03-tiedostot-ja-include.md#33-aloitus)) näppäimistön voi jakaa usealle istuimelle. Osa on näppäimistön sidontojen joukko, ei istuinnumero, koska istuinten numerot riippuvat liittymisjärjestyksestä.

```
keys GAME
- MOVE.NORTH k up
- SWITCH tab

keys GAME keyboard 2
- MOVE.NORTH w
- SWITCH q
- join e
```

- **Osa 1** on osio ilman `keyboard`-määrettä. `keyboard N` on lisäosa. Osio tunnistetaan kontekstin nimestä ja osan numerosta, joten `keys GAME` ja `keys GAME keyboard 2` ovat eri lohkoja yhdistämisessä ja pelaajan `overrides/`-paikkauksessa. `modal` ja `passthrough` annetaan osassa 1, ja lisäosan eri arvo on latausvirhe.
- **Ristiriita.** Sama näppäin kahdessa näppäimistöosassa samassa kontekstissa on latausvirhe, koska moottori ei tietäisi, kenen painallus se on.
- **Muut kontekstit.** Konteksti ilman lisäosia, kuten moottorin `MENU` oletuksineen, käyttää osan 1 sidontoja kaikille näppäimistön istuimille, ja ohjaajan rajaa reititys (moottoridokumentti, luku [8](../moottori/08-syote.md)).
- **Peliohjaimet** käyttävät samaa `pad`-osiota. Laite erottaa istuimet, joten ohjainkohtaisia osioita ei tarvita. **Hiiri** on yhteinen laite, jota käytetään vuorotellen: sen painallukset menevät istuimelle, jonka vuoro on (moottoridokumentti, luku [8.2](../moottori/08-syote.md#82-istuimet-ja-laitteet)).
- **`join`** liittää uuden istuimen. Moottori sitoo sen vain kontekstissa `JOIN` (`enter`, peliohjaimella `start`). Pelin peruskontekstissa peli sitoo sen itse jokaiseen osaan, josta liittyminen kesken pelin on mahdollista, ja lisäosa ilman `join`-sidontaa saa varoituksen. `leave_seat` poistaa istuimen, ja se kuuluu pelin taukovalikkoon.
- **Konteksti `JOIN`** on uuden pelin liittymisruutu. Liittämättömältä laitteelta `enter` on `join`, liitetyltä `confirm`, ja `back` poistaa laitteen istuimen. Ladatussa pelissä `skip` ohittaa tallennetun istuimen.
- `keyboard N` yhden istuimen pelissä saa varoituksen.

**Konteksti `DIRECTION`** on moottorin konteksti parametrin lähteelle `ask` (RogueScript, luku [7.1](../roguescript/07-komennot-ja-tekoaly.md#71-komennot)). Se käyttää kontekstin `AIM` suuntasidontoja, ja `back` peruu komennon.

**Katselukursori: konteksti `LOOK`.** Moottorin toiminto `look` avaa kontekstin `LOOK`, jossa näppäimin tai peliohjaimella ohjattava kursori tekee saman kuin hiiren osoitin: `hover`-alue (luku [11.1](#111-asettelu)) näyttää kursorin ruudun kuvauksen, `confirm` kulkee kursorin ruutuun klikkausmatkana (`walk`, luku [3.3](03-tiedostot-ja-include.md#33-aloitus)) ja `back` sulkee kontekstin. Kursori alkaa pelaajan ruudusta, ja `next` ja `prev` hyppäävät näkyvien entiteettien välillä. Kontekstilla on samat oletussidonnat kuin kontekstilla `AIM`. Moottori ei sido toimintoa `look` itse, koska peruskonteksti on pelin oma, joten peli sitoo sen (`- look x`, `pad`: `- look north`). Tähtäyksessä (`AIM`) `hover`-alue näyttää samoin kohdistimen ruudun kuvauksen (moottoridokumentti, luku [8.1](../moottori/08-syote.md#81-hiiri)).

- **Pysyvä kursori.** `cursor` avaa saman kursorin pysyvänä: se ei sulkeudu matkan alkaessa, ja moninpelissä se on istuimen oma ja toimii myös muiden vuorolla. Silloin `confirm` ja `follow` tallentavat ennakkokäskyn, jonka moottori ajaa istuimen seuraavalla vuorolla. `back` peruu käskyn ja sulkee sitten kursorin.
- **Seuraaminen.** `follow` seuraa kursorin alla olevaa toimijaa ja muuten kentän `leader` antamaa johtajaa (luku [3.3](03-tiedostot-ja-include.md#33-aloitus)). Mikä tahansa painallus katkaisee sen.
- **Verbikolikko.** `verbs` avaa kursorin tai osoittimen ruutuun valikon komennoista, joille ruutu kelpaa kohteeksi (luku [11.2](#112-valikot)). Peliohjaimella se toimii katselukursorissa pelin sidonnalla (`pad LOOK`: `- verbs west`).
- **Fokus.** `focus NIMI` siirtää näppäinfokuksen telakoituun valikkoon (luku [11.1](#111-asettelu)). Fokuksessa valikko käyttää kontekstin `MENU` sidontoja `passthrough`-kontekstina, `confirm` valitsee ja `back` palauttaa fokuksen kartalle.
- Moottori ei sido toimintoja `cursor`, `follow` ja `focus`, joten peli sitoo ne.

**`toggle PANEELI`** näyttää tai piilottaa paneelin (luku [11.5](#115-paneelit)). Debug-paneeli sidotaan tyypillisesti kontekstiin `ALWAYS`: `- toggle DEBUG f12`.

## 11.4 Viestit ja loki

```
messages
- MESSAGE.COMBAT.HIT TARGET AMOUNT
- MESSAGE.LOOT.PICKUP ITEM
- MESSAGE.WAIT

log LOG history 200 merge
- MESSAGE.COMBAT
- MESSAGE.LOOT.PICKUP
- SPEECH
```

- **Viesti** on rivi `- AVAIN [PARAMETRIT]`. Avain on sama kuin kielitiedoston avain (`MESSAGE.COMBAT.HIT Osuit kohteeseen {TARGET}, vahinko {AMOUNT}.`). Lataus tarkistaa, että tekstin `{...}`-kohdat vastaavat parametreja. Viestillä ei ole erillistä luokkaa, vaan avaimen ryhmä toimii luokkana: `MESSAGE.COMBAT.HIT` kuuluu ryhmiin `MESSAGE.COMBAT` ja `MESSAGE`.
- **Monta osiota:** `messages`-osiot yhdistyvät riveittäin (luku [4.8](04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)).
- **Lokiin kirjoittaminen.** RogueScriptin `log` käyttää vain tässä ilmoitettuja viestejä. Modaalinen `say` ei vaadi ilmoitusta, mutta lataus tarkistaa sen paikkamerkit kutsun argumentteja vasten (RogueScript, luku [9.6](../roguescript/09-kohtaukset-ja-aikajanat.md#96-say-ja-log)).
- **Valinta.** `log`- ja `logs`-rivi on ryhmä tai yksittäinen avain, ja se valitsee viestit, joiden avain on rivin nimi tai alkaa sillä. Viesti voi näkyä useassa lokissa. Värin ratkaisee tarkin osuma: avainrivi voittaa ryhmärivin ja pidempi ryhmä lyhyemmän. Viesti, jota yksikään `log`-lohko ei valitse, näkyy kaikissa lokeissa. Lataus tarkistaa, että rivin nimi on jonkin viestiavaimen alku. Moottorin ryhmä `SPEECH` sisältää `say`-repliikit.
- `log` sitoo asettelun alueen (`LOG`) ja valitsee näytettävät viestit: `log ALUE [history N | history all] [merge] [more]`. `history` on muistissa pidettävien viestien määrä, ja `history all` pitää kaikki pelikerran viestit. `merge` yhdistää peräkkäiset samat viestit.
- **Lokin lukeminen: `read`.** Moottorin toiminto `read LOKI` (`- read LOG o`, valikon kohdassa `- READ_LOG read LOG`) avaa lokin luettavaksi koko ikkunan kokoisena: viestit rivittyvät ikkunan leveyteen, ja näkymä alkaa uusimmasta. Pelaaja selaa taaksepäin aina `history`-määrän alkuun asti, `history all` -lokissa pelin ensimmäiseen viestiin. Ylimmällä rivillä on teksti `<LOKI>.TITLE` ja alimmalla `<LOKI>.READ`, jos ne on kirjoitettu (`LOG.READ ↑↓ selaa  esc sulje`). Värit ja `fade` ovat samat kuin lokin alueessa, ja `fade` lasketaan näkymän riveistä. Näkymä käyttää kontekstia `MENU`: `cursor.north` ja `cursor.south` vierittävät rivin, `page.up` ja `page.down` näkymän korkeuden, `first` ja `last` siirtävät alkuun ja loppuun, rulla ja pystysuora pyyhkäisy vierittävät, ja `back` tai `confirm` sulkee näkymän. Pelin aika pysähtyy lukemisen ajaksi kuten valikossa (moottoridokumentti, luku [7.4](../moottori/07-esitys.md#74-loki-ja-dialogit)).
- **Pitkä viesti.** Viesti rivittyy alueen leveyteen sanojen välistä. Jos lokilla ei ole `more`-sanaa ja rivitetty viesti ei mahdu alueeseen, sen viimeinen näkyvä rivi katkaistaan merkillä `…` kuten paneelin kentässä (luku [11.5](#115-paneelit)). Yhden rivin lokissa pitkä viesti näkyy siis yhtenä katkaistuna rivinä.
- **Sivutus `more`.** Kun syötteen odotukseen mennessä on tullut enemmän uusia viestejä kuin alueeseen mahtuu, moottori näyttää ne alueen kokoisina erinä ja kirjoittaa erän loppuun tekstin `<LOKI>.MORE` (esimerkiksi `LOG.MORE --Lisää--`). Mikä tahansa näppäin näyttää seuraavan erän, ja `back` ohittaa loput. Yhden rivin viestirivi ei siis kadota viestejä (moottoridokumentti, luku [7.4](../moottori/07-esitys.md#74-loki-ja-dialogit)).

## 11.5 Paneelit

Paneeli on ASCII-piirros, jonka legendi sijoittaa kentät: tekstit, arvot, parametrit ja napit. Paneeli piirretään asettelun alueeseen tai avataan modaalisena koko näytön päälle.

```
panel STATS(who: entity)
LLLLLL ddd  GGGGG gggggg  HHH hhh(xxx)  SSSS sss(yyy)

legend:
- L text STATUS.LEVEL
- d value game.DEPTH.now
- G text STATUS.GOLD
- g value who.PURSE.gold
- H text STATUS.HP
- h value who.HP.now right
- x value who.HP.max
- S text STATUS.STR
- s value who.STRENGTH.now right
- y value who.STRENGTH.max
```

- **Kenttä** on saman merkin yhtenäinen alue. Vaakajakso on yksi rivi, ja suorakulmio (sama merkki usealla peräkkäisellä rivillä samoissa sarakkeissa) on monirivinen kenttä, joka rivittää tekstin sanojen välistä. Välilyönti on tyhjä.
- **Kirjaimelliset merkit.** Piirroksen merkki, jota legendi ei mainitse, on osa kuvaa ja piirretään sellaisenaan. Yllä `(`, `)` ja `/` ovat kuvaa. Legendin merkit kannattaa siksi valita niin, ettei kuva käytä niitä.
- **Kenttälajit.**
  - `text AVAIN` näyttää tekstiavaimen valitulla kielellä, joten piirrokseen ei kirjoiteta käännettäviä sanoja. Avaimen paikkamerkit annetaan kuten listavalikon `row`-rivillä.
  - `value LAUSEKE` näyttää kyselylausekkeen arvon. Nimiarvo näkyy tekstinä (luku [10](10-tekstit.md)). Lista tai kokoelma näkyy monirivisessä kentässä jäsen riviä kohden ja yksirivisessä pilkuin eroteltuna, joten esimerkiksi näkyvien asioiden selite on yksi kenttä.
  - `param NIMI` näyttää otsikon parametrin arvon sellaisenaan. Se on lyhyt muoto lausekkeelle `value NIMI`, ja lataus tarkistaa, että nimi on otsikon parametri.
  - `button TOIMINTO text AVAIN` on nappi: kenttä näyttää avaimen tekstin keskitettynä kumpaankin suuntaan, ja `click` kenttään tekee toiminnon kuten telakoidun valikon kohta. Iso nimi on komento, pieni nimi moottorin toiminto argumentteineen (`look`, `read LOG`, `tab PACK`) ja `open VALIKKO` avaa valikon. `hold PANEELI` näyttää paneelin koko ikkunan kokoisena niin kauan kuin nappia pidetään painettuna, ja vapautus sulkee sen heti; alustalla, joka ei kerro vapautusta, seuraava klikkaus tai näppäin sulkee sen. Napin ulkoasu on `panels`-rivillä `button` (`- button border rounded fg YELLOW bg BLACK`, luku [9.3](09-ulkoasu.md#93-paneelit-lokit-ja-esitys)): reuna piirretään, kun kenttä on vähintään 3×3, ja ilman riviä nappi on käänteisin värein. Parametrillinen komento kysyy parametrinsa kuten näppäimellä. Osoittimen alla nappi korostuu, ja nappi on himmeä, kun sen toimintoa ei voi käyttää, esimerkiksi kun pelaajalla ei ole vuoroa. Painettuna pidetty nappi toistaa toimintonsa kuten pidetty näppäin, joten kosketusnäytön suuntanapit kävelevät. Tekstin paikkamerkki `{^TOIMINTO}` näyttää toiminnon näppäimen (luku [10](10-tekstit.md)).
- **Tasaus.** Kentän perään kirjoitetaan `left` (oletus), `right` tai `center`. Mahtumaton teksti katkaistaan merkillä `…`.
- **Parametrit.** Otsikko voi saada parametreja RogueScriptin muodossa kuten listavalikko (`panel GRAVE(who: text, cause: name, gold: number)`). Asettelussa parametrit annetaan alueen legendissä (luku [11.1](#111-asettelu)), ja modaalisena paneeli avataan logiikasta tai valikosta muodolla `open PANEELI(…)`.
- **Modaalinen paneeli.** `open PANEELI(…)` näyttää paneelin keskellä näyttöä piirroksen kokoisena, kunnes pelaaja painaa `confirm` tai `back`. Paneeli käyttää kontekstin `MENU` sidontoja, joten ohjeen kehote voi näyttää näppäimen paikkamerkillä `{^confirm}` (luku [10](10-tekstit.md)). Sopii hautakiveen, voittoruutuun, pistelistaan ja ohjeruutuun.
- **Kieli.** Piirroksen kuvaa ei käännetä: käännettävät sanat kirjoitetaan `text`-kenttiin.
- **Ulkoasu.** Asettelun alueeseen piirretyn paneelin koko on piirroksen koko, ja värit ja reunat tulevat `panels`-osiosta (luku [9.3](09-ulkoasu.md#93-paneelit-lokit-ja-esitys)).
- **Debug-paneeli** `panel DEBUG` voi näyttää minkä tahansa lausekkeen, ja `toggle DEBUG` (luku [11.3](#113-näppäimet-hiiri-ja-gamepad)) avaa ja sulkee sen. Moottorin käytös on moottoridokumentin luvussa [7.7](../moottori/07-esitys.md#77-paneelit).

```
panel GRAVE(who: text, cause: name, gold: number)
                       __________
                      /          \
                     / rrrrrrrrrr \
                    /              \
                   |  nnnnnnnnnnnnn |
                   |  ggggggggg Au  |
                   |  kkkkkkkkkkkkk |
                   |  ccccccccccccc |
                   |  yyyy          |
                  *|     *  *  *    | *
         ________)/\\_//(\/(/\)/\//\/|_)_______

legend:
- r text GRAVE.REST center
- n param who center
- g param gold right
- k text GRAVE.KILLED_BY center
- c param cause center
- y value today().year
```

---

← [10. Tekstit](10-tekstit.md) · [Sisällys](README.md) · [12. Äänten ilmoitukset](12-aanten-ilmoitukset.md) →
