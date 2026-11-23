# Paar-Timing und Gewichtskonsolidierung auf rekrutierten Pfaden

Im Strukturversuch erzeugte der erstmals wieder aktive Ausgang
153 → 105 auf Seed 34123 sieben Prä-Ankünfte und 18 neue
Eligibility-Änderungen. Das sichtbare Gewicht am Trainingsende
war gleich dem Startwert. Die balancierte Kontrolle rekrutierte
die Quellzelle ebenfalls.
[Ausgangsbefund](CONTINGENCY_STRUCTURAL_EXPLORATION_RESULTS_20261123.md)

Das Experiment prüft, welche Information die zeitlichen Paare
auf den beobachteten Pfaden bereits tragen und wie der lokale
Lernfaktor und die numerische Speicherung sie in Gewichte
übersetzen.

## Vergleich gepaarter Geschichten

Die vier bekannten Seeds 34123, 34127, 34141 und 34157
verwenden die Bedingung `recent_retain` des Strukturversuchs.
Pro Seed werden beide Zuordnungen und eine balancierte
Trainingsfolge vom identischen entwickelten Checkpoint
rekonstruiert. Alle löschen vor dem Training die geerbte
Eligibility und erhalten pro Eingabekanal dieselbe Gesamtzahl
von Root-Spikes. Im balancierten Training treten beide möglichen
Folgebilder je Cue gleich häufig auf. Wo ein balancierter Lauf
bereits archiviert ist, muss dessen Training exakt reproduziert
werden.

Die ausgewerteten Kanten sind die Vereinigung der bereits
archivierten terminalen Kanten beider Zuordnungen pro Seed.
Sie werden in allen drei Geschichten betrachtet. Der Live-Lauf
erhält die gesamte Netzstruktur und alle Bildungsentscheidungen.
Der Offline-Replay verwendet anschließend ausschließlich die
beobachteten Ankünfte dieser Kanten, Postsynapsen-Spikes,
chemische Zustände und exakten Integrationsgrenzen. Er beginnt
mit deren Zustand nach dem Löschen der Eligibility und den
vorhandenen Paarzeitstempeln. Die native Variante muss sichtbare
Gewichte, Rundungsreste, Eligibility, Prä-Spuren und deren Uhren
auf den beobachteten Kanten exakt reproduzieren.

## Faktoren im Offline-Replay

| Variante | Lokaler Lernfaktor | Speicherung |
| --- | --- | --- |
| `original` | Nativ | Akkumuliert mit vorzeichenbehaftetem Rest |
| `direct_f32` | Nativ | Direkte f32-Aktualisierung |
| `constant_positive` | Konstant +1 | Akkumuliert mit Rest |
| `constant_direct_f32` | Konstant +1 | Direkte f32-Aktualisierung |

Der native Faktor wirkt auf das Zeitintegral der
vorzeichenbehafteten Eligibility. Die native Speicherung trägt
unveröffentlichte Gewichtsmasse zur nächsten Aktualisierung.
Alle vier Rechnungen verwenden dieselben beobachteten
Ereignisse. Die konstante Variante ändert nur
den lokalen Multiplikator in der Offline-Rechnung.

Für jede Kante und Geschichte werden positive und negative
Tag-Integrale, angeforderte Beiträge, veröffentlichte Änderungen,
Rest, abgeschnittene Beiträge und Anfragen ohne sichtbare
Einzeländerung ausgewiesen. Die Größen erlauben die
Unterscheidung von kleinen Modulationsfaktoren, sich aufhebenden
Beiträgen und noch gespeichertem Rundungsrest. Als
Richtungsdiagnose wird gezählt, für wie viele der 23
archivierten Kante–Zuordnung-Paare das zugeordnete vorzeichenbehaftete
Signal positiv und größer als dasselbe Signal auf derselben
Kante im entgegengesetzten und balancierten Training ist.
Das gilt separat für Tag-Integral, angeforderte Änderung und
sichtbare Gewichtsänderung. Der rekrutierte Ausgang 153 → 105
wird zusätzlich einzeln ausgewiesen.

Der Befund gilt für die vorhandenen terminalen Kanten auf
bekannten Seeds. Der Offline-Replay prüft lokale
Lernumsetzung bei beobachteter Dynamik. Erst ein späterer
Test auf einem fortlaufend lernenden Netz kann die Rückwirkung
alternativer Gewichte auf die nächste Erfahrung und den Abruf
bestimmen.

## Ausführung

Der Runner wiederholt jeden Record vollständig. Python prüft
Zeitreihen und Veröffentlichungsarithmetik unabhängig. Eingaben,
Quellen, Binary und Prüfer werden vor dem Hauptlauf versiegelt.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
old=research/results/visual_contingency_structural_exploration_20261123
root=research/results/visual_contingency_consolidation_20261123
mkdir -p "$root/input/capacity" "$root/input/exploration" "$root/raw"
for seed in 34123 34127 34141 34157; do
  name="v00000-s${seed}-i000.json"
  cp "$old/input/capacity/$name" "$root/input/capacity/$name"
  cp "$old/raw/$name" "$root/input/exploration/$name"
  research/target/release/nerva-research contingency-structural-consolidation \
    "$root/input/capacity/$name" "$root/input/exploration/$name" \
    > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_consolidation_analysis.py "$root"
```
