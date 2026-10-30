[RLML](../README.md) › [RogueScript](README.md)

# 13. Avoimet kysymykset

- **Rivin jatkaminen.** Luku [2.2](02-yleinen-syntaksi.md#22-lohkot-ja-kommentit) ei kerro, saako pitkän lausekkeen jatkaa seuraavalle riville avoimen sulun sisällä (`foo(a,` ja seuraavalla rivillä `b)`). Speksin koodissa näin ei tehdä missään, ja toteutus (31.10.2026) käsittelee jokaisen rivin omana lauseenaan. Sallitaanko jatko sulkujen sisällä, vai pysyykö rivi aina yhtenä lauseena?
- **Tapahtumat luonnoksessa.** Käsittelijät ajetaan kohtauksen aikana luonnosta vasten. Pitääkö jokin tapahtuma (esimerkiksi saavutus) rajata niin, että se laukeaa vasta kirjauksessa?
- **`cancel` ja `after`.** Estääkö `cancel` myös `after`-käsittelijät? Moottorin tapahtumia kysymys ei koske, koska niissä ei ole `before`-vaihetta.
- **Vakiokirjaston törmäys.** Ratkaistu 24.10.2026: törmäys palauttaa jatkokomennon (luku [7.1](07-komennot-ja-tekoaly.md#71-komennot)), joten hyökkäys maksaa hyökkäyksen hinnan ja ei-vihamieliseen törmäys ei vie aikaa. Alkuperäinen kysymys: Jos liikekomento palauttaa `false` lähetettyään `STD.BUMP`-tapahtuman, pelaajan törmäyshyökkäys ei vie vuoroa (luku [7.1](07-komennot-ja-tekoaly.md#71-komennot)). Komennon on siksi tiedettävä, käsitelläänkö törmäys hyökkäyksenä.
- **Datan rajapinnat.** Kartan `zone`, prefabin parametrit, valikon `then` ja `with`, paikalliset osiot ja `timer`-komponentin tapahtumat kuvataan myöhemmin.
- **Kaikki kohdat suljettu.** Mitä tapahtuu, jos `choice`-kohdan tai valikon jokainen kohta on piilotettu tai himmennetty?
- **Taulun muunnin.** Taulu on staattista dataa. Tilanteesta riippuva heitto voisi saada muuntimen (`FRUITS.draw(bonus: depth)`), jolloin on päätettävä, mitä nopan alueen ulkopuolinen tulos tuottaa (dataformaatit, luku [13](../dataformaatit/13-avoimet-kysymykset.md)).
- **Checkpointit.** Säilytetäänkö vain viimeisin checkpoint vai lista, josta pelaaja voi valita? Lista vaatisi checkpointeille nimet.
- **Esitysaikainen ulkoasu.** Hallusinaatio vaihtaa näkyvien hirviöiden merkit vain esityksessä. Nyt sen voi tehdä vain muuttamalla `render.tile`-kenttää, mikä muuttaa tilaa pelkän esityksen takia. Tarvitaanko esitysaikainen tilen vaihto?

Suljettu 25.10.2026 (robotfindskitten-toteutustesti): kortti logiikassa on entiteetin nimen kutsu (luku [2.8](02-yleinen-syntaksi.md#28-ison-nimen-kutsu)), komponentin käsittelijä jättää subjektin pois (luku [3.1](03-komponentit.md#31-määrittely)), jokerin laji ja tekstiavainten jokeri (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)), tyhjä väli (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)), ruutujoukon `pick` ja `shuffled` (luku [5.4](05-funktiot-ja-lausekkeet.md#54-satunnaisuus)) sekä etualan kohtauksen kartta (luku [9](09-kohtaukset-ja-aikajanat.md)).

Suljettu 25.10.2026 (Rogue-toteutustesti): pinot ovat erillisiä ilmentymiä (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)), arvojonot listoja (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)), ja tapahtuman laajennus oletusarvoisella loppuparametrilla ei riko käsittelijöitä (luku [4](04-tapahtumat-ja-kasittelijat.md)).

Suljettu 23.10.2026 (ilta): komentojen `wait`. Odotus on sallittu missä tahansa komennon rungossa, ja kohta tallentuu rakenteellisena polkuna (luku [11.1](11-tallennus-ja-korvaaminen.md#111-tallennus)). Rekursion raja: kutsusyvyys `game.call_depth` (luku [5.2](05-funktiot-ja-lausekkeet.md#52-ohjausrakenteet-ja-muuttujat)).

---

← [12. Kielioppi](12-kielioppi.md) · [Sisällys](README.md)
