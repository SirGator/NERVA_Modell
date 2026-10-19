# M7: Synapsen und ihre funktionale Wirkung prüfen

[Ergebnisse und Grenzen](../M7_EXPERIMENT.md). M7 untersucht die bestehenden
M6-Core-off-Seeds 400–409 erneut. Es ergänzt keine Lernregel und keine interne
Aufgabenarchitektur. Die gesamte Versuchsausführung liegt in `NERVA_Modell/Experimente/`.

Vom Projektverzeichnis `DSVLM`:

```bash
# Ein bestehendes M6-Ergebnis als kontrollierte Referenz verwenden:
./NERVA_Modell/Experimente/run.sh m7 \
  --reference Messungen/Experimente/m6-core-control/results/core_D_off/seed-400.json \
  --output /tmp/meine-m7-synapsendiagnose

# Matrixplan für alle zehn vorhandenen Seeds speichern und ausführen:
python3 NERVA_Modell/Experimente/m7/run.py plan --output Messungen/Experimente/meine-m7-matrix
python3 NERVA_Modell/Experimente/m7/run.py run --jobs 4 --output Messungen/Experimente/meine-m7-matrix

# Archivierte Messdaten unabhängig neu auswerten:
python3 NERVA_Modell/Experimente/m7/run.py report --output Messungen/Experimente/m7-synapse-audit
```

`--references` kann auf einen anderen vollständigen M6-Core-Kontrollordner zeigen.
Das Manifest speichert Pfad und SHA-256 jeder Referenz. Es muss beim Fortsetzen
unverändert sein. Die einzelne Rust-CLI verlangt ein neues Ausgabeverzeichnis;
die Matrix überspringt vollständig validierte Ergebnisse. Nach einem abgebrochenen
Einzellauf ein neues Ziel verwenden oder ausschließlich seinen unvollständigen
Ordner entfernen. Es gibt keine künstlichen Ersatzwerte bei fehlenden Messungen.

Eine Referenz muss ein abgeschlossenes `NERVA-M6-v1`-Ergebnis mit Variante D,
2-Kanal-Sensor/32 Core-Zellen und ausgeschalteter Core-Homöostase sein. Alle
M6-Pflichtprüfungen müssen bestanden sein. Kopiengewicht, Tau, Zeitprotokoll,
Vernetzung, Normierung und Training werden aus der strengen M6-Konfiguration
übernommen; der Verstärkungsfaktor wird unabhängig neu berechnet. Neuronale
Eingänge bleiben aktuelle Frames und eigene ausgeführte Tasten. Wirkungslabels
und Phasen dienen nur der externen Aufzeichnung.

| Pfad | Verantwortung |
|---|---|
| `NERVA_Modell/Experimente/src/m7.rs` | Passive Trainingsaufzeichnung, Gewichtsledger, Prä-/Post-Partner, exakte Replays und Gegenprüfungen |
| `NERVA_Modell/Experimente/src/m7_main.rs` | Strenge CLI, neue Ausgabe-Direktorie, Abbruchsignal |
| `NERVA_Modell/Experimente/m7/run.py` | Referenzmanifest, Prozesssteuerung, unabhängige CSV-Prüfung und Auswertung |
| `NERVA_Modell/Modell/src/model.rs` | Bestehendes Modell; zusätzliche eingefrorene Diagnose-API für kontrollierte Gewichtsänderungen |
| `NERVA_Modell/Modell/src/m4/normalization.rs` | Wiederverwendete Berechnung des unveränderten anfänglichen Sensorstrombudgets |
| `NERVA/` | Bestehende neuronale Regeln und kontrollierte Runtime-Synapsenänderung; für M7 unverändert |
| `NERVA_Modell/nerva-qemu/` | Bestehende modellunabhängige Frames und virtuelle Tastencodes; für M7 unverändert |

Pro Seed entstehen `result.json`, `events.csv`, `pairs.csv` und `weights.csv`.
Die Ereignisse und Zeitpaare betreffen das Training. Die Gewichtsdatei enthält
zusätzlich Freeze, Ruhe und Interferenz. Der JSON-Bericht enthält die komplette
Anfangstopologie und 60 Gegenprüfungen: drei Arme, zwei Belegungen, zwei Tasten,
fünf Bedingungen. `result.json` wird erst nach erfolgreichem Abschluss aus einer
`.partial`-Datei veröffentlicht. Die CSVs können bei Abbruch unvollständig sein.

Die Matrixauswertung erzeugt:

- `edge-comparison.csv`: Anfangs-, normale und umgekehrte Gewichte jeder einzelnen Synapse.
- `weights-summary.csv`: Verstärkung, Abschwächung und unveränderte Kanten je Population.
- `pair-summary.csv`: LTP/LTD, Zeitabstände und trialübergreifende Partner.
- `mapping-summary.csv`: Gewichtsvergleich und ausdrücklich topologischer Zuordnungswert.
- `counterfactuals.csv`: Spike-/Potenzialantworten gegenüber unverändertem Replay.
- `summary.md`, `diagnostics.json`: Übersicht, Audit-Zähler und deskriptive Kontraste.

Die Gewichtsdiagnose-API verlangt eingefrorene STDP, Entwicklung und Homöostase.
Sie verwendet NERVAs vorhandene kontrollierte `update_synapse`-Operation.
Ungültige IDs oder Gewichte verändern keinen Zustand. Zellen, Topologie,
Spuren und Uhren werden nicht zurückgesetzt. Bereits unterwegs befindliche
Ankünfte behalten ihre bei Emission festgelegte Amplitude. Alle Eingriffe
geschehen auf getrennten exakt reproduzierten Instanzen nach 2 Sekunden Ruhe.
Ausgangszustand und Elternparameter werden geprüft.

```bash
cargo test --offline --release --manifest-path NERVA_Modell/Modell/Cargo.toml --test synapse_intervention
cargo test --offline --release --manifest-path NERVA_Modell/Experimente/Cargo.toml --test m7
python3 -W error::ResourceWarning -m unittest discover -s NERVA_Modell/Experimente/m7/tests -v
```

Der Python-Selbsttest erzeugt zwei kleine echte Rust-Läufe im temporären Ordner.
Er prüft zusätzlich absichtlich verfälschte Zeitpartner und Retentionsgewichte.
Diese Softwareprüfungen fließen nicht in die wissenschaftlichen Messdaten ein.
Die Rust-Tests verlangen bytegleiche parallele JSON-/CSV-Ergebnisse und erkennen
ungültige Gewichtsinterventionen ohne Veränderung des Replayzustands.
