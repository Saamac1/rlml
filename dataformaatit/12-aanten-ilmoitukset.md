[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 12. Äänten ilmoitukset

Äänen DSL (proseduraaliset efektit ja musiikki) on oma erillinen kielensä, eikä se noudata tämän dokumentin yhteisiä sääntöjä. RLML:n puolella äänet ilmoitetaan nimillä, ja soittaminen kuuluu pelilogiikkaan. Jos yhtään ilmoitusta ei ole, pelissä ei ole ääntä.

## 12.1 Ilmoitukset

```
workdir "/audio"
audioscript "fireball.sfx" as SFX.FIREBALL
audio "explosion.wav" as SFX.EXPLOSION
preload audioscript "bach.song" as MUSIC.TOWN
preload audio "hit.sfx.wav" as SFX.HIT
render audioscript "door.sfx"
render audioscript "step.sfx" as SFX.STEP
```

- `audioscript` ilmoittaa skriptatun äänen (äänen DSL:n tiedosto, esimerkiksi `.sfx` tai `.song`) ja `audio` valmiin äänen (WAV). Laji tulee sanasta eikä tiedostopäätteestä.
- `as NIMI` antaa äänelle nimen, jolla siihen viitataan. Nimi kuuluu yhteiseen nimiavaruuteen (luku [4.3](04-nimiavaruudet.md#43-nimiavaruuteen-kuuluvat-nimet)), ja ryhmä (`SFX.*`) kattaa koko ryhmän.
- `preload` ilmoituksen edessä lataa äänen etukäteen. Muuten ääni ladataan ensimmäisellä soitolla.
- `render audioscript "polku"` renderöi skriptin WAV-tiedostoksi. Tulos nimetään skriptin mukaan (`door.sfx.wav`), ja `to "muu.wav"` antaa toisen nimen. Rivin lopussa oleva `as NIMI` ilmoittaa tuloksen samalla `audio`-äänenä.
- **Renderöinnin paikka:** kehityksessä, kun projektikansio on kirjoitettava, WAV kirjoitetaan skriptin viereen. Jakelussa valmis WAV on mukana. Jos se puuttuu tai on vanhempi kuin skripti, moottori renderöi sen käyttäjädatan välimuistiin.
- `workdir "/audio"` asettaa työkansion, johon sen jälkeen kirjoitetut tiedostopolut liitetään. Loppukauttaviiva on valinnainen, ja alkukauttaviiva kiinnittää polun pelin juureen. Muuten polku on suhteessa tiedostoon, jossa ilmoitus on. `workdir` on tiedostokohtainen ja nollautuu jokaisen tiedoston alussa.

## 12.2 Väylät

Mikserin väylien alkuvoimakkuudet ilmoitetaan erikseen, ja ne asetetaan peliä ladattaessa:

```
bus 1 volume 100%
bus 2 volume 80%
```

- Väylä on numero. Väylä 0 on oletusväylä.
- **Nimetty väylä:** väylän numerolle voi antaa nimen vakiolla (`- BUS_SFX 1`, luku [2.4](02-yleinen-syntaksi.md#24-arvot)).

## 12.3 Pelimaailman äänet

Pelimaailman ääni on pelin tilaa, jonka olennot voivat kuulla, ja ääniefekti on sen esitys. Äänilajit ilmoitetaan nimettömässä osiossa `sounds`:

```
sounds
- SOUND.STEP level 3 effect SFX.STEP
- SOUND.SHOUT level 8 effect SFX.SHOUT
- SOUND.WATERFALL level 6 continuous effect SFX.WATER
- SOUND.MUSIC_BOX level 5 continuous effect SONG.BOX
```

- **Rivi** on `- NIMI level N [continuous] [effect ÄÄNI]`. Nimi on yleinen nimi, ja ryhmä `SOUND` on suositus.
- **`level`** on voimakkuus lähteessä. Ääni heikkenee yhdellä tasolla ruutua kohden.
- **`continuous`** tekee lajista jatkuvan äänen, joka soi kestonsa tai lähteensä elinajan. Ilman sitä laji on kertaääni.
- **`effect`** viittaa ilmoitettuun ääneen (luku [12](#)), jonka moottori soittaa, kun pelaaja kuulee äänen. Moninpelissä ääni soi kerran, kun yksikin jäsen kuulee sen.
- **Kuvaus** on tekstiavain `<LAJI>.DESC` (`SOUND.STEP.DESC Kuulet askelia.`).
- **Lähde.** Moottorin komponentti `sound LAJI` tekee entiteetistä jatkuvan äänen lähteen (`- sound SOUND.MUSIC_BOX`). Kertaäänet ja kestolliset äänet syntyvät logiikasta funktiolla `noise` (moottoridokumentti, luku [6.8](../moottori/06-ajo.md#68-äänet-ja-kuuleminen)).
- **Kuulija.** Moottorin komponentti `hearing [acuity N]` antaa entiteetille kuulon, ja `acuity` lisätään kuultuun tasoon (oletus 0).
- **Esto.** Arvo `sound` tilen tai entiteetin `blocks`-riveillä pysäyttää äänen (luku [7.5](07-tilet-maastokerrokset-tilejoukot-ja-ominaisuudet.md#75-tilen-ominaisuudet-tiles)): `- WALL blocks move sight sound`.
- **Peitto.** `game`-lohkon kenttä `masking N` (oletus 3) on raja, jota hiljempana kuuluva ääni peittyy herätysvälin kovimman äänen alle. Se lukittuu pelikerralle kuten `call_depth`.

---

← [11. Käyttöliittymän data](11-kayttoliittyman-data.md) · [Sisällys](README.md) · [13. Avoimet kysymykset](13-avoimet-kysymykset.md) →
