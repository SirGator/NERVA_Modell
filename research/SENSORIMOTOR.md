# Größen- und Umweltadapter für das sensomotorische Modell

Stand **05.11.2026**. `sensorimotor_size` bindet das bestehende rekurrente
`nerva_vm::model::Model` an die generische Rust-Schnittstelle `Scenario` und
den gemeinsamen Forschungs-Harness an. Neuronendynamik, STDP, Utility,
Homöostase, Wachstum und Pruning werden unverändert aus dem Modell verwendet.
Es werden keine Trainingslabels, Decoderergebnisse oder Belohnungen in das
Modell zurückgeführt. QEMU wird für diesen kontrollierten Versuch nicht benötigt.

Erweiterung: [frei skalierbare Größen und derselbe Diagnoseversuch je Größe](SCALING.md).
Die frühere Modellgrenze 16–1.024 ist durch positive Größen mit expliziten
Ressourcenbudgets ersetzt. Die folgenden 256-/1.024-Beispiele bleiben historische
Startprotokolle.

## Ausführen

Vom Projektverzeichnis `DSVLM`:

```bash
# Feste Modellreferenz: 256 Core-Neuronen, sechs Bedingungen, zwei Seeds.
./NERVA_Modell/research/run.sh plan NERVA_Modell/research/config/sensorimotor_baseline.toml
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/sensorimotor_baseline.toml
./NERVA_Modell/research/run.sh report NERVA_Modell/research/config/sensorimotor_baseline.toml

# Gemeinsamer Harness: 256 und 1.024 Core-Neuronen, gepaarte Bedingungen.
./NERVA_Modell/research/run.sh research-plan NERVA_Modell/research/config/sensorimotor_sizes.toml
./NERVA_Modell/research/run.sh research NERVA_Modell/research/config/sensorimotor_sizes.toml
./NERVA_Modell/research/run.sh research-report NERVA_Modell/research/config/sensorimotor_sizes.toml
```

Die neuen TOMLs sind ausführbar. Geänderte Quellen oder Konfigurationen benötigen
einen neuen `--output`-Ordner. `report` und `research-report` verifizieren vorhandene
Records, bauen abgeleitete Tabellen neu auf und starten keine fehlenden Trials.
Die Ergebnisse dieses ersten Laufs stehen in
[SENSORIMOTOR_RESULTS_20261105.md](SENSORIMOTOR_RESULTS_20261105.md).

## Referenzmodell und Welt

Die Referenz verwendet `ModelConfig::default()` mit zwei expliziten
Protokollentscheidungen: A/D-Motorlayout und deterministische, aus dem aktuellen
Bild berechnete `Position7Ch`-Sensorik. Der Sensor benutzt keine Weltzustände,
Aktionslabels oder Zukunftsbilder. Dies ist eine Modellbaseline mit diagnostischer
Sensorik, keine Baseline autonomer Rohpixelverarbeitung in einer VM.

Die Größe bezeichnet **Verarbeitungsneuronen**. Sie wird zusammen mit der
Gesamtgröße gespeichert: 256 → 267 und 1.024 → 1.035 Neuronen, einschließlich
sieben Sensor-, zwei Motor- und zwei Aktionskopiezellen. Anfangsverdrahtung,
initiale Synapsenzahl und tatsächlicher E/I-Anteil werden ebenfalls protokolliert.
Die räumliche Anordnung und die E/I-Rundung ändern sich mit der Größe; dies ist
kein isolierter Vergleich abstrakter Neuronenzahlen bei identischer Geometrie.

Die bestehende siebenpositionige `MotionWorld` bleibt während Training und Probe
erhalten. Es gibt keinen Reset pro Aktion. Jeder Viererblock führt die Welt
vom Mittelpunkt zurück zum Mittelpunkt, ohne die Randbegrenzung zu erreichen.
Zwischen Training und Probe zeigt die getrennte visuelle Kalibration andere
statische Bilder; sie verändert die gespeicherte Position dieser Welt nicht.
Das ist eine persistente diskrete Bewegungswelt mit konsistenten A/D-Regeln,
keine Kollisionsphysik und keine offene dynamische Umgebung.

## Sechs gepaarte Bedingungen

| Bedingung | Training | Probe | Plastizität im Training |
|---|---|---|---|
| `active` | A/D-Kommando mit Aktionskopie, Bewegung folgt dem Kommando nach Verzögerung | Gekoppelte Bewegung mit Aktionskopie | Vorhandene STDP-, Utility-, Homöostase- und Strukturregeln |
| `active_frozen` | Gleiche Kommandos, Bilder und Zeitpunkte | Wie `active` | Alle Plastizität deaktiviert |
| `passive` | Dieselben Bilder und Zeitpunkte wie `active`; keine ausgeführten eigenen Kommandos an den Modellport | Gekoppelte externe Bewegung ohne Aktionskopie | Vorhandene Regeln |
| `passive_frozen` | Wie `passive` | Wie `passive` | Alle Plastizität deaktiviert |
| `decoupled` | Kommandos und Bewegungsrichtung sind innerhalb jedes Viererblocks vollständig gekreuzt | Gekoppelte Bewegung mit Aktionskopie | Vorhandene Regeln |
| `decoupled_frozen` | Wie `decoupled` | Wie `decoupled` | Alle Plastizität deaktiviert |

Die Exploration ist vorgeschrieben und balanciert; die neuronalen Motorvorschläge
bestimmen diese Versuchsbilder nicht. „Aktiv“ bezeichnet hier eigene ausgeführte
A/D-Kommandos mit Rückkopplung, keine autonome Handlungsstrategie. Motorneuronen
und ihre vorhandenen internen Rückkopplungen bleiben in allen Armen vorhanden.
Die entkoppelte Bedingung erhält denselben Bildverlauf und dieselben
Aktionshäufigkeiten, verändert aber deren Zuordnung. Sie ist kein weißes Rauschen;
zeitliche Regelmäßigkeiten bleiben möglich.

Der Harness prüft pro Seed identische Anfangsnetze und identische Bildverläufe
mit identischen Aktions-/Wirkungszeitpunkten. Passive Arme dürfen keine
Aktionskopieankünfte erhalten; eingefrorene Trainingsarme dürfen weder
Gewichts-/Homöostaseupdates noch Wachstum/Pruning aufweisen.

## Messung ohne Zukunftsleck

Eine Aktion hat drei Abschnitte: 50 ms aktuelles Bild, die konfigurierbare
Aktionsverzögerung bei weiterhin altem Bild und 50 ms Wirkungsbild. Die
Vorhersage wird aus den Core-Spikezahlen während der Verzögerung berechnet,
**bevor** die Welt die Bewegung ausführt oder das Wirkungsbild gelesen wird.
`forecast_us < effect_us` und der tatsächliche Zeitabstand werden protokolliert.

Nach dem Training wird die gesamte Plastizität eingefroren. Ein externer
visueller Zentroiddecoder wird auf sieben statischen Positionsbildern kalibriert
und anschließend auf sieben getrennten Präsentationen geprüft. Er erhält
keine Aktions-/Zukunftspaare als Trainingsbeispiele. Danach liest derselbe Decoder
die Vorhersageaktivität aus. Sein Ausleseergebnis gelangt ausschließlich zum
Evaluator. Die sieben visuellen Prüfbeobachtungen sind eine kleine technische
Messvoraussetzung, kein belastbarer Generalisierungsnachweis.

Die Parameter des trainierten Netzes werden vor und nach der gesamten
Kalibration/Probe exakt verglichen. Die vollständige Wiederholung startet das
Modell erneut und vergleicht sämtliche Metriken und deterministischen Details,
einschließlich Ereignis- und Netzwerkdigests.

| Endpunkt | Bedeutung / Einheit |
|---|---|
| `prediction_accuracy` | Anteil richtig vorhergesagter nächster sichtbarer Positionen |
| `position_mae` | Mittlerer absoluter Positionsfehler, in Gitterzellen |
| `persistence_mae` | Fehler der konstanten Vorhersage „Position bleibt gleich“, gleiche Einheit |
| `majority_accuracy` | Häufigste tatsächliche Zielposition in der Probe als deskriptive Vergleichsgrenze; keine trainierte neuronale Policy |
| `visual_accuracy` | Genauigkeit des externen Auslesers bei der getrennten visuellen Kalibration |
| `core_rate_hz`, `peak_core_rate_hz`, `silent_fraction` | Mittlere/populationsweite Spitzenrate und Anteil während des gesamten Assays stiller Core-Zellen |
| `observed_events` | Anzahl beobachteter nativer Ereignisse im gesamten Assay; Aufwandindikator, keine FLOPs |
| `initial_synapses`, `final_synapses`, `maximum_synapses` | Strukturumfang; zusätzlich Gesamt-/Core-Neuronenzahl und hemmender Core-Anteil |
| `weight_updates`, `homeostasis_updates`, `formed`, `pruned` | Änderungen der bestehenden lokalen Regeln während des Trainings |
| `action_copy_arrivals`, `parameters_preserved` | Kontrollnachweise für Rückkopplung und eingefrorene Messung |

Die Roh-Records enthalten Aktivität, Struktur und Ereigniszähler je Zeitfenster
als `training_curve` und `measurement_windows`. Das sind noch keine
Vorhersage-Lernkurven an unabhängigen Trainingszwischenständen. Hostzeit wird
wie bisher separat im Record gespeichert; Speicherverbrauch wird nicht gemessen.
Die Raten-/Endlichkeitsgrenzen operationalisieren Aktivitätsstabilität und
beweisen keine allgemeine dynamische Stabilität.

## Szenarioprofile im gemeinsamen Harness

`planning/profiles.py` trennt Kontrollgruppen, Messgrößen, Voraussetzungen,
Budgetfaktor und externe Abschlussentscheidung vom Forschungscontroller.
`m0_selection` behält seine vier Kontrollen und bestehende Bewertungsregel.
`sensorimotor_size` verwendet sechs Kontrollen; ein Job enthält eine
Modellausführung mit vollständiger Wiederholung, also zwei Ausführungen statt
der vier nativen Trainingsdurchläufe eines M0-Selektionsjobs mit zwei Endpunkten.

`size_environment` und `plasticity_diagnostics` sind die freigegebenen
Analysemethoden des neuen Profils. Unpassende M0-Verfahren werden vor dem ersten Versuch abgelehnt.
Neue Szenarien müssen ihre Endpunkte und Policies explizit registrieren; sie
erhalten keine erfundenen M0-Metriken. Die bisherige separate externe
Selektionsanwendung (`select`) bleibt bei M0.

Die Auswertung enthält je Größe und Bedingung alle separaten Endpunkte,
Seedwerte und Kontrollkontraste. Für die optionale Finalistenauswahl dient
ausschließlich der aktive Genauigkeitsgewinn gegenüber der zugehörigen Kontrolle
ohne Plastizität als Zielgröße. Es wird kein allgemeiner Kognitionsscore gebildet.
Aktiv-minus-passiv und aktiv-minus-entkoppelt vergleichen die jeweiligen
Lerngewinne; sie bleiben explorativ. Der Passivkontrast umfasst die Entfernung
der Aktionskopie sowohl während Training als auch während Probe.

Die Mindestanforderungen gelten für alle sechs Bedingungen und jeden Seed:
vollständige Wiederholung, erhaltene Messparameter, endliche/beschränkte
Core-Aktivität, nicht vollständig stummer Core, Ereignis-/Synapsenbudgets,
gepaarte Erfahrung und visuelle Auslesegenauigkeit mindestens 0,5. Negative
Messwerte bleiben im Archiv. Bei verletzten Voraussetzungen ist `quality=null`,
während die deskriptiven Rohwerte erhalten bleiben.

Ein geeigneter Finalist wird vor dem ersten Zugriff auf getrennte Prüfseeds
eingefroren. Seine Abschlussprüfung gegen Baseline verlangt bestandene Gates,
positiven kontrolladjustierten Gewinn, ein positives unteres 95-%-Bootstraplimit,
zwei gepaarte einseitige Vorzeichentests mit Schwelle `alpha/2`, eingehaltenes
Ressourcenverhältnis sowie einen Vorteil gegenüber Mehrheits- und
Persistenzvergleich. Die beiden letzten Vergleiche sind zusätzliche
Mindestbedingungen, keine gesonderten Bestätigungsbehauptungen. Ist kein Kandidat
geeignet, bleiben die Prüfseeds unbenutzt.

## Validierung und Herkunft

Der native Build-Fingerabdruck enthält jetzt auch die Rust-Quellen und
Cargo-Manifeste von `Modell` und `nerva-qemu`, neben Research und NERVA. Der
Research-Lockfile bindet den tatsächlich verwendeten Abhängigkeitsgraphen.
Die Python-Harnessidentität enthält das neue Profil und die neue Analysemethode.
JSON-Hüllen, Rohdateihashes, Ausgabe-Locks und SQLite-Kataloge werden aus dem
vorhandenen Runner wiederverwendet.

Die alte M0-Referenz wurde vor dem neuen Build zusätzlich als
`results/phase1_m0_baseline_20261105/reference-runner` und
`reference-sources.zip` gesichert. Sie lässt sich mit ihrem alten Programmstand
weiter prüfen; ihre Belege werden nicht auf den neuen Adapterstand umgeschrieben.

```bash
cargo test --offline --manifest-path NERVA_Modell/research/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/research/Cargo.toml --all-targets -- -D warnings
python3 -W error::ResourceWarning -m unittest discover \
  -s NERVA_Modell/research/tests -p 'test_*.py' -v
```

Zusätzliche Tests prüfen echte 1.024-Core-Läufe, deterministische Wiederholung,
identische Bilder/Anfangsnetze, vollständig gekreuzte entkoppelte Aktionen,
Zeitordnung, eingefrorene Regeln, fehlende passive Aktionskopien, Ereignisbudget,
sechsfaches Harnessbudget, getrennte Prüfseeds, erneute Auswertung ohne neue
Trials, beschädigte Archive und Abweisung ungültiger später Designpunkte.
Synthetische positive und negative Endpunkte prüfen die neue externe Policy;
sie sind keine NERVA-Messergebnisse.

## Integrierte synaptische Konkurrenz

`mechanisms.synaptic_competition` aktiviert den vorhandenen lokalen Mechanismus
im selben Adapter; Standard ist `false`. Voraussetzung sind deaktivierte
Strukturentwicklung und Homöostase. Größe, Kontrollen, Budgets, externe Auswertung
und vollständiger Replay bleiben die des Harness.
[Technische Konfiguration](config/competition_smoke.toml). Dieser Größenvergleich
ist kein Ersatz für das historische M11-Protokoll oder dessen Bestätigung.
