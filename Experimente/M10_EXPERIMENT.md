# M10: Signalweg und feste Auslesung lokalisieren

Diagnoseplan vom **27.10.2026**, nach Abschluss und Kenntnis der M9-Ergebnisse,
vor den neuen M10-Replaymessungen. **Retrospektive explorative Ursachenanalyse,
keine neue Bestätigung auf unbenutzten Seeds.** Es wird keine neue biologische
Regel ergänzt und kein neuer Ausleser auf den Aktionslabels trainiert.

**Status: alle 20 Replaydiagnosen abgeschlossen und auditiert.**
[Ergebnisse](../../Messungen/M10_EXPERIMENT.md).

## Fragestellung und festgelegter Umfang

M9 zeigt mehr Aktivität, aber keinen funktionalen Lernnachweis. Die vorhandenen
Gewichtsinterventionen erlauben bereits, Gewichtsunterschiede, Spikezählmuster
und Ausleserentscheidungen getrennt zu betrachten. M10 untersucht alle vier
M9-Bedingungen, vier Arme, beide Belegungen und beide primären Zeitpunkte auf
sämtlichen vorhandenen Seeds 610–629. Fälle werden nicht nach Einzelerfolg gewählt.

Zusätzliche passive Replaytraces werden ausschließlich für die vorher festgelegte
Kombinationsbedingung auf allen 20 Seeds aufgezeichnet. Training und Prüfungen
werden exakt wie im ursprünglichen M9 wiedergegeben. Verglichen werden `sham`,
`restore_core` und `restore_all_plastic` auf unabhängigen eingefrorenen Zuständen.
Alle ursprünglichen Ergebnisse und sechs CSV-Dateien müssen reproduziert werden.
Ein eigener technischer Kurzlauf verwendet die vorhandenen M9-v2-Kurzlaufdaten.

## Messungen

1. Belegungsabhängige Core-Gewichtsunterschiede, einschließlich eingefrorener und
   entkoppelter Kontrollen.
2. Unterschied der A-/D-Spikezählmuster gegenüber der Gleichklassifikation durch
   den vorhandenen externen visuellen Spikezähl-Ausleser.
3. Nach gezielter Gewichtsrücksetzung: Spikezähl-L1, veränderte Spikezeiten,
   Entscheidung und vorzeichenbehafteter Abstand zur richtigen visuellen Klasse.
4. Im Replay: sämtliche externen Eingänge, synaptischen Ankünfte mit tatsächlicher
   signierter Amplitude und Spikezeiten in 50 ms Startbild sowie 10/10/30 ms
   Aktionsintegration; Ankunfts- und Amplitudensummen getrennt pro Core-Zelle.
5. Membranpotenziale vor und nach jeder Phase, **auf Neuronenkopien auf einen
   gemeinsamen Zeitstempel fortgeschrieben**. Die Simulation wird dabei nicht
   verändert. Die bisherigen M9-Potenziale sind gespeicherte Zustandswerte mit
   unterschiedlichen letzten Aktualisierungszeiten; daraus allein wird keine
   synchrone Spannungskarte abgeleitet.

Der LIF-Code modelliert synaptische Eingangsimpulse, keine biophysikalischen
Leitfähigkeitsströme. Refraktärzeiten, Leck und gleichzeitige Hemmung begrenzen
die Interpretation von Eingangssummen. Gewichte bleiben während sämtlicher
Proben eingefroren; es wird kein Wirkungsbild gezeigt. Der vorhandene Ausleser
und seine Parameter bleiben identisch.

## Auswertung und Grenzen

Vorab festgelegt sind Häufigkeiten von Gewichts-, Aktivitäts- und
Entscheidungsänderungen, A-/D-Unterscheidbarkeit, Eingangsimpuls- und synchrone
Potenzialdifferenzen sowie Kontrollarmvergleiche. Berichtet wird deskriptiv nach
Seed, Belegung, Zeitpunkt und Eingriff. Viele Zweige desselben Seeds sind keine
zusätzlichen unabhängigen Replikate. Es gibt keinen neuen Signifikanz- oder
Lernnachweis aus dieser Wiederverwendung bereits untersuchter Seeds.

Ein Unterschied im Spikezählmuster bei gleicher Klassifikation zeigt einen
Engpass dieser konkreten Auslesung; er beweist weder identische neuronale
Antworten noch allgemeine Unauslesbarkeit der Information. Ein Eingriffseffekt
belegt den Gewichtsbeitrag bei festem Zustand, nicht alleinige Speicherung oder
Hinreichendheit der Gewichte. Reisende Impulse behalten ihre Emissionsamplituden.

Der Python-Audit rekonstruiert Zähler, Impulssummen, Aktionsmerkmale und Decoder-
Entscheidungen aus den Ereignissen. Jede Messung muss den ursprünglichen
M9-Ergebnisbaum bis auf JSON-Fließkomma-Rundung in der letzten f64-Stelle
reproduzieren; Ganzzahlen bleiben exakt. Alte CSVs müssen bytegleich sein.
Die synchronen Potenziale werden zusätzlich unabhängig mit Eingängen, Leck,
Reset und Refraktärzeiten nachgerechnet; f32-Akkumulationstoleranz 2×10⁻⁵.

[Ausführung](m10/README.md). Quellen und Manifeste werden vor den Replayläufen
über SHA-256 festgehalten; vorhandene wissenschaftliche Rohdaten werden nicht
überschrieben.
