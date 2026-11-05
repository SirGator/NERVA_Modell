# Populationsvergleich: Sensorregulierung als bedingter Auslöser

Der vorregistrierte Lauf vom **06.11.2026** ist vollständig ausgeführt.
Das Hinzunehmen der Sensorneuronen zur Core-Homöostase senkt im festgelegten
Assay die visuelle Auslesbarkeit bei **256, 1.025 und 4.096 Core-Neuronen**.
Diese drei Kontraste bestehen die vorher festgelegte Fehlerkorrektur. Bei 17
Core-Neuronen ist der deskriptive Rückgang nach diesem Kriterium nicht bestätigt.

Die externe Anregung bleibt unverändert. Mit Sensorregulierung feuern die
Sensorneuronen im Mittel etwa **34,3 % seltener**. Negative gespeicherte
Sensorströme und der Aktivitätsrückgang passen zu einer Abschwächung der
sensorischen Spikeweitergabe. Sie isolieren jedoch keine vollständige
Vermittlungskette bis zum Ausleseverlust.

## Vorab festgelegter Versuch

[Protokoll](HOMEOSTASIS_POPULATIONS.md) ·
[Konfiguration](config/homeostasis_populations.toml) ·
[Vorregistrierung](results/homeostasis_populations_20261106/preregistration.json).

Vier Größen, vier Bereiche (`off`, `core`, `core_sensor`, `all`), 24 neue
gepaarte Seeds, unveränderte STDP, ausgeschaltete Strukturentwicklung und je
sechs Rückkopplungs-/Plastizitätsbedingungen. Die Primärtests verwenden den
vorher festgelegten aktiven Arm.

Die Seedprüfung fand keine Überschneidungen mit 186 bekannten oder reservierten
Seeds aus 218 geprüften lokalen Katalogdateien. Die Registrierung wurde bei
null vorhandenen Roh-Records vor dem ersten neuronalen Lauf gespeichert.

Drei benachbarte Bereichskontraste und zwei Endpunkte je Größe ergeben **24
Tests** mit gemeinsamer Fehlerwahrscheinlichkeit 0,05. Strikte Schwelle:
`p < 0,00208333`. Die zweiseitigen konservativen Vorzeichentests behalten
Gleichstände im vollständigen n. Die mittlere Differenz muss zur Mehrheit der
Seedvorzeichen passen. Alle 24 Seedpaare bleiben enthalten.

## Visuelle Auslesbarkeit

Mittlere visuelle Prüfgenauigkeit über die 24 Seeds:

| Core-Neuronen | Homöostase aus | Nur Core | Core + Sensor | Gesamtes Modell |
|---:|---:|---:|---:|---:|
| 17 | 56,55 % | 54,17 % | 40,48 % | 41,07 % |
| 256 | 81,55 % | 80,36 % | 52,98 % | 52,98 % |
| 1.025 | 63,69 % | 76,19 % | 37,50 % | 37,50 % |
| 4.096 | 38,10 % | 58,93 % | 25,00 % | 25,00 % |

Vorregistrierter Kontrast **Core + Sensor minus nur Core**:

| Core-Neuronen | Änderung in Prozentpunkten | Verbesserte / verschlechterte / gleiche Paare | Zweiseitiges p | Rückgang bestätigt |
|---:|---:|---:|---:|---|
| 17 | −13,69 | 2 / 13 / 9 | 0,83882 | Nein |
| 256 | −27,38 | 0 / 23 / 1 | 0,0000029802 | Ja |
| 1.025 | −38,69 | 1 / 23 / 0 | 0,0000029802 | Ja |
| 4.096 | −33,93 | 0 / 21 / 3 | 0,00027716 | Ja |

Der Kontrast fügt Sensorregulierung bei bereits reguliertem Core hinzu. Er
misst somit einen bedingten totalen Effekt dieser zusätzlichen Population;
eine reine Sensor-Homöostase ohne Core-Regulierung war nicht Teil des Designs.

Die anderen visuellen Primärkontraste bestehen die Fehlerkorrektur nicht:
`core − off` bei keiner Größe und `all − core_sensor` ebenfalls bei keiner
Größe. Insbesondere ist der deskriptive Anstieg von 20,83 Prozentpunkten bei
4.096 Neuronen mit Core-Homöostase kein bestätigter Verbesserungsnachweis
(`p = 0,063915`). Ebenso belegen die gleichen Mittelwerte von `core_sensor`
und `all` in den größeren Netzen keine allgemeine Gleichwertigkeit.

Unadjustierte 95-%-Bootstrapintervalle für den Sensor-Zusatzkontrast betragen
−22,02 bis −4,76, −33,33 bis −22,02, −47,02 bis −29,76 und −42,86 bis −25,00
Prozentpunkte in der Größenreihenfolge. Sie sind deskriptiv und ersetzen die
vorregistrierte Entscheidung nicht; bei 17 Neuronen führen insbesondere die
vielen Gleichstände zum Nichtbestehen des konservativen Vorzeichentests.

## Sensoraktivität und gespeicherte Ströme

Die erzeugte Eingangsspikerate liegt in allen Bereichen und Größen im Mittel
bei **16,897 Hz pro Eingangskanal**. Die tatsächliche Rate der Sensorneuronen
ist separat erfasst. Beide beziehen sich auf alle sieben visuellen Kanäle,
einschließlich gerade inaktiver Kanäle; sie sind nicht die Rate eines einzelnen
aktiven Kanals.

| Bereich | Tatsächliche Sensorrate, gesamter Assay | Während Training | Während eingefrorener Messung | Sensorstrom nach Training, Mittel |
|---|---:|---:|---:|---:|
| aus | 16,897 Hz | 16,815 Hz | 16,937 Hz | 0 |
| nur Core | 16,897 Hz | 16,815 Hz | 16,937 Hz | 0 |
| Core + Sensor | 11,108 Hz | 12,277 Hz | 10,530 Hz | −4,286 |
| gesamtes Modell | 11,108 Hz | 12,277 Hz | 10,530 Hz | −4,286 |

Diese Mittelwerte gelten hier für alle vier Netzgrößen. Die erzeugten
Eingangsspikezahlen je Beobachtungsfenster sind über alle Bereiche und
Kontrollarme eines Größen-/Seedpaares identisch geprüft. Die Sensorneuronen
verändern also ihre Antwort auf die gleiche Anregung.

Die Stromangaben sind Modell-Potentialeinheiten pro Sekunde. Mit Sensorregulierung
liegt das gespeicherte sensorische Minimum nach Training bei −10, das Maximum
bei 0. Diese Ströme bleiben während der eingefrorenen Messung erhalten;
Einfrieren setzt sie nicht zurück. Der Aktivitätsrückgang ist kein gemessener
Rückgang des Informationsgehalts um 34,3 %.

Bei 256 Core-Neuronen beträgt die mittlere Core-Rate beispielsweise 0,3981 Hz
ohne Homöostase, 0,3369 Hz bei Core-Regulierung und 0,1319 Hz mit zusätzlicher
Sensorregulierung. Die entsprechenden mittleren gespeicherten Core-Ströme
nach Training sind 0, −0,1737 und −0,0986. Diese Diagnosewerte werden
deskriptiv berichtet; sie bilden keine zusätzliche Testfamilie.

## Separater Vorhersagefehler

**Keiner der zwölf vorregistrierten MAE-Kontraste** besteht die gemeinsame
Fehlerkorrektur. Für den Sensor-Zusatzkontrast ergeben sich:

| Core-Neuronen | MAE nur Core | MAE Core + Sensor | Mittlere Änderung | Zweiseitiges p |
|---:|---:|---:|---:|---:|
| 17 | 1,5156 | 1,5104 | −0,0052 | 1,0000 |
| 256 | 0,9010 | 1,0000 | +0,0990 | 0,022656 |
| 1.025 | 0,9531 | 1,0000 | +0,0469 | 0,83882 |
| 4.096 | 0,8698 | 1,0000 | +0,1302 | 0,0066108 |

Die Differenz ist jeweils der Fehler des höheren Bereichs minus der des
niedrigeren Bereichs, in Gitterzellen. Ein positiver Wert bedeutet einen höheren
aufgezeichneten Fehler. Die Schwelle bleibt auch beim größten Netz 0,00208333.

Beide visuellen Ausleser des Sensor-Zusatzkontrasts bestehen das frühere
0,5-Gate nur in 7/24, 13/24, 4/24 beziehungsweise 0/24 Paaren. Schlechte
Kalibration verhindert eine isolierte Aussage über interne Vorhersagefähigkeit.
Die hier geprüften bedingten Populationseffekte sind außerdem andere Kontraste
als der frühere Gesamtvergleich `all − off` auf anderen Seeds.

## Kontrollen, Umfang und Aussagegrenzen

Alle technischen Prüfungen bestehen: vollständige Wiederholung, unveränderte
Messparameter, gleiche Anfangsnetze und Erfahrungen, richtige Zeitordnung und
exakt eingehaltene Populationszugehörigkeit. Ausgeschlossene Zellen erhalten
keine Homöostaseupdates; ihre gespeicherten Ströme bleiben beim Ausgangswert.
Alle eingeschalteten Populationen zeigen tatsächlich Updates. Wachstum und
Pruning bleiben in jedem Lauf aus. Die eingefrorenen Kontrollendpunkte stimmen
über alle vier Bereiche überein.

Es wurden **2.304 Roh-Records und 4.608 vollständige Modellausführungen
einschließlich Wiederholung** erzeugt. Kein Seed wurde wegen schlechter
Auslesbarkeit nachträglich ausgeschlossen. Die nachfolgenden erneuten
Auswertungen starten keine neuen neuronalen Trials und verändern keine
Roh-Records oder vorregistrierten Entscheidungen.

Dies lokalisiert einen Ausleseengpass im festen Positionssensor-Assay und
kontrollierter Exploration. Eine reine Strommediation, sensorische
Informationsmenge, autonome Handlungswahl oder allgemeine Fähigkeiten werden
dadurch nicht bestätigt. Motor- und Aktionskopiezellen werden im letzten
Kontrast gemeinsam hinzugefügt und nicht getrennt untersucht.

## Daten und Verifikation

[Archiv](results/homeostasis_populations_20261106/):

- [Vorregistrierung und Seed-Audit](results/homeostasis_populations_20261106/preregistration.json).
- [Alle Kontraste, Diagnosen und Seedpaare](results/homeostasis_populations_20261106/population-result.json).
- [Kompakte Zusammenfassung](results/homeostasis_populations_20261106/population-summary.md).
- [Native Metriktabelle](results/homeostasis_populations_20261106/metrics.csv).
- [Unveränderlicher SHA-256-Prüfnachweis](results/homeostasis_populations_20261106/verification-receipt.json).

Alle **53 Python-Tests und 17 Research-Rust-Tests** bestehen. Darunter sind
größenparametrisierte Populationsprüfungen und unveränderte frühere
Referenzdynamiken. Der bestehende M5/M4-Modelltest der Bereiche besteht ebenfalls.
Clippy ist für Research und Modell mit `-D warnings` sauber. Das frühere
Homöostase-Bestätigungsarchiv wurde zusätzlich mit seiner Originalbinärdatei
erfolgreich erneut ausgewertet.

Nativer Build-Fingerabdruck: `da9eedd0a7ec1e99`.
SHA-256 der ausgeführten Release-Binärdatei:
`d776f28c196908e9e642bd1d60211e621311eac3f36007f859625c75f6bfa28e`.
Originalbinärdatei (`reference-runner`), Quellen/Tests/Konfigurationen
(`reference-sources.zip`) und Verifikationslogs (`verification/`) sind im
Archiv gesichert; `source-snapshot.json` bindet ihre SHA-256-Werte.

```bash
# Vorhandenen vollständigen Lauf prüfen, ohne neue neuronale Trials:
./NERVA_Modell/research/run.sh localize-homeostasis report
```
