[RLML](../README.md) › [RLML-vakiokirjasto (STD)](README.md)

# 2. Prioriteettivyöhykkeet

STD:n käsittelijät käyttävät kolmea sovittua tasoa (RogueScript, luku [4](../roguescript/04-tapahtumat-ja-kasittelijat.md)). Peli asettaa omat sääntönsä niiden suhteen: suoja tasolla 10 ehtii muuttaa vahinkoa ennen kuin STD soveltaa sen, ja viesti tasolla -10 näkee lopputuloksen.

| Prioriteetti | Tehtävä | STD:n käsittelijät |
| --- | --- | --- |
| 10 | muokkaa parametreja | – (pelin suojat ja heikkoudet) |
| 0 | soveltaa | `SPAWN_PLAYER`, `JOIN_PLAYER`, `PART_PLAYER`, `GROUP_ARRIVAL`, `APPLY_DAMAGE`, vierekkäisyys |
| -10 | reagoi | `DEATH`, viestit |

Samalla tasolla STD:n käsittelijä ajetaan ennen pelin omaa, kun STD:n include on ennen pelin määrittelyjä (luku [1.3](01-rajaus-periaatteet-ja-lataus.md#13-lataus-ja-moduulit)).

---

← [1. Rajaus, periaatteet ja lataus](01-rajaus-periaatteet-ja-lataus.md) · [Sisällys](README.md) · [3. Komponentit ja oletuspelaaja](03-komponentit-ja-oletuspelaaja.md) →
