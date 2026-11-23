# Lokale Synapsensuche für bisher stille Zielzellen

Bei 3.840 Entscheidungen an stillen Vergleichszellen verhinderte
der bisherige Mindestscore jede Neubildung trotz ausreichendem
Antrieb und freier Nachbarn. An 867 dieser Zeitpunkte hatte ein
freier erregender Nachbar in den vorangegangenen 100 ms gefeuert.
Alle acht untersuchten stillen Relaisausgänge gingen im nativen
Training durch den Nutzungsregler verloren.
[Ausgangsbefund](CONTINGENCY_STRUCTURAL_DECISIONS_RESULTS_20261123.md)

## Eingriffe

Vier bereits archivierte Seeds 34123, 34127, 34141 und 34157
durchlaufen je beide Cue–Folgebild-Zuordnungen in sechs Armen.
Die Arme starten je Seed auf demselben entwickelten Checkpoint,
löschen dieselbe Entwicklungs-Eligibility und nutzen native
Core-Homöostase, STDP und Synapsenbildung. Das Training umfasst
je acht Wiederholungen und dauert vier Sekunden. Die drei
Bildungsbedingungen werden mit nativer Entfernung und vollständig
unterdrückter Entfernung gekreuzt. Im zweiten Fall werden weder
alte noch neue Kanten entfernt. Der Nutzungsregler wird weiterhin
ausgewertet.

- `native` nutzt die bestehende Kandidatenregel.
- `recent` eröffnet eine zusätzliche Option, wenn der native
  Score keinen Kandidaten freigibt, der lokale Wachstumsantrieb
  über 0,5 liegt und die Zielzelle in den letzten 100 ms
  keinen Spike abgegeben hat. Mindestens ein freier erregender
  Nachbar im Radius 0,25 muss in diesem Fenster gefeuert haben.
  Gewählt wird der Nachbar mit dem jüngsten Spike. Bei Gleichstand
  entscheiden Distanz und Zell-ID.
- `nearest` nutzt dieselbe Gelegenheitsschwelle und wählt
  unter allen freien erregenden Nachbarn den räumlich nächsten.
  Bei Gleichstand entscheidet die Zell-ID.

Beide Zusatzbedingungen erlauben je Zielzelle höchstens eine
explorative Bildung im Training, auch wenn deren Kante später
entfernt wird. Anfangsgewicht, Verzögerung, Plastizität, Profile
und Verbrauch von Wachstumsantrieb entsprechen der nativen
Bildung. Veränderungen der Netztrajektorie können Zahl und
Zeitpunkt der realisierten Gelegenheiten verschieben. Diese
Zahlen werden je Arm ausgewiesen. Die Regeln erhalten keinerlei
Zielgruppen, Zuordnungslabels, Kantenlisten oder Rewards.

## Endpunkte

Verglichen werden die gepaarten Unterschiede innerhalb von Seed,
Zuordnung und Entfernungsbedingung. Die Bildungszahlen trennen
native und explorative Kanten. Für die explorativen Kanten werden
tatsächliche Ankünfte und im Endgraphen erhaltene Kanten erfasst.
Die Rekrutierung wird für alle im archivierten festen Training
stillen Core-Zellen sowie separat für die drei ausgewählten
Kapazitätsrelais gemessen. Ein Spike nach einer neuen Ankunft ist
zeitliche Evidenz. Bei den drei Relais kann zusätzlich der
Integrationsbatch mit seinen tatsächlichen Amplituden geprüft
werden. Die terminalen Endpunkte sind Ankünfte mit einer Quelle
aus diesem Training, neue STDP-Tag-Änderungen und erhaltene
terminale Kante–Zuordnung-Paare. Der Faktor Entfernung gilt für
den ganzen Core und kann seine Gesamtaktivität verändern.

Elf Sekunden nach Trainingsende werden aus unabhängigen
Rekonstruktionen desselben eingefrorenen Trainingszustands die
beiden Original-Cues und Schwarz geprüft. Ein vorläufiger
Teiltreffer verlangt mehr Spikes in der passenden Zielgruppe
als in der falschen, als unter entgegengesetzter Zuordnung
und als im archivierten Arm mit fester Topologie. Root-Input,
Ruhe und Schwarz werden wie im Strukturversuch kontrolliert.
Jeder Arm mit einem vorläufigen Teiltreffer erhält zusätzlich
eine balancierte Trainingskontrolle mit identischen Bildhäufigkeiten.
Ein Teiltreffer muss sie übertreffen. Vier konsistente
Cue–Zuordnungs-Kombinationen ergeben einen vollständigen Treffer.

Wenn ein `recent`-Teiltreffer außerdem die `native`- und
`nearest`-Arme der gleichen Entfernungsbedingung in der passenden
Zielgruppe übertrifft, werden alle überlebenden explorativen
Übertragungen vor dem Abruf in einer identischen Kopie akut
blockiert. Der Eingriff verwirft ihre bereits unterwegs
befindlichen Impulse. Er prüft einen unmittelbaren Beitrag zum
Abruf. Frühere Effekte dieser Kanten auf die Trainingshistorie
bleiben in beiden Kopien erhalten.

Die Eingriffe dienen der Mechanismusprüfung auf bekannten Seeds.
Ein möglicher Nutzen der Regel für kontinuierliches Lernen
erfordert prospektive Versuche mit neuen Seeds, Cues und späterem
Lernen. Die Zählungen pro Kante, Zelle und Stimulus gelten
innerhalb eines Runs als abhängige Messungen.

## Reproduktion

Der Rust-Runner verlangt die exakte Rekonstruktion des archivierten
`native_prune`-Trainings samt Abruf und wiederholt sämtliche
Arme. Der Python-Prüfer rekonstruiert die Kandidatenwahl für
explorative Geburten anhand der aufgezeichneten Zeiten,
Nachbarschaften und Kantenlebensläufe. Der Rust-Pfad setzt den
Ausschluss durch die native Regel durch. Der Prüfer misst
Ankünfte, Paarereignisse, Kontrollkontraste und Abruf aus
denselben Rohdaten. Eingaben, Quellen, Binary und Prüfer werden
vor dem Hauptlauf über SHA-256 gebunden.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
old=research/results/visual_contingency_structural_20261123
root=research/results/visual_contingency_structural_exploration_20261123
mkdir -p "$root/input/capacity" "$root/input/structural" "$root/raw"
for seed in 34123 34127 34141 34157; do
  name="v00000-s${seed}-i000.json"
  cp "$old/input/capacity/$name" "$root/input/capacity/$name"
  cp "$old/raw/$name" "$root/input/structural/$name"
  research/target/release/nerva-research contingency-structural-exploration \
    "$root/input/capacity/$name" "$root/input/structural/$name" \
    > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_exploration_analysis.py "$root"
```
