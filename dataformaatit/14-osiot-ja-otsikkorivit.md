[RLML](../README.md) › [RLML-dataformaatit](README.md)

# 14. Osiot ja otsikkorivit

| Otsikko | Muoto | Luku |
| --- | --- | --- |
| `game` | `game NIMI`, rivit `- start KARTTA \| NOSTO`, `- at HAKU`, `- seed N`, `- call_depth N`, `- loop_steps N`, `- keys KONTEKSTI`, `- title VALIKKO`, `- layout ASETTELU`, `- naming KYSELY`, `- walk KOMENTO`, `- walk_stop KYSELY`, `- leader KYSELY`, `- rest KOMENTO`, `- follow_distance N`, `- follow_ai TEKOÄLY`, `- masking N`, `- seats a..b`, `- party switch \| each`, `- leash on \| off` ja `- KOMPONENTTI kentät`; lyhyt muoto `game NIMI start … at …` | 3.3, 5.5 |
| `profile` | `profile NIMI`, rivit `- KOMPONENTTI kentät` | 3.5 |
| `include` | `include POLKU.PÄÄTE` (jokerit `*`, `**`) | 3 |
| `namespace` | `namespace NIMI` tiedoston alussa | 4.2 |
| `override` | määrittelyn tai rivin edessä | 4.5 |
| `patch`, `new`, `drop` | `patch` lohkon otsikon edessä, `new` ja `drop` rivin alussa, `new` myös uuden lohkon edessä `overrides/`-tiedostossa | 4.8 |
| `settings` | `- asetus arvo`, esim. `- mouse on` | 3.2 |
| `autosave` | `- slots N` ja `- on KOHTA [every N]` | 3.4 |
| `constants` | `- NIMI arvo` | 2.4 |
| `component` | `- kenttä tyyppi [default arvo]` (myös RogueScriptissä) | 5.1 |
| `entity` | `entity NIMI [extends EMO]`, rivit `- KOMPONENTTI kentät` ja `- drop KOMPONENTTI` | 5.2 |
| `deck` | `deck NIMI [shuffled]`, kortit `- KOHDE ...` | 6.1 |
| `table` | `table NIMI NOPPA`, rivit `- TULOS KOHDE ...` | 6.5 |
| nosto | `PAKKA [N \| all] [copy]` tai `TAULU [N]` | 6.2, 6.5 |
| `terrain` | `- KERROS [default] [pass] [fill TILE]` | 7.2 |
| `tileset` | `tileset NIMI`, rivit tilejä, välejä ja joukkoja | 7.3 |
| `tiles` | `- TILE [OMINAISUUS arvot ...]`, esim. `- DOOR blocks move sight`, `- WATER only RAFT SHIP`, `- FLOOR lit`, `- FLOOR.dark lit false forget` | 7.5 |
| `map` | `map NIMI [extends POHJA] [size LxK] [transient] [wrap] [nosave]`, piirros, `legend:`, `vars:`, `at:`, `areas:` (generoitu kartta RogueScriptissä) | 8.1, 8.10, 8.11 |
| `atlas` | `atlas NIMI sheet LxK grid SxR [fill TILE]`, `legend:`, `vars:`, `areas:`; lehdet ovat karttoja `NIMI.<sarake><rivi>` (`MAAILMA.B10`) | 8.12 |
| `at:` | `- X,Y [TILE] [KERROS TILE ...] [with …] [as …] [wire …]` kartan tai prefabin alilohkona | 8.9 |
| `areas:` | `- X,Y size LxK as ALUE` kartan alilohkona; saman nimen rivit muodostavat yhdisteen | 8.11 |
| `prefab` | `prefab NIMI`, piirros, `legend:`, `at:` (sijoitus generaattorissa) | 8.7 |
| paikkahaku | `at KOHDE [near [ANKKURI] [ETÄISYYS]]`, `at SIIRTYMÄ from ANKKURI` | 8.8 |
| `colors` | `- NIMI hex` | 9.1 |
| `tiles.glyphs` | `tiles.glyphs ascii` tai `tiles.glyphs spritesheet "polku" LxK`; sarjarivi `- TILE.a..b series "MERKIT" fg VÄRIT` tai `- TILE.a..b series ALKU..LOPPU` | 9.2 |
| `tiles.glyphs paper` | `- TILE [LAJI] [määreet] [decor]` | 9.6 |
| `maps.theme paper` | `- KARTTA sea \| cave \| ship`, `- default TAPA` | 9.6 |
| `areas.labels paper` | `- level N [in KARTTA] [määreet]` tai `- ALUE [in KARTTA] [määreet]`; määreet `size N`, `spacing wide \| normal`, `place land \| water \| center`, `at X,Y`, `hide`, `outline dotted` | 9.6 |
| `panels`, `logs`, `presentation` | tyylin rivit | 9.3 |
| `layers` | `- NIMI LÄHDE [where ehto] [remember] [smooth AIKA]` | 9.5 |
| `language` | `language KIELI` tiedoston alussa, rivit `AVAIN teksti` ilman `-`-merkkejä | 10 |
| `layout` | piirros ja `legend:`; kartta-alueen rivi `- m map [follow center \| follow margin N] [turn TYYLI] [cursor TYYLI]`, hover- ja lauserivin alueet `- h hover` ja `- e sentence`, telakoitu valikko `- v menu NIMI`, paneelin rivi `- s PANEELI[(argumentit)]` | 11.1 |
| `menu` | `menu NIMI [(parametrit)] [list] [size LxK] [context K] [final] [verbs]`, kohdat `- AVAIN [TOIMINTO] [hide when ehto] [disabled when ehto]` tai `- AVAIN adjust KENTTÄ a..b [pool NIMI]`; listavalikolla rivit `source`, `row`, `sort`, `hide when`, `disabled when`, `then`, `keep` | 11.2 |
| `keys`, `pad` | `keys KONTEKSTI [modal \| passthrough] [keyboard N]`, `pad KONTEKSTI [deadzone P%]`; rivit `- TOIMINTO näppäimet` (keys-riveillä myös hiiren nimet `click`, `rclick`, `mclick`, `wheel.up`, `wheel.down`) | 11.3 |
| `messages`, `log` | `- AVAIN [PARAMETRIT]`, `log ALUE [history N] [merge] [more]` | 11.4 |
| `panel` | `panel NIMI [(parametrit)]`, piirros ja `legend:` riveillä `- M text AVAIN`, `- M value LAUSEKE` ja `- M param NIMI`, kukin valinnaisella tasauksella `left`, `right` tai `center` | 11.5 |
| `workdir`, `audioscript`, `audio`, `preload`, `render` | äänten ilmoitukset | 12.1 |
| `bus` | `bus N volume P%` | 12.2 |

---

← [13. Avoimet kysymykset](13-avoimet-kysymykset.md) · [Sisällys](README.md)
