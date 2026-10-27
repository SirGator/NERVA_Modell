# M10 ausführen

[Diagnoseplan](../M10_EXPERIMENT.md). M10 wertet vorhandene M9-Daten aus und
ergänzt passive Traces auf denselben Seeds. Es ist keine neue Bestätigungsreihe.

Vom Projektverzeichnis `DSVLM`:

```bash
# Plan und Quellprüfsummen vor den zusätzlichen neuronalen Messungen speichern:
python3 NERVA_Modell/Experimente/m10/run.py plan --source Messungen/Experimente/m9-v2-confirm --output Messungen/Experimente/m10-path-audit

# Technische Prüfung auf den alten Kurzlaufdaten:
python3 NERVA_Modell/Experimente/m10/run.py run --stage smoke --source Messungen/Experimente/m9-v2-smoke --output /tmp/mein-m10-smoke

# Alle M9-Quellfälle auswerten; die Kombination auf allen 20 Seeds passiv messen:
python3 NERVA_Modell/Experimente/m10/run.py run --jobs 8 --source Messungen/Experimente/m9-v2-confirm --output Messungen/Experimente/m10-path-audit

# Gesamten Audit und Bericht aus gespeicherten Daten wiederholen:
python3 NERVA_Modell/Experimente/m10/run.py report --output Messungen/Experimente/m10-path-audit
```

Die 20 zusätzlichen Ergebnisse liegen unter `traces/`; jedes enthält die
unveränderten M9-Messungen und 96 eingefrorene Eingriffszweige mit je vier
zusätzlichen Phasentraces. Eingänge werden als signierte tatsächliche
Ankunftsimpulse gespeichert. Potenziale werden auf Kopien zum gemeinsamen
Phasenzeitpunkt berechnet. Neben JSON entstehen dieselben sechs CSVs wie in M9;
der Audit verlangt deren Bytegleichheit mit der Quelle.

`weight-effects.csv`, `action-discrimination.csv` und `mapping-weights.csv`
enthalten die retrospektive Diagnose aller 80 Quellfälle. `trace-effects.csv`
ergänzt synchrone Potenzial- und Eingangsvergleiche der Kombination.
`summary.csv`, `summary.md` und `diagnostics.json` fassen die Diagnose zusammen.
Vorhandene Manifeste und Ergebnisse müssen vor Wiederverwendung passen.
