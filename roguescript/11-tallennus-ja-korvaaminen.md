[RLML](../README.md) › [RogueScript](README.md)

# 11. Tallennus ja korvaaminen

## 11.1 Tallennus

Kohtausten korutiineille ei tarvita suorituskohtaa, koska pysyvä tila muuttuu vasta kokonaisen toiminnon lopussa ja etualan kohtauksen aikana tallennus on estetty. Ainoa tallennettava suorituskohta on odottavan komennon kohta (alla).

| Rakenne | Mitä tallennetaan |
| --- | --- |
| etualan kohtaus, dialogi ja `open`-valikko | ei mitään: tallennus on estetty, ja automaattinen tallennus ottaa kohtausta edeltävän tilan (moottoridokumentti, luku [5.4](../moottori/05-aloitus-tallennus-ja-checkpointit.md#54-automaattinen-tallennus)) |
| jonossa oleva etualan kohtaus | nimi ja argumentit |
| taustakohtaus | nimi, argumentit, `scope` ja `bind`, kellon arvo ja silmukoiden vaiheet |
| jaettu pakka ja `deck copy`-kentän kopio | nostopinon järjestys ja poistopakka |
| taulu | ei mitään: taulu on tilaton |
| odottava komento (`wait N`, `wait scene`, `wait confirm` tai `open`) | komennon nimi, rungon tiiviste, argumentit, paikalliset nimet, kohta rakenteellisena polkuna (alla) ja `wait N` -odotuksessa jatkoherätys |
| checkpoint | viimeisin checkpoint tilannekuvana (luku [10.2](10-aloitus-ja-checkpointit.md#102-checkpointit)) |
| keskeneräinen tweeni pelin ajan taustakohtauksessa | jäljellä oleva reitti tai ruudut (luku [9.1](09-kohtaukset-ja-aikajanat.md#91-rivi)) |
| profiili | aloitushetken tilannekuva ja pelikerran omat kirjoitukset; profiili itse tallentuu erikseen (luku [10.4](10-aloitus-ja-checkpointit.md#104-profiili-profile)) |

Taustakohtaus on helppo tallentaa, koska aikajana on dataa eikä suoritettavaa koodia. Jonossa oleva etualan kohtaus alkaa vasta, kun komento on päättynyt, joten käsittelijän käynnistämä kohtaus ei jätä keskeneräistä suorituskohtaa.

**Odottavan komennon kohta.** Komennossa odotus on sallittu vain sen omassa rungossa (luvut [5.1](05-funktiot-ja-lausekkeet.md#51-funktioiden-lajit) ja [7.1](07-komennot-ja-tekoaly.md#71-komennot)), joten odottava komento on aina yksi kehys eikä kutsupinoa tallenneta. Kohta on rakenteellinen polku (moottoridokumentti, luku [6.1.6](../moottori/06-ajo.md#616-monivaiheinen-komento)):

- **Odotuskohdan numero.** Lataus numeroi rungon odotuskohdat ja silmukat lähdejärjestyksessä. Numero kertoo lauseen sekä sitä ympäröivät silmukat ja `if`-haarat, joten haaroja ei tallenneta.
- **Silmukoiden tila** ulkoa sisään: `repeat`-silmukasta kierros, `for`-silmukasta alussa kiinnitetty joukko ja indeksi, `while`-silmukasta ei mitään.
- **Paikalliset nimet** arvoineen, myös silmukkamuuttujat, paikalliset listat ja pakan kopiot.

Esimerkiksi luvun [7.1](07-komennot-ja-tekoaly.md#71-komennot) `DIG`, joka odottaa kolmannella kierroksella, tallentuu kohtana odotus 0 ja `repeat`-silmukan kierros 2.

**Jatkaminen.** Ladattu peli ei aja runkoa alusta vaan jatkaa odotuskohdan jälkeisestä lauseesta, ja silmukat jatkavat tallennetuista tiloistaan.

**Muuttunut runko.** Tiiviste lasketaan rungon jäsennyspuusta, joten kommentit ja välilyönnit eivät vaikuta siihen. Jos pelin päivitys tai modi muuttaa komennon runkoa tai poistaa komennon, kesken oleva odotus hylätään ladattaessa: jo tehdyt muutokset jäävät voimaan, hintaa ei peritä, ja moottori kirjaa ajon varoituksen.

## 11.2 Korvaaminen

Korvaaminen toimii nimen tasolla kuten datassa. `override` korvaa koko määrittelyn: komponentin, funktion, komennon, nimetyn käsittelijän (`as NIMI`), tekoälyn, generaattorin, generoidun kartan tai kohtauksen.

Kirjasto tai modi ei voi korvata yksittäistä riviä funktion keskeltä. Jos se haluaa muuttaa vain esimerkiksi vahingon laskentaa, sen on kopioitava koko funktio. Kopio ei saa myöhempiä korjauksia, joita alkuperäiseen tehdään. Siksi vakiokirjasto ja kirjastot kirjoitetaan pieninä nimettyinä funktioina ja käsittelijöinä, jotta jokaisen kohdan voi korvata erikseen, kuten `STD.APPLY_DAMAGE`, `STD.HIT_MESSAGE`, `STD.SPAWN_PLAYER`, `STD.STRIKE` ja vierekkäisyyden käsittelijät luvuissa [4](04-tapahtumat-ja-kasittelijat.md), [4.2](04-tapahtumat-ja-kasittelijat.md#42-vakiokirjaston-tapahtumat) ja [10.1](10-aloitus-ja-checkpointit.md#101-aloitus).

Datan `patch` (dataformaatit, luku [4.8](../dataformaatit/04-nimiavaruudet.md#48-lohkojen-yhdistäminen-patch-new-ja-drop)) ei koske RogueScriptin määrittelyjä.

---

← [10. Aloitus ja checkpointit](10-aloitus-ja-checkpointit.md) · [Sisällys](README.md) · [12. Kielioppi](12-kielioppi.md) →
