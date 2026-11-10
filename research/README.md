# Research-Harness

Der Harness führt kontrollierte Versuche mit festen Konfigurationen,
Parameterrastern, Seeds und Kontrollgruppen aus. Er prüft Replay und
Ergebnisintegrität und stellt externe Analysen sowie einen SQLite-Katalog
bereit. Neuronale Mechanismen stammen aus NERVA und dem laufenden Modell.

[Installation](../../Doku/EINSTIEG.md) · [Forschungsstand](../../Doku/FORSCHUNGSSTAND.md) ·
[Protokolle und Methoden](dokumentation/README.md) ·
[Ablage und Reproduktion](ARCHIVING.md) · [Projektstartseite](../../README.md)

## Voraussetzungen und erster Lauf

Der [Präzisionsvergleich](PRECISION.md) prüft jetzt unveränderte f32-
Direktupdates gegen einen synapsenlokalen f64-Restbetrag, mit eingefrorenen
und entkoppelten Kontrollen:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/precision_pilot.toml
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/precision_pilot.toml
```

Erhaltene Änderungsmasse, native Spikewirkung und Assoziationsgewinn werden
getrennt ausgewertet.
[Ergebnisse des festen Piloten](PRECISION_RESULTS_20261110.md).

Die [kausale Diagnosekette A–D](CAUSAL_CHAIN.md) ergänzt garantierte
Positivkontrollen und einen festen Gewichtsverstärkungsvergleich:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/causal_chain_pilot.toml
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/causal_chain_pilot.toml
```

Deterministische Kalibrierungen werden einmal gemeinsam ausgeführt; nur der
Hippocampus-Vergleich wird über die Modellseeds wiederholt.
[Pilotbefunde und unabhängige Prüfung](CAUSAL_CHAIN_RESULTS_20261110.md).

Für den gezielten Hippocampus-Vergleich dient der Harness als
[Forschungskontrollzentrum](CONTROL.md): Neuromodulation an/aus, Drei-Faktor-
gegen Pair-STDP und die Diagnose Eingang → Aktivität → Gewicht → Abruf.
Ein fester Datensatz bedient alle drei Prioritäten, mit Wiederaufnahme,
begrenzten parallelen Seeds und archiviertem Programmstand:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/hippocampal_control_pilot.toml
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/hippocampal_control_pilot.toml
./NERVA_Modell/research/run.sh control status NERVA_Modell/research/config/hippocampal_control_pilot.toml
```

[Erster 256-Zellen-Pilot und unabhängige Prüfung](HIPPOCAMPAL_CONTROL_RESULTS_20261110.md):
kein positiver gewichtsbedingter Abrufgewinn auf den beiden geprüften Seeds.

Alle Befehle beginnen in **`DSVLM/main/`**. Benötigt werden Rust/Cargo und
Python **3.11+**. TOML und SQLite verwenden die Python-Standardbibliothek.
Der Einstieg baut den Rust-Runner mit `--offline --release`; Cargo-Pakete
müssen vorher lokal vorhanden sein. Der M0-Kurzlauf braucht keine QEMU-VM.

```bash
# Adapter, Parameter, Metriken und Kontrollen anzeigen:
./NERVA_Modell/research/run.sh list

# Den kleinen Beispielplan validieren und speichern:
./NERVA_Modell/research/run.sh plan NERVA_Modell/research/config/smoke.toml --output /tmp/nerva-erster-versuch

# Ausführen oder einen identischen unterbrochenen Lauf fortsetzen:
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/smoke.toml --output /tmp/nerva-erster-versuch

# Vorhandene Records prüfen und Tabellen/Datenbank neu aufbauen:
./NERVA_Modell/research/run.sh report NERVA_Modell/research/config/smoke.toml --output /tmp/nerva-erster-versuch
```

`plan` schreibt bereits Manifest und Quellkonfiguration, startet aber keine
neuronalen Läufe. Der Kurzlauf enthält **2 Varianten × 2 Seeds × 4 Kontrollen =
16 Läufe**, mit vollständigem Replay 32 Ausführungen. Er ist eine technische
Prüfung. Die größere [stdp_learning.toml](config/stdp_learning.toml) enthält
12 Varianten × 6 Seeds × 4 Kontrollen = 288 Läufe und 576 Ausführungen.

Ohne `--output` werden Ergebnisse unter
`NERVA_Modell/research/results/<experiment.name>/` abgelegt. Dieser Bereich ist
von Git ausgeschlossen. Dauerhafte Ergebnisse separat sichern; `/tmp` dient
nur dem Beispiel. Bei geänderten Parametern oder Quellen einen neuen Ausgabeordner
verwenden. Gleicher Plan und gleicher Build ermöglichen die Wiederaufnahme.

Negative wissenschaftliche Ergebnisse sind reguläre Ergebnisse. CLI-Fehler
bedeuten ungültige Eingaben, beschädigte Archive oder fehlgeschlagene
Ausführung; ein nicht bestandenes Lernkriterium allein ist kein CLI-Fehler.

## Den passenden Ablauf wählen

Die Befehlsfamilien verwenden unterschiedliche Konfigurationsverträge.
Zuerst die jeweilige Planfunktion verwenden, um Umfang und Budget zu prüfen.

| Aufgabe | Planen / Ausführen / erneut auswerten | Anleitung und Beispielkonfiguration |
| --- | --- | --- |
| Festes Raster eines Szenarios | `plan` / `run` / `report` | Diese Seite; [smoke.toml](config/smoke.toml) |
| Screening, Zustandskarten, gepaarte Eingriffe und MAP-Elites | `research-plan` / `research` / `research-report` | [Forschungsmethoden](HARNESS.md); [research_smoke.toml](config/research_smoke.toml) |
| Externe S1/S2-Selektion mit Abschlussvalidierung | `select-plan` / `select` / `selection-report` | [Selektion](SELECTION.md); [selection_smoke.toml](config/selection_smoke.toml) |
| Retrospektiver Decoder auf M9-Archiven | `diagnose-plan` / `diagnose` / `diagnosis-report` | [Diagnostik](DIAGNOSTICS.md); [diagnosis_m9.toml](config/diagnosis_m9.toml) |
| Vorregistrierter Homöostasevergleich | `confirm-homeostasis plan` / `run` / `report` | [Protokoll](HOMEOSTASIS_CONFIRMATION.md) |
| Homöostase nach Zellpopulation | `localize-homeostasis plan` / `run` / `report` | [Protokoll](HOMEOSTASIS_POPULATIONS.md) |

Bei den beiden letzten Zeilen wird der Präfix beibehalten, zum Beispiel
`./NERVA_Modell/research/run.sh confirm-homeostasis report`.
Die retrospektive M9-Diagnose benötigt die archivierten Daten, aber keinen
Rust-Build und keine neue neuronale Simulation.

## Szenarien und Protokolle

| Thema | Dokumentation |
| --- | --- |
| Feste native M0-Referenz | [Baseline und Messbericht](PHASE1_BASELINE.md) |
| Sensorimotorik und persistente Bewegungswelt | [Adapter](SENSORIMOTOR.md), [Größen-/Umweltvergleich](SENSORIMOTOR_RESULTS_20261105.md) |
| Skalierbare Core-Größen und Ressourcenbudgets | [Skalierung](SCALING.md), [Diagnoselauf](SCALING_RESULTS_20261105.md) |
| Homöostase und visuelle Auslesbarkeit | [Bestätigung](HOMEOSTASIS_RESULTS_20261105.md), [Populationsvergleich](HOMEOSTASIS_POPULATIONS_RESULTS_20261106.md) |
| Hippocampus und Neuromodulation | [Bildprotokoll](HIPPOCAMPAL.md), [Integration und Befunde](HIPPOCAMPAL_RESULTS_20261109.md) |
| Kleine Gewichtsänderungen und ihre Wirkung | [Gewichtsdiagnostik](WEIGHT_DIAGNOSTICS.md), [kausales Protokoll bei 256 Zellen](CAUSALITY_256.md) |
| Historische M3–M11-Reihen | [Protokollarchiv](dokumentation/versuchsreihen/README.md), [Migration](dokumentation/versuchsreihen/MIGRATION.md) |

Die früheren eigenständigen Experimentprogramme sind entfernt. Neue Studien
verwenden den Harness; vorhandene historische Startbefehle benötigen ihren
damaligen Quellstand. Ein neuer Harness-Kurzlauf repliziert oder bestätigt
keinen früheren M11-Piloten.

Der Sensorimotorik-Adapter enthält den externen Centroid-Decoder und
`mechanisms.synaptic_competition` (Standard: `false`). Konkurrenz benötigt
`mechanisms.development = false` und `mechanisms.homeostasis = false`.
[Technische Beispielkonfiguration](config/competition_smoke.toml).

## Konfiguration

```toml
[experiment]
name = "stdp_learning_test"
scenario = "m0_sequence"
seeds = [7, 19, 41, 73, 101, 149]
verify_replay = true
max_runs = 1000

[[parameters]]
name = "learning.a_plus"
values = [0.01, 0.02, 0.04, 0.08]

[[parameters]]
name = "learning.a_minus"
values = [0.01, 0.02, 0.04]

[measurements]
metrics = ["sequence_score", "stability", "firing_rate"]
```

`[base]` setzt gemeinsame Werte; Parameterachsen überschreiben diese Werte.
Beispielsweise reduziert `training_repetitions = 4` unter `[base]` die Trainings-
länge. Verschachtelte Basiswerte wie `[base.learning] a_plus = 0.02` sind möglich.
Die Achsennamen sind ausdrücklich Zeichenketten, beispielsweise
`name = "learning.a_plus"`.

Ein leeres Raster ergibt eine Variante. Seeds, Achsen, Achsenwerte, Metriken
und Kontrollnamen müssen eindeutig sein. Unbekannte Felder/Parameter/Metriken,
ungeeignete Typen, nicht unterstützte Eingriffe und doppelte effektive Varianten
durch f32-Rundung werden abgewiesen. **Alle Kombinationen** werden vor der ersten
Simulation validiert. Ungültige Kombinationen werden nicht still ausgelassen.
`max_runs` zählt inklusive Kontrollgruppen, standardmäßig höchstens 10.000;
explizit sind 1 bis 100.000 möglich.

Welche Parameter ein Szenario tatsächlich unterstützt, zeigt `list`. Im ersten
M0-Adapter sind Lernraten, STDP-Zeitskalen/-Fenster, ausgewählte Homöostase- und
Neuronparameter sowie die vorhandenen Sequenz-/Gewichts-/Zeitparameter verfügbar.
Seed und Gruppen-Schalter werden nicht als freie Parameter überschrieben.

## Kontrollgruppen und Messungen

Der M0-Adapter bietet die vorhandenen Gruppen an:

| Name | Native Gruppe | Verhalten |
|---|---|---|
| `none` | G1 | Geordnete Sequenz mit STDP |
| `disable_stdp` | G2 | Gleiche geordnete Sequenz, Gewichte eingefroren |
| `shuffle_input` | G3 | Zufällige Sequenz mit STDP |
| `enable_homeostasis` | G4 | Geordnete Sequenz mit STDP und Homöostase |

Ohne Auswahl werden alle vier ausgeführt. Eine explizite Auswahl steht unter
`[experiment]`, zum Beispiel `interventions = ["none", "disable_stdp"]`.
Dann werden die Metriken berechnet, das vollständige M0-Erfolgskriterium bleibt
wegen fehlender Kontrollen **nicht bewertet** (`passed = null`). Gleiches gilt
bei ausgeschalteter vollständiger Wiederholung (`verify_replay = false`).

`stability` bezeichnet das bestehende binäre M0-Kriterium zum Abklingen der
Aktivität; `firing_rate` ist die mittlere Populationsrate der eingefrorenen
Probe. `training_rate_hz` erfasst die Trainingsrate getrennt. Gespeichert werden
auch Treffer, falsche Übergänge, stille/persistente Zellen und Freeze-/Replay-
Prüfungen. Die angeforderten Metriken bestimmen die CSV-Zeilen und Zusammenfassung;
die JSONs und SQLite enthalten alle Metriken für die Szenarioauswertung.

Die M0-Entscheidung reproduziert das vorhandene Kriterium: vollständige Sequenz,
Nichtkollaps, Stabilität, vollständige/frozen Wiedergabe und positive Vorteile
gegenüber G2/G3 mit den bestehenden einseitigen Vorzeichentests. Gleichstände
werden dort wie bisher ausgeschlossen. Tests vergleichen die Adapterergebnisse
und Entscheidung mit dem nativen `ExperimentRunner::run_study`. Der M0-Adapter
vergleicht bei vollständiger Wiedergabe die kompletten nativen Ergebnisse
inklusive Ereignisfolgen, bevor er die kompakten Messwerte sammelt.

Eine Entscheidung gilt **je Parametervariante**. Sie enthält keine Korrektur
über das Parameterraster und keinen Nachweis auf unabhängigen Aufgaben/Seeds.
Ein M0-Befund ist kein Nachweis gelernter Handlungswirkungen aus M9–M11.
Der einfache Rasterlauf (`run`) wählt keinen Sieger und startet keine weitere
Generation. Die gesonderten Selektionsbefehle stehen in [SELECTION.md](SELECTION.md).

## Ergebnisablage

| Datei | Inhalt |
|---|---|
| `manifest.json` | Normalisierte Spezifikation, vollständiges Raster, Seeds, Eingriffe, effektive Konfigurationen, Ausführungszahlen und Build-Fingerabdruck |
| `source.toml` | Erste verwendete TOML-Datei |
| `records/*.json` | Pro Lauf Identität, alle Metriken, vollständige native Konfiguration, Anfangs-/Endgewichte, Sequenzantworten und native Ereignis-Digests |
| `metrics.csv` | Angeforderte Metriken je Variante/Seed/Kontrollgruppe |
| `summary.json`, `summary.md` | Deskriptive Zusammenfassung und Entscheidung je Variante |
| `results.sqlite` | Abfragbarer lokaler Katalog aller Varianten, Versuche, Metriken und Entscheidungen |
| `catalog-provenance.json` | SHA-256 von Manifest, Original-TOML, Frontend und ausgeführter Binärdatei |

Effektive Konfigurationen enthalten die vollständige native Rust-Debugdarstellung
einschließlich nicht frei einstellbarer Defaults. Das Build bindet die Inhalte
der NERVA-/Research-Rustquellen und Cargo-Konfigurationen/-Lockdatei an das Archiv.
Es werden keine vollständigen Ereignisprotokolle in die Datenbank kopiert;
native Digests und Konfigurationen ermöglichen eine erneute Wiedergabe.

Gleiche Ausgabeordner dürfen nur mit genau gleichem normalisiertem Plan und
Build wiederverwendet werden. Vollständige Records werden auf Identität,
Metrikvertrag und Fingerabdruck geprüft und nicht überschrieben. Fehlende Läufe
werden nachgeholt. Teilweise geschriebene Dateien werden atomar veröffentlicht;
der Ausgabeordner ist während Rust-Ausführung durch eine Betriebssystemsperre
geschützt, die auch bei Prozessabbruch freigegeben wird. Bei geänderten Parametern,
Quellen oder Defaults ist ein neuer Ausgabeordner erforderlich.

Die internen FNV-1a-Fingerabdrücke sind deterministische Prüfungen gegen
versehentliche Beschädigung, **keine kryptografische Absicherung**. SQLite
enthält zusätzlich SHA-256 jedes verwendeten Roh-Records. Host-Laufzeiten
werden separat gespeichert und nicht auf deterministische Gleichheit geprüft.
`results.sqlite` ist eine nach vollständiger Auswertung atomar neu aufgebaute
Momentaufnahme; JSONs und Manifest bleiben die maßgeblichen Originale.

Beispiel einer SQLite-Abfrage (über Python `sqlite3` oder ein SQLite-Werkzeug):

```sql
SELECT t.variant_id, t.intervention, AVG(m.value)
FROM trials AS t JOIN metrics AS m ON m.trial_id = t.id
WHERE m.name = 'sequence_score'
GROUP BY t.variant_id, t.intervention;
```

Seeds werden als Dezimaltext gespeichert, damit auch alle gültigen u64-Seeds
ohne SQLite-Integerüberlauf erhalten bleiben.

## Architektur und Erweiterung

- `spec.rs`: `ExperimentSpec`, strikte Eingabeprüfung und `VariantGenerator`.
- `scenario.rs`: `Scenario`, `Intervention` und `TrialMetrics` als Schnittstelle.
- `runner.rs`: Raster, Seeds, Replayprüfung und Auswertung ohne M0-Sonderlogik.
- `collector.rs`: Sperre, atomare Records, Integrität und abgeleitete Tabellen.
- `scenarios/m0.rs`: Konfiguration, vier native Eingriffe, Metriken und M0-Kriterium.
- `main.rs`: explizite Adapterregistrierung und JSON-CLI.
- `scenarios/m0_selection.rs`: kontrollierte Mechanismen und verzögerte Endpunkte.
- `scenarios/continual.rs`: A/B-Akquisition mit plastischen Lernphasen und eingefrorenen Proben.
- `scenarios/sensorimotor.rs`: Größen-/Rückkopplungsassay mit bestehendem Modell und persistenter Bewegungswelt.
- `scenarios/hippocampal.rs`, `scenarios/hippocampal/`: Bildpilot, Gewichts- und kausale Lerndiagnose.
- `readout.rs`: externer Centroid-Decoder des Sensorimotorik-Adapters.
- `planning/profiles.py`, `analysis/size_environment.py`: szenariospezifische Kontroll-/Bewertungsverträge und gepaarte Größenvergleiche.
- `diagnostics/`, `analysis/diagnostic_decoder.py`, `analysis/bottleneck_diagnostics.py`: externe Decoder, Kontrollgates und Fehlerhypothesen ([Details](DIAGNOSTICS.md)).
- `evolution/`: Population, Mutation, Auswahl, Bewertung, Generationen und Historie.
- `analysis/`, `planning/`, `research_memory/`, `harness.py`: gemeinsame Forschungspläne,
  Analysen, MAP-Elites, Scheduler und regelbasierte Folgeversuche ([Details](HARNESS.md)).
- `run.py`: TOML-Eingang und SQLite-Katalog, ausschließlich Standardbibliothek.

Ein neues Szenario implementiert `Scenario` mit eigenem Konfigurationstyp,
Parameterzuordnung, Eingriffen, Ausführung und optionaler wissenschaftlicher
Entscheidung. Es wird einmal an der Anwendungsgrenze registriert; Parameterraster
und neue Seeds benötigen danach keinen neuen Runner-Code. Ein zweites neutrales
Testszenario prüft diese Unabhängigkeit.

Der Rasterlauf führt Studienjobs nacheinander aus. Die externe S1/S2-Selektion
ist als eigene Anwendungsschicht implementiert. Neue biologische Regeln,
generische Sperren beliebiger Synapsen während M0 und ein KI-Agent sind keine
implementierten Suchmöglichkeiten. Insbesondere wird `BlockSynapses` nicht als scheinbar
wirksamer Eingriff angeboten: Ein weiterer Szenarioadapter muss dessen Zeitpunkt
und Semantik explizit mit der vorhandenen Runtime-Sperr-API definieren.
Der bisherige `nerva::experiment::ExperimentRunner` und die vorhandenen M0-
Aufrufe behalten ihr Verhalten. Die Referenzversuchs-API ergänzt explizite
Mechanismusschalter und zusätzliche Ruhe vor der eingefrorenen Probe für den
neuen `m0_selection`-Adapter. Die Library hängt nicht von `research` ab.

## Verifikation

```bash
cargo test --offline --manifest-path NERVA_Modell/research/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/research/Cargo.toml --all-targets -- -D warnings
python3 -W error::ResourceWarning -m unittest discover -s NERVA_Modell/research/tests -p 'test_*.py' -v
```

Geprüft werden native M0-Gleichheit, Erfolgskriterium, Rasterzählung,
Validierungsfehler, andere Szenarien, Wiederaufnahme, manipulierte Records,
Replayabweichungen, Ausgabesperre und Schutz fremder Dateien sowie TOML/SQLite
einschließlich unveränderter Rohdaten bei erneuter Auswertung.
