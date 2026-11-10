# Laufendes NERVA-Modell

[Projektstartseite](../../README.md) · [Installation](../../Doku/EINSTIEG.md) ·
[Entwicklungsablauf](../../Doku/ENTWICKLUNG.md)

`NERVA_Modell/Modell/` enthält das ausführbare visuelle Modell: Wahrnehmung, rekurrente
Verarbeitung, lokales Lernen und Aktionen über die vollständige virtuelle Tastatur.
Kontrollierte Forschungsversuche und externe Auswertungen liegen ausschließlich
im [Research-Harness](../research/README.md).

```text
VM-Bild → Sensorik → räumlicher rekurrenter Core → Motorik → Tastatur
                       ↕ Wachstum / Pruning          │
                       ↕ STDP / Homöostase            └→ Rückführung
```

Das lokale **Live-Dashboard** zeigt Netzaktivität, VM-Bild, Tasten und
Lern-/Strukturänderungen. Es startet mit:

```bash
./NERVA_Modell/Modell/run-demo.sh gui
```

[Bedienung und Voraussetzungen](../Dashboard/README.md). Die bisherigen
Terminalaufrufe bleiben erhalten.

Vom aktiven Projektverzeichnis `DSVLM/main/` starten. Die Skripte bauen
offline; für `demo` und `smoke` werden QEMU und GNU-binutils benötigt:

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

## Gemeinsames Gewebe und lokale Neuromodulation

Die Mechanismen wurden zuerst in [NERVA](../../NERVA/docs/neuromodulation.md)
implementiert und geprüft; dieser Host baut daraus ein gemeinsames Netzwerk.
Elektrische Zellprofile, Arealfeld, Rezeptorantwort, chemische Felder,
signierte Lern-Eligibility und Kurzzeitplastizität stammen aus der Library.
Der historische Cortical Core bleibt die Standardeinstellung.

```bash
./NERVA_Modell/Modell/run-demo.sh demo --architecture hippocampal
./NERVA_Modell/Modell/run-demo.sh demo --architecture hippocampal --no-neuromodulation
./NERVA_Modell/Modell/run-demo.sh demo --architecture hippocampal --area-transition-width 0
./NERVA_Modell/Modell/run-demo.sh demo --architecture cortical --neuromodulation
```

Die CLI aktiviert Chemie beim Hippocampus-Profil standardmäßig; in `ModelConfig`
wird `neuromodulation` ausdrücklich gesetzt. Width 0 ist der scharfe Vergleich,
0,16 der weiche Standard. Cortex-, DG-, CA3- und CA1-ähnliche Einflüsse mischen
geeignete lokale Eigenschaften. Alle Zellen bleiben in derselben `Network`;
Neuronen speichern keine feste kognitive Arealidentität. Lokale Verbindungen
können die Übergänge kreuzen, und neue Synapsen erhalten dieselben Library-Profile.

Die Quellzellen sind normale Zellen mit plastischen Eingängen. Ihre eigenen
Spikes setzen drei verschiedene Modulatorarten frei. Es gibt keinen externen
Belohnungs-, Neuheits- oder Sollhandlungsalgorithmus. Hippocampale Varianten
verwenden denselben Drei-Faktoren-Modus auch ohne Chemie; ohne dritten Faktor
konsolidieren die markierten Gewichte dann nicht. Die Chemie-Ablation erhält
die anfänglichen Zellen, Rezeptor-/Quellprofile und Verbindungen unverändert.
STP und neuronale Aktivität laufen dabei weiter.

`report.json` enthält `hippocampus` mit Profilverteilung, Quellenkonfiguration,
aktiven chemischen Pulsen, tatsächlich modulierten Zellen und STP-Synapsen.
Das sind technische Diagnosen. Die Profile sind synthetisch und keine validierte
biologische Kalibrierung; ein Gedächtnis- oder Kognitionsgewinn ist damit nicht
behauptet. [Fester Bildvergleich](../research/HIPPOCAMPAL.md).

Geprüft sind native Replay-/Ablations- und Wachstumsfälle sowie ein echter
12-Schritt-QEMU-Lauf mit 128 Processing-Zellen und vollständiger Tastaturprobe.
Der Lauf erzeugte 1.342 Core-Spikes, 2.036 Motorimpulse, 486 Gewichtsänderungen,
177 aktive chemische Pulse und 38 STP-Synapsen. Ein Vergleich mit gesicherten
Originalquellen bestätigt die bisherigen Zustandswerte/Schritte des Standard-
Cortical-Modells exakt: [Prüfbeleg](verification/cortical-default-compatibility.json).

| Pfad | Aufgabe |
|---|---|
| `src/main.rs` | Laufender Bild–Modell–Tastatur-Zyklus und Live-CLI |
| `src/lib.rs` | Modellbibliothek für die Live-Anwendung und externe Versuche |
| `src/model.rs` | Neuronale Ausführung, Rückführung, Freeze und Entwicklungsschritte |
| `src/model/topology.rs` | Räumliches Grundgerüst; standardmäßig 256 Processing-Zellen |
| `src/model/area.rs` | Host-Anbindung der Areal- und Zellprofile aus NERVA |
| `src/model/config.rs` | Modellkonfiguration und Dynamikparameter |
| `src/model/sensory.rs` | Optionale Pixel-/Diagnosesensoren und vorhandene deterministische/Poisson-Kodierer |
| `src/model/audit.rs` | Optionale Eingangs-/Ereignisaudits und gemeinsamer Freeze-Vergleich |
| `src/direct.rs` | Optionaler direkter Sensor→Motor-Modellaufbau |
| `src/visual_vm.rs` | Lebenszyklus der visuellen Demo-VM; auch von Versuchen verwendbar |
| `guest/visual.S` | Visuelle A/D-Umgebung im Gast |
| `tests/` | Modell-, Bildsignal- und Live-VM-Tests |
| `../../Messungen/Modell/` | Protokolle und Bildschirmaufnahmen des laufenden Modells |

Das Modell bindet `../../NERVA` und `../nerva-qemu` ein. Es hat keine Abhängigkeit
vom Research-Harness. Eigene ausgeführte Tasten werden zum Core zurückgeführt;
interner VM-Zustand, Zielkoordinaten und Auswerterlabels bleiben außerhalb des Modells.

Standardmäßig laufen 256 Processing-Neuronen mit dünner räumlicher Vernetzung,
Plastizität, Homöostase und Strukturentwicklung aus NERVA. Die Standardtastatur
enthält 105 deutsche ISO-Tasten. `--motor-decoder timing` verwendet bei nichtleeren
Zählgleichständen einen eindeutigen frühesten ersten Spike; `--motor-decoder count`
wählt die historische Regel. Modifier bleiben unabhängig.

Details: [Visuelles Modell](VISUAL_MODEL.md), [Tastaturbelegung](KEYBOARD_LAYOUT.md),
[Architektur](../../ARCHITECTURE.md). Der aktuelle
[Forschungsstand](../../Doku/FORSCHUNGSSTAND.md) ordnet die Messberichte ein.
[Historische Versuchsreihen](../research/dokumentation/versuchsreihen/README.md)
beschreiben frühere Protokolle.

```bash
cargo test --offline --release --manifest-path NERVA_Modell/Modell/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/Modell/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path NERVA_Modell/Modell/Cargo.toml -- --check
```

Die eigenständigen M3–M11-Experimentprogramme einschließlich der M4–M6-Brücken
sind entfernt. Frühere Startbefehle werden nicht weitergeleitet.
[Protokolle und Ergebnisse](../research/dokumentation/versuchsreihen/README.md)
bleiben als historische Dokumentation erhalten. Neue Studien starten im Harness.


## Kausale Lerndiagnose

Der Harness kann über `Model::with_sensor_and_learning` ausdrücklich
`LearningRule::PairStdp` oder `LearningRule::ThreeFactor` wählen. Die
Netzkonstruktion bleibt gleich; `LearningRule::Automatic` erhält die bisherigen
Live-Standards. Das [Forschungskontrollzentrum](../research/CONTROL.md)
vergleicht beide Regeln und Chemie an/aus auf identischen initialen Netzwerken.

`Model::enable_learning_audit(max_records)` aktiviert die begrenzte Library-
Beobachtung. Nach jedem Frame liefert `take_learning_audit()` Tags, Modulatorwerte,
Gewichtseinzeländerungen und Integrationssummen; unvollständige Aufzeichnungen
werden als Fehler zurückgegeben. `disable_learning_audit()` liefert Restdaten
zurück und schaltet die Beobachtung ab. Keine neuen Modellparameter sind nötig.

Der [kausale Harness-Versuch](../research/CAUSALITY_256.md) rekonstruiert identische
trainierte Checkpoints, prüft `same_replay_state` und ändert danach ausschließlich
Gewichte auf eingefrorenen Instanzen. [Ergebnisse und Grenzen](../research/results/hippocampal_causality_256_20261109/RESULTS.md)
trennen tatsächliche Lernwirkung und den künstlichen Diagnosekontrast.
