# Skalierung und größenunabhängige Plastizitätsdiagnose

Am **05.11.2026** umgesetzt und geprüft. Positive Core-Größen sind frei
konfigurierbar; dieselbe parametrisierte Prüfsuite und derselbe 2×2-Diagnoseversuch
werden auf jede angegebene Größe angewendet. Die bisherigen Lernregeln bleiben
erhalten. Die technischen Prüfungen mit 65.536 Core-Neuronen bestehen.

## Tatsächlich ausgeführte technische Prüfungen

- Research: 15 Rust-Tests bestanden, darunter dieselben Rückkopplungs- und
  Diagnoseprüfungen für 1, 17, 65, 257 und 1.025 Core-Neuronen.
- Zusätzlich dieselben sechs Rückkopplungsbedingungen mit vollständiger
  Wiederholung für **4.096 und 65.536** Core-Neuronen bestanden. Der zusätzliche
  Lauf dieser parametrisierten Prüfung dauerte auf diesem Host etwa 65 Sekunden;
  daraus wird keine allgemeine Laufzeitgarantie abgeleitet.
- Topologieinvarianten auch mit **65.536** Core-Neuronen geprüft.
- Python: alle **41 Tests** bestanden. Die sechs szenariospezifischen Tests
  zusätzlich mit `NERVA_TEST_SIZES=1,37,1025` ausgeführt und bestanden.
- Modellregressionen bestanden; die abschließenden Index-/Skalierungsprüfungen
  bestehen ebenfalls. Der explizite QEMU-Integrationstest bleibt wie bisher
  ausgelassen, weil die neue Prüfung keine VM benötigt.
- Fünf Tests des lokalen Kandidatenscorers mit aktiviertem `development`-Feature
  bestanden. Clippy ist für NERVA, Modell und Research jeweils mit
  `-D warnings` sauber.
- Die vor dem Umbau gesicherten Netzwerkdigests für 17 und 256 Core-Neuronen
  stimmen für Anfang, Training und Ende unverändert überein.

Diese Prüfungen belegen Größen-/Kontrollsemantik und Reproduzierbarkeit.
Sie verlangen keinen vorgegebenen kognitiven Leistungswert bei jeder Größe.

## Explorative Diagnose mit ursprünglicher Trainingslänge

[plasticity_diagnosis.toml](config/plasticity_diagnosis.toml) untersucht
17, 256, 1.025 und 4.096 Core-Neuronen. Für jede Größe werden Strukturentwicklung
und Homöostase vollständig gekreuzt; STDP bleibt eingeschaltet. Jede Kombination
erhält dieselben sechs Rückkopplungs-/Plastizitätskontrollen. Acht Trainings-
und acht Probekommandos entsprechen der Länge des vorherigen Größenversuchs.

Neue Explorationsseeds: `1410000001`, `1410104730`.
Getrennte, vorab festgelegte Prüfseeds: `1420000011`, `1420104740`.
Alle Varianten derselben Größe haben identische Anfangsnetze und Bildverläufe;
dies wurde vor der Kontrastberechnung geprüft.

Visuelle Prüfgenauigkeit des aktiven Arms, Mittel über beide Explorationsseeds:

| Core-Größe | Entwicklung aus, Homöostase aus | Entwicklung aus, Homöostase an | Entwicklung an, Homöostase aus | Entwicklung an, Homöostase an |
|---:|---:|---:|---:|---:|
| 17 | 78,57 % | 50,00 % | 71,43 % | 50,00 % |
| 256 | 78,57 % | 57,14 % | 78,57 % | 57,14 % |
| 1.025 | 64,29 % | 28,57 % | 57,14 % | 28,57 % |
| 4.096 | 57,14 % | 28,57 % | 50,00 % | 21,43 % |

Auf diesen zwei Seeds ist der visuelle Ausleser mit eingeschalteter Homöostase
im Mittel schwächer. Bei 256 Core-Neuronen verändert der Entwicklungsschalter
die mittlere visuelle Prüfgenauigkeit bei festem Homöostasestatus nicht.
Das zuvor vermutete Pruning erklärt den beobachteten Ausleseverlust damit
in diesem kleinen Vergleich nicht allein. Dies sind explorative totale
Interventionseffekte; es wird keine allgemeine Schädlichkeit der Homöostase
oder Wirkungslosigkeit von Entwicklung behauptet.

Synapsenzahlen nach dem Training, Mittel im aktiven Arm:

| Core-Größe | Anfang | Entwicklung aus, beide Homöostasestände | Entwicklung an, Homöostase aus | Entwicklung an, Homöostase an |
|---:|---:|---:|---:|---:|
| 17 | 124 | 124 | 146 | 117,5 |
| 256 | 1.080 | 1.080 | 282 | 271,5 |
| 1.025 | 4.156 | 4.156 | 900,5 | 888,5 |
| 4.096 | 16.440 | 16.440 | 3.356,5 | 3.347 |

Das Abschalten der Strukturentwicklung verhindert erwartungsgemäß Wachstum und
Pruning, das Abschalten der Homöostase deren Updates. Entwicklung umfasst beide
Strukturregeln; ein isolierter Pruningeffekt wurde damit nicht gemessen.
Alle seedgepaarten Effekte, Interaktionen, Fehlermetriken und Gates stehen im
[Diagnosebericht](results/plasticity_diagnosis_20261105/reports/plasticity_by_size.json).

## Separate Abschlussprüfung

Der nach der vorhandenen Trainingspolicy eingefrorene Finalist verwendet 256
Core-Neuronen mit Entwicklung und Homöostase. Die optionale Abschlussprüfung
prüft ausschließlich seinen kontrolladjustierten aktiven Vorhersagegewinn
gegen die vorab festgelegte 17er-Baseline, nicht die obigen Mechanismenkontraste.

Sie bestätigt keine Verbesserung: `held_out_quality_improved=false`. Unter
anderem verfehlt die Baseline das visuelle Gate; der Finalist übertrifft die
naiven Vergleichsgrenzen nicht zuverlässig und erfüllt die übrigen
Bestätigungsanforderungen nicht. Mit zwei Prüfseeds ist das Protokoll ohnehin
eine technische Prüfung und kein belastbarer Bestätigungsversuch.

## Technische Vorprüfung und Datenintegrität

Der zuvor ausgeführte [Kurzlauf](config/plasticity_scaling.toml) verwendet
dieselben Größen und dasselbe Raster mit vier statt acht Trainings-/Probekommandos.
Er hat eigene Explorations-/Prüfseeds (`131…` beziehungsweise `132…`) und dient
der technischen Vorprüfung. In dieser kürzeren Phase tritt das starke Pruning
der ursprünglichen Acht-Kommandos-Messung noch nicht auf. Seine Ergebnisse
werden deshalb nicht mit dem längeren Diagnoseprotokoll vermischt.

Jeder der beiden Läufe umfasst 16 Explorationskonfigurationen mit 192 Jobs
und zwei getrennte Prüfkonfigurationen mit 24 Jobs: **216 Records und
432 vollständige Modellausführungen einschließlich Wiederholung je Lauf**.
Die Explorationskontraste bleiben unabhängig von der optionalen Abschlussprüfung.

- [Diagnosearchiv mit acht Kommandos](results/plasticity_diagnosis_20261105/):
  [Bericht](results/plasticity_diagnosis_20261105/reports/plasticity_by_size.json),
  [Abschlussentscheidung](results/plasticity_diagnosis_20261105/final-result.json),
  [Prüfnachweis](results/plasticity_diagnosis_20261105/verification-receipt.json).
- [Technisches Kurzarchiv](results/plasticity_scaling_20261105/):
  [Bericht](results/plasticity_scaling_20261105/reports/plasticity_by_size.json),
  [Prüfnachweis](results/plasticity_scaling_20261105/verification-receipt.json).

Beide Archive wurden im Reportmodus ohne neue neuronale Trials erneut geprüft.
Alle 432 Roh-Records behalten ihre SHA-256-Werte; sämtliche zugehörigen
SQLite-Kataloge bestehen die Integritätsprüfung. Die Prüfnachweise binden
Manifest, ausgeführte Binärdatei, Rohdateien und gespeicherte Verifikationslogs.
Unter `verification/` liegen die Test-/Clippy-Protokolle einschließlich der
65.536er-Prüfungen. Die ausgeführte Binärdatei ist als `reference-runner` gesichert.
Die zugehörigen Quellen, Tests, Konfigurationen und Cargo-Dateien liegen außerdem
als `reference-sources.zip` mit SHA-256-Dateiliste in `source-snapshot.json` vor.

Nativer Build-Fingerabdruck: `5b02ae55003aa897`.
SHA-256 der Release-Binärdatei:
`4d5604a469345ba03fb45af4d48f52edd6773e8fad9fb69d66217daff9beae08`.

Die früheren 256-/1.024-Messungen behalten ihre ursprünglichen Belege; ihre
Originalbinärdateien und damaligen Quellen wurden vor dem Ausbau zusätzlich
gesichert. Aufruf und Parametrisierung stehen in [SCALING.md](SCALING.md).
