# Laufendes NERVA-Modell

`NERVA_Modell/Modell/` enthält das ausführbare visuelle Modell: Wahrnehmung, rekurrente
Verarbeitung, lokales Lernen und Aktionen über die vollständige virtuelle Tastatur.
Kontrollierte Forschungsversuche und ihre Auswertungen liegen im eigenen Ordner
[NERVA_Modell/Experimente/](../Experimente/README.md).

```text
VM-Bild → Sensorik → räumlicher rekurrenter Core → Motorik → Tastatur
                       ↕ Wachstum / Pruning          │
                       ↕ STDP / Homöostase            └→ Rückführung
```

Vom Projektverzeichnis `DSVLM` starten:

```bash
./NERVA_Modell/Modell/run-demo.sh
./NERVA_Modell/Modell/run-demo.sh demo --steps 100 --seed 7
./NERVA_Modell/Modell/run-demo.sh demo --frozen
./NERVA_Modell/Modell/run-demo.sh demo --keyboard wasd
./NERVA_Modell/Modell/run-demo.sh demo --fixed-topology
./NERVA_Modell/Modell/run-demo.sh demo --no-feedback
./NERVA_Modell/Modell/run-demo.sh demo --model direct
./NERVA_Modell/Modell/run-demo.sh attach /tmp/nerva-qmp.sock
./NERVA_Modell/Modell/run-demo.sh layout
./NERVA_Modell/Modell/run-demo.sh smoke
```

`demo` startet eine eigene visuelle QEMU-VM. `attach` verwendet eine vorhandene
VM. `smoke` prüft den vollständigen Weg mit echten Pixeln und allen 105 Tastencodes.
Neue Laufprotokolle und Bildschirmaufnahmen werden unter `Messungen/Modell/` abgelegt.
Die bisherigen Messläufe liegen unverändert unter `Messungen/Experimente/`.

| Pfad | Aufgabe |
|---|---|
| `src/main.rs` | Laufender Bild–Modell–Tastatur-Zyklus und Live-CLI |
| `src/lib.rs` | Modellbibliothek für die Live-Anwendung und externe Versuche |
| `src/model.rs` | Neuronale Ausführung, Rückführung, Freeze und Entwicklungsschritte |
| `src/model/topology.rs` | Dünnes räumliches Grundgerüst mit 256 Processing-Zellen |
| `src/model/config.rs` | Modellkonfiguration und Dynamikparameter |
| `src/model/sensory.rs` | Optionale Pixel-/Diagnosesensoren und vorhandene deterministische/Poisson-Kodierer |
| `src/model/audit.rs` | Optionale Eingangs-/Ereignisaudits und gemeinsamer Freeze-Vergleich |
| `src/m4/`, `src/m4_main.rs` | JSON-Brücke für einzelne kontrollierte M4-Läufe mit dem vorhandenen Modell |
| `src/m5.rs`, `src/m5_main.rs` | Gezielte Aktionskopie-/Homöostase-Versuche mit bestehenden Mechanismen |
| `src/m6.rs`, `src/m6_main.rs`, `src/m6/` | Kombinierter JSON-Versuch, exakte Zustandsreplays und getrennte Einzelzelldiagnose |
| `src/readout.rs` | Wiederverwendete externe Messauswerter; keine neuronale Lernregel |
| `src/direct.rs` | Optionaler direkter Sensor→Motor-Modellaufbau |
| `src/visual_vm.rs` | Lebenszyklus der visuellen Demo-VM; auch von Versuchen verwendbar |
| `guest/visual.S` | Visuelle A/D-Umgebung im Gast |
| `tests/` | Modell-, Bildsignal- und Live-VM-Tests |
| `../../Messungen/Modell/` | Protokolle und Bildschirmaufnahmen des laufenden Modells |

Das Modell bindet `../../NERVA` und `../nerva-qemu` ein. Es hat keine Abhängigkeit
von `NERVA_Modell/Experimente/`. Eigene ausgeführte Tasten werden zum Core zurückgeführt;
interner VM-Zustand, Zielkoordinaten und Auswerterlabels bleiben außerhalb des Modells.

Standardmäßig laufen 256 Processing-Neuronen mit dünner räumlicher Vernetzung,
Plastizität, Homöostase und Strukturentwicklung aus NERVA. Die Standardtastatur
enthält 105 deutsche ISO-Tasten. `--motor-decoder timing` verwendet bei nichtleeren
Zählgleichständen einen eindeutigen frühesten ersten Spike; `--motor-decoder count`
wählt die historische Regel. Modifier bleiben unabhängig.

Details: [Visuelles Modell](VISUAL_MODEL.md), [Tastaturbelegung](KEYBOARD_LAYOUT.md),
[Architektur](../../ARCHITECTURE.md). Forschungsstand und Messberichte:
[NERVA_Modell/Experimente/README.md](../Experimente/README.md).

```bash
cargo test --offline --release --manifest-path NERVA_Modell/Modell/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/Modell/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path NERVA_Modell/Modell/Cargo.toml -- --check
```

Frühere Aufrufe wie `./NERVA_Modell/Modell/run-demo.sh m3` funktionieren als Weiterleitung an
`./NERVA_Modell/Experimente/run.sh m3`. Die Versuchsprogramme werden aus `NERVA_Modell/Experimente/` gebaut.

Die M4-JSON-Brücke startet mit `./NERVA_Modell/Modell/run-demo.sh m4 --config CONFIG --output RESULT`.
Die Testsuite und Ergebnisse bleiben in `NERVA_Modell/Experimente/`. Stand und Grenzen:
[M4_STATUS.md](M4_STATUS.md).

M5 startet entsprechend mit `./NERVA_Modell/Modell/run-demo.sh m5 --config CONFIG --output RESULT`.
Die Versuchspläne und Auswertung liegen unter [NERVA_Modell/Experimente/m5/](../Experimente/m5/README.md),
die [M5-Ergebnisse hier](../Experimente/M5_EXPERIMENT.md). Die Standardwerte des
Live-Modells bleiben Kopiegewicht 0,9, Core-Tau 20 ms und globale Homöostase;
die neuen Bereiche und Signalparameter werden in M5 ausdrücklich konfiguriert.

M6 startet mit `./NERVA_Modell/Modell/run-demo.sh m6 --config CONFIG --output RESULT` und
benötigt keine laufende VM. Die visuelle Kalibrierung erfolgt auf einem exakten
Replayzweig; die Elterninstanz wird unmittelbar am Trainingsende geprüft.
[Pläne und Aufrufe](../Experimente/m6/README.md),
[M6-Ergebnisse einschließlich der getrennten Core-Kontrolle](../Experimente/M6_EXPERIMENT.md).

Die anschließende [M7-Synapsendiagnose](../Experimente/M7_EXPERIMENT.md) liegt
vollständig im Experimentpaket. Das Modell stellt dafür nur einen kontrollierten
Gewichtseingriff bei eingefrorenen Regeln und die vorhandene Zustandsprüfung bereit.

[M8](../Experimente/M8_EXPERIMENT.md) startet über `./NERVA_Modell/Experimente/run.sh m8`.
Das Modell stellt dafür Parameter für Core-Gewichte, Core-Schwelle, lokale
Verbindungsdichte, Core-Verzögerung, bestehende STDP-Zeitskalen und visuelle
Eingangsgewichte bereit. Sämtliche bisherigen Standardwerte bleiben erhalten.
