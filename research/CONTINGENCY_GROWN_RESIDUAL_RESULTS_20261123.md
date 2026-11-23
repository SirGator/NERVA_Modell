# Der Abzug der balancierten Richtung verbessert den Abruf nicht

Der Rest nach Abzug der balancierten Richtung bestand
**0 von 32** Cue–Zuordnungs-Vergleichen. Das gilt sowohl
für das Selektivitätskriterium gegenüber den Kontrollen
als auch für die zusätzliche Verbesserung gegenüber
dem ursprünglichen Lernmuster bei gleicher Dosis.
[Versuchsplan](CONTINGENCY_GROWN_RESIDUAL.md)

## Gewichts- und Funktionskontrollen

Auf jedem der vier Seeds wurde derselbe eingefrorene
gewachsene Träger wie im vorigen Experiment rekonstruiert.
Seine Proben stimmten exakt mit dem Archiv überein. Das
Restmuster behielt vor Normierung je nach Zuordnung und
Seed zwischen 19 % und 76 % der L2-Norm des ursprünglichen
Musters. Verglichen wurden neben dem Träger und der
balancierten Richtung auch der innerhalb von Zielzellen
permutierte Rest und die Projektion auf eine permutierte
Kontrollrichtung.

| Seed | Richtungstreue Dosis | Resttreffer | Begrenzte Dosis | Resttreffer | Künstliche Referenz bei begrenzter Dosis |
| --- | ---: | ---: | ---: | ---: | ---: |
| 34123 | 0,276 | 0/4 | 4,455 | 0/4 | 4/4 |
| 34127 | 0,228 | 0/4 | 1,575 | 0/4 | 1/2 |
| 34141 | 0,505 | 0/4 | 1,122 | 0/4 | 0/4 |
| 34157 | 0,344 | 0/4 | 1,120 | 0/4 | 1/4 |

Auf Seed 34123 erzwangen die Gewichtsgrenzen eine kleinere
richtungstreue Norm als im Ausgangsversuch. Die
Kontrollarme wurden bei der neuen Norm von 0,276 neu
gemessen. In den übrigen richtungstreuen und allen
begrenzten Vergleichen konnten die archivierten
Kontrollarme bei unveränderter Norm herangezogen werden.
Keine künstliche Referenz bestand bei der niedrigen
richtungstreuen Dosis. Die Umkehrreferenz auf 34127
enthielt wie zuvor keine Gewichtsänderung.

## Wirkung auf die Spikefolge

Die Restmuster änderten gegenüber dem Träger 7 von 16
Core-Spikefolgen bei richtungstreuer und 11 von 16 bei
begrenzter Norm. Auf 34157 sank im einzigen vorherigen
Teiltreffer die Zahl passender Zielspikes von zwei auf
einen. Die balancierte Richtung, die Ortskontrolle und
die Projektionskontrolle erzeugten jeweils ebenfalls
einen. Auf 34123 rief die künstliche Referenz bei gleicher
begrenzter Norm beide Zuordnungen vollständig ab.

Die publizierten Restmuster bei der begrenzten Norm hatten
Richtungskosinuswerte von 0,553 bis 0,997 gegenüber dem
Rest vor Veröffentlichung. Bei richtungstreuer Dosis
blieben die Richtungen innerhalb der vorgegebenen Toleranz.
Die Ergebnisse betreffen genau diese beiden publizierbaren
Dosen auf bekannten Seeds. Die begrenzte Dosis lässt
offen, wie sich eine gleich starke unverzerrte Richtung
verhalten hätte.

Der nächste kausale Test sollte in der laufenden
Plastizität ansetzen. Eine prüfbare Hypothese ist, dass
lokales Paar-Timing erst dann eine unterscheidbare Richtung
erzeugt, wenn die Regel einen langsam geschätzten Beitrag
aus der Hintergrundaktivität abzieht. Der Vergleich auf
neuen Seeds braucht gepaarte und balancierte Reizfolgen
mit denselben Häufigkeiten sowie den Abruf nach
nachfolgendem Lernen. Die Projektion dieses Versuchs
verwendet getrennte Trainingsgeschichten und liefert
keine Umsetzung einer solchen lokalen Regel.

## Reproduktion

Alle vier Records wurden vollständig wiederholt. Ein
unabhängiger Prüfer rekonstruierte die Projektionen,
Gewichte, Dosisevidenz und Abrufkriterien. Der aus den
archivierten Quellen neu gebaute Runner reproduzierte
den Kurzlauf bytegenau und der weiterverwendete
Dosisrunner bestand seinen archivierten Kurzlauf.
Die vollständige Projektprüfung umfasste 468 Rust-,
124 Harness-Python- und 10 Dashboard-Tests.
Modell `8b38b8a` und Library `9a3a5fb` waren vor dem
Hauptlauf veröffentlicht.
[Kompakte Daten](dokumentation/visual_contingency_grown_residual_20261123.json)

Das geprüfte Rohdatenarchiv liegt unter
`research/results/archives/visual_contingency_grown_residual_20261123.zip`.
Es enthält Eingaben, Rohdaten, Quellen, Binary und beide
Auswertungen. `result_index.py` ergänzt die gepaarten
Spike- und Margenvergleiche.

```bash
root=research/results/visual_contingency_grown_residual_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_grown_residual_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/result_index.py" "$root" | cmp - "$root/result-index.json"
sha256sum -c research/results/archives/visual_contingency_grown_residual_20261123.zip.sha256
```
