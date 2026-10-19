# M8: Funktionale Weiterleitung mit vorhandenen Mechanismen

M8 variiert sechs vorhandene Modellparameter einzeln und vergleicht zusätzliche
Core-Aktivität mit tatsächlicher Vorhersageleistung. Es werden keine neue Lernregel
und keine vorgegebenen A→links-/A→rechts-Pfade eingeführt.

## Start im Projektverzeichnis

```bash
# Plan speichern, noch keine neuronalen Läufe:
python3 NERVA_Modell/Experimente/m8/run.py plan --stage pilot --output Messungen/Experimente/mein-m8-pilot

# Pilot: alle 13 Bedingungen, Seeds 500–502:
python3 NERVA_Modell/Experimente/m8/run.py run --stage pilot --jobs 4 --output Messungen/Experimente/mein-m8-pilot

# Unveränderte Bedingungen auf zehn weiteren Seeds 510–519:
python3 NERVA_Modell/Experimente/m8/run.py run --stage confirm --jobs 4 --output Messungen/Experimente/mein-m8-confirm

# Archivierte Daten unabhängig neu prüfen:
python3 NERVA_Modell/Experimente/m8/run.py report --output Messungen/Experimente/m8-confirm

# Einzelfall; neues Ausgabeziel verwenden:
./NERVA_Modell/Experimente/run.sh m8 \
  --config Messungen/Experimente/m8-confirm/configs/a_weight2/seed-510.json \
  --output /tmp/mein-m8-lauf.json
```

Die Rust-CLI braucht keine VM. Standard sind 256 Trainingstrials, 128 visuelle
Kalibriertrials und 128 Trials pro Online-Probe. Kleine Zähler sind mit
`--train-trials`, `--test-trials`, `--calibration-trials` für separate Softwareprüfungen
möglich; der gespeicherte wissenschaftliche Plan verwendet die Standardzähler.
Vorhandene Manifeste müssen identisch bleiben. Vollständig validierte Ergebnisse
werden beim Fortsetzen weiterverwendet. Einzelergebnisse und ihre Begleitdateien
werden nicht überschrieben; bei Abbruch ein neues Ziel verwenden oder nur dessen
unvollständige Dateien entfernen.

## Vorab festgelegte Bedingungen

Eine Referenz und zwei Varianten je Faktor ergeben 13 Bedingungen:

| Faktor | Referenz | Zusätzliche Werte | Reichweite |
|---|---|---|---|
| A: Rekurrentes Gewicht | Faktor 1 | 2 / 4 | Alle Core→Core-Gewichte, Erregung und Hemmung |
| B: Erregbarkeit | Schwelle 1 | 0,8 / 0,6 | Nur Core-Neuronen |
| C: STDP-Zeitskala | 20 ms | 50 / 100 ms | Bestehende Plus- und Minus-Zeitkonstanten gemeinsam |
| D: Verzögerung | Geseedet 0,5–2 ms | Einheitlich 0,5 / 5 ms | Nur Core→Core |
| E: Verbindungszahl | Höchstens 4 ausgehend | Höchstens 8 / 12 | Weitere zufällige lokale Core-Kanten |
| F: Sensorische Gewichte | Faktor 1 | 0,5 / 1,5 | Nur visuelle Sensor→Core-Gewichte |

Fest bleiben 32 Core-Zellen, der pixelbasierte 2-Kanal-Diagnosesensor,
deterministische Kodierung, drei Kopien derselben ausgeführten Taste, Kopiegewicht
0,9, Core-Tau 20 ms und 50 ms Aktions-/Wirkungsabstand. Homöostase und Entwicklung
sind aus. STDP-Amplituden 0,0001/0,00012, Gewichtsgrenzen 0–2 und Paarfenster
100 ms bleiben erhalten. Bei C wird auch die Depression zeitlich verbreitert.

Der **sensorische Ratenverstärker wird aus dem Referenznetz berechnet und über
alle Bedingungen desselben Seeds festgehalten**. F verändert daher tatsächlich
den initialen Sensorstrom statt durch Neunormierung aufgehoben zu werden. Das
erwartete Populationsmittel beträgt ungefähr 10 in der Referenz, 5 bei F=0,5 und
15 bei F=1,5. Erzeugte sensorische Impulse müssen trotzdem über alle Fälle identisch
bleiben; Unterschiede entstehen durch synaptische Wirkung und neuronale Dynamik.

E fügt erst nach Aufbau des bisherigen Grundgerüsts zusätzliche Kanten aus einer
eigenen geseedeten Zufallsfolge hinzu. Bestehende Neuronen, Positionen, Polaritäten,
Originalkanten sowie Sensor-, Motor- und Kopieanschlüsse bleiben erhalten. Neue
Kanten haben keine Selbstverbindung, keinen doppelten Endpunkt und höchstens
Abstand 0,25. Die vorhandenen Nachbarn begrenzen den tatsächlich erreichbaren
Fan-out. Bei D werden die bisherigen Zufallsziehungen weiterhin verbraucht,
damit andere Anschlüsse nicht mitverändert werden.

## Konfiguration

Beispiel eines vollständigen M8-JSONs:

```json
{
  "protocol": "NERVA-M8-v1",
  "case_id": "a_weight2",
  "seed": 510,
  "train_trials": 256,
  "test_trials": 128,
  "calibration_trials": 128,
  "condition": { "parameter": "core_weight_scale", "value": 2 }
}
```

Erlaubt sind ausschließlich die sechs genannten Parameter mit ihren zwei
festgelegten Werten und `reference: 1`. Unbekannte Felder, kombinierte Eingriffe
und andere Werte werden abgewiesen. Die vier Arme `before`, `aged_frozen`, `paired`
und `scrambled` sowie beide Belegungen sind fest. Der neuronale Actor erhält
ausschließlich aktuelle Frames und eigene ausgeführte Tasten, keine Labels,
Weltpositionen, Zukunftsbilder oder Auswerterwerte.

## Getrennte Proben und Kriterien

Die Kalibrierung erfolgt wie in M6 ausschließlich auf einem exakten Replayzweig.
Die Elterninstanz bleibt am eingefrorenen Trainingscheckpoint. Die zwei primären
Prüfzeitpunkte sind:

- `checkpoint_probe`: A und D aus je einem eigenen identischen Trainingscheckpoint.
- `rest_checkpoint_probe`: A und D aus je einem identischen Checkpoint nach
  unabhängigen 2 Sekunden neutralem Bild. Davor läuft keine Online-Probe.

Jede dieser Messungen enthält zwei Beispiele je Belegung/Seed. Die erste Prognose
benötigt wie bisher 50 ms Startbild und 50 ms Aktionsintegration; noch kürzere
Zustände sind damit nicht abgedeckt. Pro Prüfzeitpunkt ergänzt ein weiterer
identischer Replayzweig **ohne Aktionskopie** die Messung spontaner Aktivität.
Die gleiche No-copy-Antwort dient beiden Tasten als Hintergrundkontrolle.

Sekundär bleiben die vollständige 128-Trial-Online-Probe ab dem Trainingsende,
ihre ersten 16 Trials, sequenzielle Ruhe-/Interferenzproben und der Zeitbin-Auswerter
erhalten. Die sequenzielle Ruheprobe folgt auf die Online-Probe und ist ausdrücklich
von der unabhängigen primären Ruheprobe verschieden. Gewichte bleiben beim Prüfen
eingefroren; das belegt keine Konsolidierung bei weiterlaufender Plastizität.

Ein Lernnachweis verlangt mindestens zehn Seeds, visuelle Auslesbarkeit mindestens
80 % in beiden Belegungen sowie einen positiven Spikezähl-Vorhersagevorteil
gegenüber Zufall und allen drei Kontrollarmen in beiden Belegungen. Auch
Belegungswechsel und beidseitig richtige Antworten müssen allen Kontrollen
überlegen sein. Die M6-Bootstrapkriterien werden zusätzlich durch einseitige exakte
gepaarte Vorzeichentests abgesichert: jeder erforderliche Kontrast muss
`p ≤ 0,05/(13×2)` erfüllen. Alle Kontraste sind gemeinsam erforderlich; über die
13 Bedingungen und zwei primären Endpunkte wird Bonferroni verwendet.

Gleichstände werden im Vorzeichentest ausgelassen. Mit nur zehn Seeds ist das
bewusst streng und begrenzt die Teststärke. Bootstrapintervalle beschreiben
Mittelwertunterschiede; die exakten Tests prüfen gerichtete Vorteile der gepaarten
Differenzen. Zusätzliche Spikes, autonome Aktivität, visuelle Auslesbarkeit oder
eine stärkere topologische Gewichtszuordnung allein gelten nicht als Lernnachweis.

## Dateien und Zuständigkeiten

`NERVA_Modell/Experimente/src/m8.rs` definiert die strenge Konfiguration und setzt die
Modellparameter zusammen. `src/m8_main.rs` veröffentlicht ein Ergebnis erst nach
erfolgreichem Abschluss. `NERVA_Modell/Experimente/m8/run.py` enthält Plan, Prozesssteuerung,
unabhängigen Audit und Statistik. `NERVA_Modell/Modell/` stellt sechs Parameter bestehender
Dynamik sowie optionale Messungen im bereits vorhandenen Replayprotokoll bereit.
NERVA und der QEMU-Adapter erhalten keine neue neuronale Regel oder Abhängigkeit.
Die Live-Standardwerte ändern sich nicht.

Zusätzlich zu JSON, `.details.csv`, `.audit.csv`, `.sensor.csv`, `.copies.csv` und
`.branches.csv` entsteht `.propagation.csv`. Sie enthält jede Aktionsprobe mit
den drei tatsächlichen Kopie-Zielzellen, sämtlichen Core-Spikezeiten vor dem
Wirkungsbild und Spikes außerhalb dieser Zielzellen. No-copy-Spikezeiten stehen
gesondert im JSON. 56 Zweige je Fall werden einschließlich der No-copy-Zweige
mit Checkpoint, Endzeit, exakter Replaygleichheit und unveränderter Elternzeit
protokolliert. `--raw-events` ist optional; Standardläufe speichern die für
Vorhersage und Weiterleitung benötigten Ereignisse in der Propagationsdatei.

Die Auswertung erzeugt `summary.md`, `summary.csv`, `prediction-contrasts.csv`,
`propagation-summary.csv`, `weight-summary.csv` und `diagnostics.json`. Sie
vergleicht zusätzlich über alle 13 Fälle die erzeugten Sensorimpulse, ursprünglichen
Kanten und die Zellgeometrie. Angaben zu Zusatzspikes werden den No-copy- und
eingefrorenen Kontrollen gegenübergestellt.

```bash
cargo test --offline --release --manifest-path NERVA_Modell/Modell/Cargo.toml --test experimental_parameters
cargo test --offline --release --manifest-path NERVA_Modell/Experimente/Cargo.toml --test m8
python3 -W error::ResourceWarning -m unittest discover -s NERVA_Modell/Experimente/m8/tests -v
```

Die Softwaretests prüfen unveränderte Originalverschaltung bei höherer Dichte,
räumliche Grenzen, gezielte Parameterreichweite, strenge Eingaben, bytegleiche
parallele Ergebnisse, richtige No-copy-/Ruhezeiten und absichtlich verfälschte
Spikezahlen. Historische vollständige M4-/M6-Wiederholungen prüfen darüber hinaus,
dass die neuen optionalen Messungen den ursprünglichen Datenvertrag erhalten.
