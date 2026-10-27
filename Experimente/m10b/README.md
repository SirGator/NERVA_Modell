# M10-B ausführen

[Auswertungsdefinition](../M10B_EXPERIMENT.md). Reine Auswertung vorhandener
M9-Daten; keine Simulation und kein neuer Ausleserfit.

[Abgeschlossene Ergebnisse](../../../Messungen/M10B_EXPERIMENT.md): Die
Kombination erreicht diagnostisch 42,5 % richtige Paar-Rangfolgen sofort und
45,0 % nach Ruhe. Dies ist keine neue Einzelvorhersageleistung.

Vom Projektverzeichnis `DSVLM`:

```bash
python3 NERVA_Modell/Experimente/m10b/run.py plan --source Messungen/Experimente/m9-v2-confirm --output Messungen/Experimente/m10b-alignment
python3 NERVA_Modell/Experimente/m10b/run.py run --source Messungen/Experimente/m9-v2-confirm --output Messungen/Experimente/m10b-alignment
python3 NERVA_Modell/Experimente/m10b/run.py report --output Messungen/Experimente/m10b-alignment

# Separater technischer Test auf den vorhandenen Kurzlaufdaten:
python3 NERVA_Modell/Experimente/m10b/run.py run --stage smoke --source Messungen/Experimente/m9-v2-smoke --output /tmp/mein-m10b-smoke
```

Das Manifest fixiert alle Quellprüfsummen und die Auswertungsdefinition.
Vorhandene Manifeste müssen identisch bleiben. `pairs.csv` enthält Projektionen,
gemeinsamen Bias, Ausrichtung, Paar-Rangfolge und die ursprüngliche Genauigkeit.
`summary.csv` fasst je Bedingung/Arm/Zeitpunkt/Eingriff zusammen;
`contrasts.csv` enthält deskriptive Vergleiche über gepaarte Seeds.
`diagnostics.json` und `summary.md` dokumentieren Audit und Interpretation.

Die Paar-Rangfolge setzt eine bekannte balancierte A-/D-Paarung voraus und darf
nicht als neue Einzelvorhersageleistung oder Lernerfolg berichtet werden.
