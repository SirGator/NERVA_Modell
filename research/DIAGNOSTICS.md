# Diagnose vor Optimierung

Stand **05.11.2026**. Forschungsziel: Mit kleinen kontrollierten Versuchen die
früheste belegbare Grenze einer Lernfähigkeit eingrenzen. M0-Parameteroptimierung
hat gegenüber dem M9/M10-Engpass und kontinuierlichem Lernen niedrigere Priorität.
Neue Lernmechanismen und Codeänderungen werden nicht automatisch eingeführt.

## Zwei Richtungen im selben Harness

| Richtung | Experiment | Stand |
|---|---|---|
| A: Repräsentation/Auslesen | Externer Diagnose-Decoder auf M9 | Implementiert und retrospektiver Pilot ausgeführt |
| A: Zeitliche Zuordnung | Handlung–Wirkung-Verzögerungen kontrolliert variieren | Geplanter Folgeversuch; M9-Aufgabenadapter noch erforderlich |
| A: Synaptische Konkurrenz | Bestehenden M11-Piloten zuerst bewerten | Vorhandene Daten nutzen; keine automatisch gestarteten weiteren Varianten |
| B: Kontinuierliches Lernen | A lernen, B lernen, A/B auf eingefrorenen Kopien testen | Nativer Adapter und technischer Kurzlauf implementiert/ausgeführt |
| B: Skalierung/Generalisierung | Kapazität, Verbindungen und überlagerte Muster | Geplant; eigener Aufgabenadapter erforderlich |

`planning/diagnostic_controller.py` führt beide Richtungen mit Voraussetzungen,
Prioritäten und Stufenübergängen. Technische Fehler führen zur Reparatur des
Versuchs. Nicht informative Piloten beenden die jeweilige Hypothese. Eine
Bestätigung wird erst nach einem informativen, unterstützenden Piloten und
mit reservierten neuen Seeds vorgeschlagen. Das Portfolio unterscheidet
ausführbare Versuche und noch erforderliche Adapter.

Raster und S1/S2-Selektion bleiben für gezielte manuelle Untersuchungen erhalten.
Die Vorschläge des Forschungsgedächtnisses priorisieren jetzt Diagnose und den
kleinen kontinuierlichen Lerntest vor weiteren M0-Karten. Der Controller führt
keine unbegrenzten Versuchsserien oder automatische Modellübernahmen aus.

## A: Diagnose-Decoder ausführen

Aus `NERVA_Modell/`:

```bash
./research/run.sh diagnose-plan research/config/diagnosis_m9.toml
./research/run.sh diagnose research/config/diagnosis_m9.toml
./research/run.sh diagnosis-report research/config/diagnosis_m9.toml
```

Python 3.11+ genügt; dieser Archivtest benötigt weder Cargo-Build noch neue
neuronale Simulationen. Das vollständige M9-Archiv muss lokal vorliegen.
Der Archivpfad wird relativ zur TOML-Datei aufgelöst.

Die Methode verwendet `ExperimentContext`, `ExperimentPlan` und `AnalysisReport`.
`diagnostics/archive.py` prüft Quellen und Proben, `analysis/diagnostic_decoder.py`
passt die externen Decoder an und `diagnostics/runner.py` historisiert den Lauf.

### Datentrennung und Kontrollen

- Explizit gewählter Fall: M9 `weight4_tau100`. Trainingsseeds 610–619,
  vollständig getrennte Testseeds 620–629. Beide Wirkungsmappings und Aktionen
  liegen in beiden Partitionen; keine zeilenweise Vermischung desselben Seeds.
- Arme: untrainiert (`before`), zeitlich/expositionsangepasst eingefroren
  (`aged_frozen`), gekoppelt trainiert (`paired`) und entkoppelt (`scrambled`).
- Merkmale: neuronweise Spikezahlen und fünf neuronweise Zeitbins während der
  Aktionsphase. Eine Aktions-only-Kontrolle und Aktionsklassifikation aus
  neuronalen Merkmalen prüfen vorhandene Aktionsinformation.
- Wirkungslabel, Mapping, Seed, feste Vorhersage und Aktivität nach dem
  Wirkungsbild sind keine neuronalen Merkmale. Potenziale mit unterschiedlichen
  letzten Aktualisierungszeiten werden nicht verwendet.
- Die M9-Adaptergeometrie identifiziert Processing-Neuronen zwischen z=-1 und
  z=1; Count-Längen und Ereignis-IDs müssen exakt dazu passen.
- SHA-256/Größe jedes JSONs werden gegen `raw-sha256.csv` geprüft. Nur unveränderte,
  eingefrorene `sham`-Proben mit unverändertem Elternzustand werden zugelassen.
  Alle Spikezeiten müssen im 50-ms-Fenster vor dem Wirkungsbild liegen;
  Ereigniszahlen und Count-Vektoren müssen übereinstimmen.

Die linearen Ridge-Proben verwenden vorab fest lambda=1. Standardisierung und
Koeffizienten werden nur aus unmittelbaren Trainingsproben berechnet. Alle 20
Modelle werden vor der ersten Testauswertung unveränderlich gespeichert.
Unmittelbare und nach Ruhe gewonnene Testproben verwenden dieselben Modelle.
Es gibt kein Nachkalibrieren und keine Decoderauswahl auf Testgüte.
Die Implementierung verwendet ausschließlich die Python-Standardbibliothek.
Methodenreferenzen: [Ridge](https://scikit-learn.org/stable/modules/generated/sklearn.linear_model.Ridge.html),
[gruppierte Datentrennung](https://github.com/scikit-learn/scikit-learn/blob/main/doc/modules/cross_validation.rst).

Alle 16 Screeningvergleiche werden gezeigt; alpha=0,05 wird durch 16 geteilt.
Ein Lernsignal benötigt positive seedgepaarte Vorteile über Zufall und über
untrainierte, eingefrorene und entkoppelte Kontrollen. Eine Lücke zum festen
Ausleser wird erst nach diesem Kontrollvergleich als Verdacht eingetragen.
Statistische Einheiten sind Seeds, nicht einzelne Spikes oder Probezeilen.

Die M9-Seeds wurden früher bereits untersucht. Dieser Test ist deshalb ein
**retrospektiver Pilot**, keine neue unabhängige Bestätigung. Negative lineare
Decodierung beweist keine Informationsabwesenheit: Nichtlineare Codes,
ungeeignete Merkmale und fehlende Ausrichtung zwischen Netzwerken bleiben offen.

### Tatsächlicher Decoder-Pilot

640 vorhandene Proben, 20 externe Decoder, **null neue neuronale Simulationen**:

| Wirkungsdecoder im gekoppelten Arm | Training | Test sofort | Test nach Ruhe |
|---|---:|---:|---:|
| Spikezahlen | 60,0 % | 50,0 % | 52,5 % |
| Zeitbins | 62,5 % | 50,0 % | 50,0 % |

Aktions-only-, untrainierte, eingefrorene und entkoppelte Wirkungskontrollen
erreichen im Test jeweils 50 %. Ein abgesicherter Lerngewinn wird nicht erkannt.
Auch neuronale Aktionsklassifikation generalisiert im gekoppelten Arm nicht
über Zufall (45,0 % sofort, 42,5 % nach Ruhe). Die positive Kontrolle des
seedübergreifenden Decoders ist damit nicht ausreichend etabliert.

Die erste Folgehypothese betrifft deshalb die Merkmalsbasis/Ausrichtung zwischen
Netzwerken: getrennte Probe-Trials innerhalb desselben Netzwerks mit einer
funktionierenden Aktionskontrolle. Der Controller deutet dies nicht als kaputte
neuronale Verarbeitung und startet keine große Bestätigungsserie.

Die nach Prüfung der positiven Kontrolle präzisierte Controller-Einordnung
verändert die Decoderwerte nicht. Der initiale Pilot ist samt damaligen Quellen
unter `results/m9_representation_diagnosis_initial/` erhalten; die aktuelle
Auswertung liegt unter `results/m9_representation_diagnosis/`.

## B: Kontinuierliches Lernen ausführen

```bash
./research/run.sh plan research/config/diagnostic_continual_smoke.toml
./research/run.sh run research/config/diagnostic_continual_smoke.toml
./research/run.sh report research/config/diagnostic_continual_smoke.toml
```

`continual_sequences` verwendet denselben Rust-ResearchRunner und NERVAs
vorhandene Simulation/Pair-STDP. Sechs erregende Musterzellen und eine
inhibitorische Zelle werden ohne aufgabenspezifische Kanten aufgebaut.
Processing-Zellen erhalten eindeutige externe Pulse; Sensorik und
Handlungswirkungen werden in diesem isolierten technischen Test nicht geprüft.

Untrainierte A/B-Proben → A=0→1→2 trainieren → A/B prüfen → B=3→1→5 trainieren →
A/B prüfen. Lernen bleibt während A und B plastisch, Proben erfolgen auf
eingefrorenen Kopien. Kontrollen: gleich langer Abschnitt ohne B, Plastizität
nach A eingefroren bei gleichen B-Eingaben und getrenntes B=3→4→5.
Alle Bedingungen besitzen je Seed denselben A-Trainingszustand und gleiche
Zeitfenster. Jede Probe und jeder ganze Studienjob werden vollständig wiederholt.

Der Score zählt zwei zeitlich korrekte Folgemuster abzüglich zusätzlicher
falscher Ausgaben; Maximum ist 2. Beide Aufgaben müssen selektiv erworben sein,
bevor ein Interferenzverdacht zulässig wird. Der native Indikator
`interference_assessable` betrifft den überlappenden Zielarm; der Python-Bericht
weist zusätzlich Akquisition und Kontrollen aus.

### Tatsächlicher technischer Lauf

Zwei neue Seeds, vier Bedingungen, **acht vollständig wiederholte Studienjobs**.
Beide Seeds zeigen dasselbe Muster:

| B-Phase | A nach A-Training | A nach B-Phase | B nach B-Phase |
|---|---:|---:|---:|
| Überlappend, Plastizität aktiv | 2 | 1 | 1 |
| Keine B-Eingaben, gleiche Dauer | 2 | 2 | 0 |
| Nach A eingefroren | 2 | 2 | 0 |
| Getrenntes B, Plastizität aktiv | 2 | 2 | 2 |

Im überlappenden Arm bleiben beide richtigen Folgemuster vorhanden, es tritt
aber jeweils eine zusätzliche falsche Ausgabe auf. Der Scoreverlust beweist
keine Zerstörung der alten Spur. Weil B nicht selektiv erworben wird, bleibt
eine Vergessensbestätigung gesperrt. Der Controller kennzeichnet
`overlap_selectivity` und schlägt einen gegenbalancierten Überlappungs-/
Kontextversuch vor. Replay-, Stabilitäts- und Freezeprüfungen bestehen.
Das kleine Ergebnis gilt für diese synthetische Aufgabe und ist kein M9-Nachweis.

## Diagnosekette und Archive

`analysis/bottleneck_diagnostics.py` führt Wahrnehmung, Verarbeitung, Lernen,
Speicherung, Auslesen und Verhalten getrennt. Fehlende Evidenz bleibt `untested`.
Hypothesen sind Verdachtsmomente mit Folgeversuch, keine automatischen Beweise.

Die Diagnosearchive enthalten `harness-manifest.json`, `frozen-decoders.json`,
`diagnosis-report.json`, `diagnosis-summary.md`, `decoder-scores.csv`,
`next-experiments.json` und `diagnosis.sqlite`. Sie binden Quellen, konkrete
Archivdateien, Datentrennung und Budget mit den vorhandenen atomaren SHA-256-Hüllen.
Reportmodus wertet dieselben eingefrorenen Modelle erneut aus.

Das A/B-Archiv liegt unter `results/continual_sequences_technical/`, mit nativen
Records/CSV/SQLite sowie `continual-analysis-protocol.json` (Quell- und
Manifestbindung) und `continual-analysis.json` (Akquisitionsgates und Diagnose).
Geänderte Quellen/Protokolle benötigen einen passenden neuen Ausgabeordner
beziehungsweise den gebundenen Programmstand.

## Verifikation

```bash
python3 -W error::ResourceWarning -m unittest discover -s research/tests -p 'test_*.py' -v
cargo test --offline --manifest-path research/Cargo.toml
cargo clippy --offline --manifest-path research/Cargo.toml --all-targets -- -D warnings
```

Tests prüfen unter anderem eine analytische Ridge-Lösung, Skalierung ohne
Testdatenzugriff, Aktionsinformation ohne Lerngewinn, synthetische
Repräsentationen, positive Kontrollen, Proben ohne Wirkungsbild, Manipulation,
Wiederaufnahme ohne Refitting, unbekannte Diagnosestufen sowie native
A/B-Akquisition, eingefrorene Kopien und unveränderte Kontrollgewichte.
