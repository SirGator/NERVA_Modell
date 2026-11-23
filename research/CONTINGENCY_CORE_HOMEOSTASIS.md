# Core-Homöostase und neue Lernereignisse auf stillen Abrufpfaden

Die acht terminalen Kanten mit stiller Quelle auf Seed 34123 erhielten
während des visuellen Trainings keinen Prä-Impuls. Ihre Gewichtsänderungen
stammten aus der Entwicklungs-Eligibility. Dieser Versuch prüft, ob der
installierte lokale Regler während des Trainings zusätzliche Quellen
rekrutiert und auf den betreffenden Pfaden neue Paarereignisse ermöglicht.
[Ausgangsbefund](CONTINGENCY_ELIGIBILITY_RESULTS_20261123.md)

## Training auf identischer entwickelter Topologie

Der Versuch verwendet die vier Seeds mit archiviertem Folgetraining
aus dem Kapazitätsversuch sowie deren Eligibility-Kontrollen. Pro Seed
werden beide gegensätzlichen Zuordnungen auf Kopien desselben entwickelten
Checkpoints trainiert. Das Training verwendet acht Wiederholungen und
50 ms Abstand zwischen den Reizanfängen. Jedes Training dauert vier
Sekunden. Die Sensorik erhält dieselbe zeitliche Folge von Bildern.
Zielgruppen, Kanten-IDs und Abrufkriterien werden nur in der Diagnose
verwendet.

| Arm | Homöostase im Training | Eligibility am Start |
| --- | --- | --- |
| `off_keep` | aus | erhalten |
| `off_clear` | aus | auf null gesetzt |
| `core_keep` | nur Core | erhalten |
| `core_clear` | nur Core | auf null gesetzt |

Gewichte, synapsenlokale Restbeträge und Paarhistorie starten in allen
Armen gleich. Die Entwicklung bleibt nach dem Checkpoint eingefroren.
Die Core-Bedingung verwendet den bestehenden lokalen Regler mit seinem
nativen 100-ms-Intervall. Für die beiden ausgeschalteten Arme müssen
die beobachteten Reizfenster, Spikes und Endgewichte exakt mit den
archivierten Kontrollen übereinstimmen. In den eingeschalteten Armen
muss sich die lokale Regelung durch Änderungen ausschließlich innerhalb
des Core nachweisen lassen.

## Endpunkte

Die primäre Gruppe sind die acht bereits archivierten terminalen
Kante–Zuordnung-Paare, deren Quellzellen im nativen Training still
waren. Die übrigen terminalen Kanten werden ebenfalls erfasst.
Alle Kanten stammen aus den gemeinsam wirksamen Ablationen der
diagnostischen Abrufreferenz. Die Referenzgewichte werden für dieses
Training und seinen Abruf nicht verwendet.

Berichtet werden Prä-Emissionen und Ankünfte sowie tatsächlich
aufgezeichnete Eligibility-Änderungen in den Prä- und Post-Hooks.
Eine Ankunft zählt zur Trainingsphase, wenn ihr rekonstruierter
Sendezeitpunkt ab Trainingsbeginn liegt. Zu einem neuen Paar
zählt eine Tag-Änderung nur, wenn der zugehörige Prä-Impuls aus
der Trainingsphase stammt und auch der Post-Spike in der Phase
liegt. Beide müssen innerhalb des nativen Paarfensters liegen.
Die Prä-Ankunft wird bei gleicher Zeit vor dem Post-Hook verarbeitet.
Die Auswertung zählt die Gesamtaktivität separat. Ein Anstieg
neuronaler Aktivität allein belegt keinen neuen Lernpfad.

Der vorab festgelegte kausale Kontrast für die Primärgruppe ist
`core_clear − off_clear` für neue Ankünfte und Paaränderungen.
Der Vergleich `core_keep − off_keep` prüft die Wirkung mit der
Entwicklungsspur. Die Differenz dieser beiden Wirkungen wird als
Interaktion berichtet. Angeforderte Gewichtsbeiträge, veröffentlichte
Endgewichte und die aufgezeichnete Änderung der intrinsischen Ströme
auf den Quellzellen sind zusätzliche mechanistische Endpunkte.

Sichtbare Core→Core-Endgewichte werden in identische eingefrorene
Kopien des entwickelten Ausgangszustands übertragen. Der Abruf von
beiden Original-Cues und Schwarz erfolgt wie im Kapazitätsversuch.
Er prüft selektiven Abruf durch Gewichte allein. Ein Erfolg erfordert
für beide Cues unter beiden Zuordnungen mehr Spikes in der passenden
Gruppe als in der falschen Gruppe, beim Reset und unter der
gegensätzlichen Zuordnung. Schwarz- und Ruheaktivität dürfen nicht
steigen. Der Versuch erfasst keinen Abruf durch erhaltene
Reglerzustände, keine neuen Bildmuster und keine unabhängige
Bestätigung auf neuen Seeds.

## Ausführung

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
capacity=research/results/visual_contingency_capacity_20261123
eligibility=research/results/visual_contingency_eligibility_20261123
root=research/results/visual_contingency_core_homeostasis_20261123
mkdir -p "$root/input/capacity" "$root/input/eligibility" "$root/raw"
for seed in 34123 34127 34141 34157; do
  name="v00000-s${seed}-i000.json"
  cp "$capacity/records/$name" "$root/input/capacity/$name"
  cp "$eligibility/raw/$name" "$root/input/eligibility/$name"
  research/target/release/nerva-research contingency-homeostasis \
    "$root/input/capacity/$name" "$root/input/eligibility/$name" \
    > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
python3 research/contingency_homeostasis_analysis.py "$root"
```

Pro Record wird der vollständige native Ablauf wiederholt. Der Hauptlauf
bindet beide Quellstände, alle Eingaben, Runner und Auswertung vorab
per SHA-256.
