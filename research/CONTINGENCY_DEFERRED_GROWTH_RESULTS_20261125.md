# Aufgeschobene Kantenbildung verlängert die Evidenz, verbessert aber keinen Erwerb

Auf sechs vorab festgelegten Seeds mit zwei Bildabbildungen erreichte
**keine** der vier Bedingungen einen gemeinsamen A→B-Pfad: **0 von 24**
in allen vier Armen. Die Aufschubbedingungen erreichten zudem **0 von 24**
bei Aufgabe A und **0 von 60** bei Aufgabe B, während die unmittelbaren
Bedingungen **1 von 24** und **3 von 60** erreichten. Der Aufschub hat
den Erwerb also nicht verbessert, sondern die wenigen Treffer entfernt.
[Versuchsplan](CONTINGENCY_DEFERRED_GROWTH.md)

Die vier Bedingungen kreuzten die Quellenwahl nach letzter Aktivität
oder nach Vorlauf-Evidenz mit sofortiger oder um 500 ms aufgeschobener
explorativer Bildung. Alle vier starteten auf identischen entwickelten
Checkpoints. Vorwärts, umgekehrt und balanciert trainierte A-Historien
wurden mit allen drei B-Historien gekreuzt. Die eingefrorenen Proben
erfolgten nach elf Sekunden Ruhe. Pruning blieb im gesamten Test
ausgesetzt; native Kantenbildung, Homöostase und Gewichtslernen liefen
weiter.

| Seed | Layout | A aktuell / Vorlauf / aufgeschoben aktuell / aufgeschoben Vorlauf | B aktuell / Vorlauf / aufgeschoben aktuell / aufgeschoben Vorlauf |
| --- | --- | ---: | ---: |
| 34601 | Spaltenfolge | 1 / 1 / 0 / 0 | 0 / 0 / 0 / 0 |
| 34609 | Spaltenfolge | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| 34617 | Spaltenfolge | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| 34627 | Zeilenfolge | 0 / 0 / 0 / 0 | 3 / 2 / 0 / 0 |
| 34639 | Zeilenfolge | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| 34651 | Zeilenfolge | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |

Die Zielpools waren in allen 24 A- und 60 B-Vergleichen auswertbar.
Die drei B-Treffer lagen allein auf Seed 34627, die beiden A-Treffer
allein auf Seed 34601. Der gepaarte Vergleich der Vorlaufregel gegen
Aktualität ergab bei A **1** Treffer in beiden Armen, **0** nur im
Vorlaufarm und **0** nur im Kontrollarm; bei B **2** in beiden Armen,
**1** nur im Kontrollarm und **0** nur im Vorlaufarm. Für den
gemeinsamen Endpunkt gab es in keinem Arm einen Treffer. Für den
bedingten A-Erhalt waren nach A nur drei Fälle auswertbar; keiner
bestand ihn.

## Technische Wirkung: der Eingriff wirkt, der Nutzen fehlt

Der Aufschub wirkt genau wie vorgesehen. In den aufgeschobenen Armen
entstand **keine** explorative Kante vor der Freigabe (`before_open` je
Arm **0** gegenüber **2027** in den unmittelbaren Armen), und es wurden
mehr Kanten gebildet: **8.062** und **8.036** gegenüber **7.877** und
**7.838**; explorativ **2.726** und **2.728** gegenüber **2.708** und
**2.704**. Die Vorlaufregel erhielt in der verlängerten Phase mehr
Evidenz: sie nutzte **636** statt **364** positiver Evidenz und wählte
in **218** statt **116** Entscheidungen eine andere Quelle als die
Aktualitätsregel.

Der Lesbarkeitsengpass liegt jedoch vor der Quellenwahl. Der Zielpool
antwortete in **4 von 24** auswertbaren A-Fällen und **9 von 60**
B-Fällen — in allen vier Bedingungen gleich. Der Aufschub änderte diese
Rate nicht. Er entfernte aber die wenigen Fälle, in denen der Zielpool
die balancierte Exposition überstieg (**1 → 0** bei A und **3 → 0** bei
B), obwohl der Zielpool weiterhin genau so oft antwortete. Die
gebildeten Graphen unterschieden sich zwischen den Regeln deutlich
(Seed 34601: 1.095 gemeinsame, 60 nur im Vorlaufarm und 70 nur im
Kontrollarm gebildete Quell–Ziel-Paare; in **0 von 12** verglichenen
Phasen waren die Endgraphen identisch) — ohne Unterschied im Abruf.

Der Befund ist damit doppelt negativ: Die verlängerte Evidenzphase
verbessert die Quellenwahl technisch messbar, aber sie ändert nichts
an derSeltenheit der Antwort des Zielpools. Der Aufschub ist ein
diagnostischer Eingriff mit fester Wartezeit; er belegt weder, dass die
Quellenwahl richtig wäre, noch dass sie falsch ist.

## Grenzen

Sechs Seeds, 24 A- und 60 B-Vergleiche; die Trefferzahlen sind
einstellig und damit keine statistische Aussage. Die Bedingungen sind
innerhalb eines Seeds voneinander abhängig (B-Zweige derselben
A-Historie sind keine unabhängigen Wiederholungen). Der Aufschub
verändert die Zeitachse der gesamten Phase und ist damit mehr als ein
reiner Quellenwahl-Eingriff. Die Auswertung beschreibt die Bedingungen
dieses Laufs; sie sagt nichts über spätere Versuche mit anderen Seeds
oder anderen Bildlayouts.

## Nächster Gegenversuch

Die Zielpools antworten in 20 von 24 A- und 51 von 60 B-Fällen gar
nicht. Bevor die Quellenwahl weiter verändert wird, ist dieser Engpass
zu lokalisieren: ein kausaler Test setzt auf einem Seed die
Cue-zu-Ziel-Pfade des Ausgangszustands auf eine künstliche
Hochfrequentz und prüft, ob der Zielpool dann antwortet. Antwortet er
nicht, liegt der Mangel in der Reizweitergabe und nicht in der
Bildungspolitik; antwortet er, ist die Kantenbildung der Engpass. Der
Gegenversuch braucht neue Seeds.

## Reproduktion

Der Lauf wurde am 25.11.2026 nach einem Abbruch fortgesetzt: der
versiegelte Treiber `run.sh` und alle versiegelten Artefakte blieben
unverändert, `resume.sh` führte ausschließlich die fehlenden Seeds mit
derselben archivierten Binärdatei, derselben Shard-Spezifikation und
derselben Record-Form aus. `preregistration-check.log` weist für alle
versiegelten Artefakte `OK` aus. Der native Harness wiederholte jeden
Trial und prüfte die Gleichheit der vollständigen Messdaten. Der
unabhängige Python-Prüfer rekonstruierte Gewichtslebenszyklen,
Eingangsereignisse, Vorlagen, Akquisitions- und Erhaltskriterien.
Library `c13d9f7` und Modell `6feed99` waren vor dem Hauptlauf
veröffentlicht.

```bash
root=research/results/visual_contingency_deferred_growth_20261125
(cd "$root" && sha256sum -c preregistration.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_deferred_growth_analysis.py" "$root" | cmp - "$root/analysis.json"
```
