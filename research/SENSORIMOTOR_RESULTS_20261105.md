# Erster sensomotorischer Größen-/Rückkopplungsvergleich

Am **05.11.2026** ausgeführt. Der neue Adapter funktioniert mit 256 und
1.024 Verarbeitungsneuronen und den sechs gepaarten Bedingungen. In diesem
kleinen technischen Protokoll entsteht **kein belastbarer Nachweis gelernter
Handlungsfolgen**. Die visuelle Messvoraussetzung wird nicht auf allen Seeds
erfüllt; es wird deshalb kein Finalist ausgewählt und keine Abschlussvalidierung
auf den reservierten Seeds ausgeführt.

## Protokoll

- [Feste Modellreferenz](config/sensorimotor_baseline.toml): 256 Core-Neuronen,
  zwei Seeds, sechs Bedingungen, 12 Studienjobs mit vollständiger Wiederholung.
- [Größenvergleich im gemeinsamen Harness](config/sensorimotor_sizes.toml):
  256 und 1.024 Core-Neuronen, dieselben zwei Explorationsseeds und alle sechs
  Bedingungen. Die 256er-Referenz ist zugleich die vorab festgelegte Baseline.
- Explorationsseeds: `1210000001`, `1210104730`.
  Vorab reservierte Prüfseeds: `1220000011`, `1220104740`; sie bleiben unbenutzt.
- Je Modell acht Trainings- und acht Probekommandos mit 20 ms Verzögerung,
  sieben visuellen Kalibrations- und sieben getrennten visuellen Prüfbeobachtungen.
- Vorhandene lokale Modellregeln unverändert; A/D-Motorlayout und
  deterministische diagnostische Positionssensorik. Vorgeschriebene Exploration
  in persistenter diskreter Bewegungswelt, keine autonome Zielwahl.

[Aufbau, Endpunkte und Kontrollsemantik](SENSORIMOTOR.md).
Die beiden kleinen Seedmengen und kurzen Episoden dienen der technischen
Funktionsprüfung, nicht einer wissenschaftlichen Bestätigung.

## Gemessene Endpunkte

Mittelwerte über beide Explorationsseeds:

| Core-Neuronen | Gesamtneuronen | Aktive Vorhersagegenauigkeit | Aktive Kontrolle ohne Plastizität | Visuelle Prüfgenauigkeit nach Training | Anfangssynapsen | Synapsen nach Training |
|---:|---:|---:|---:|---:|---:|---:|
| 256 | 267 | 25,0 % | 6,25 % | 50,0 % | 1.080 | 272 |
| 1.024 | 1.035 | 25,0 % | 12,5 % | 35,71 % | 4.152 | 887 |

Alle drei plastischen Bedingungen – aktiv, passiv und entkoppelt – erreichen
bei beiden Größen dieselbe Vorhersagegenauigkeit von 25 %. Ihre jeweiligen
Kontrollen ohne Plastizität erreichen innerhalb jeder Größe ebenfalls denselben
Wert. Die Differenz der Lerngewinne zwischen aktiv/passiv und aktiv/entkoppelt
ist deshalb auf beiden Seeds null. Das ist keine Aussage allgemeiner
Gleichwertigkeit dieser Bedingungen.

Die häufigste tatsächliche Zielposition kommt in 50 % der Proben vor. Alle
gemessenen Vorhersagegenauigkeiten liegen unter dieser deskriptiven
Vergleichsgrenze. Der mittlere Positionsfehler der plastischen Arme beträgt
eine Gitterzelle, ebenso wie bei der Vorhersage „Position bleibt gleich“.
Ein positiver Genauigkeitsunterschied gegenüber einer schwachen eingefrorenen
Kontrolle genügt daher hier nicht für einen Vorhersagenachweis.

Die visuelle Prüfgenauigkeit des aktiven trainierten Arms beträgt pro Seed:

| Core-Neuronen | Seed 1210000001 | Seed 1210104730 |
|---:|---:|---:|
| 256 | 4/7 | 3/7 |
| 1.024 | 3/7 | 2/7 |

Das vorab konfigurierte Gate verlangt mindestens 0,5 **auf jedem Seed und in
jeder Bedingung**. Ein Mittelwert von 0,5 im 256er-Netz erfüllt diese Bedingung
nicht. Beide Größen erhalten daher `eligible=false`, `quality=null`; die
Rohmesswerte bleiben vollständig im Archiv.

## Aktivität, Struktur und Aufwand

Die Aktivitäts-, Wiederholungs-, Ressourcen- und Paarungsgates bestehen bei
beiden Größen. Alle Werte sind endlich; die Core-Aktivität ist nicht vollständig
stumm und bleibt unter dem konfigurierten populationsweiten Spitzenratenlimit.
Diese Grenzen belegen keine allgemeine dynamische Stabilität.

Die unveränderten Entwicklungsregeln entfernen im kurzen Training viele
Synapsen: Im aktiven 256er-Arm verbleiben 266 bzw. 278; im 1.024er-Arm 884 bzw.
890. Ob dieses Pruning die visuelle Auslesbarkeit verursacht, wurde nicht
intervenierend geprüft und bleibt eine Folgefrage.

Der mittlere native Ereigniszähler des aktiven Arms beträgt 4.747,5 bei 256
und 12.205,5 bei 1.024 Core-Neuronen, jeweils über Training, visuelle Kalibration
und Probe. Die mittlere Core-Rate beträgt etwa 0,1313 bzw. 0,0377 Hz pro Zelle.
Ereigniszahlen und Synapsenzahlen sind Aufwandindikatoren; daraus wird kein
FLOP-, Speicher- oder Laufzeitvorteil abgeleitet.

## Umfang, Archive und Verifikation

Die Modellreferenz speichert 12 Jobs; der Harness speichert weitere 24 Jobs
für die beiden Größen. Insgesamt entstanden **36 Roh-Records und 72 vollständige
Modellausführungen einschließlich deterministischer Wiederholung**. Die
256er-Referenz wird in diesen getrennten Archiven bewusst zweimal gemessen.
Kalibrations-/Probekommandos sind Teile dieser Ausführungen und werden nicht
als zusätzliche Modelle gezählt.

Im Harness waren höchstens 48 Jobs beziehungsweise 96 Modellausführungen
eingeplant, einschließlich 24 reservierter Bestätigungsjobs. Tatsächlich wurden
24 Jobs beziehungsweise 48 Modellausführungen benötigt. Abschlussstatus:
`not_run_no_eligible_candidate`; `held_out_quality_improved=false`.

- [Modellreferenz](results/sensorimotor_baseline_20261105/):
  [Zusammenfassung](results/sensorimotor_baseline_20261105/summary.json),
  [Metriken](results/sensorimotor_baseline_20261105/metrics.csv),
  [Prüfnachweis](results/sensorimotor_baseline_20261105/verification-receipt.json).
- [Harnessarchiv](results/sensorimotor_sizes_20261105/):
  [Gepaarter Größenbericht](results/sensorimotor_sizes_20261105/reports/sizes_and_feedback.json),
  [Abschlussstatus](results/sensorimotor_sizes_20261105/final-result.json),
  [Prüfnachweis](results/sensorimotor_sizes_20261105/verification-receipt.json).

Beide Archive wurden anschließend im Reportmodus erneut ausgewertet. Alle
36 Rohdateihashes blieben gleich; es starteten keine neuronalen Trials.
Die vier SQLite-Kataloge bestehen `PRAGMA integrity_check`. Die Prüfnachweise
binden jeden Roh-Record und das jeweilige Manifest mit SHA-256.

Nativer Build-Fingerabdruck: `d57114b6d7495d77`.
SHA-256 der ausgeführten Release-Binärdatei:
`d1cc07b5d5747e780ee00729d0f15d1ebc5a79abb9b02d2f231553d6a2db2ddd`.
Lokale Ergebnisarchive bleiben wie bisher außerhalb der Versionierung.

Die Tests des Research-Pakets bestehen: 13 Rust-Tests, die bisherigen 35
Python-Tests und vier zusätzliche Python-Tests für das neue Szenario.
Clippy mit `-D warnings` ist sauber. Die neue native Prüfung umfasst auch
1.024 Core-Neuronen. Der frühere M0-Referenzbeleg wurde mit der vor dem Ausbau
archivierten Originalbinärdatei ebenfalls erfolgreich erneut geprüft.

Die nächste gezielte Forschungsfrage ist die schwache visuelle Auslesbarkeit
nach Training. Eine vorab festgelegte Entwicklungs-/Pruningintervention mit
gleichen Anfangsnetzen und Erfahrungen könnte deren Ursache prüfen; die
hier gemessenen Unterschiede allein identifizieren sie noch nicht.
