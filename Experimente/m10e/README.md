# M10-E: Core-Gewichte und Rekurrenz kreuzen

[Definition](../M10E_EXPERIMENT.md). Zwei Gewichtszustände × vier Sperren, nach
identischer Baseline. Rücksetzungen finden hier vor der Aktion statt; bei M10
fanden sie vor der Baseline statt. Alle vier trainierten Zweige müssen M10-D
reproduzieren. Neue Ursachenanalyse auf bekannten Seeds, keine Bestätigung.

Vom vollständigen Projektverzeichnis aus:

```bash
cargo build --offline --release --manifest-path NERVA_Modell/Experimente/Cargo.toml --bin nerva-m10e
python3 NERVA_Modell/Experimente/m10e/run.py plan --stage smoke --source Messungen/Experimente/m9-v2-smoke --recurrence-source Messungen/Experimente/m10d-smoke --output Messungen/Experimente/m10e-smoke
python3 NERVA_Modell/Experimente/m10e/run.py run --stage smoke --source Messungen/Experimente/m9-v2-smoke --recurrence-source Messungen/Experimente/m10d-smoke --output Messungen/Experimente/m10e-smoke
python3 NERVA_Modell/Experimente/m10e/run.py plan --source Messungen/Experimente/m9-v2-confirm --recurrence-source Messungen/Experimente/m10d-recurrence --output Messungen/Experimente/m10e-crossed-weights
python3 NERVA_Modell/Experimente/m10e/run.py run --jobs 8 --source Messungen/Experimente/m9-v2-confirm --recurrence-source Messungen/Experimente/m10d-recurrence --output Messungen/Experimente/m10e-crossed-weights
python3 NERVA_Modell/Experimente/m10e/run.py report --output Messungen/Experimente/m10e-crossed-weights
```

Quellen und Methode werden vor Ausführung fixiert, Programme und Binärdatei
vor und nach Ausführung per SHA-256 geprüft. Quellarchive werden nicht geändert.
Vorhandene Ergebnisse werden vor Wiederverwendung vollständig geprüft und nicht
überschrieben. Alle sechs historischen CSVs müssen bytegleich sein.
