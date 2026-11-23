# Abruf nach Abzug des balancierten Gewichtsmusters

Die zugeordneten Gewichtsmuster ähnelten im letzten Versuch
stark der balancierten Richtung. Ein verstärktes Lernmuster
bestand einen einzelnen Abrufvergleich. Dieser Folgeversuch
prüft, ob der zur balancierten Richtung orthogonale Rest
besser zur erlernten Zuordnung passt.
[Ausgangsbefund](CONTINGENCY_GROWN_DOSE_RESULTS_20261123.md)

## Eingriff und Kontrollen

Der Vergleich verwendet die vier bekannten Seeds 34123,
34127, 34141 und 34157. Der gewachsene balancierte Zustand
und sämtliche Gewichte außerhalb des bisherigen Umfangs
sind in allen Armen identisch. Der Runner rekonstruiert
das archivierte native Training und prüft die Gleichheit
der Trägerproben. Eingriffe verwenden nur die bereits
vor der neuen Wachstumsphase vorhandenen, im
Kapazitätsversuch zugelassenen Core-Kanten.

Für eine gelernte Richtung `d` und die balancierte Richtung
`b` ist die Restkomponente `d − b·(d·b)/(b·b)`. Pro
Zuordnung werden zusätzlich folgende Kontrollen geprüft.

- **Original.** Die unveränderte gelernte Richtung enthält
  den abgezogenen Anteil wieder. Sie erhält dieselbe
  Gewichtsänderungsnorm wie der Rest.
- **Ortskontrolle.** Der Rest wird innerhalb der Eingänge
  jeder Zielzelle permutiert. Dadurch bleiben die je Ziel
  vorliegenden Werte, Vorzeichen und Nullen vor der
  Normierung erhalten.
- **Projektionskontrolle.** Von der ursprünglichen
  Lernrichtung wird eine Projektion auf die innerhalb
  jeder Zielzelle permutierte balancierte Richtung
  abgezogen. Damit kann ein Effekt des Abzugs mit einem
  Abzug auf einer Kontrollrichtung verglichen werden.

Beide Permutationen verwenden den bereits vorher festgelegten
Seed-Algorithmus. Als weitere Vergleiche dienen der
unveränderte Träger, die balancierte Richtung, der Rest der
umgekehrten Zuordnung und die künstliche Referenz bei
derselben Dosis. Eine fehlende Referenz-Zuordnung bleibt
als solche gekennzeichnet.

## Normierung und Endpunkte

Die begrenzte Dosis ist exakt die Dosis aus dem vorigen
Versuch. Publiziert wird die skalierte Richtung mit
Gewichtsgrenzen 0 und 2. Normierungsfehler, abgeschnittene
Koordinaten und Richtungskosinus werden ausgewiesen.
Die richtungstreue Dosis ist das Minimum aus der
archivierten Dosis und 90 % der kleinsten zulässigen
Norm der sechs neuen Richtungen. Der Normfehler darf
höchstens 1 % betragen, der Richtungskosinus muss
mindestens 0,999 erreichen. Archivierte Kontrollen
werden bei exakt gleicher Dosis wiederverwendet.
Sinkt die richtungstreue Dosis, werden die betroffenen
Kontrollarme auf demselben Träger neu gemessen.

Jeder Cue startet nach elf Sekunden Schwarzbildruhe in
einer eigenen Rekonstruktion. Zwei Original-Cues und
Schwarz liefern die Ereignisse für den Vergleich mit
identischen Root-Spikes. Ein spezifischer Resttreffer
verlangt im passenden Pool mehr Spikes als im falschen,
als beim unveränderten Träger, bei der Umkehrrichtung
und bei der balancierten, der Orts- und der
Projektionskontrolle. Ruhe und Schwarz dürfen keine
zusätzliche Core-Aktivität zeigen. Als Verbesserung
zählt der Treffer, wenn er zusätzlich die ursprüngliche
Lernrichtung bei gleicher Dosis übertrifft. Voller
Abruf verlangt vier Treffer pro Seed. Kandidatenmargen
in zeitlich übereinstimmenden Integrationsbatches werden
ergänzend ausgewertet.

Die Projektion greift auf getrennte Trainingsgeschichten
zu und dient der Offline-Diagnose. Ein späterer
Online-Versuch müsste eine solche Trennung mit
Information aus der lokalen Aktivität selbst erreichen.

## Reproduktion

Jeder native Record wird vollständig wiederholt. Der
unabhängige Prüfer rekonstruiert Projektionen,
Permutationen, Gewichte und Kontrollergebnisse aus
den Rohdaten. Eingaben, Quellen, Binary und Prüfer
werden vor dem Hauptlauf mit SHA-256 gebunden.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
old=research/results/visual_contingency_grown_dose_20261123
root=research/results/visual_contingency_grown_residual_20261123
mkdir -p "$root/input/capacity" "$root/input/consolidation" "$root/input/grown_dose" "$root/raw"
for seed in 34123 34127 34141 34157; do
  name="v00000-s${seed}-i000.json"
  cp "$old/input/capacity/$name" "$root/input/capacity/$name"
  cp "$old/input/consolidation/$name" "$root/input/consolidation/$name"
  cp "$old/raw/$name" "$root/input/grown_dose/$name"
  research/target/release/nerva-research contingency-grown-residual \
    "$root/input/capacity/$name" "$root/input/consolidation/$name" "$root/input/grown_dose/$name" \
    > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_grown_residual_analysis.py "$root"
```
