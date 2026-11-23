# Übernommene Lernspuren auf den rekurrenten Abrufpfaden

Auf Seed 34123 konnte eine künstlich verstärkte Relaisreferenz beide Hinweise
unter beiden Zuordnungen selektiv abrufen. Die Quellen von acht der elf
gemeinsam zurückgesetzten terminalen Kanten feuerten in der jeweiligen nativen
Trainingsaufzeichnung nicht, obwohl sich ihre Gewichte änderten. Dieser Versuch
trennt geerbte Eligibility, frühere Paarzeitstempel und neu ankommende Impulse.
[Ausgangsbefund](CONTINGENCY_CAPACITY_RESULTS_20261123.md)

## Beobachtungen und Eingriffe

Verwendet werden alle vier Kapazitäts-Records mit nativem Folgetraining
auf Seeds 34123, 34127, 34141 und 34157. Der fünfte Seed hatte keinen
Referenzteiltreffer und deshalb kein archiviertes Folgetraining.
Pro Record werden beide Zuordnungen mit vollständiger Lernbeobachtung
rekonstruiert. Das native Training muss sämtliche archivierten Reizfenster,
Root- und Core-Spikes und veröffentlichten Endgewichte exakt reproduzieren.
Die gezielte Auswertung umfasst alle terminalen Kanten, deren Gewichte in
den archivierten Ablationen der jeweiligen Zuordnung zurückgesetzt wurden.
Beide Zuordnungen werden auch bei einer leeren Kantenauswahl ausgeführt.

Auf jeder aufgezeichneten Historie werden vier Kopien derselben lokalen
Lernregel offline integriert:

| Arm | Entwicklungszustand unmittelbar vor dem Training |
| --- | --- |
| `original` | Alle Traces und Zeitstempel erhalten |
| `clear_eligibility` | Nur die vorzeichenbehaftete Drei-Faktor-Eligibility jeder Kante auf null gesetzt |
| `clear_pairing` | Nur die Postsynapsen-Traces und Paarzeitstempel der Lernregel geleert |
| `clear_both` | Eligibility und regelinterne Paarhistorie geleert |

Bei jeder Kopie bleiben Gewichte, Restbeträge der Gewichtspublikation,
synapsenlokaler Prä-Trace und Zeitanker der Eligibility erhalten. Im
Offline-Vergleich verwenden alle Arme dieselben prä- und postsynaptischen
Ereignisse, chemischen Proben und Integrationszeiten. Die native Kopie
muss Gewichte, Lernspuren, Zeitanker, Regelzustand und Restbeträge
des echten Trainings exakt treffen.

Für eine beobachtete Ankunft wird der Sendezeitpunkt aus Ankunft minus
der nativen Verzögerung rekonstruiert. Sendezeiten vor dem Beginn des
Trainings kennzeichnen bereits reisende Impulse aus dem Checkpoint.
Bei Sendezeiten ab Trainingsbeginn muss der zugehörige Core-Spike in
der archivierten Historie vorliegen.

## Vorab festgelegte Endpunkte

Für die terminalen Kanten werden angeforderte positive und negative
Gewichtsbeiträge und die veröffentlichte Gewichtsänderung je Arm getrennt
berichtet. Der Eligibility-Effekt ist der Unterschied zwischen nativem
und eligibility-freiem Replay. Der Effekt der Paarhistorie ist entsprechend
der Unterschied zum Arm ohne Paarhistorie. Die Interaktion ist
`original − clear_eligibility − clear_pairing + clear_both`.
Die primäre Teilgruppe umfasst terminale Kanten, deren Quelle im jeweils
archivierten Training keinen Spike emittierte. Alle übrigen terminalen
Kanten werden separat ausgewertet. Bei null Ankünften wird der reine
Zerfall der geerbten Eligibility im Arm ohne Paarhistorie aus den
Zeitankern und der aufgezeichneten gehaltenen Chemie unabhängig geprüft.

Zusätzlich beginnt ein **natives** Training mit entfernten Eligibility-Werten
auf sonst identischem eingefrorenem Checkpoint. Damit bleiben Änderungen
späterer Spike- und Chemiehistorien möglich. Stimmen die beobachteten
Lernereignisse und chemischen Zustände weiterhin exakt mit dem
ursprünglichen Training überein, müssen die Endgewichte dem entsprechenden
Offline-Arm exakt entsprechen. Beide Interventionsresultate werden als
Core→Core-Gewichte im eingefrorenen Abruf geprüft. Bei identischem
Core-Gewichtsvektor wird dieselbe Probe verwendet. Zielgruppen, Reset,
gegensätzliche Zuordnung und Schwarzbild-Kontrolle folgen dem
Kapazitätsversuch. Der Vergleich betrifft die ursprünglichen Cues.

Ein Herkunftsnachweis für Gewichtsänderungen ist kein Abrufnachweis.
Die Studie bewertet bekannte Pfade aus dem Kapazitätsversuch, deren spätere
Relevanz durch künstliche Gewichte bestimmt wurde. Sie prüft die Bedingungen
für späteres Lernen an diesen Pfaden und bestätigt keine Generalisierung.

## Ausführung

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
source_run=research/results/visual_contingency_capacity_20261123
new_run=research/results/visual_contingency_eligibility_20261123
mkdir -p "$new_run/input" "$new_run/raw"
for seed in 34123 34127 34141 34157; do
  name="v00000-s${seed}-i000.json"
  cp "$source_run/records/$name" "$new_run/input/$name"
  research/target/release/nerva-research contingency-eligibility "$new_run/input/$name" \
    > "$new_run/raw/$name.partial" || exit
  mv "$new_run/raw/$name.partial" "$new_run/raw/$name"
done
python3 research/contingency_eligibility_analysis.py "$new_run"
```

Die Hauptmessung bindet beide Quellstände, Binary, Eingabedateien und
Auswertung vorab per SHA-256. Der native Befehl gibt einen Record erst
nach exakter Übereinstimmung der vollständigen Wiederholung aus.
