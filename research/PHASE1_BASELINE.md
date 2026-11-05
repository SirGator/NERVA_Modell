# Feste M0-Referenz für die nächste Forschungsphase

Am **05.11.2026** mit unveränderten neuronalen Regeln und nativen M0-Defaults
ausgeführt. Dies ist der erste umgesetzte Teil der
[nächsten Forschungsphase](NEXT_PHASE.md), eine Regressionsreferenz auf bekannten
Seeds. Es ist keine unabhängige Bestätigung und keine Baseline des vollständigen
sensomotorischen Modells.

## Protokoll und Ergebnis

[Konfiguration](config/phase1_baseline.toml): eine Variante ohne
Parameterüberschreibungen, sechs historische Seeds, neun native M0-Neuronen
(acht erregend, eines hemmend), vier Kontrollen, zwölf Metriken und vollständige
deterministische Wiederholung. 24 Studienjobs ergeben 48 native Ausführungen.

| Gruppe | Sequenzscore, Mittel | Proberate, Hz | Stabile Läufe | Vollständige Wiederholung identisch |
|---|---:|---:|---:|---:|
| Geordnet mit STDP (`none`) | 3,0 | 16,667 | 6/6 | 6/6 |
| Geordnet ohne STDP (`disable_stdp`) | 0,0 | 5,556 | 6/6 | 6/6 |
| Gemischte Eingabe mit STDP (`shuffle_input`) | 0,0 | 5,556 | 6/6 | 6/6 |
| Geordnet mit STDP und Homöostase (`enable_homeostasis`) | 3,0 | 16,667 | 6/6 | 6/6 |

In allen Gruppen bleiben die Gewichte während der eingefrorenen Proben
unverändert, die Raten endlich und der Anteil persistenter Zellen null.
Das bestehende native M0-Kriterium ist erfüllt (`passed = true`). Der Scorevorteil
gegenüber jeder der beiden Kontrollen beträgt auf jedem Seed drei Punkte;
der native einseitige Vorzeichentest ergibt jeweils p = 0,015625. Es wird kein
neues Bestätigungskriterium daraus abgeleitet. Die bekannten Seeds, die feste
Aufgabe und der identische Score der trainierten Gruppen erlauben keine
Aussage über allgemeine Kognition, neue Aufgaben oder Homöostase-Äquivalenz.

## Ablage und Prüfnachweis

Das [Archiv](results/phase1_m0_baseline_20261105/) enthält:

- [Native Zusammenfassung](results/phase1_m0_baseline_20261105/summary.json)
  und [Metriktabelle](results/phase1_m0_baseline_20261105/metrics.csv).
- [Manifest mit effektiven Konfigurationen](results/phase1_m0_baseline_20261105/manifest.json),
  24 Roh-Records und SQLite-Katalog.
- [Unveränderlichen SHA-256-Beleg](results/phase1_m0_baseline_20261105/baseline-receipt.json)
  mit jedem Rohdateihash, Binärdatei-/Protokollhash und nativer Entscheidung.

Die erneute Prüfung mit `baseline.py verify` startete keine neuronalen Trials.
Alle 24 Rohdateihashes und der Beleg blieben unverändert. Der neu aufgebaute
SQLite-Katalog enthält 24 Trials und besteht `PRAGMA integrity_check`.

Nativer Build-Fingerabdruck: `d120265e9936e512`.
SHA-256 der ausgeführten Release-Binärdatei:
`cab9c9283c285300892e062517b80880dcbd79d982fafe133f5932af76ecaa99`.
Werkzeuge: Rust/Cargo 1.98.1, Python 3.12.3; Offline-Build mit vorhandenen
Abhängigkeiten. Die lokalen Ergebnisarchive sind wie bisher nicht versioniert.

## Wiederholen und prüfen

Vom Projektverzeichnis `DSVLM`:

```bash
# Bestehende versiegelte Messung mit der archivierten Originalbinärdatei prüfen:
python3 NERVA_Modell/research/baseline.py verify \
  --binary NERVA_Modell/research/results/phase1_m0_baseline_20261105/reference-runner

# Eine eigene Referenz mit dem aktuellen Programmstand erstellen:
python3 NERVA_Modell/research/baseline.py run --output /tmp/nerva-phase1-baseline

# Gezielte automatisierte Verifikation:
python3 -W error::ResourceWarning -m unittest discover \
  -s NERVA_Modell/research/tests -p 'test_baseline.py' -v
```

Die drei Integrationstests bestehen. Sie prüfen den echten nativen Referenzlauf,
unveränderte erneute Auswertung und Wiederaufnahme sowie Abweisung veränderter
Metriken, fehlender oder zusätzlicher Records, veränderter Manifeste,
beschädigter Belegprüfsummen und fehlender Archive.

Der nachfolgende [Größen-/Umweltadapter](SENSORIMOTOR.md) ist inzwischen mit
256 und 1.024 Verarbeitungsneuronen umgesetzt. Vor seinem neuen Build wurden
die Originalbinärdatei und die damaligen Quellen zusätzlich unter
`reference-runner` und `reference-sources.zip` im M0-Archiv gesichert. Der
ursprüngliche Beleg bleibt unverändert. Höhere Netzgrößen und neue lokale
Mechanismen sind weiterhin offen.
