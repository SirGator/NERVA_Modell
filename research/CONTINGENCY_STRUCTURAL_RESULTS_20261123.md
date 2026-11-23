# Strukturplastizität erreicht die stillen Relais kaum

Die lokale Strukturentwicklung bildete während des visuellen Trainings
**1.108 Synapsen**. Nur eine davon führte zu einem der drei zuvor stillen
Relais und lieferte einen einzigen unterschwelligen Impuls. Alle acht
zuvor stillen terminalen Kante–Zuordnung-Paare wurden entfernt. Der
erhaltene Trainingszustand bestand **0 von 16 selektiven Abrufvergleichen**.
[Versuchsplan](CONTINGENCY_STRUCTURAL.md)

## Rekrutierung während des Trainings

Vier bekannte Seeds und zwei gegensätzliche Zuordnungen wurden auf
demselben entwickelten Ausgangszustand je Seed verglichen. Beide Arme
löschten vor dem viersekündigen Training die Entwicklungs-Eligibility
und nutzten Core-Homöostase. Der feste Arm reproduzierte die archivierten
Trainingsframes, Endgewichte und terminalen Paarereignisse.

| Endpunkt über beide Zuordnungen und alle Seeds | Feste Topologie | Aktive Strukturentwicklung |
| --- | ---: | ---: |
| Gebildete Synapsen | 0 | 1.108 |
| Entfernte Synapsen aus Anfangsgraph und Neubildungen | 0 | 692 |
| Neubildungen im Endgraphen | 0 | 717 |
| Core-Spikes während des Trainings | 8.606 | 10.047 |
| Neue Eingänge an den drei zuvor stillen Relais | 0 | 1 |
| Ankünfte über diese Eingänge | 0 | 1 |
| Spikes der zuvor stillen Relais | 0 | 0 |
| Entfernte dieser acht terminalen Kante–Zuordnung-Paare | 0 | 8 |
| Prä-Ankünfte und neue Tag-Änderungen an diesen acht Paaren | 0 / 0 | 0 / 0 |
| Prä-Ankünfte und neue Tag-Änderungen an allen 23 terminalen Paaren | 259 / 289 | 241 / 311 |

Auf Seed 34123 entstand bei der umgekehrten Zuordnung nach rund
100 ms die Synapse von Zelle 92 zu Relais 93. Sie lieferte einen
positiven Impuls von 0,240 Modelleinheiten. Das Membranpotential
erreichte im selben Integrationsschritt −0,032 bei einer Schwelle von
1,234. Die Synapse wurde rund 400 ms nach ihrer Bildung entfernt.
Die acht stillen terminalen Paare wurden 0,5 bis 2,2 s nach
Trainingsbeginn entfernt. Ihre postsynaptischen Zellen feuerten in
beiden Armen weiterhin. Da Bildung und Entfernung gemeinsam
aktiviert wurden, erfasst der Vergleich ihre gemeinsame Wirkung.

## Abruf aus dem trainierten Zustand

Die Abrufproben behielten die tatsächlich entstandenen Graphen,
Gewichte und Zellzustände. Beide Original-Cues und Schwarz wurden nach
je elf Sekunden Ruhe separat vorgelegt. Die Root-Spikefolgen stimmten
in allen Vergleichen überein. Es gab in beiden Armen keine Core-Spikes
während der Ruhe oder der Schwarzprobe. In zwei der 16 Vergleiche
feuerte die passende Zielgruppe im aktiven Arm einmal. Dieselbe
Zielgruppe feuerte unter der entgegengesetzten Zuordnung auf denselben
Cue einmal beziehungsweise dreimal. Somit wurde keine balancierte
Kontrolle und keine Akutblockade ausgelöst.

## Eingrenzung des nächsten Versuchs

Eine nachträgliche Auswertung der festgehaltenen Homöostase-Trajektorie
ergab, dass Relais 93 und 153 in beiden Zuordnungen an allen 40
Wartungszeitpunkten über der Wachstumsschwelle von 0,5 lagen. Relais
103 lag in beiden Zuordnungen an 35 von 40 Zeitpunkten darüber.
Die Werte stammen aus dem festen Arm. Im aktiven Arm können veränderte
Ankünfte und der Verbrauch von Wachstumsantrieb die Trajektorie
verschieben.

Die aktuelle Kandidatenregel bewertet das Produkt beider Spike-Traces
und die zeitliche Nähe beider letzter Spikes, jeweils mit abklingender
Aktivität, und zieht Distanzkosten ab. Bei einer Zelle, die noch nie
gefeuert hat, sind beide Aktivitätsterme null. Ihr Kandidatenscore bleibt
damit auch bei hohem Wachstumsantrieb unter der positiven Schwelle.
Der Befund legt nahe, im nativen Replay
zunächst den Wachstumsantrieb, den Aktivitätsanteil der Kandidatenscores
und die Nutzungsverläufe vor den Entfernungen zu erfassen. Damit lässt
sich ein Gegenversuch zur Kandidatenauswahl unabhängig von einem
Gegenversuch zur Entfernung planen. Alle stillen terminalen Paare
stammen aus einem Seed. Der Versuch prüft die bekannten Cues zu
einem festen Abrufzeitpunkt.

## Reproduktion

Vier native Records stimmten bei vollständiger Wiederholung exakt
überein. Der Python-Prüfer rekonstruierte aus den Rohdaten die Topologie,
die Herkunft der Ankünfte, die elektrischen Batches und die Abrufkriterien.
Die archivierten Quellstände reproduzierten den Kurzlauf nach
unabhängigem Build bytegenau. Die Projektprüfung umfasste 464
Rust-, 101 Harness-Python- und 10 Dashboard-Tests. Der zusätzliche
Rust-Test zur balancierten Kontrolle und die 18 gezielten
Kontingenztests bestanden ebenfalls.
[Kompakte Daten](dokumentation/visual_contingency_structural_20261123.json)

Modell `0f04226` und Library `9a3a5fb` wurden vor dem Hauptlauf
veröffentlicht. Das lokale, geprüfte ZIP liegt unter
`research/results/archives/visual_contingency_structural_20261123.zip`.
Es enthält beide archivierten Eingaben je Seed, Roh- und Analysedaten,
Quellstände, Binary und SHA-256-Prüfbelege. Die nachträgliche
deskriptive Aggregation liegt dort als `result_index.py`.

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
root=research/results/visual_contingency_structural_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
python3 "$root/contingency_structural_analysis.py" "$root"
python3 "$root/result_index.py" "$root"
sha256sum -c research/results/archives/visual_contingency_structural_20261123.zip.sha256
```
