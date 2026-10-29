[RLML](../README.md) › [RogueScript](README.md)

# 1. Rajaus ja periaatteet

RogueScript on RLML 0.3:n upotettu pelilogiikan kieli. Se on tavallinen ohjelmointikieli: sisennys merkitsee lohkoja, parametrit ovat suluissa, ja kielessä on `if`, `for` ja `return`.

Dataformaatit kuvataan erillisdokumentissa “[RLML-dataformaatit](../dataformaatit/README.md)”. RogueScriptillä on oma syntaksinsa, eivätkä dataformaattien rivi-, lohko- ja kommenttisäännöt koske sitä. Arvojen kirjoitusasu (noppa, prosentti, aika, koko, suunta), nimet, nimiavaruudet ja korvaaminen ovat yhteiset.

Kielen periaatteet:

- **Logiikka asuu komponenteissa.** Komponentti on luokka, jolla on kentät ja käsittelijät. Entiteetin käytöksen määräävät sen komponentit.
- **Moottori ei tunne pelin käsitteitä.** Kuolema, vahinko, inventaario, törmäys ja pelaajan luonti ovat vakiokirjaston sääntöjä nimiavaruudessa `STD`, ja peli voi korvata ne.
- **Tekoäly käyttää pelaajan komentoja.** Tekoäly on funktio, joka palauttaa komennon.
- **Pitkät toiminnot ovat korutiineja.** Komento ja kohtaus (myös dialogi) voivat odottaa kesken suorituksen. Käsittelijä ja funktio eivät odota, vaan ne käynnistävät kohtauksen (luku [9](09-kohtaukset-ja-aikajanat.md)).
- **Ajoitettu esitys on aikajana.** Kohtaus kirjoitetaan riveinä, joilla on aikaleima ja toiminto.
- **Toistettavuus.** Kaikki satunnaisuus tulee pelin siemenestä, ja suoritusjärjestys on määrätty. Esityksellä on oma satunnaisvirtansa, joten esitys ei vaikuta pelin kulkuun (luku [5.4](05-funktiot-ja-lausekkeet.md#54-satunnaisuus)).

---

[Sisällys](README.md) · [2. Yleinen syntaksi](02-yleinen-syntaksi.md) →
