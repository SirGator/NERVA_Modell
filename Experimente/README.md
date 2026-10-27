# NERVA-Experimente

`NERVA_Modell/Experimente/` enthält kontrollierte Versuche, Trainings-/Prüfabläufe,
Auswertungen. Gespeicherte Messungen liegen unter `Messungen/Experimente/`. Das laufende visuelle Modell liegt in
[NERVA_Modell/Modell/](../Modell/README.md) und wird als Bibliothek eingebunden.
Das Modell selbst benötigt dieses Paket nicht.

[Forschungsfahrplan M9–M13](ROADMAP.md): M9-v2 und die retrospektive M10-Diagnose
sind abgeschlossen; die weiteren Schritte hängen vom Befund ab.

Vom Projektverzeichnis `DSVLM` starten:

```bash
./NERVA_Modell/Experimente/run.sh help
./NERVA_Modell/Experimente/run.sh m3
./NERVA_Modell/Experimente/run.sh sensorimotor
./NERVA_Modell/Experimente/run.sh agency
./NERVA_Modell/Experimente/run.sh study --steps 40 --seeds 7,19,42
./NERVA_Modell/Experimente/run.sh study-replay Messungen/Experimente/recurrent-study
./NERVA_Modell/Experimente/run.sh serial
python3 NERVA_Modell/Experimente/m4/runner.py run --stage gate --output Messungen/Experimente/mein-m4 \
  --command './NERVA_Modell/Modell/run-demo.sh m4 --config {config} --output {output}'
```

M3, Sensorimotorik, Agency und die Wiedergabe gespeicherter Aufnahmen benötigen
keine VM. `study`, `record` und die serielle Demo starten QEMU.
Die optionale lokale QEMU-Installation wird weiterhin aus `NERVA_Modell/Modell/.tools/qemu`
verwendet. Eine Systeminstallation oder `QEMU_SYSTEM_X86_64` hat Vorrang.

| Pfad | Aufgabe |
|---|---|
| `src/main.rs` | CLI für Aufzeichnung, Rekurrenz-, Retention- und Agency-Versuche |
| `src/experiment.rs` | Aufzeichnungen, gepaarter Entwicklungsversuch und exakte Wiedergabe |
| `src/retention.rs` | Trainieren, laufendes Modell einfrieren und kontrolliert prüfen |
| `src/agency.rs` | Zielversuch, Kontrollagenten und vertauschte Tastenwirkung |
| `src/sensorimotor/` | Getrennte Motor- und Wirkungsversuche |
| `src/m3/` | Bekannte Aktion vor dem Wirkungsbild, Freeze-Kontrollen und Motor-Scan |
| `src/serial_model.rs`, `src/protocol.rs`, `guest/boot.S` | Frühere serielle Zuordnungsdemo |
| `m4/` | Python-Orchestrator, 20 Konfigurationen, Diagnose-Gate und unabhängige Datenaudits |
| `m5/` | Gezielte Ursachenversuche: motorische Integration und sensorische Homöostase |
| `m6/` | Kombinierter Wirkungslerntest mit Replay-Kalibrierung und separater Core-Kontrolle |
| `src/m7.rs`, `m7/` | Synapsen-/Zeitpaardiagnose und kontrollierte Gewichtsgegenprüfungen auf M6-Replays |
| `src/m8.rs`, `m8/` | Sechs Parameterscans mit Kontrollarmen, unabhängigen Checkpoint-/Ruheproben und No-copy-Kontrollen |
| `src/m9.rs`, `m9/` | 2×2-Kombinationstest v2 mit Gewichtsrücksetzungen und 20 Bestätigungsseeds; v1 weiter reproduzierbar |
| `src/m10.rs`, `m10/` | Retrospektive Signalweg-/Ausleserdiagnose und passive Replaytraces auf den vorhandenen M9-Seeds |
| `m10e/` | Core-Gewichtsreset nach identischer Baseline mit vier Rekurrenzsperren gekreuzt; feste Ausleser und bekannte Checkpoints |
| `m10d/` | Vier kontrollierte Übertragungssperren für rekurrente Core-Erregung/Hemmung auf eingefrorenen bekannten M9-Checkpoints |
| `m10c/` | Retrospektive Bilanz von Eingängen, Refraktärfilter, Leck und Resets auf der festen neuronalen Auslesergewichtung |
| `m10b/` | Retrospektive Diagnose der visuellen Wirkungsausrichtung und des gemeinsamen Decoderbias; keine neuen neuronalen Läufe |
| `tests/` | Versuchs-, Auswertungs-, Wiederholungs- und QEMU-Tests |
| `../../Messungen/Experimente/` | Versuchsberichte, Rohdaten und historische Messläufe |

Die Versuchswelten liefern dem Modell weiterhin nur Bilder und eigene ausgeführte
Tasten. Ziel-/Positionswerte und Wirkungsklassen dienen ausschließlich der externen
Auswertung. Lokale Lernregeln kommen weiterhin aus `NERVA/`.

| Versuch | Beschreibung und Ergebnisse |
|---|---|
| Rekurrenter Core | [RECURRENT_EXPERIMENT.md](RECURRENT_EXPERIMENT.md) |
| Trainieren–Einfrieren–Prüfen | [RETENTION_EXPERIMENT.md](RETENTION_EXPERIMENT.md) |
| Eigenständige Handlungsfähigkeit | [AGENCY_EXPERIMENT.md](AGENCY_EXPERIMENT.md) |
| Motorik und Wirkungslernen | [SENSORIMOTOR_EXPERIMENT.md](SENSORIMOTOR_EXPERIMENT.md) |
| M3: Handlungswirkung | [M3_EXPERIMENT.md](M3_EXPERIMENT.md) |
| M4: Sensorik, Zeitabstände, Lernmechanismen; drei und zehn neue Seeds | [M4_EXPERIMENT.md](M4_EXPERIMENT.md) |
| M5: Aktionskopie und Homöostase-Populationen; Seeds 200–209 | [M5_EXPERIMENT.md](M5_EXPERIMENT.md) |
| M6: Kombination, unmittelbare Probe; Seeds 300–309 und separate Core-Kontrolle 400–409 | [M6_EXPERIMENT.md](M6_EXPERIMENT.md) |
| M7: Belegungsabhängige Gewichtsänderungen und ihre Wirkung; bestehende Seeds 400–409 | [M7_EXPERIMENT.md](M7_EXPERIMENT.md) |
| M8: Rekurrente Weiterleitung und stärkere Zuordnung; 13 Bedingungen, neue Seeds 510–519 | [M8_EXPERIMENT.md](M8_EXPERIMENT.md) |
| M9-v2: Kombination und Gewichtskausalität; 20 neue Bestätigungsseeds, kein funktionaler Lernnachweis | [Ergebnisse](../../Messungen/M9_EXPERIMENT.md) · [Versuchsplan](M9_EXPERIMENT.md) |
| M10: Elektrisch wirksame Gewichte, selten geänderte Ausleserentscheidung; retrospektive Ursachenanalyse | [Ergebnisse](../../Messungen/M10_EXPERIMENT.md) · [Diagnoseplan](M10_EXPERIMENT.md) |
| M10-E: Core-Gewichtsbeitrag und Rekurrenz gekreuzt, identische Zustände vor der Aktion | [Ergebnisse](../../Messungen/M10E_EXPERIMENT.md) · [Definition](M10E_EXPERIMENT.md) |
| M10-D: Rekurrente Erregung und Hemmung getrennt blockiert; feste Checkpoints und Ausleser | [Ergebnisse](../../Messungen/M10D_EXPERIMENT.md) · [Definition](M10D_EXPERIMENT.md) |
| M10-C: Unpassende Ausrichtung bereits in der Eingangsbilanz; 1.920 rekonstruierte Aktionsfenster | [Ergebnisse](../../Messungen/M10C_EXPERIMENT.md) · [Definition](M10C_EXPERIMENT.md) |
| M10-B: Gemeinsamer Offset erklärt die Fehler nicht allein; diagnostische Paar-Rangfolge 42,5/45,0 % | [Ergebnisse](../../Messungen/M10B_EXPERIMENT.md) · [Definition](M10B_EXPERIMENT.md) |
| Serielle Zuordnung | [SERIAL_MODEL.md](SERIAL_MODEL.md) |

Die bisherigen kontrollierten Messreihen liefern keinen belastbaren Nachweis
erlernter Handlungswirkungen. Die historischen Messdaten wurden unverändert nach
`Messungen/Experimente/` verschoben. Pfadangaben innerhalb alter JSON-Berichte
können noch frühere Ablageorte nennen. Relative Aufnahmepfade und Links innerhalb
der Messläufe bleiben erhalten. Neue Live-Läufe werden unter `Messungen/Modell/`
gespeichert; neue Experimentläufe unter `Messungen/Experimente/`.

```bash
cargo test --offline --release --manifest-path NERVA_Modell/Experimente/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/Experimente/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path NERVA_Modell/Experimente/Cargo.toml -- --check
```

Frühere Skriptaufrufe über `NERVA_Modell/Modell/run-demo.sh` werden weitergeleitet. Für die
Versuchs-Binaries mit Cargo ist nun `--manifest-path NERVA_Modell/Experimente/Cargo.toml` nötig.
