# M9 ausführen

[Vorab festgelegter Versuchsplan](../M9_EXPERIMENT.md): 2×2 aus Core-Gewichten
×1/×4 und STDP-Zeitkonstanten 20/100 ms. Pilot und Bestätigung auf 20 neuen
Seeds sind abgeschlossen: **kein funktionaler Lernnachweis**.
[Wissenschaftlicher Bericht](../../../Messungen/M9_EXPERIMENT.md).
Der Runner benötigt keine VM.

Vom Projektverzeichnis `DSVLM`:

```bash
# Nur Pläne speichern; keine neuronalen Läufe:
python3 NERVA_Modell/Experimente/m9/run.py plan --stage pilot --output Messungen/Experimente/m9-v2-pilot
python3 NERVA_Modell/Experimente/m9/run.py plan --stage confirm --output Messungen/Experimente/m9-v2-confirm

# Separater technischer Kurzlauf, vier Bedingungen, Seed 692, 8/8/8 Trials:
python3 NERVA_Modell/Experimente/m9/run.py run --stage smoke --jobs 4 --output /tmp/mein-m9-smoke

# Wissenschaftliche Reihen, feste Trialzahlen, alle Bedingungen:
python3 NERVA_Modell/Experimente/m9/run.py run --stage pilot --jobs 4 --output Messungen/Experimente/m9-v2-pilot
python3 NERVA_Modell/Experimente/m9/run.py run --stage confirm --jobs 4 --output Messungen/Experimente/m9-v2-confirm

# Rohdaten erneut unabhängig prüfen und Bericht erzeugen:
python3 NERVA_Modell/Experimente/m9/run.py report --output Messungen/Experimente/m9-v2-confirm
```

Pilot: Seeds 600–602; Bestätigung: **20 neue Seeds 610–629**.
Pilot und Bestätigung verwenden 256/128/128 Trials. Abweichende Zähler akzeptiert
der Runner ausschließlich mit `--stage smoke`. Bestehende Manifeste müssen exakt
passen; vorhandene Ergebnisse werden vor Wiederverwendung vollständig geprüft.
Unvollständige Begleitdateien werden nicht überschrieben: ein neues Ziel wählen.

## Einzelkonfiguration

```json
{
  "protocol": "NERVA-M9-v2",
  "case_id": "weight4_tau100",
  "seed": 610,
  "train_trials": 256,
  "test_trials": 128,
  "calibration_trials": 128,
  "condition": { "core_weight_scale": 4, "stdp_tau_us": 100000 }
}
```

```bash
./NERVA_Modell/Experimente/run.sh m9 --config /pfad/config.json --output /tmp/mein-m9.json
```

Nur die vier festgelegten Kombinationen sind erlaubt. Andere Felder, weitere
Parameter und andere Werte werden abgewiesen. Die CLI erlaubt kleinere gültige
Trialzahlen für technische Tests; die wissenschaftlichen Manifeste bleiben fest.
Die Bedingungsnamen im Python-Runner sind `reference`, `weight4`, `tau100` und
`weight4_tau100`.

## Dateien und Prüfung

Ein Ergebnis umfasst JSON sowie `.details.csv`, `.audit.csv`, `.sensor.csv`,
`.copies.csv`, `.branches.csv` und `.propagation.csv`; `--raw-events` ist optional.
Replayzeiten, eingefrorene Gewichte, No-copy-Proben und tatsächlich gemessene
Spikes werden mit dem bestehenden M8/M6-Audit geprüft. Der Matrixaudit prüft
identische Anfangskanten, Zellgeometrie, erzeugte visuelle Spikezahlen und
Referenznormierung über die Bedingungen.

Der Bericht erzeugt `summary.md`, `summary.csv`, `prediction-contrasts.csv`,
`propagation-summary.csv`, `weight-summary.csv`, `factorial-contrasts.csv` und
`diagnostics.json`. Zusätzlich enthält `causal-contrasts.csv` die Rücksetzungstests.
Das JSON speichert 96 weitere unabhängige Zweige mit Decoderparametern,
Gewichtsänderungen und Spikezeiten. Ein vollständiger M9-v2-Nachweis verlangt
Verhaltensvorteile **und** schlechtere Vorhersagen nach Zurücksetzen sämtlicher
plastischer Gewichte, jeweils in beiden Belegungen und bei mindestens 20 Seeds.
Primäre Familienkorrektur: 4×2, also α=0,00625. Faktorielle
Kontraste sind ausdrücklich deskriptiv. `weight-summary.csv` enthält zusätzlich
`visual_retained_fraction`.

```bash
cargo test --offline --release --manifest-path NERVA_Modell/Experimente/Cargo.toml --test m8 --test m9
python3 -W error::ResourceWarning -m unittest discover -s NERVA_Modell/Experimente/m8/tests -v
python3 -W error::ResourceWarning -m unittest discover -s NERVA_Modell/Experimente/m9/tests -v
```

## Frühere v1-Dateien

Die vor dieser Überarbeitung gespeicherten Manifeste `m9-pilot`, `m9-confirm`
und der technische Lauf `m9-smoke` bleiben unverändert. Sie verwenden v1 mit
zehn geplanten Bestätigungsseeds und ohne Gewichtsinterventionen. Sie sind keine
M9-v2-Ergebnisse. Der Rust-Aufruf unterstützt v1 weiterhin; der frühere Runner
steht als `v1.py` zur Verfügung:

```bash
python3 NERVA_Modell/Experimente/m9/v1.py report --output Messungen/Experimente/m9-smoke
```
