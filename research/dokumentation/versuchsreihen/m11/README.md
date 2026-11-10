> Historische Dokumentation. Die eigenständigen Experimentprogramme wurden entfernt; aufgeführte alte Startbefehle sind nicht mehr verfügbar. Aktuelle Ausführung: [Research-Harness](../../../README.md).

# M11 ausführen

Stand 28.10.2026: Konkurrenzmechanismus und Runner sind implementiert.
Technischer Kurzlauf auf Seed 792 und Pilot 700–702 sind abgeschlossen und
auditiert. Die Bestätigung auf 710–729 steht noch aus.
[Pilotergebnisse](../../../../../Messungen/Berichte/M11_EXPERIMENT.md).
[Methodik](../M11_EXPERIMENT.md) · [Maschinenlesbare Definition](protocol.json).
Das historische Statusfeld der vorab gespeicherten Definition beschreibt ihren
Erstellungszeitpunkt; der aktuelle Ausführungsstand steht in der Methodik.

M11 vergleicht die M9-Kombination (Core-Gewichte ×4, STDP-Tau 100 ms) mit genau
zusätzlich eingeschalteter lokaler Konkurrenz. Die normalen Modellwerte ändern
sich nicht. Es wird keine VM benötigt.

Vom Projektverzeichnis `DSVLM`:

```bash
# Nur Manifest und alle Konfigurationen speichern:
python3 NERVA_Modell/research/experiments/m11/run.py plan --stage smoke --output /tmp/mein-m11-smoke

# Technischer Kurzlauf: Seed 792, beide Varianten, 8/8/8 Trials:
python3 NERVA_Modell/research/experiments/m11/run.py run --stage smoke --output /tmp/mein-m11-smoke

# Pilot: 700–702, unveränderte 256/128/128 Trials:
python3 NERVA_Modell/research/experiments/m11/run.py run --stage pilot --output /tmp/mein-m11-pilot --previous /tmp/mein-m11-smoke

# Bestätigung: 710–729, dieselben Bedingungen und Trialzahlen:
python3 NERVA_Modell/research/experiments/m11/run.py run --stage confirm --output /tmp/mein-m11-confirm --previous /tmp/mein-m11-pilot

# Vorhandene Rohdaten unabhängig erneut prüfen und Berichte erzeugen:
python3 NERVA_Modell/research/experiments/m11/run.py report --output /tmp/mein-m11-smoke
```

Diese Befehle zeigen die vollständige Folge; die Dokumentation führt sie nicht
selbst aus. `--previous` prüft die vollständige vorherige technische Stufe mit
gleicher Quell-/Protokollkennung. Die Vorhersagequalität des Piloten ist kein
Auswahlkriterium. Der Runner erzeugt keine zusätzlichen Varianten oder Seeds.

Die JSON-Datei `protocol.json` ist der feste Versuchsvertrag, keine einzelne
Simulation. Nur der M11-Runner versteht diesen Vergleich. Beispiel einer
Einzelkonfiguration:

```json
{
  "protocol": "NERVA-M11-v1",
  "case_id": "competition",
  "seed": 792,
  "train_trials": 8,
  "calibration_trials": 8,
  "test_trials": 8,
  "condition": {"competition": true}
}
```

```bash
./NERVA_Modell/research/experiments/run.sh m11 --config /pfad/config.json --output /tmp/neues-m11.json
```

Andere Parameter oder Bedingungsnamen werden abgewiesen. Wissenschaftliche
Seedlisten und Trialzahlen sind im Python-Runner fest. Ein direkter Rust-Aufruf
ist noch keine Bestätigungsreihe.

## Daten und Prüfung

Vor dem ersten Lauf speichert `manifest.json` die vorab festgelegte Definition,
SHA-256-Quellkennung sowie Python-/Rust-Version; alle einzelnen Konfigurationen
liegen unter `configs/`. `build.json` ergänzt vor neuronaler Ausführung den
SHA-256-Hash des kompilierten Programms. Eine OS-Dateisperre schützt den Ausgabeordner.

Je Variante und Seed entstehen natives JSON und die sechs bestehenden M9-CSVs:
`.details.csv`, `.audit.csv`, `.sensor.csv`, `.copies.csv`, `.branches.csv` und
`.propagation.csv`. `.execution.json` protokolliert den Prozessabschluss;
`.sha256.json` schützt die vollständigen Rohdateien vor unbemerkten Änderungen.
Bestehende vollständige Ergebnisse werden geprüft und wiederverwendet.
Unvollständige native Ausgaben werden nicht überschrieben; für einen neuen
Versuch ist ein neuer Ausgabeordner zu verwenden. Geänderte Quellen oder
Definitionen erfordern ebenfalls einen neuen Ausgabeordner.

`competition_pools` beschreibt für beide Varianten die anfänglichen Budgets.
Im Konkurrenzfall rekonstruiert `competition_audit` jede STDP-Änderung sowie
jede Normalisierung bis zu sämtlichen Endgewichten. Normalisierung wird in
Python unabhängig durch Bisektion statt des Rust-Active-Set-Verfahrens geprüft.
Feste Poolmitgliedschaft, Budget, Reihenfolge, Sättigungen, vollständiges
Einfrieren und Übereinstimmung beider Anfangsnetze sind erforderlich.
Replayzweige zeichnen diesen bereits gespeicherten Lernverlauf nicht erneut auf;
ihre vollständigen neuronalen Zustände und gemessenen Verläufe werden weiterhin
exakt verglichen. Die Referenz verwendet die bisherigen M9-Audits ohne den
zusätzlichen STDP-Einzelverlauf; ihr Fehlen bedeutet nicht, dass sie nicht lernt.

`decision.json` trennt Lernkriterien, den Vorteil gegenüber der Referenz und den
Erhalt nach Ruhe. `contrasts.json` enthält sämtliche Kontroll-, Reset- und
Variantenvergleiche einschließlich erfolgloser Kontraste. `diagnostics.json`,
`summary.csv` und `summary.md` fassen die geprüften Rohdaten zusammen. Nur die
Bestätigung auf allen 20 Seeds kann wissenschaftliche Kriterien erfüllen.

## Softwareprüfungen

```bash
cargo test --offline --manifest-path NERVA/Cargo.toml competition --lib
cargo test --offline --release --manifest-path NERVA_Modell/research/experiments/Cargo.toml --test m11 --test m9
python3 -W error::ResourceWarning -m unittest discover -s NERVA_Modell/research/experiments/m11/tests -v
```

Geprüft werden auch Referenzparität mit M9, vollständiges Replay, LTD ohne
Post-Spike, zeitgleiche STDP-Hooks vor einer einzigen Normalisierung und
unveränderte Emissionsamplituden reisender Impulse. Synthetische Daten prüfen,
dass ein echter Vorteil die festgelegten Kriterien erfüllen kann, während
Gleichstand, nur eine erfolgreiche Belegung, fehlender Erhalt oder fehlende
Gewichtskausalität keine Bestätigung ergeben. Manipulierte Rohdaten müssen
abgewiesen werden. Diese Softwaretests sind keine neuronalen Lernnachweise.
