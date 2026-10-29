[RLML](../README.md) › [RogueScript](README.md)

# 12. Kielioppi

Kielioppi on EBNF-muodossa. `INDENT` ja `DEDENT` syntyvät neljän välilyönnin sisennyksestä. `NAME` on iso nimen osa, `name` pieni nimi ja `QNAME` pisteillä ryhmitelty iso nimi. `INT`, `TIME` (`500ms`, `1s`), `DICE`, `PERCENT`, `SIZE` (`20x10`) ja `STRING` ovat literaaleja. Sääntö `file` kuvaa `.rogue`-tiedoston ja `block_body` datatiedoston koodilohkon sisällön.

## 12.1 Määrittelyt

```
file        = [ "namespace" QNAME NL ] block_body ;   (* .rogue-tiedosto *)
block_body  = { [ "override" ] decl } ;               (* koodilohko *)
decl        = component | event | handler | function | command
            | ai | generator | mapdecl | scene ;

params      = param { "," param } ;
param       = name ":" type [ "=" expr ] [ source ] ;
source      = "from" expr | "aim" expr [ "at" expr ] | "ask" [ expr ] | "count" ;
type        = "number" | "percent" | "dice" | "dir" | "side" | "name"
            | "deck" [ "copy" ] | "table" | "entity" | "card"
            | "collection" [ "card" | "name" ]
            | "list" type
            | "pos" | "bool" | "area" | "command" | "scene"
            | "component" | "handle" | "camera" | "light" | "range" | "label" | "sound" | "text"
            | "dict" ( "entity" | "name" | "number" ) type
            | QNAME ;                                   (* komponenttiarvo listan jäsenenä *)
fields      = field { "," field } ;
field       = name ":" fieldtype [ "=" expr ] ;
fieldtype   = type
            | "collection" [ INT ] [ "merge" QNAME ] [ "modifies" ]    (* kuten datassa *)
            | "list" type [ INT ] [ "sort" [ name ] [ "desc" ] ] ;
names       = name { "," name } ;

component   = "component" QNAME [ "(" [ fields ] ")" ]
              ( NL | ":" NL INDENT handler { handler } DEDENT ) ;
event       = "event" QNAME "(" [ params ] ")" NL ;
handler     = phase evname "(" [ names ] ")" [ "as" QNAME ]
              [ "priority" [ "-" ] INT ] [ "when" expr ] ":" block ;
phase       = "before" | "on" | "after" ;
evname      = QNAME | name ;              (* pieni nimi = moottorin tapahtuma *)
function    = [ "present" ] "fn" QNAME "(" [ params ] ")" [ "->" rtype ] ":" block
            | "query" QNAME "(" [ params ] ")" "->" rtype ":" block ;
rtype       = type | "name" "|" "text" ;  (* name|text vain naming-kyselylle *)
command     = "command" QNAME "(" [ params ] ")" [ "cost" INT ] ":" block
            | "command" QNAME "=" call NL ;
ai          = "ai" QNAME ":" block ;      (* self kontekstista, paluuarvo command *)
generator   = "generator" QNAME "(" [ params ] ")" [ "size" SIZE ] ":" block ;
mapdecl     = "map" QNAME [ "transient" ] [ "wrap" ] [ "nosave" ] "generate" call NL ;
```

- `deck copy` on sallittu vain komponentin kentän tyyppinä. Funktion parametrina ja paluuarvona tyyppi on `deck`. Moottorin tapahtuman käsittelijässä `phase` on `on` tai `after` (luku [4.1](04-tapahtumat-ja-kasittelijat.md#41-moottorin-tapahtumat)).
- `fn` ilman paluutyyppiä ei palauta arvoa (luku [5.1](05-funktiot-ja-lausekkeet.md#51-funktioiden-lajit)). `collection` ilman jäsenlajia on entiteettien kokoelma (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)).
- Komponentin kenttä (`field`) hyväksyy kokoelman koon ja säännöt kuten datassa (`items: collection 26 merge STACKABLE`) sekä listan koon ja järjestyksen (`scores: list SCORE 10 sort score desc`). Parametrina ja paluuarvona `collection` ja `list` kirjoitetaan ilman niitä. `ask` ilman lauseketta on suunnan tai viereisen ruudun kysely, lukuvälin kanssa lukukenttä ja luvun kanssa tekstikenttä (luku [7.1](07-komennot-ja-tekoaly.md#71-komennot)). `count` on sallittu vain `number`-parametrille, ja komennolla on enintään yksi.
- Käsittelijän `names` saa olla tapahtuman parametreja lyhyempi, kun pois jätetyillä loppuparametreilla on oletusarvo (luku [4](04-tapahtumat-ja-kasittelijat.md)). Komponentin käsittelijän `names` ei sisällä subjektia, vaan alkaa tapahtuman toisesta parametrista (luku [3.1](03-komponentit.md#31-määrittely)).
- Tyyppinä `QNAME` on sallittu vain listan jäsenenä, ja sen on oltava komponentti.

## 12.2 Lauseet

```
block   = NL INDENT stmt { stmt } DEDENT ;
stmt    = postfix assign expr NL
        | name "=" "wait" ( "number" | "text" ) "(" expr ")" NL
        | name "=" "open" call NL
        | "if" expr ":" block { "elif" expr ":" block } [ "else" ":" block ]
        | "for" name "in" expr ":" block
        | "repeat" expr ":" block
        | "while" expr ":" block
        | "break" NL
        | "match" expr ":" NL INDENT case { case } DEDENT
        | "choice" ":" NL INDENT option { option } DEDENT
        | "emit" call NL
        | "wait" waitarg NL
        | "open" call NL
        | "return" [ expr ] NL
        | "cancel" NL
        | call NL ;
assign  = "=" | "+=" | "-=" ;            (* += lisää myös listaan *)
waitarg = expr | "confirm" | "scene" call | ( "number" | "text" ) "(" expr ")" ;
case    = ( expr | "else" ) ":" block ;
option  = NAME [ "(" args ")" ] [ "hide" "when" expr ] [ "disabled" "when" expr ]
          ( ":" block | "adjust" postfix range [ "pool" postfix ] NL ) ;
```

- `hide` ja `disabled` ovat varattuja sanoja, joten ensimmäinen lauseke päättyy `disabled`-sanaan.
- `wait`- ja `open`-lause on sallittu vain komennon ja korutiinikohtauksen rungossa. Käsittelijässä ja funktiossa se on latausvirhe, jonka lataus tarkistaa kieliopin jälkeen (luvut [4](04-tapahtumat-ja-kasittelijat.md), [5.1](05-funktiot-ja-lausekkeet.md#51-funktioiden-lajit) ja [11.1](11-tallennus-ja-korvaaminen.md#111-tallennus)).
- `wait number`, `wait text` ja `open` sijoituksena palauttavat arvon, ja ne ovat odotuskohtia kuten muutkin odotukset (luvut [9.7](09-kohtaukset-ja-aikajanat.md#97-dialogit) ja [11.1](11-tallennus-ja-korvaaminen.md#111-tallennus)). Säädettävällä kohdalla (`adjust`) ei ole runkoa.

## 12.3 Lausekkeet

```
expr     = or_expr { "where" or_expr } ;
or_expr  = and_expr { "or" and_expr } ;
and_expr = not_expr { "and" not_expr } ;
not_expr = [ "not" ] test ;
test     = range [ cmp range | "has" [ "no" ] ( QNAME | name )
                 | "is" ( QNAME | postfix ) | "in" range ] ;
cmp      = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
range    = sum [ ".." [ sum ] ] ;
sum      = term { ( "+" | "-" ) term } ;
term     = unary { ( "*" | "/" | "%" ) unary } ;
unary    = [ "-" ] postfix ;
postfix  = atom { "." ( name | NAME | INT ) | "(" [ args ] ")" | "[" expr "]" } ;
atom     = literal | name | NAME | "." NAME | wildcard | list | "{" "}" | "(" expr ")" ;
wildcard = [ "." ] QNAME ( ".*" | ".**" ) ;   (* ilman välilyöntiä *)
list     = "[" [ expr { "," expr } ] "]" ;
call     = postfix ;                         (* postfix, joka päättyy sulkuihin *)
args     = arg { "," arg } ;
arg      = [ ( name | NAME ) ":" ] expr ;
literal  = INT | TIME | DICE | PERCENT | SIZE | STRING
         | "true" | "false" | "none" | "all" ;
```

- Atomi `"." NAME` on juuriviittaus (`.LOOT`). Postfix `"." INT` on tilen indeksi (`GLYPH.NKI.1`), ja `range` tekee siitä indeksivälin (`GLYPH.NKI.1..20`).
- Ketjukutsu (luku [2.7](02-yleinen-syntaksi.md#27-ketjukutsu)) on postfix, jossa pientä nimeä seuraavat sulut (`deck.draw(3)`), joten kielioppi ei muutu. Merkitys ratkaistaan latauksessa: pieni nimi ennen sulkuja on moottorin funktio, ja sitä edeltävä postfix on sen ensimmäinen argumentti. Koska `postfix` on `range`- ja `where`-sääntöjen sisällä, väli ja suodatus vastaanottajana tarvitsevat sulut.
- Ison nimen kutsu (luku [2.8](02-yleinen-syntaksi.md#28-ison-nimen-kutsu)) on postfix, jossa isoa nimeä seuraavat sulut. Entiteetin nimen kutsun argumentti `NAME ":"` nimeää kirjoittajan komponentin ja `name ":"` moottorin komponentin, ja paljas ensimmäinen argumentti on määrä.
- `where` on vasemmalle liittyvä, joten `a where x where y` on `(a where x) where y`. Jokainen `where` sitoo oman `it`-nimensä, ja sisempi peittää ulomman (luku [5.3](05-funktiot-ja-lausekkeet.md#53-lausekkeet)).
- Unaarinen miinus on operandin etumerkki: `x = -1`, `from(self, 2, -1)`. Binäärinen `-` tulee aina operandin jälkeen, joten `a - b` on erotus. `-1..3` on väli, ja `-1d3` vaihtaa heiton etumerkin. Suunnalle unaarinen miinus antaa vastasuunnan (`-d`, luku [5.3](05-funktiot-ja-lausekkeet.md#53-lausekkeet)).
- `%` on jakojäännös, ja `+` kahdelle `text`-arvolle yhdistää ne (luku [5.3](05-funktiot-ja-lausekkeet.md#53-lausekkeet)).
- `list` on listaliteraali (luku [2.6](02-yleinen-syntaksi.md#26-tyypit)).
- Jokeri `.*` tai `.**` kirjoitetaan ison nimen perään ilman välilyöntiä. Se on yksi merkki eikä kertolasku, ja ketju jatkuu sen jälkeen: `ITEM.*.pick(18)`.
- `has` hyväksyy ison tai pienen nimen. Pieni nimi on moottorin komponentti: `has ai`, `has no movement`.
- Kontekstisanat `map`, `ai` ja `from` ovat lausekkeessa nimiä (luku [2.4](02-yleinen-syntaksi.md#24-varatut-sanat-ja-paikalliset-nimet)): `map.DEPTH`, `self.ai.behavior`, `from(label(START), 2, -1)`.

## 12.4 Aikajana

```
scene  = "scene" QNAME "(" [ params ] ")"
         ( ":" block
         | "timeline" [ "background" [ "realtime" ] ] ":" tbody ) ;
tbody  = NL INDENT { track | row } DEDENT ;
track  = "track" NAME [ "loop" span ] ":" NL INDENT row { row } DEDENT ;
row    = [ time [ span ] ] action NL
       | [ time ] "when" expr ":" NL INDENT row { row } DEDENT
         [ "else" ":" NL INDENT row { row } DEDENT ]
       | time "mark" NAME NL ;
action = [ "always" ] [ "when" expr ":" ] ( call | "wait" waitarg ) ;
time   = span | "+" span | NAME [ ( "+" | "-" ) span ] ;
span   = TIME                                  (* etuala ja realtime *)
       | INT ;                                 (* background, kierroksia *)
```

Lataus tarkistaa, että `span`-arvon muoto vastaa kohtauksen kelloa.

---

← [11. Tallennus ja korvaaminen](11-tallennus-ja-korvaaminen.md) · [Sisällys](README.md) · [13. Avoimet kysymykset](13-avoimet-kysymykset.md) →
