# Welche lokalen Entscheidungen die Rekrutierung begrenzen

Die Strukturentwicklung erzeugte im vorherigen Training 1.108 neue
Kanten, darunter einen Eingang zu den drei zuvor stillen Relais. Er
lieferte einen unterschwelligen Impuls. Alle acht stillen terminalen
Kante–Zuordnung-Paare wurden entfernt.
[Ausgangsbefund](CONTINGENCY_STRUCTURAL_RESULTS_20261123.md)

Dieses Replay zeichnet den Zustand am tatsächlichen Wartungszeitpunkt
vor jeder Topologieänderung auf. Es unterscheidet fehlenden
Wachstumsantrieb, fehlende freie Nachbarn und Ablehnung durch die lokale
Scoringregel. Bei entfernten Kanten rekonstruiert es den gespeicherten
und abgeklungenen Nutzen sowie den Zeitgeber unterhalb der Schwelle.
Die Entwicklungs- und Lernmechanismen bleiben im nativen Arm aktiv.
Ausgewertet wird die Laufzeituhr beim Aufruf des Controllers. Sie
kann vom Ende des zugehörigen Beobachtungsfensters abweichen.

## Vergleich

Verwendet werden die vier archivierten Seeds 34123, 34127, 34141 und
34157 und beide ursprünglichen Zuordnungen. Die 40 Wartungszeitpunkte
pro Training werden unabhängig rekonstruiert. Für jedes Training
werden die zuvor identifizierten stillen Relais und alle terminalen
Kanten aus dem Kapazitätsversuch beobachtet. Dazu kommen je zwölf
Core-Zellen aus den im festen Training stillen und aktiven Zellen.
Bei weniger als zwölf Zellen werden alle verwendet. Die Auswahl
greift in der sortierten Gruppe der Länge `N` an den Indizes
`floor(i*N/min(12,N))` zu. Die Vergleichszellen sind eine
deskriptive Stichprobe auf bekannten Seeds.

Pro Zelle und Wartungszeitpunkt gibt es vier mögliche Befunde.

- Der Wachstumsantrieb liegt höchstens auf seiner Schwelle.
- Bei höherem Antrieb fehlt ein geometrisch erreichbarer,
  noch unverbundener Core-Nachbar.
- Bei höherem Antrieb und freien Nachbarn verfehlen alle
  Kandidaten den Mindestscore.
- Die Regel plant eine Bildung.

Für die dritte Kategorie werden aktive Quellen als diejenigen mit
einem letzten Spike innerhalb der konfigurierten Recency-Zeitkonstante
vor der Entscheidung gezählt. Deren Polarität wird separat erfasst.
Die Rohdaten enthalten die Spike-Traces, letzten Spikezeiten,
Distanzen und Affinitäten aller lokal verfügbaren Verbindungen.
Damit bleiben gealterte Zielaktivität, ein stilles lokales Umfeld und
fehlende räumliche Möglichkeiten unterscheidbar. Die Zuordnung der
beobachteten Zellen dient ausschließlich der Auswertung.

Bei terminalen Kanten wird geprüft, ob der kontinuierlich beobachtete
Zeitraum mit niedrigem Nutzen und die aktuelle Nutzenschwelle die
Entfernung erklären. Ausgewiesen werden die erste beobachtete
Unterschreitung, der native Start des Zeitgebers und die tatsächliche
Entfernungszeit. Die Anfangsphase kann noch Evidenz und Zeitgeber aus
der Entwicklung enthalten. Die letzte Spikezeit einer Zelle kann
ebenfalls aus der Entwicklung stammen.

## Prüfung

Der Rust-Runner verlangt für jedes Training dieselben Frames,
Eingangs- und Core-Spikes, Gewichtszählungen, Topologieereignisse,
Endgewichte und Paarereignisse wie im archivierten aktiven Arm.
Zusätzlich muss ein zweiter vollständiger Durchlauf identische
Entscheidungsdaten liefern. Der Python-Prüfer rekonstruiert die
Geometrie, Scores, Auswahl und Entfernungsentscheidungen aus den
gespeicherten Zuständen und gleicht sie mit den archivierten
Topologieereignissen ab.

Ein unter hohem Antrieb regelmäßig verworfener Kandidat begründet
einen getrennten späteren Eingriff in die Kandidatenauswahl.
Unterschwellige Nutzungsverläufe können einen separaten Eingriff in
das Entfernen begründen. Eine alternative Regel und ein neuer
Gedächtnisnachweis sind eigene Versuche.

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
old=research/results/visual_contingency_structural_20261123
root=research/results/visual_contingency_structural_decisions_20261123
mkdir -p "$root/input/structural" "$root/raw"
for seed in 34123 34127 34141 34157; do
  name="v00000-s${seed}-i000.json"
  cp "$old/raw/$name" "$root/input/structural/$name"
  research/target/release/nerva-research contingency-structural-decisions \
    "$old/input/capacity/$name" "$root/input/structural/$name" \
    > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
python3 research/contingency_structural_decisions_analysis.py "$root"
```
