# Nächste Forschungsphase: lokale Regeln, Größe und Umwelt

Abgleich mit dem vorhandenen Code am **05.11.2026**. Grundlage ist der
Gesamtplan „Overengineering mit anschließender experimenteller Selektion“.
Der Ausbau soll Beziehungen zwischen Mechanismen, Architektur und Lernbedingungen
untersuchen. Lokales Lernen ohne Backpropagation, Trainingslabels oder zentrale
Lerninstanz im Modell bleibt verbindlich. Externe Messungen und Selektion sind
von den lokalen Lernregeln getrennt.

## Ausgangslage und korrigierter Forschungsstand

Der gemeinsame Harness ist bereits gelaufen: Der
[kleine Durchlauf](HARNESS.md#verifikation) und der
[größere Folgetest](FOLLOWUP_20261105.md) sind dokumentiert und lokal archiviert.
Die Aussage des eingereichten Plans, der neue Harness sei noch nicht
durchgelaufen, ist daher überholt. Bestätigte M0-Kontraste und fehlender
Qualitätsgewinn gegenüber der Baseline bleiben auf ihre Aufgabe beschränkt.
Die zwölf Kandidaten sind keine nachgewiesenen kognitiven Fähigkeiten.

Es gibt drei unterschiedliche Versuchsebenen:

| Ebene | Vorhanden | Grenze für den neuen Plan |
|---|---|---|
| Native M0-Sequenz | `m0_sequence`, feste vier Kontrollen, deterministische Wiederholung | Fest acht erregende und ein hemmendes Neuron; keine freie Größenskalierung |
| Externe Forschung | `m0_selection`, Morris, 2D-Karten, 2×2-Kontraste, Parameterstress, MAP-Elites und getrennte Bestätigung; neu `sensorimotor_size` mit eigenem Bewertungsprofil | Methoden und Endpunkte müssen pro Szenario explizit unterstützt sein; bisherige M0-Selektion bleibt eigenständig |
| Sensomotorisches Modell | Rekurrenter Core, Bildsensorik, Aktionskopien, verzögerte Bewegungswelt, M3–M11-Experimente; gemeinsamer Größen-/Umweltadapter und größenunabhängige Diagnose | Positive Core-Größen mit expliziten Ressourcenbudgets; Topologie und sechs Bedingungen bis 65.536 Core-Neuronen technisch geprüft. Freie E/I-/Topologieachsen und komplexere Welten fehlen noch |

Zusätzlich misst `continual_sequences` A/B-Akquisition und Interferenz mit
plastischen Trainingsphasen und eingefrorenen Abzweigproben. Dieser Adapter
ist über `run.py run` erreichbar, aber noch nicht über den gemeinsamen Harness.
Ein Einzelrunner mit generischer `Scenario`-Schnittstelle macht die externe
M0-Selektion noch nicht szenariounabhängig.

## Mechanismenbestand

„Teilweise vorhanden“ bedeutet vorhandene Dynamik oder ein Baustein; eine
Funktion ist erst nach einem passenden kontrollierten Versuch belegt.

| Kandidat | Codebefund | Nächste konkrete Erweiterung |
|---|---|---|
| Lokale Prediction Errors | Kein expliziter lokaler Lernmechanismus für Fehler vorhergesagter Sensorzustände in den geprüften Lern-/Modellpfaden | Vorhersagezustand, Zielzeitpunkt und lokal zugängliches Fehlersignal definieren; Fehlerlogging von Fehlerlernen trennen |
| Eligibility Traces | `NERVA/src/learning/utility.rs`, `core/synapse.rs`: synapsenlokale Utility-Eligibility; zusätzlich STDP-Spuren in `learning/traces.rs` | Vorhandene Spuren wiederverwenden; verzögerte lokale Fehlerzuweisung separat definieren und gegen Zeitversatz testen |
| Synaptische Konkurrenz | `learning/competition.rs`, Runtime-Integration, expliziter Modellschalter und M11 | An neuen Adapter anbinden; bestehende Pool-Mitgliedschaft und anfängliche Budgets erhalten. Aktuell nur bei ausgeschalteter Entwicklung und Homöostase zulässig |
| Adaptive Inhibition | Hemmende Zellen/Kanten, motorische Konkurrenz und zelluläre Homöostase vorhanden | Eigene lokale hemmende Plastizitätsregel fehlt; Homöostase und feste Hemmung nicht als Ersatz ausgeben |
| Arbeitsgedächtnis-Dynamik | Membranzeitkonstanten, Rekurrenz, Burst-, Adaptations- und Rebound-Dynamik (`core/intrinsic.rs`) | Verzögerte Cue-Aufgabe mit Störung und zeitaufgelöstem Auslesen; nutzbare Information statt bloß anhaltender Aktivität messen |
| Memory Consolidation | Utility-Gedächtnis für Strukturentscheidungen und eingefrorene Retentionsproben vorhanden | Keine explizite Konsolidierungsregel belegt; Erhalt unter weiterlaufender Plastizität und Interferenz prüfen |
| Replay/Reaktivierung | `run_verified` und Replayprüfungen wiederholen Versuche technisch deterministisch | Eigene interne Reaktivierung fehlt; technische Wiedergabe ist kein neuronaler Replaymechanismus |
| Intrinsische Motivation | Intrinsische Ströme/Erregbarkeit vorhanden | Keine daraus folgende interessenabhängige Handlungswahl implementiert; lokale Neuheits-/Fortschrittssignale und ihre Handlungskopplung entwickeln |
| Learning Progress | Kein entsprechender Regelpfad in den geprüften Mechanismen | Zeitliche Fehlerverbesserung lokal schätzen, lernbare Dynamik gegen Rauschen und passive Signale kontrollieren |
| Funktionales Wachstum/Pruning | `development/`: lokale Kandidaten, atomare Strukturpläne, Utility-basiertes Pruning; Modellintegration vorhanden | Bestehende Regeln exponieren; funktionalen Nutzen getrennt von Gewichtsgröße messen; inkompatible Kombinationen vorab ablehnen |
| Mehrere Plastizitäts-Zeitskalen | STDP, Homeostase, Utility-Eligibility/-Gedächtnis und Strukturentwicklung haben eigene Zeitskalen | Gemeinsame Adapterparameter und Zeitachsenvergleich; Utility-Erhalt nicht mit Konsolidierung gleichsetzen |
| Sensorimotorische Vorhersage | Aktionskopien, rückgekoppelte Bewegungswelt und kontrollierte M3–M11-Proben vorhanden | An gemeinsamen Adapter anbinden; trainiert/ungeübt, aktiv/passiv und Wirkungstausch vergleichen. Bisherige M9/M10-Diagnosen belegen keine zuverlässig erlernten Handlungswirkungen |

Unabhängige Schalter sind ein Ausbauziel. Bestehende Inkompatibilitäten werden
als Validierungsfehler erhalten, bis ihre Semantik durch neue Implementierung
und Prüfungen abgesichert ist. Zwölf boolesche Schalter allein ergäben 4.096
Kombinationen, bereits vor Größen, Umwelten, Parametern und Seeds. Deshalb
zuerst einzelne Mechanismen und kleine faktorielle Designs untersuchen.

## Erste implementierte Etappe: feste M0-Referenz

[phase1_baseline.toml](config/phase1_baseline.toml) verwendet unveränderte native
M0-Defaults, ohne Basisüberschreibungen, Parameterraster oder Selektion. Sechs
historische Seeds `[7, 19, 41, 73, 101, 149]` dienen absichtlich der Regression;
sie sind keine neuen Bestätigungsseeds. Alle vier nativen Kontrollgruppen und
alle zwölf vorhandenen M0-Metriken sind fest eingetragen. Das ergibt 24 Jobs
und mit vollständiger Wiederholung 48 native Ausführungen; die nativen internen
eingefrorenen Proben sind darin enthalten.

[baseline.py](baseline.py) führt den bestehenden Runner aus, prüft anschließend
im Reportmodus alle Records und erstellt einen unveränderlichen SHA-256-Beleg
für Konfiguration, Original-TOML, Manifest, Binärdatei und jeden Roh-Record.
Er hält außerdem den nativen Quellfingerabdruck, den Verifikatorhash, den
Metrikvertrag, Jobzahlen und die native M0-Entscheidung fest. Die erneute
Auswertung muss dieselben Rohdateihashes ergeben; SQLite wird auf Integrität
und Trialzahl geprüft. Ein negatives M0-Ergebnis darf regulär archiviert werden.

Vom Projektverzeichnis `DSVLM`:

```bash
python3 NERVA_Modell/research/baseline.py run --output /tmp/nerva-m0-referenz
python3 NERVA_Modell/research/baseline.py verify \
  --binary NERVA_Modell/research/results/phase1_m0_baseline_20261105/reference-runner
```

`run` setzt unvollständige, noch nicht versiegelte Versuche mit dem bestehenden
Runner fort. Bei bereits versiegelten Archiven führen beide Modi nur die
erneute Auswertung aus. `verify` verlangt einen bestehenden Beleg und startet
keine fehlenden Trials. Abgeleitete Tabellen und SQLite werden dabei neu
aufgebaut. Geänderte Records, Quellenfingerabdrücke, Binärdateien oder Protokolle
werden abgewiesen. Nach einer Änderung einen neuen `--output`-Ordner verwenden;
den alten Beleg behalten. `--binary` kann eine vorhandene Research-Binärdatei
angeben; sonst erfolgt der vorhandene Offline-Release-Build.

Archiv: [results/phase1_m0_baseline_20261105/](results/phase1_m0_baseline_20261105/).
Es enthält `manifest.json`, `source.toml`, `records/`, `summary.json`,
`summary.md`, `metrics.csv`, `results.sqlite` und `baseline-receipt.json`.
Der Beleg erkennt Änderungen; für spätere Reproduktion müssen der zugehörige
Quellstand und die Build-Abhängigkeiten weiterhin verfügbar sein.

Diese erste Etappe sichert die native M0-Referenz und den Codeabgleich. Danach
wurde die [sensomotorische Modellreferenz samt Größen-/Umweltadapter](SENSORIMOTOR.md)
ergänzt: bestehende neuronale Regeln, diagnostische Bildsensorik, persistente
Bewegungswelt und 256-/1.024-Core-Vergleich. Allgemeine Fähigkeitsassays und
die weiteren Umweltstufen bleiben offen.

## Metrikvertrag für den Ausbau

Jeder Endpunkt erhält Einheit, Zeitfenster, Messzeitpunkt und passende
Kontrollgruppe. Nicht vorhandene Endpunkte bleiben „nicht gemessen“;
fehlgeschlagene Messvoraussetzungen werden nicht als Score null verborgen.

| Bereich | Heute nutzbar | Noch zu implementierender Messvertrag |
|---|---|---|
| Vorhersage | M0-Sequenztreffer und Fehlübergänge; bestehende sensorimotorische Proben | Fehler zukünftiger Sensorsignale je Horizont; Persistenz- und Zufallskontrolle |
| Gedächtnis | Eingefrorene M0-Retention; A/B-Akquisitions-/Interferenzprobe | Verzögerungskurve unter Plastizität und Störung; Interferenz erst nach nachgewiesener Akquisition beider Aufgaben bewerten |
| Kausalität | Aktionskopie-/Gewichts-/Rekurrenzeingriffe in bestehenden Reihen | Gleicher Sensorverlauf mit eigener, passiver und entkoppelter Aktion; Intervention vor dem Wirkungsbild |
| Generalisierung | Getrennte Seeds derselben M0-Aufgabe | Vorab getrennte Objekt-, Dynamik- und Aufgabenverteilungen; Seeds allein sind keine neue Aufgabe |
| Transfer | Noch kein allgemeiner Endpunkt | Vortrainiert gegen frisches Netz bei gleichem Lernbudget auf Zielaufgabe |
| Selbstorganisation | Gewichte, Aktivität, Struktur-/Utility-Audits | Vorab definierte Gruppenstabilität und Spezialisierung, mit Aktivitäts-/Topologiekontrollen |
| Adaptivität | A/B-Sequenzen und einzelne Parameter-Stresstests | Kontrollierter Regelwechsel innerhalb einer Episode; Erholungskurve statt unabhängiger frischer Netze |
| Stabilität | Raten, stille/persistente Zellen, Abklingen, endliche Werte | Dieselben Endpunkte über Lernzeit, Größen und mehrere Seeds |
| Effizienz | Ereigniszähler im Selektionsadapter, Hostzeit in Records | Synapsenzahl, Speicherbedarf und klar definierte Operationszähler; Ereignisse nicht als FLOPs ausgeben |

Der neue Adapter soll Lernkurven an festgelegten Zwischenständen liefern.
Bestehende Endpunktmessungen werden nicht nachträglich als vollständige
Lernkurven bezeichnet. Externe Decoder können Information diagnostizieren;
ihre Trainingsziele dürfen nicht in die lokalen Regeln zurückfließen.

## Reihenfolge und Abnahmekriterien

| Schritt | Konkrete Arbeit | Abschlusskriterium / Stand |
|---|---|---|
| 1. Baseline sichern | Mechanismenbestand, feste M0-Referenz, Metrikvertrag und sensomotorische Modellreferenz | M0-Referenz und Modellreferenz mit diagnostischer Sensorik implementiert und gemessen |
| 2. Größe und Umwelt | Neuer `Scenario`-Adapter mit vorhandenen lokalen Regeln und persistenter einfacher Bewegungswelt; szenariospezifische Harnessbewertung | Technischer Erstlauf mit 256 und 1.024 Verarbeitungsneuronen, gepaarten aktiv/passiv/entkoppelt-Kontrollen, Wiederholung und Budget abgeschlossen; Ergebnisse noch kein Lernnachweis |
| 3. Mechanismen-Pool | Zuerst vorhandene Utility-/Konkurrenz-/Strukturregeln anbinden; danach lokale Vorhersagefehler, verzögerte Fehlerzuweisung und Arbeitsgedächtnisassay | Jeder Schalter hat einen wirksamen Eingriff, einen deaktivierten Referenzpfad, Parametergrenzen und eine passende Kontrollaufgabe |
| 4. Kombinationen | Günstiges Screening, anschließend vorab gewählte Paare und Interaktionen; kein blindes vollständiges Raster | Gepaarte Kontraste und Interaktionen je Metrik; Trainings- und Prüfseeds bleiben getrennt |
| 5. Selektion/Vereinfachung | Pareto-/QD-Archiv erweitern, redundante Mechanismen prüfen | Vergleich bei gleichem Budget; akute Ablation eines trainierten Netzes und Neuaufbau/Neutraining ohne Mechanismus getrennt archivieren |
| 6. Wissenschaftliche Bestätigung | Hypothesen, Effektgrößen, Testfamilien und Erfolgskriterien vorab fixieren; neue Aufgaben und Seeds | Unabhängige Wiederholung mit kontrollierten Interventionen; keine nachträgliche Auswahl anhand der Bestätigungsdaten |

Der Adapter interpretiert „256 Neuronen“ als Coregröße und speichert zusätzlich
die Gesamtgröße von 267 Neuronen. Die [Skalierungserweiterung](SCALING.md) erlaubt
jetzt auch 4.096, 16.384, 65.536 und andere positive Größen innerhalb des Budgets.
Räumliche Kandidatensuchen sind indexiert; Funktionalität, Ressourcenaufwand und
Umweltabhängigkeit müssen weiterhin getrennt untersucht werden.

Die Umweltstufen werden schrittweise ergänzt: einfache persistente Dynamik,
Kollisionen, Verdeckung/Navigation, mehrstufige verzögerte Wirkungen, mehrere
Akteure und Regelwechsel. Die vorhandene diskrete Bewegungswelt bietet einen
Ausgangspunkt, aber keine allgemeine Kollisionsphysik oder fertige fünfstufige
Welt. Sensorauflösung, Encoder, Rauschen, Aktionszahl, Aktionskopie, Verzögerung
und aktive/passive Wahrnehmung bleiben separate Achsen; komplexere Stufen
ersetzen diese kontrollierten Einzelvergleiche nicht.

## Verifikation der ersten Etappe

```bash
python3 -W error::ResourceWarning -m unittest discover \
  -s NERVA_Modell/research/tests -p 'test_baseline.py' -v
```

Die Tests führen die echte native Referenz aus und prüfen das feste Protokoll,
erneute Prüfung ohne neue Trials, unveränderte Rohdaten/Belege sowie Abweisung
veränderter Metriken, fehlender/zusätzlicher Records, geänderter Manifeste und
beschädigter Belegprüfsummen. Ein fehlendes Archiv wird durch `verify` nicht
angelegt. Der erste Messbericht steht in [PHASE1_BASELINE.md](PHASE1_BASELINE.md).
