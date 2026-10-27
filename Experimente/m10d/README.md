# M10-D: Rekurrenz gezielt blockieren

[Versuchsdefinition](../M10D_EXPERIMENT.md). Technischer Pilot und anschließende
Diagnose auf bekannten Seeds sind getrennt; keine unabhängige Bestätigung.

Vom Projektverzeichnis aus, zuerst das unveränderte Programm bauen:

```bash
cargo build --offline --release --manifest-path NERVA_Modell/Experimente/Cargo.toml --bin nerva-m10d
python3 NERVA_Modell/Experimente/m10d/run.py plan --stage smoke --source Messungen/Experimente/m9-v2-smoke --output Messungen/Experimente/m10d-smoke
python3 NERVA_Modell/Experimente/m10d/run.py run --stage smoke --source Messungen/Experimente/m9-v2-smoke --output Messungen/Experimente/m10d-smoke
python3 NERVA_Modell/Experimente/m10d/run.py plan --source Messungen/Experimente/m9-v2-confirm --output Messungen/Experimente/m10d-recurrence
python3 NERVA_Modell/Experimente/m10d/run.py run --jobs 8 --source Messungen/Experimente/m9-v2-confirm --output Messungen/Experimente/m10d-recurrence
```

Nur vorhandene Quell-JSONs mit passenden SHA-256 werden verwendet. Programm- und
Binärprüfsummen werden vor Ausführung gespeichert und danach erneut geprüft.
Ausgaben werden atomar publiziert und nicht überschrieben. Vorhandene Ergebnisse
werden vor Wiederverwendung vollständig validiert. Die sechs historischen CSVs
müssen bytegleich zu den ursprünglichen M9-Dateien bleiben.

```bash
python3 NERVA_Modell/Experimente/m10d/run.py report --output Messungen/Experimente/m10d-recurrence
```

Die vollständigen Traces enthalten Baseline und drei Aktionsphasen, Sperrlisten,
Queue-Stornierungen, Zustände und Ereignisse. Die zusammenfassende Tabelle
enthält alle Varianten; `pairs.csv` trennt Seeds/Belegungen, `contrasts.csv`
enthält die vorab definierten deskriptiven Vergleiche.
