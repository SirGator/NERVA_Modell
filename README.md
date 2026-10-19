# Modell- und Versuchscode

| Ordner | Inhalt |
|---|---|
| [Modell/](Modell/README.md) | Laufendes visuelles Modell und Modellbibliothek |
| [Experimente/](Experimente/README.md) | Kontrollierte Versuche, Python-Orchestratoren und Auswertungscode |
| [nerva-qemu/](nerva-qemu/README.md) | Modellunabhängige VM-, Bild- und Tastaturadapter |

Die neuronale Bibliothek liegt unter [../NERVA/](../NERVA/README.md).
Daten und Ergebnisse werden unter [../Messungen/](../Messungen/README.md)
gespeichert. Versuchskonfigurationen und Protokollbeschreibungen gehören zum Code;
generierte Konfigurationen eines Messlaufs liegen zusammen mit dessen Ergebnissen.

Aufrufe vom Projektverzeichnis `DSVLM`:

```bash
./NERVA_Modell/Modell/run-demo.sh
./NERVA_Modell/Experimente/run.sh m3
```
# NERVA_Modell
