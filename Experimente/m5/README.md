# M5: zwei Ursachenversuche

Der Rust-Einzelfall liegt unter `NERVA_Modell/Modell/src/m5.rs`, die Versuchsdefinitionen,
Auswertung und Messdaten unter `NERVA_Modell/Experimente/`. A prüft die Überschwellung von
Aktionskopien; B isoliert die Population mit aktiver Homöostase. Es werden keine
neuen neuronalen Lernregeln verwendet.

Vom Projektverzeichnis:

```bash
# Vorab den Plan speichern, ohne Modellläufe:
python3 NERVA_Modell/Experimente/m5/run.py plan --stage pilot --output /tmp/mein-m5-pilot

# Die zwei gezielten Versuche auf drei Seeds:
python3 NERVA_Modell/Experimente/m5/run.py run --stage pilot --jobs 4 \
  --output Messungen/Experimente/mein-m5-pilot \
  --command './NERVA_Modell/Modell/run-demo.sh m5 --config {config} --output {output}'

# Gleiche Bedingungen auf zehn neuen Seeds 200–209:
python3 NERVA_Modell/Experimente/m5/run.py run --stage confirm --jobs 4 \
  --output Messungen/Experimente/mein-m5-confirm \
  --command './NERVA_Modell/Modell/run-demo.sh m5 --config {config} --output {output}'

# Gespeicherte Messungen unabhängig neu prüfen und auswerten:
python3 NERVA_Modell/Experimente/m5/run.py report --output Messungen/Experimente/m5-confirm
python3 -m unittest discover -s NERVA_Modell/Experimente/m5/tests -v
```

Die ausgeführten Reihen liegen unter `Messungen/Experimente/m5-pilot` und `Messungen/Experimente/m5-confirm`.
Abgeschlossene JSONs werden nicht überschrieben. Wiederaufnahme validiert
Konfiguration und Ergebnis; bei Teilfehlern bleiben Sidecars zur Diagnose
erhalten. Für einen neuen Versuch ein neues Ausgabeverzeichnis verwenden.
`--train-trials`, `--test-trials` und `--calibration-trials` steuern B; für die
wissenschaftlichen Messungen wurden 256/128/128 verwendet. A verwendet immer
seine sechs Interventionen und den festen 50-ms-Stichtag.

## JSON-Vertrag

Alle sechs Felder sind Pflichtfelder, unbekannte Felder/Werte werden abgewiesen:

```json
{
  "protocol": "NERVA-M5-v1",
  "experiment": "sensor_homeostasis",
  "base": { "...": "vollständige gültige NERVA-M4-v1-Konfiguration" },
  "homeostasis_scope": "core",
  "action_copy_weight": 0.9,
  "core_membrane_tau_us": 20000
}
```

`base` folgt unverändert dem [M4-Vertrag](../m4/README.md). Ihr
`params.homeostasis` muss zum äußeren Bereich passen. Bereiche: `none`, `core`,
`core_sensor`, `all`. `core_sensor` umfasst den Processing-Core und ausschließlich
die visuellen Sensorneuronen. Gewichte müssen endlich in 0–10 liegen,
Core-Zeitkonstanten in 1.000–100.000 µs. Der Runner schreibt vollständige,
direkt ausführbare Dateien unter `configs/`.

`motor_threshold` verlangt 2-Kanal-Sensorik, deterministische Kodierung,
32 Core-Neuronen, STDP und Feedback an, Homöostase und Entwicklung aus,
Startdrive null. Der isolierte Versuch verwendet nur das neutrale Bild und
drei diagnostische Kopien derselben Taste, keine VM-Bewegung. Die Zustände
werden für jede Intervention frisch und bitgleich aufgebaut. Es gibt keine
visuelle Kalibrierung und keine behauptete Vorhersageleistung in A.

`sensor_homeostasis` verwendet den M4-Lauf mit gewählter Modellkonfiguration
und zusätzlicher sensorischer Instrumentierung. Der Ergebnisvertrag enthält
die M4-Arme/-Checks, beide Belegungen, die angeforderte äußere Konfiguration,
Anfangsnetz-Fingerabdruck und den geprüften Populationsbereich. Unter
`m5_sensor` stehen je Arm/Belegung tatsächliche Sensorneuronen-Emissionen,
erzeugte Eingangsimpulse, Populationsupdates und sensorische intrinsische Ströme.

## Dateien und Endpunkte

A schreibt immer `.events.csv` sowie JSON mit einzelnen Core-Spikes,
plastischen Ankünften, tatsächlichen Gewichtsänderungen und analytisch auf
50 ms projizierten Membranpotentialen. Die Aktionskopien bei 1/10.001/20.001 µs
und die Anzahl eingehender Kopien werden geprüft. Signalerfolg ist kein Lernen.

B schreibt die bisherigen `.details.csv` und `.audit.csv` sowie zusätzlich
`.sensor.csv`. `--raw-events` ergänzt für B das vollständige Ereignisprotokoll.
Der unabhängige Auswerter prüft die CSV-Zeitgrenzen, Balance, Freeze und Ablationen,
Sensor-CSV-Zähler gegen JSON und ausgeschlossene Populationen. Er vergleicht
Anfangsnetze, erzeugten Eingang, Normierung und eingefrorene Kontrollarme über
alle Homöostasebereiche. Fehlende Bedingungen/Seeds brechen die vollständige
Auswertung ab.

`motor-summary.csv` berichtet beide Signalkriterien und reale plastische Pfade.
`sensor-summary.csv` trennt Weiterleitung, Strom, visuelle Auslesbarkeit und
Vorhersage. `prediction-contrasts.csv` enthält sämtliche gepaarten Kontrollen
je Belegung, Prüfzeitpunkt und Auswerter. `diagnostics.json` enthält außerdem
die gezielten Populationskontraste und Datenaudits.

Ein bestätigter Vorhersagevorteil verlangt mindestens zehn Seeds, beide
Belegungen, alle drei Prüfzeitpunkte, strikt positive untere 95-%-Grenzen
gegen alle drei Kontrollen und mindestens 80 % visuelle Auslesbarkeit pro
Belegung. Die Intervalle sind deskriptiv und nicht für multiple Vergleiche
korrigiert. Die erste Probe folgt nach visueller Kalibrierung; B enthält noch
keine Kombination beider in M5 untersuchten Änderungen.

[Ergebnisse, Interpretation und Grenzen](../M5_EXPERIMENT.md)
