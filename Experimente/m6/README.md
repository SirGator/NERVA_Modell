# M6 ausführen und auswerten

M6 kombiniert Einzel-/Dreifach-Aktionskopien mit visueller Homöostase an/aus.
Die neuronale Ausführung liegt in `NERVA_Modell/Modell/`; Python plant, startet und prüft
gespeicherte Ergebnisse. Es werden keine neuen neuronalen Lernregeln eingeführt.
[Gemessene Ergebnisse und Grenzen](../M6_EXPERIMENT.md).

## Aufrufe vom Projektverzeichnis

```bash
# Vorabplan speichern, noch keine neuronalen Läufe:
python3 NERVA_Modell/Experimente/m6/run.py plan --stage pilot --output Messungen/Experimente/mein-m6-pilot

# Drei Pilot-Seeds, acht Bedingungen; benötigt keine QEMU-VM:
python3 NERVA_Modell/Experimente/m6/run.py run --stage pilot --jobs 4 \
  --output Messungen/Experimente/mein-m6-pilot \
  --command './NERVA_Modell/Modell/run-demo.sh m6 --config {config} --output {output}'

# Gleiche acht Bedingungen auf zehn neuen Seeds 300–309:
python3 NERVA_Modell/Experimente/m6/run.py run --stage confirm --jobs 4 \
  --output Messungen/Experimente/mein-m6-confirm \
  --command './NERVA_Modell/Modell/run-demo.sh m6 --config {config} --output {output}'

# Gespeicherte vollständige Daten unabhängig neu prüfen und auswerten:
python3 NERVA_Modell/Experimente/m6/run.py report --output Messungen/Experimente/m6-confirm

# Einen vorhandenen Fall auf ein neues Ergebnisziel ausführen:
./NERVA_Modell/Modell/run-demo.sh m6 \
  --config Messungen/Experimente/m6-confirm/configs/diag_D/seed-300.json \
  --output /tmp/mein-m6-seed300.json
```

Standard: 256 Trainingstrials, 128 Kalibriertrials, 128 Trials pro laufender Probe.
`--train-trials`, `--calibration-trials` und `--test-trials` dienen getrennten
kleinen Softwareprüfungen; geänderte Zähler werden im Manifest gespeichert und
sind keine Wiederholung des archivierten Bestätigungsplans. Ein vorhandenes
Manifest muss identisch sein. Gültige Einzelergebnisse können weiterverwendet
werden; `report` benötigt alle Ergebnisse und sämtliche Standard-CSV-Dateien.
Die Rust-CLI überschreibt keine vorhandenen Ergebnis- oder Begleitdateien.
Bei einem abgebrochenen Einzellauf ein neues Ziel verwenden oder ausschließlich
dessen unvollständige Dateien entfernen.

## Konfiguration und Bedingungen

Das strenge JSON-Objekt hat die Form:

```json
{
  "protocol": "NERVA-M6-v1",
  "variant": "D",
  "base": {
    "protocol": "NERVA-M4-v1",
    "case_id": "diag_D",
    "seed": 300,
    "arms": ["before", "aged_frozen", "paired", "scrambled"],
    "train_trials": 256,
    "test_trials": 128,
    "calibration_trials": 128,
    "params": {
      "sensor": "effect_2ch",
      "resolution": 0,
      "encoding": "deterministic",
      "processing": 32,
      "action_effect_delay_ms": 50,
      "motor_feedback": true,
      "stdp": true,
      "homeostasis": true,
      "development": false,
      "initial_drive": 0,
      "pruning_min_us": 5000000,
      "normalization": "equal_input_current"
    }
  }
}
```

Die Basis verwendet den vorhandenen strengen M4-Vertrag. Akzeptiert werden nur
`effect_2ch`/32 oder `pixels`/Auflösung 16/128 Core-Neuronen, deterministische
Kodierung, 50 ms Wirkungsabstand, Feedback/STDP an, Entwicklung aus und Bedarf null.
Kopiegewicht 0,9 und Core-Tau 20 ms sind fest.

| Variante | Kopien | Sensor-Homöostase | Core-Homöostase im Hauptplan |
|---|---:|---|---|
| A | 1 | an | an |
| B | 3 | an | an |
| C | 1 | aus | an |
| D | 3 | aus | an |

Motor- und Kopiezellen sind immer von Homöostase ausgeschlossen. Für die separate
explorative Core-Kontrolle akzeptieren ausschließlich C/D zusätzlich
`base.params.homeostasis=false`. Das schaltet auch die Core-Homöostase aus und
wird in den effektiven Protokolldaten ausgewiesen. Der Hauptplan bleibt unverändert.

## Daten und Prüfschritte

Zu `RESULT.json` gehören:

| Datei | Inhalt |
|---|---|
| `.details.csv` | Jede Aktion, reale Bewegungs-/Bildgrenze, Vorhersagen und Core-Aktivität vor der Wirkung |
| `.audit.csv` | Neuronale Phasen, Gewichts-/Homöostase-/Strukturänderungen |
| `.sensor.csv` | Erzeugte Sensorimpulse, echte Sensor-Spikes, Ströme und Populationsupdates |
| `.copies.csv` | Tatsächliche Kopienemissionen und jede synaptische Ankunftszeit |
| `.branches.csv` | Checkpoint, Replaygleichheit und unveränderte Elternzeit für Kalibrier-/Testzweige |
| `.events.csv` | Optional mit `--raw-events`: vollständige passive Ereignisaufzeichnung |

Der JSON-Bericht enthält beide Belegungen und vier Kontrollarme mit den Proben
`checkpoint_probe`, `probe`, `retained_probe` und `interference_probe`. Die
2-Proben-Checkpointmessung teilt ihren Ausgangszustand; die 128 Online-Proben
laufen nacheinander. `early_count` wertet zusätzlich die ersten 16 aus.
`protocol_details` nennt die tatsächlich verwendeten Parameter und Zeitgrenzen.

`report` prüft Konfiguration, Fall/Seed, Zeitkausalität, Klassen-/Aktionsbalance,
unabhängige entkoppelte Zuordnung, gleiche ausgeführte Tasten bei umgekehrter
Belegung, Freeze und Populationseinschränkungen. Vorhersagegenauigkeit,
Belegungswechsel und beidseitig richtige Antworten werden aus Einzelvorhersagen
nachgerechnet und mit den CSVs verglichen. Sensorzähler müssen mit dem JSON
übereinstimmen. Alle Kopien müssen tatsächlich vor der Wirkung ankommen.
Replayzweige dürfen die Elternzeit nicht verändern.

Über A–D werden zusätzlich Anfangsnetze, Normierung, erzeugte visuellen Impulse
und identische eingefrorene Sensor-An-/Aus-Kontrollen verglichen. Die Auswertung
schreibt `summary.md`, `summary.csv`, `prediction-contrasts.csv`, `diagnostics.json`
und `signal-diagnostics.json`; diese lassen sich aus den Messdaten neu erzeugen.
Erfolg wird pro Prüfzeitpunkt getrennt bewertet. Details zu den strengen
Kontroll-/Umkehrkriterien stehen im [Bericht](../M6_EXPERIMENT.md).

## Separate mechanistische Folgeprüfungen

Diese Aufrufe wiederholen die klar getrennten Zusatzprüfungen, nicht den Hauptplan:

```bash
# Einzelzell-Gegenprüfung nach Training; Pilotkonfiguration D:
./NERVA_Modell/Modell/run-demo.sh m6 --intrinsic-audit \
  --config Messungen/Experimente/m6-pilot/configs/diag_D/seed-000.json \
  --output /tmp/mein-m6-zelltest.json

# Variante D, nur Core-Homöostase an/aus, neue Seeds 400–409:
python3 NERVA_Modell/Experimente/m6/core_control.py plan --output Messungen/Experimente/meine-core-kontrolle
python3 NERVA_Modell/Experimente/m6/core_control.py run --jobs 4 --output Messungen/Experimente/meine-core-kontrolle
python3 NERVA_Modell/Experimente/m6/core_control.py report --output Messungen/Experimente/m6-core-control
```

Der Einzelzellmodus braucht D, 2-Kanal/32 und mindestens 256 Trainingstrials.
Er zeichnet die realen Eingänge intern auf, reproduziert die unveränderte
Zellantwort und setzt auf einer getrennten Zellkopie nur den konstanten
intrinsischen Strom null. Sein eigenes JSON-Protokoll heißt
`NERVA-M6-intrinsic-audit-v1`; es erzeugt keine M6-Matrix-CSVs und keine
Vorhersageleistung. Der archivierte Plan verwendet Core-Homöostase an.

`core_control.py` speichert einen eigenen unveränderlichen Plan und verwendet
anschließend denselben M6-Modellport, dieselben vier Arme, beide Belegungen und
dieselben Erfolgskriterien. Sein An-/Aus-Vergleich bleibt explorativ, weil er
erst nach der ursprünglichen M6-Reihe motiviert wurde.

## Zuständigkeiten und Reproduzierbarkeit

`NERVA_Modell/Modell/src/m6.rs` verbindet das vorhandene Modell mit den kontrollierten Frames,
führt Training/Freeze aus und erzeugt exakte Replayzweige. Der neuronale Actor
kennt ausschließlich aktuelles Bild, eigene ausgeführte Taste und Dauer.
Die Testwelt und externen Wirkungsklassen liegen außerhalb dieser Eingangsgrenze.
`NERVA_Modell/Modell/src/m6/mechanisms.rs` enthält die Zellkopien-Diagnose. Python verändert
keine neuronalen Zustände. NERVA und Adapter bleiben unabhängig.

`Model::same_replay_state` vergleicht zusätzlich zum Netz den Runtime- und
Modellzustand. Die generische NERVA-Prüfung schließt undurchsichtige Compute- und
Utility-Objekte aus; ihre Aufrufer müssen identische zustandslose Implementierungen
garantieren. M6 verwendet dafür den festen CPU-Pfad und dieselben zustandslosen
Utility-Regeln. Passive Beobachtungs-/Profilingprotokolle sind ausgenommen.
Einige Modellteile werden diagnostisch über ihre Debug-Darstellung verglichen;
M6 verwendet den geordneten deterministischen Generator. Daraus folgt keine
allgemeine portable Serialisierungs- oder CPU/GPU-Gleichheitsgarantie.

```bash
python3 -m unittest discover -s NERVA_Modell/Experimente/m6/tests -v
cargo test --offline --release --manifest-path NERVA_Modell/Modell/Cargo.toml --test m6
```

Die Tests prüfen insbesondere ausstehende Ereignisse bei gleichem sichtbarem
Netz, strenge Konfigurationen und bytegleiche serielle/parallele JSON-/CSV-Ausgabe.
Die historische M4-Ausgabe bleibt im vollständigen Wiederholungslauf bytegleich.
