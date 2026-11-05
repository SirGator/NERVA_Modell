# Homöostase nach Zellpopulation lokalisieren

Vorregistrierter Folgeversuch vom **06.11.2026** zum
[bestätigten Homöostaseeffekt](HOMEOSTASIS_RESULTS_20261105.md). Er vergleicht
dieselben Größen **17, 256, 1.025 und 4.096 Core-Neuronen** mit **24 neuen
gepaarten Seeds**. STDP bleibt unverändert an, Strukturentwicklung bleibt aus.

Die bereits vorhandenen Modellbereiche sind jetzt im Research-Adapter über
`homeostasis.scope` zugänglich:

| Wert | Regulierte Populationen |
|---|---|
| `off` | keine |
| `core` | ausschließlich Verarbeitungsneuronen |
| `core_sensor` | Verarbeitungs- und visuelle Sensorneuronen |
| `all` | Sensor-, Core-, Motor- und Aktionskopiezellen |

Die lokalen Regeln, Zielraten und Eingangsgewichte werden dabei nicht geändert.
Die Auswahl der Core-Zellen nutzt deren sortierte IDs, damit auch die begrenzten
Bereiche keine vollständige lineare Suche durch den Core pro Zelle benötigen.
Unveränderte Standardbereiche und die bisherigen neuronalen Referenzdigests
werden weiter geprüft.

[Konfiguration](config/homeostasis_populations.toml) ·
[Werkzeug](localize_homeostasis.py) ·
[Analyse](analysis/homeostasis_populations.py).

## Vorab festgelegte Kontraste

Im aktiven Arm werden drei Übergänge je Größe untersucht:

1. `core − off`: Wirkung der Core-Homöostase.
2. `core_sensor − core`: zusätzliche Sensorregulierung bei bereits reguliertem Core.
3. `all − core_sensor`: zusätzliche Motor- und Aktionskopieregulierung bei bereits reguliertem Core und Sensorik.

Jeder Kontrast hat die separaten Endpunkte **visuelle Prüfgenauigkeit** und
**externer Positions-MAE**. Beide werden zweiseitig geprüft. Das ergibt
`3 × 2 × 4 = 24` Tests in einer vorab festgelegten Familie. Bei Familienalpha
0,05 gilt die strikte Bonferronischwelle **p < 0,00208333** je Test.

Die konservativen exakten Vorzeichentests verwenden alle Seedpaare; Gleichstände
bleiben im vollständigen n als Nichtgewinne. Das Vorzeichen des mittleren
Unterschieds muss mit der Mehrheit der Seedvorzeichen übereinstimmen.
Zusätzliche 95-%-Bootstrapintervalle mit 4.000 deterministischen Ziehungen sind
deskriptiv und nicht familienkorrigiert. Ein nicht bestätigter Kontrast ist kein
Äquivalenznachweis.

Das sind bedingte totale Interventionseffekte. Die Sensorwirkung ist auf einen
regulierten Core bezogen; Motor- und Aktionskopiezellen werden gemeinsam
hinzugenommen. Ein separater Effekt jeder Restpopulation oder ein isolierter
Vermittlungsmechanismus wird damit nicht identifiziert.

## Neue Beobachtungen ohne Eingriff ins Lernen

Der Adapter protokolliert jetzt getrennt:

- **Erzeugte Eingangsspikes** des unveränderten Frame-Encoders.
- **Tatsächliche Spikes der visuellen Sensorneuronen** nach neuronaler Verarbeitung.
- Sensor- und Core-Raten je Zelle und Sekunde, jeweils während Training und
  eingefrorener Kalibration/Probe sowie über den gesamten Assay.
- Gespeicherte intrinsische Homöostaseströme je Population: Anzahl, Mittel,
  Minimum und Maximum vor Training, danach und nach der Messung.
- Homöostaseupdates je Sensor-, Core-, Motor- und Aktionskopiepopulation.

Die gespeicherten Ströme haben die Modell-Einheit Potentialeinheiten pro Sekunde.
Es werden keine biologischen Strommessungen behauptet. Ein durchschnittlicher
Populationsstrom allein beschreibt keine vollständige Zellzustandsverteilung.

Jedes Beobachtungsfenster enthält Raten, Populationen und Stromzusammenfassungen.
Die zusätzlichen skalaren Metriken heißen unter anderem `root_sensor_rate_hz`,
`sensor_rate_hz`, `training_sensor_rate_hz`, `measurement_sensor_rate_hz`,
`sensor_intrinsic_current` und `core_homeostasis_updates`. Raten und Ströme sind
deskriptive Diagnosegrößen; sie erzeugen keine zusätzlichen Bestätigungstests.

Damit kann geprüft werden, ob bei unveränderter externer Anregung die
Sensorneuronen weniger Spikes aussenden. Eine gleichzeitige Änderung
von Rate, Strom und Auslesbarkeit beweist allein noch keine Vermittlungskette
und trennt keine direkten Stromwirkungen von nachgelagertem STDP.

## Kontrollen und Integrität

Jede Größe und jeder Bereich durchlaufen die bisherigen sechs Bedingungen:
aktiv, passiv und entkoppelt, jeweils mit und ohne Plastizität. Die Primärtests
verwenden ausschließlich den vorher festgelegten aktiven Arm.

Vor der Auswertung werden innerhalb jedes Größen-/Seedpaares gleiche
Anfangsnetze, Bilder, Zeitpunkte und erzeugte Eingangsspikezahlen je Fenster
über alle Bereiche und Kontrollarme verlangt. Die eingefrorenen Endpunkte
müssen übereinstimmen. Es dürfen weder Strukturänderungen noch Updates außerhalb
der tatsächlich ausgewählten Populationen auftreten. Die intrinsischen Ströme
ausgeschlossener Zellen bleiben bei ihrem Ausgangswert; während der eingefrorenen
Messung bleiben sämtliche gespeicherten Ströme erhalten.

Technisch fehlerhafte Daten stoppen die Auswertung. Schlechte visuelle Ausleser
werden nicht nachträglich ausgeschlossen. Der MAE bleibt ein End-to-End-Endpunkt
des festgelegten externen Decoders und isoliert keine interne Vorhersagefähigkeit.

Die Vorregistrierung wurde vor dem ersten neuronalen Lauf bei null vorhandenen
Records gespeichert. Die Seedprüfung fand keine Überschneidungen mit **186**
bekannten oder reservierten Seeds aus **218** lokalen Katalogdateien. Umfang:
**16 Varianten × 24 Seeds × 6 Kontrollen = 2.304 Jobs**, mit vollständiger
Wiederholung **4.608 Modellausführungen**. Es gibt keine Finalistenauswahl.

## Ausführen

Vom Projektverzeichnis `DSVLM`:

```bash
./NERVA_Modell/research/run.sh localize-homeostasis plan
./NERVA_Modell/research/run.sh localize-homeostasis run
./NERVA_Modell/research/run.sh localize-homeostasis report
```

Die Befehle unterstützen `--config`, `--output` und `--binary`. `run` setzt
fehlende Jobs mit demselben Programmstand fort; `report` startet keine fehlenden
Trials. Geänderte Protokolle oder Programme benötigen eine neue Ausgabe und neue
Seeds. Die Analyse ist größenparametrisiert; ihre Testfamilie umfasst allgemein
sechs Tests je Größe.

Archiv: `results/homeostasis_populations_20261106/`, einschließlich
`preregistration.json`, nativen Records/CSV/SQLite, `population-result.json`,
`population-summary.md` und `verification-receipt.json`.

Neue Tests prüfen die Populationszugehörigkeit für die über `NERVA_TEST_SIZES`
gewählten Größen, die unveränderten Referenzdynamiken, Fensterdiagnostik,
Familienkorrektur, fehlende oder veränderte Daten und erneute Auswertung ohne
neue Trials. Der bestehende Modelltest bestätigt weiterhin das alte M5/M4-
Verhalten der Bereiche.

[Ergebnisse des vollständigen Laufs](HOMEOSTASIS_POPULATIONS_RESULTS_20261106.md).
