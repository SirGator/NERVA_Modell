# Gleiche Gewichtsdosis auf gewachsenen Graphen

Im vorangegangenen Versuch lagen auf 10 von 23 ausgewählten
terminalen Kante–Zuordnung-Paaren kleine positive native
Gewichtsänderungen über beiden Trainingskontrollen. Der
Versuch hier prüft, ob das vollständige native Muster auf den
erhaltenen Ausgangskanten bei höherer Dosis einen selektiven
Abruf trägt.
[Ausgangsbefund](CONTINGENCY_CONSOLIDATION_RESULTS_20261123.md)

## Gemeinsamer Zustand und Gewichtsmuster

Jeder der vier bekannten Seeds 34123, 34127, 34141 und 34157
verwendet seinen gewachsenen Zustand aus der balancierten
`recent_retain`-Trainingsgeschichte. Der Runner rekonstruiert
diesen Zustand und friert die Regeln ein. Jede Probe erhält
eine unabhängige Rekonstruktion desselben Zustands. Der
gemeinsame Träger erhält neue Verbindungen, Zellzustände,
Entwicklungsverlauf und die Gewichte des balancierten Armes.
Die diagnostische Änderung betrifft ausschließlich die
bereits am entwickelten Checkpoint vorhandenen plastischen
erregenden Core→Core-Kanten aus dem Kapazitätsprotokoll.

Die natürliche Richtung je Vorwärts-, Umkehr- und
Balanciert-Geschichte ist der veröffentlichte Endwert minus
dem entwickelten Anfangswert auf diesen Kanten. Alle
Kanten des festgelegten Umfangs gehen ein, einschließlich
Nulländerungen. Die Auswertung der terminalen Kanten aus
dem letzten Versuch bestimmt keine Teilmenge. Die beiden
vertauschten Kontrollen permutieren die jeweilige Richtung
innerhalb der Eingänge zur selben postsynaptischen Zelle.
Eine feste Seed-Permutation wird auf beide Zuordnungen
angewandt. So bleiben die pro Ziel vorliegenden
Änderungswerte einschließlich Vorzeichen und Nullen erhalten.

Die künstliche Kapazitätsreferenz setzt für jede Zuordnung
die im archivierten Referenzarm veränderten Ausgangskanten
auf ihre archivierten Referenzgewichte. Alle anderen
Gewichte stammen vom gewachsenen Träger. Sie wird in voller
Stärke und bei den beiden Vergleichsdosen geprüft. Sie
dient als funktionale Kontrolle des gewachsenen Graphen.
Wo eine Referenz-Zuordnung keine Gewichtsdifferenz enthält,
entspricht ihr voller Zustand dem Träger. Die Zuordnung
entfällt bei normierten Referenzproben und bleibt in der
Referenzbilanz als unvollständig sichtbar. Die übrigen Arme
bilden weiterhin einen gepaarten Dosisvergleich.

## Dosen und Endpunkte

- **Richtungstreu.** Die Sollnorm ist 90 % der kleinsten
  zulässigen L2-Änderungsnorm entlang aller verfügbaren
  unbeschnittenen Richtungen. Erlaubt sind höchstens 1 %
  relativer Normfehler und ein Richtungskosinus von
  mindestens 0,999.
- **Begrenzt.** Die Sollnorm ist 90 % des kleinsten Werts
  aus den positiven vollen Referenznormen und den
  unter Gewichtsgrenzen erreichbaren positiven Normen. Eine
  Bisection entlang jeder Richtung publiziert
  `clamp(Basis + Skalierung × Richtung, 0, 2)` als f32.
  Alle Arme halten die Norm innerhalb von 1 % ein.
  Kosinus und Zahl der durch die Grenzen beschnittenen
  Koordinaten werden ausgewiesen. Diese Probe prüft
  eine Transformation der ursprünglichen Richtung.

Die Dosen werden vor den Abrufproben allein aus den
Gewichten und ihren Grenzen bestimmt. Der native
Träger ohne Intervention bleibt die Nullkontrolle.
Nach elf Sekunden Schwarzbildruhe werden die beiden
Original-Cues und eine Schwarzprobe separat vorgelegt.
Die Root-Spikes müssen übereinstimmen.

Der passende Zielpool muss mehr Spikes als der falsche,
als im Träger und als im entgegengesetzten Zuordnungsmuster
haben. Beim natürlichen Muster muss er zusätzlich dieselbe
Gruppe unter balanciertem Muster und zugehöriger
vertauschter Richtung übertreffen. Ruhe und Schwarz dürfen
gegenüber dem Träger keine zusätzliche Core-Aktivität
auslösen. Volle Selektivität verlangt beide Cues unter
beiden Zuordnungen. Die Bewertung zählt Teiltreffer
separat. Erreicht die Referenz am gleichen Prüfpunkt
keinen vollständigen Abruf, bleibt die dort nachgewiesene
Abrufkapazität begrenzt.

Die Ankünfte und Integrationsbatches der Zielzellen
zeigen zusätzlich elektrische Verschiebungen. Ein
gepaarter Margenvergleich verwendet übereinstimmende
Integrationszeitpunkte und markiert Änderungen der
Spike-Trajektorie. Der Versuch prüft die elektrische
Wirkung vorhandener Gewichtsmuster. Er verwendet bekannte
Seeds und keine fortlaufende Gewichtsverstärkung im Live-Netz.

## Reproduktion

Der native Runner wiederholt jeden Record vollständig.
Die unabhängige Python-Prüfung rekonstruiert Richtung,
Gewichtsdosis, Spike-Kriterien und elektrische Batches.
Eingaben, Quellen, Binary und Prüfer werden vor dem
Hauptlauf über SHA-256 gebunden.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
old=research/results/visual_contingency_consolidation_20261123
root=research/results/visual_contingency_grown_dose_20261123
mkdir -p "$root/input/capacity" "$root/input/consolidation" "$root/raw"
for seed in 34123 34127 34141 34157; do
  name="v00000-s${seed}-i000.json"
  cp "$old/input/capacity/$name" "$root/input/capacity/$name"
  cp "$old/raw/$name" "$root/input/consolidation/$name"
  research/target/release/nerva-research contingency-grown-dose \
    "$root/input/capacity/$name" "$root/input/consolidation/$name" \
    > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_grown_dose_analysis.py "$root"
```
