# Strukturplastizität während des visuellen Trainings

Auf dem eingefrorenen Graphen erhöhte Core-Homöostase bei drei
stillen Relais den strukturellen Antrieb, ohne ihren intrinsischen
Strom zu ändern. Dieser Versuch lässt die vorhandene lokale Bildung
und Entfernung von Synapsen darauf reagieren. Er prüft, ob neue
Eingänge die zuvor stillen Quellen rekrutieren, ob dadurch neue
Paarereignisse auf den bekannten terminalen Kanten entstehen und
ob der trainierte Graph selbst selektiven Abruf trägt.
[Ausgangsbefund](CONTINGENCY_CORE_HOMEOSTASIS_RESULTS_20261123.md)

## Gepaarter Vergleich

Verwendet werden die vier Seeds mit archiviertem Folgetraining
34123, 34127, 34141 und 34157. Beide Arme starten je Seed und
Zuordnung mit exakt demselben entwickelten Checkpoint. Die
Eligibility wird bei beiden vor dem Training gelöscht und die
vorhandene Core-Homöostase eingeschaltet. Beide erhalten acht
Wiederholungen derselben Cue- und Folgebildfolgen mit 50 ms Abstand
zwischen den Reizanfängen. Das gesamte Training dauert vier
Sekunden.

- `frozen` hält die Topologie fest und muss die frühere
  `core_clear`-Bedingung exakt reproduzieren.
- `growing` aktiviert die bereits implementierte lokale
  Strukturentwicklung während desselben Trainings. Bildung
  und Entfernung wirken in diesem Arm gemeinsam. Der Vergleich
  schätzt ihre Gesamtwirkung, einschließlich späterer Änderungen
  der Lernhistorie.

Es werden keine Zielgruppen, Kantenlisten, Zuordnungslabels oder
Rewards an die Entwicklungs- oder Lernregel übergeben. Die
archivierten Zielgruppen und terminalen Pfade dienen ausschließlich
der Auswertung. Die frühen und späten Zeitpunkte der Bildung und
Entfernung werden aufgezeichnet. Außerhalb des Core bleibt die
Topologie der beobachteten Arme fest.

## Rekrutierung im Training

Der primäre mechanistische Endpunkt betrifft dieselben acht
terminalen Kante–Zuordnung-Paare mit zuvor stillen Quellen aus dem
Kapazitäts- und Eligibility-Versuch. Ausgewertet werden alle
neuen Eingänge an ihren Quellen, tatsächliche Ankünfte über
diese Kanten, gespikte Zeitpunkte und die Zahl der neuen
Prä-/Post-Tag-Änderungen auf den bekannten terminalen Kanten.
Der Sendezeitpunkt des Prä-Impulses ergibt sich aus Ankunft
minus Kantenverzögerung. Neue Paaränderungen verlangen ein
Prä-Ereignis aus dieser Trainingsphase und einen passenden
Post-Spike im nativen Paarfenster. Das überschneidende
Zeitfenster kann auch Schwarzbildphasen umfassen. Eine
Rekrutierung in Schwarz ist kein selektiver Abruf.

Für die ursprünglich stillen Quellen werden die elektrischen
Integrationsgrenzen und ankommenden Amplituden gespeichert.
Eine neue Ankunft vor einem Spike ist zeitliche Evidenz.
Ein Vergleich am aufgezeichneten Integrationszeitpunkt kann
die Notwendigkeit bestimmter gleichzeitiger Eingänge in diesem
Batch prüfen. Er ersetzt keine alternative Netztrajektorie.
Verlorene terminale Kanten und fehlende Ankünfte werden getrennt
berichtet.

## Abruf mit erhaltener Topologie

Nach Ende des Trainings werden Lernen, Entwicklung und Homöostase
eingefroren. Die tatsächlich entstandenen Graphen, Gewichte und
Zellzustände bleiben erhalten. Jede Probe startet auf einer
deterministisch rekonstruierten Kopie des jeweiligen trainierten
Zustands nach elf Sekunden Schwarzbildruhe. Verglichen werden
die beiden Original-Cues und Schwarz. Die Probefolge eines
Cues wird unabhängig von den übrigen Cues gemessen.

Eine vorläufige Cue–Zuordnungs-Kombination besteht, wenn die
passende Zielgruppe mehr Spikes hat als die andere Zielgruppe,
als dieselbe Zielgruppe unter der entgegengesetzten Zuordnung
und als im gepaarten eingefrorenen Graphen. Schwarz- und
Ruheaktivität dürfen gegenüber dem eingefrorenen Graphen nicht
ansteigen. Gibt es irgendeinen solchen Teiltreffer, wird die
balancierte Zuordnung mit derselben Bildhäufigkeit und aktiver
Strukturentwicklung rekonstruiert. Dann muss jede erfolgreiche
Kombination zusätzlich die balancierte Kontrolle übertreffen.
Ein vollständiger Erfolg verlangt vier bestandene Kombinationen.
Gemischte Ergebnisse werden als Teiltreffer ausgewiesen.

Gibt es nach diesen Kontrollen einen Teiltreffer und neue
aktivierte Kanten im zugehörigen Endgraphen, werden in einer
identischen gefrorenen Kopie alle solchen neuen Übertragungen
vor der Probe blockiert. Bereits reisende Impulse dieser Kanten
werden verworfen. Der Vergleich prüft gemeinsam den Beitrag
dieser Übertragungen zum Abruf. Er trennt weder einzelne
notwendige Kanten noch einen eigenständigen Effekt der
Entfernungen während des Trainings. Die Studie prüft die
ursprünglichen Cues auf bekannten Seeds.

## Ausführung

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
capacity=research/results/visual_contingency_capacity_20261123
homeostasis=research/results/visual_contingency_core_homeostasis_20261123
root=research/results/visual_contingency_structural_20261123
mkdir -p "$root/input/capacity" "$root/input/homeostasis" "$root/raw"
for seed in 34123 34127 34141 34157; do
  name="v00000-s${seed}-i000.json"
  cp "$capacity/records/$name" "$root/input/capacity/$name"
  cp "$homeostasis/raw/$name" "$root/input/homeostasis/$name"
  research/target/release/nerva-research contingency-structural \
    "$root/input/capacity/$name" "$root/input/homeostasis/$name" \
    > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
python3 research/contingency_structural_analysis.py "$root"
```

Alle Records werden vollständig wiederholt. Eingaben, Quellen,
Runner und unabhängige Auswertung sind vor dem Hauptlauf durch
SHA-256 gebunden.
