# Modell- und Versuchscode

| Ordner | Inhalt |
|---|---|
| [Modell/](Modell/README.md) | Laufendes visuelles Modell und Modellbibliothek |
| [Experimente/](Experimente/README.md) | Kontrollierte Versuche, Python-Orchestratoren und Auswertungscode |
| [research/](research/README.md) | Research-Runner mit Parameterrastern, externer S1/S2-Selektion und SQLite |
| [nerva-qemu/](nerva-qemu/README.md) | Modellunabhängige VM-, Bild- und Tastaturadapter |

Die neuronale Bibliothek liegt unter [../NERVA/](../NERVA/README.md).
Daten und Ergebnisse werden unter [../Messungen/](../Messungen/README.md)
gespeichert. Der Research-Runner verwendet standardmäßig `research/results/`;
`--output` erlaubt einen anderen Ergebnisordner. Versuchskonfigurationen und Protokollbeschreibungen gehören zum Code;
generierte Konfigurationen eines Messlaufs liegen zusammen mit dessen Ergebnissen.

Aufrufe vom Projektverzeichnis `DSVLM`:

```bash
./NERVA_Modell/Modell/run-demo.sh
./NERVA_Modell/Experimente/run.sh m3
./NERVA_Modell/research/run.sh plan NERVA_Modell/research/config/stdp_learning.toml
```
# NERVA_Modell
