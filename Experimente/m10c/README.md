# M10-C ausführen

[Festgelegte Definition](../M10C_EXPERIMENT.md). Reine Bilanz der bestehenden
M10-Ereignisse und der bisherigen visuellen Spikezähl-Achse.

Vom Projektverzeichnis `DSVLM`:

```bash
python3 NERVA_Modell/Experimente/m10c/run.py plan --source Messungen/Experimente/m10-path-audit --output Messungen/Experimente/m10c-signal-directions
python3 NERVA_Modell/Experimente/m10c/run.py run --source Messungen/Experimente/m10-path-audit --output Messungen/Experimente/m10c-signal-directions
python3 NERVA_Modell/Experimente/m10c/run.py report --output Messungen/Experimente/m10c-signal-directions

# Softwareprüfung ausschließlich auf dem ersten bereits vorhandenen Trace:
python3 NERVA_Modell/Experimente/m10c/run.py run --stage smoke --source Messungen/Experimente/m10-path-audit --output /tmp/mein-m10c-smoke
```

`signals.csv` enthält die Differenzprojektionen je Signaltyp, `sources.csv` die
Aufteilung nach Herkunft und `edges.csv` die Einzelbeiträge aktiver Synapsen.
`transitions.csv` beschreibt Richtungswechsel, `windows.csv` die unabhängigen
Spannungsrekonstruktionsfehler. `summary.csv`, `summary.md` und `diagnostics.json`
fassen den Audit und die deskriptive Analyse zusammen.

Keine neuen neuronalen Läufe und keine neuen Ausleserfits. Die Eingangs- und
Potenzialprojektionen sind keine eigenständig kalibrierten Klassenentscheidungen.
