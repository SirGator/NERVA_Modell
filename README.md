# Modell- und Versuchscode

[Projektstartseite](../README.md) · [Installation](../Doku/EINSTIEG.md) ·
[Architektur](../ARCHITECTURE.md) · [Entwicklungsablauf](../Doku/ENTWICKLUNG.md)

| Ordner | Inhalt |
|---|---|
| [Modell/](Modell/README.md) | Laufendes visuelles Modell und Modellbibliothek |
| [research/](research/README.md) | Gemeinsamer Forschungseinstieg, Parameterraster, externe S1/S2-Selektion und SQLite |
| [Dashboard/](Dashboard/README.md) | Lokale Bedienung und Beobachtung des Modells; Quelle des Archivbetrachters |
| [Historische Versuchsreihen](research/dokumentation/versuchsreihen/README.md) | Dokumentation und frühere Protokolle ohne ausführbaren Experimentcode |
| [nerva-qemu/](nerva-qemu/README.md) | Modellunabhängige VM-, Bild- und Tastaturadapter |

Die neuronale Bibliothek liegt unter [../NERVA/](../NERVA/README.md).
Live-Protokolle und historische Daten liegen unter [../Messungen/](../Messungen/README.md).
Der Research-Runner verwendet standardmäßig `research/results/`;
`--output` erlaubt einen anderen Ergebnisordner. Versuchskonfigurationen und Protokollbeschreibungen gehören zum Code;
generierte Konfigurationen eines Messlaufs liegen zusammen mit dessen Ergebnissen.

Aufrufe vom aktiven Projektverzeichnis `DSVLM/main`:

```bash
./NERVA_Modell/Modell/run-demo.sh
./NERVA_Modell/research/run.sh list
./NERVA_Modell/research/run.sh plan NERVA_Modell/research/config/stdp_learning.toml
```

Die Startskripte bauen offline; Cargo-Abhängigkeiten vorher gemäß der
Installationsanleitung bereitstellen. Für den Harness wird Python 3.11+
benötigt. Neue Versuche erhalten eigene Ausgabeordner;
[Sicherung und Reproduktion](research/ARCHIVING.md).
