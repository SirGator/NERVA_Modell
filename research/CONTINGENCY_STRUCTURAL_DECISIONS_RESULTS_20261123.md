# Der Mindestscore blockiert den ersten neuen Eingang bei stillen Zellen

Bei **3.840 von 3.840 Entscheidungen** an den stillen
Vergleichszellen lagen der Wachstumsantrieb über seiner Schwelle und
freie lokale Nachbarn vor. Sämtliche Kandidaten verfehlten den
Mindestscore. In 867 dieser Entscheidungen hatte mindestens ein
freier erregender Nachbar in den letzten 100 ms gefeuert.
[Versuchsplan](CONTINGENCY_STRUCTURAL_DECISIONS.md)

## Neubildung

Der Replay erfasst die tatsächliche Laufzeituhr vor dem
Topologieeingriff. Er reproduzierte auf vier bekannten Seeds beide
nativen Trainingstrajektorien mit aktiver Strukturentwicklung.
Verglichen wurden je Zuordnung zwölf im festen Arm stille und
zwölf dort aktive Core-Zellen. Hinzu kamen die drei stillen
Relais aus der terminalen Kapazitätsdiagnose auf Seed 34123.

| Entscheidungen | Stille Vergleichszellen | Aktive Vergleichszellen | Stille Relais |
| --- | ---: | ---: | ---: |
| Wachstumsantrieb höchstens 0,5 | 0 | 2.770 | 10 |
| Antrieb über 0,5, kein freier lokaler Nachbar | 0 | 0 | 0 |
| Antrieb über 0,5, alle Kandidaten unter dem Mindestscore | 3.840 | 703 | 229 |
| Neubildung geplant | 0 | 367 | 1 |
| Unter dem Mindestscore trotz eines freien erregenden Nachbarn mit Spike in den letzten 100 ms | 867 | 163 | 134 |

Bei jedem aufgezeichneten Zeitpunkt fehlte den stillen
Vergleichszellen ein eigener früherer Spike. Das Produkt der
Spike-Traces und der Term für die zeitliche Nähe zweier Spikes
waren dadurch null. Mit den abgezogenen Distanzkosten bleibt ein
solcher Kandidat zwangsläufig unter dem Mindestscore von 0,01.
Das erklärt die fehlende Auswahl auch bei erhöhtem Antrieb.

Die drei beobachteten Relais hatten zuletzt vor Beginn des
Trainings gefeuert. Beim einzigen erfolgreichen Kandidaten für
Relais 93 lag dessen letzter Spike rund 248 ms zurück. Sein
Kandidatenscore betrug 0,01021. Die entstandene Kante von Zelle 92
lieferte später einen einzelnen unterschwelligen Impuls.
[Vorheriger Trainings- und Abrufbefund](CONTINGENCY_STRUCTURAL_RESULTS_20261123.md)

## Entfernung

Alle acht terminalen Kante–Zuordnung-Paare mit stiller Quelle wurden
entfernt. Die Laufzeituhr zeigte bei der Entfernung einen Abstand
von 307 bis 399 ms seit Beginn des jeweils aktuellen Zeitgebers
unter der Nutzenschwelle. Der Controller verlangt mindestens
300 ms und prüft beim nächsten Wartungsaufruf erneut. Von den
15 weiteren terminalen Paaren wurden drei entfernt. Die gepaarten
Beobachtungen belegen den tatsächlich ausgeführten Entfernungsweg.
Wie sich der Erhalt der Ausgänge auf die Rekrutierung auswirkt,
erfordert einen Eingriff.

## Nächster Gegenversuch

Die Kandidatenauswahl lässt sich gezielt öffnen, wenn der lokale
Wachstumsantrieb hoch ist, die bisherige Regel keinen Kandidaten
liefert und ein freier erregender Nachbar kürzlich gefeuert hat.
Der Gegenversuch sollte diese Bildung mit der bestehenden Regel
und mit einer gleich umfangreichen lokalen Kontrollbildung
vergleichen. Ein unabhängiger Faktor für das Entfernen von
Synapsen kann zeigen, ob neu aktivierte Zellen ihre terminalen
Ausgänge rechtzeitig nutzen. Ankünfte, Relais-Spikes, neue
Paarereignisse und selektiver Abruf sind dabei eigene Endpunkte.
Die vorliegenden Daten stammen aus wiederholten Entscheidungen
an denselben Zellen auf bekannten Seeds. Sie belegen weder
einen Rekrutierungsgewinn durch den Eingriff noch Gedächtnis.

## Reproduktion

Die vier Records wurden jeweils vollständig wiederholt. Der
unabhängige Prüfer rekonstruiert Nachbarschaften, Scores und
Nutzungszeitgeber und prüft sie gegen die archivierten
Topologieereignisse. Ein unabhängiger Build der archivierten
Quellen reproduzierte den Kurzlauf bytegenau. Die Projektprüfung
umfasste 465 Rust-, 103 Harness-Python- und 10 Dashboard-Tests.
Der neue Rust-Test zum unveränderten Ablauf mit aktivem Mitschnitt
und der Satz aus 23 Kontingenztests bestanden im gezielten Lauf.
Modell `241f143` und Library `9a3a5fb` waren vor dem Hauptlauf
veröffentlicht. [Kompakte Daten](dokumentation/visual_contingency_structural_decisions_20261123.json)

Das geprüfte lokale ZIP liegt unter
`research/results/archives/visual_contingency_structural_decisions_20261123.zip`.
Es enthält Eingaben, genaue Entscheidungen, Quellen, Binary und
Prüfbelege. Die nachträgliche deskriptive Aggregation ist als
`result_index.py` enthalten.

```bash
root=research/results/visual_contingency_structural_decisions_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_structural_decisions_analysis.py" "$root"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/result_index.py" "$root"
sha256sum -c research/results/archives/visual_contingency_structural_decisions_20261123.zip.sha256
```
