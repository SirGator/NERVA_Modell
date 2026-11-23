# Verstärkte Lerngewichte tragen einen einzelnen selektiven Abruf

Die richtungstreue Verstärkung bestand **0 von 16** Lernvergleichen.
Bei stärkerer, durch Gewichtsgrenzen begrenzter Verstärkung bestand
**1 von 16**. Die künstliche Referenz bestand bei letzterer Dosis
**6 von 14** verfügbaren Vergleichen. Auf Seed 34123 erreichte die
Referenz alle vier Kombinationen. Das aus dem Training gewonnene
Muster bestand auf diesem Graphen bei gleicher Norm keine.
[Versuchsplan](CONTINGENCY_GROWN_DOSE.md)

## Gleiche Norm auf demselben gewachsenen Graphen

Je Seed diente der eingefrorene balancierte Graph als gemeinsamer
Träger. Je nach Seed waren dort 255 bis 268 neue Kanten erhalten.
Die Richtungen verwendeten alle ursprünglich vorhandenen, im
Kapazitätsversuch zugelassenen erregenden plastischen
Core→Core-Kanten. Die neuen Kanten behielten in allen Proben ihre
balancierten Trainingsgewichte. Die Root-Spikes der Proben
stimmten exakt überein.

| Seed | Richtungstreue Dosis | Lernen / Referenz bei dieser Dosis | Begrenzte Dosis | Lernen / Referenz bei dieser Dosis | Volle Referenz |
| --- | ---: | --- | ---: | --- | --- |
| 34123 | 0,357 | 0/4 · 0/4 | 4,455 | 0/4 · 4/4 | 4/4 |
| 34127 | 0,228 | 0/4 · 0/2 | 1,575 | 0/4 · 1/2 | 1/4 |
| 34141 | 0,505 | 0/4 · 0/4 | 1,122 | 0/4 · 0/4 | 0/4 |
| 34157 | 0,344 | 0/4 · 0/4 | 1,120 | 1/4 · 1/4 | 1/4 |

Die Referenz für die Umkehrzuordnung auf 34127 enthielt keine
Gewichtsdifferenz. Bei normierten Dosen wurde diese Zuordnung
als fehlend gewertet. In voller Stärke entsprach sie dem
Träger. Für die niedrige richtungstreue Dosis blieb die
Referenz auf allen Seeds unter dem Abrufkriterium. Diese Dosis
lässt offen, ob ein stärkeres unverzerrtes Lernmuster tragen
würde.

Die hohe Dosis veränderte die Core-Spikefolge gegenüber dem
Träger auf 12 von 16 Lernproben. Bei der richtungstreuen Dosis
waren es fünf. Mehrere deutlich veränderte Folgen verfehlten
das Zielpool- oder Kontrollkriterium. Auf 34123 hatten die
publizierten Lernrichtungen bei der hohen Dosis nur noch
Kosinusähnlichkeiten von 0,317 und 0,245 zu den ursprünglichen
Richtungen. Es wurden 24 beziehungsweise 22 Koordinaten
beschnitten. Der Vergleich mit der funktionalen Referenz gilt
damit für die unter diesen Gewichtsgrenzen tatsächlich
publizierten Muster.

## Der einzelne Lernteiltreffer

Auf Seed 34157 erzeugte die Vorwärtsrichtung bei Cue 0 zwei
Spikes im passenden Pool. Balanciertes und vertauschtes Muster
erzeugten dort jeweils einen. Träger und Umkehrrichtung
erzeugten dort keine Spikes. Der falsche Pool, die Ruhe und
Schwarz blieben still. Fünf Koordinaten der Lernrichtung wurden
beschnitten, bei einem Richtungskosinus von 0,922.

Eine deskriptive Auswertung der ursprünglichen, unskalierten
Richtungen ergab Kosinusähnlichkeiten von 0,644 bis 0,982
zwischen den zugeordneten und den balancierten Mustern. Die
volle künstliche Referenz bestätigte auf 34123 die elektrische
Kapazität des gewachsenen Graphen. Die Ähnlichkeit der
Lernrichtungen legt als nächsten kontrollierten Eingriff
nahe, ihren zur balancierten Richtung gemeinsamen Anteil zu
entfernen und den verbleibenden Anteil bei gleicher Dosis
zu prüfen. Dieser Offline-Test könnte zeigen, ob die
beobachtete Lerndynamik bereits ein Abrufsignal enthält,
das von der gemeinsamen Reizstatistik überdeckt wird.
Die vorliegenden Ergebnisse verwenden bekannte Seeds und
eine diagnostische Gewichtsverstärkung am eingefrorenen Netz.

## Reproduktion

Alle vier Records wurden vollständig wiederholt. Der
unabhängige Prüfer rekonstruierte Normierung, Gewichte,
Kontrollkriterien und elektrische Batches. Ein eigener Build
der archivierten Quellen reproduzierte den Kurzlauf bytegenau.
Die Projektprüfung bestand mit 468 Rust-, 120 Harness-Python-
und 10 Dashboard-Tests. Modell `74dc79f` und Library `9a3a5fb`
waren vor dem Hauptlauf veröffentlicht.
[Kompakte Daten](dokumentation/visual_contingency_grown_dose_20261123.json)

Das geprüfte Rohdatenarchiv liegt unter
`research/results/archives/visual_contingency_grown_dose_20261123.zip`.
Es enthält gehashte Eingaben, Rohdaten, Quellen, Binary und
Prüfer. Die nachträgliche deskriptive Aggregation
`result_index.py` enthält die Richtungskosinuswerte.

```bash
root=research/results/visual_contingency_grown_dose_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_grown_dose_analysis.py" "$root" | cmp - "$root/analysis.json"
python3 "$root/result_index.py" "$root" | cmp - "$root/result-index.json"
sha256sum -c research/results/archives/visual_contingency_grown_dose_20261123.zip.sha256
```
