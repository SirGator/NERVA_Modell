# Vorregistrierter Homöostasevergleich auf neuen Seeds

Protokoll vom **05.11.2026**. Der vorherige
[größenübergreifende Diagnoseversuch](SCALING_RESULTS_20261105.md) deutete auf
schwächere visuelle Auslesbarkeit mit Homöostase hin. Der neue Vergleich prüft
diesen konkreten Effekt mit **24 neuen gepaarten Seeds**, unveränderter STDP
und ausgeschalteter Strukturentwicklung. Netzgrößen: **17, 256, 1.025 und
4.096 Verarbeitungsneuronen**.

[Konfiguration](config/homeostasis_confirmation.toml) ·
[Ausführungswerkzeug](confirm_homeostasis.py) ·
[Analysemethode](analysis/homeostasis_confirmation.py).

## Feste Endpunkte und Entscheidung

Je Größe werden zwei vorher festgelegte Endpunkte im aktiven Arm geprüft:

| Endpunkt | Gepaarte Differenz | Test |
|---|---|---|
| Visuelle Prüfgenauigkeit | Homöostase aus minus an | Einseitig auf einen Verlust mit Homöostase; positive mittlere Differenz erforderlich |
| Aufgezeichneter externer Vorhersagefehler (`position_mae`) | Homöostase an minus aus, in Gitterzellen | Zweiseitig auf eine Änderung; Vorzeichen der mittleren Differenz muss zur Mehrheit der Seedvorzeichen passen |

Die unabhängige Auswertungseinheit ist der Netzwerkseed. Sieben visuelle
Prüfpräsentationen beziehungsweise acht Aktionsproben eines Seeds werden nicht
als unabhängige Netzwerke gezählt. Alle 24 Seedpaare bleiben in der Auswertung.

Es gibt **acht Tests in einer Familie**. Bonferroni verteilt deren gesamte
Fehlerwahrscheinlichkeit von 0,05 auf `0,05 / 8 = 0,00625` je Test. Die
Entscheidung verlangt strikt `p < 0,00625`. Die exakten binomialen Vorzeichentests
zählen Gleichstände konservativ im vollständigen n als Nichtgewinne; beim
zweiseitigen Test werden beide Richtungen berücksichtigt. Die Berichte enthalten
positive, negative und gleiche Paare sowie jede einzelne Seeddifferenz.

Zusätzlich werden mit 4.000 deterministischen Bootstrapziehungen des Seedmittels
95-%-Intervalle berechnet. Diese Intervalle sind unadjustierte deskriptive Angaben;
sie ersetzen nicht die familienkorrigierte Entscheidung. Ein nicht bestandener
Test belegt keine Gleichwertigkeit.

Visuelle Genauigkeit unter 0,5 ist hier ein möglicher **Endpunkt**, kein Grund,
einen Seed auszuschließen. Der MAE-Vergleich misst den Fehler des festgelegten
externen visuellen Ausleseverfahrens. Er kombiniert Repräsentation, Kalibration
und Vorhersageaktivität; bei schwacher visueller Kalibration lässt sich daraus
keine isolierte Veränderung interner neuronaler Vorhersagefähigkeit ableiten.
Wie viele Paare in beiden aktiven Armen das visuelle 0,5-Gate erreichen, wird
separat berichtet.

## Kontrollbedingungen

Jede Größe wird mit Homöostase an/aus und den bisherigen sechs Bedingungen
ausgeführt: aktiv, passiv und entkoppelt, jeweils mit und ohne Plastizität.
Die Primärtests verwenden ausschließlich den vorab festgelegten aktiven Arm.
Die anderen Arme dienen technischen Kontrollen und deskriptiver Einordnung;
sie erzeugen keine zusätzlichen bestätigenden Hypothesen.

Prüfungen vor der Ergebnisentscheidung:

- Identische Anfangsnetze, Bilder und Zeitpunkte über die beiden
  Homöostasestände und alle Kontrollarme eines Größen-/Seedpaares.
- Vollständige deterministische Wiederholung und eingefrorene Messparameter.
- Kein Wachstum oder Pruning; unveränderte Synapsenzahl in jedem Lauf.
- Keine Homöostaseupdates bei ausgeschaltetem Schalter oder im eingefrorenen
  Arm; tatsächlich vorhandene Updates bei aktivierter Homöostase.
- Keine Gewichtsänderungen in den eingefrorenen Kontrollen, keine
  Aktionskopieankünfte in passiven Armen, richtige Vorhersage-/Wirkungszeitordnung.
- Gleiche Endpunkte der passenden eingefrorenen Kontrollen beider
  Homöostasestände sowie vollständige, endliche und budgetierte Messwerte.

Technisch ungültige oder fehlende Records stoppen die Auswertung. Es werden
keine Ersatzseeds gewählt und keine unpassenden Paare entfernt. Funktional
schwache, aber technisch gültige Netze bleiben enthalten.

## Vorregistrierung und Seedprüfung

`confirm_homeostasis.py plan` prüft zuerst die Native-Konfiguration und lokale
Seedkataloge. Die Prüfung umfasst Research- und Messarchivmanifeste sowie
konfigurierte und reservierte Seeds. Der neue Plan hatte keine Überschneidungen
mit 162 bekannten Seeds aus 215 geprüften Dateien. Dies ist eine Aussage über
die geprüften lokalen Kataloge, nicht über unbekannte externe Archive.

Vor der ersten Simulation werden Methoden, Größen, Seeds, Testfamilie,
Entscheidungsregeln, Quell-/Binärdateihashes und das native Manifest unveränderlich
in `preregistration.json` festgehalten. Bei dieser Registrierung müssen null
Roh-Records vorhanden sein. Zwei gleichzeitig angelegte Bestätigungspläne
reservieren Seeds unter einer gemeinsamen Sperre.

Der Versuch führt keine Auswahl eines Finalisten aus. Er verwendet auch keine
neuen Prüfseeds für eine nachträgliche Modelloptimierung. Beide
Homöostasekonfigurationen und alle vier Größen stehen vor der Datenerzeugung fest.

## Ausführen und wieder prüfen

Vom Projektverzeichnis `DSVLM`:

```bash
./NERVA_Modell/research/run.sh confirm-homeostasis plan
./NERVA_Modell/research/run.sh confirm-homeostasis run
./NERVA_Modell/research/run.sh confirm-homeostasis report
```

`run` setzt fehlende native Jobs mit demselben Programmstand fort. `report`
verlangt die vorhandene Vorregistrierung und startet keine fehlenden Trials.
Geänderte Protokolle, Quellen oder Binärdateien benötigen einen neuen
Ausgabeordner und bisher unbenutzte Seeds. `--config`, `--output` und `--binary`
erlauben explizite Pfade. Die Analyse ist auch für andere positive Größen
implementiert; die Testfamilie wächst dann auf zweimal die Anzahl dieser Größen.

Der feste Lauf umfasst **8 Varianten × 24 Seeds × 6 Kontrollen = 1.152 Jobs**
beziehungsweise **2.304 vollständige Modellausführungen einschließlich Wiederholung**.
Kalibration und Aktionsproben sind Teile dieser Ausführungen.

Archiv unter `results/homeostasis_confirmation_20261105/`:

| Datei | Inhalt |
|---|---|
| `preregistration.json` | Unveränderliches Protokoll, Code-/Binärdateiidentität und Seed-Audit vor Simulation |
| `manifest.json`, `source.toml`, `records/` | Bestehender nativer Runner und originale Messdaten |
| `metrics.csv`, `results.sqlite` | Tabellen und lokaler Katalog |
| `confirmation.json`, `confirmation.md` | Alle Kontraste, Entscheidungen, Einzelpaare und Interpretationsgrenzen |
| `verification-receipt.json` | SHA-256 aller Roh-Records, Vorregistrierung und Analyse; Nachweis unveränderter Rohdaten nach Report |

Die sechs zusätzlichen Tests prüfen konservative Gleichstände, beide
Vorhersagefehlerrichtungen, Familienkorrektur, Seedüberschneidungen, echte native
Vorregistrierung/Ausführung, schwache Ausleser ohne nachträglichen Ausschluss,
unveränderte erneute Auswertung sowie Abweisung beschädigter Daten und verspäteter
Registrierung.

[Ergebnisse des vollständigen Laufs](HOMEOSTASIS_RESULTS_20261105.md).
