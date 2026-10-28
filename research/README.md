# NERVA Research

Eigenständige Forschungsanwendung für konfigurierbare Parameterraster, Seeds,
Kontrollgruppen und reproduzierbare Ergebnisse. Der allgemeine Rust-Runner
kennt keine M0-Neuronen, Abläufe oder Erfolgskriterien. Der erste produktive
Szenarioadapter nutzt die vorhandene M0-Sequenz unverändert.

Die [externe Selektion](SELECTION.md) ergänzt jetzt reproduzierbare Generationen,
begrenzte S1/S2-Mutationen, harte Mindestanforderungen, Pareto-Eliten und eine
getrennte Abschlussvalidierung. Sie verwendet einen eigenen kontrollierten
M0-Adapter. Start: `./NERVA_Modell/research/run.sh select NERVA_Modell/research/config/selection_m0.toml`.

Stand 28.10.2026. Voraussetzung: aktuelles Rust/Cargo und Python **3.11+**.
Rust verwendet NERVA und `serde_json`; TOML und SQLite kommen aus der Python-
Standardbibliothek. Die Builds laufen mit `--offline`. Die benötigten Cargo-
Pakete müssen wie bei den bestehenden Anwendungen lokal verfügbar sein.
NERVA selbst erhält keine neuen Bibliotheksabhängigkeiten.

## Ausführen

Vom Projektverzeichnis `DSVLM`:

```bash
# Adapter, Parameter, Metriken und Kontrollgruppen anzeigen:
./NERVA_Modell/research/run.sh list

# Vollständige Validierung und Plan, noch keine neuronalen Läufe:
./NERVA_Modell/research/run.sh plan NERVA_Modell/research/config/stdp_learning.toml

# Den gespeicherten Plan ausführen oder unterbrochene Läufe fortsetzen:
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/stdp_learning.toml

# Nur vorhandene Ergebnisse prüfen und Tabellen/Datenbank neu aufbauen:
./NERVA_Modell/research/run.sh report NERVA_Modell/research/config/stdp_learning.toml
```

Ohne `--output` liegt das Ergebnis in `research/results/<experiment.name>/`.
Dieser Bereich ist von Git ausgeschlossen. Alternativ:

```bash
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/smoke.toml --output /tmp/mein-nerva-versuch
```

Das komplette Beispiel erzeugt **12 Parametervarianten × 6 Seeds = 72
Parameter-Seed-Kombinationen**. Mit den vier M0-Gruppen sind es **288 einzelne
Versuchsläufe**. Bei der standardmäßig eingeschalteten vollständigen Wiedergabe
wird jeder Lauf zweimal ausgeführt, also 576 Adapteraufrufe. Zusätzlich gehören
zu jedem nativen M0-Lauf bereits dessen zwei interne eingefrorene Proben.

Negative wissenschaftliche Ergebnisse sind reguläre Ergebnisse. CLI-Fehler
bedeuten ungültige Eingaben, beschädigte Archive oder fehlgeschlagene Ausführung,
nicht allein ein nicht bestandenes Lernkriterium.

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
- `evolution/`: Population, Mutation, Auswahl, Bewertung, Generationen und Historie.
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
