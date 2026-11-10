# Richtungs- und Abrufpfadanalyse

Die Suite `direction_path` untersucht, ob natürliche Gewichtsänderungen in
Richtung einer tatsächlich wirksamen künstlichen Ergänzung zeigen und an
welchen elektrischen Übergängen ihre Wirkung ausbleibt.
Sie verwendet das [Forschungskontrollzentrum](CONTROL.md), dessen begrenzte
Seed-Parallelität, vollständigen Replay, Wiederaufnahme und Quellenarchive.
[Gemessener Pilot](DIRECTION_PATH_RESULTS_20261110.md).

## Ausführen

Alle Befehle beginnen in `DSVLM/main/`. Rust/Cargo mit lokal verfügbaren
Abhängigkeiten und Python 3.11+ genügen; die Suite verwendet den CPU-Referenzpfad.

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/direction_path_smoke.toml --output /tmp/nerva-direction
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/direction_path_smoke.toml --output /tmp/nerva-direction
./NERVA_Modell/research/run.sh control status --output /tmp/nerva-direction
./NERVA_Modell/research/run.sh control report --output /tmp/nerva-direction
```

Der feste Pilot verwendet `config/direction_path_pilot.toml`: 256 Core-Zellen,
8 Trainingswiederholungen, 11 Sekunden Ruhe pro Probe, Seeds 17011/17027.
`plan` simuliert nicht. Ältere Archive werden bei geändertem Controller über
ihre eigene `producer.py report --output <absoluter-laufordner>` ausgewertet.

## Gemeinsamer Zustand und feste Aufgabe

Das bestehende Bildprotokoll bleibt erhalten: Teilmuster A → Vollmuster A
und Teilmuster B → Vollmuster B. A→B bedeutet hier allgemein Cue→Ergänzung;
die beiden Bildklassen bleiben getrennt. Es wird keine neue Lernaufgabe als
erfolgreich ausgegeben.

Alle Zweige trainieren mit nativer Drei-Faktor-STDP, eingeschalteter Chemie
und dem bereits geprüften f64-Restbetrag. Lernraten, Zellen und Topologie bleiben
unverändert. Entwicklung, Homöostase und Rückführung sind ausgeschaltet.
Auditierter und rekonstruierter trainierter Zustand müssen vor jedem Eingriff
vollständig `Model::same_replay_state` erfüllen. Ein Gewichts-Rücktausch prüft,
dass auch Warteschlangen, lokale Uhren, Tags und Restbeträge unverändert sind.

Beim Abruf ist Plastizität eingefroren. Die feste Probensequenz lautet
Schwarzbild, Vollmuster A, Vollmuster B, Teilmuster A, Teilmuster B, jeweils
nach der konfigurierten Ruhe. Vollmusterantworten des Zweigs mit Anfangsgewichten
bilden die **festen** Zielvektoren aller Vergleiche. Sie werden nicht durch
die künstlich veränderten Vollmusterantworten ersetzt.

Das ist ein Vergleich vollständiger Sequenzen am gleichen Startcheckpoint.
Spätere Proben können unterschiedliche Vorgeschichten entwickeln; Ruhe wird
nicht als garantierter vollständiger Zustandsreset behandelt.

## Begrenzte künstliche Referenzsuche

Aus den Baseline-Spikes wird einmal eine externe, ausdrücklich künstliche
Richtung auf bereits vorhandenen, aktivierten und plastischen Verbindungen
berechnet:

1. Präneuronen erhalten ihre Spikezahldifferenz zwischen den beiden Cues.
2. Postneuronen erhalten die Differenz ihrer positiven Ergänzungsanteile
   `max(Vollantwort − Cueantwort, 0)` zwischen den Klassen.
3. Das Produkt bestimmt den Gewichtseingriff; bei inhibitorischen Quellen
   kehrt sich das Vorzeichen für den Gewichtsbetrag um.
4. Pro Pfadumfang wird auf den größten absoluten Beitrag normiert.

| Pfadumfang | Veränderte Verbindungen |
| --- | --- |
| `sensory_and_recurrent` | Sensor→Core und rekurrente Core→Core-Synapsen |
| `recurrent_only` | Ausschließlich Core→Core; Sensorverbindungen bleiben exakt gleich |

Jeder Umfang prüft additive Dosen 0,25 / 1 / 2 und die entgegengesetzte Richtung
als Kontrolle vertauschter Ziele: insgesamt zwölf feste Kandidaten.
Die Gewichte werden auf [0,2] begrenzt; tatsächliche Gewichte, Normen,
Unterstützungsmenge und Clipping bleiben im Rohrecord.

Eine gültige Referenz muss für **beide** Cues aktive Antworten mit positivem
Klassenkontrast erzeugen, den Baseline-Kontrast jeweils um mehr als `1e-9`
verbessern und die vertauschte Richtung bei identischer Dosis schlagen.
Ohne Cue darf die Core-Spikezahl gegenüber der Baseline nicht steigen.
Gewählt wird die erste gültige Dosis, nicht der größte beobachtete Gewinn.
Eine gültige rekurrente Referenz hat Vorrang; beide Pfadumfänge werden separat
berichtet. Ohne Erfolg bleibt die Richtungsreferenz `null`.

Verglichen werden natürliche Nettodeltas und die **tatsächlich installierten**
Referenzdeltas vom gemeinsamen Anfangsgewicht: Cosinus, Skalarprodukt,
L2-Normen, Vorzeichenübereinstimmung und natürliche L1-Masse auf den
Referenzsynapsen, global und nach Verbindungspfad. Bei Nullnorm ist der Winkel
undefiniert, nicht null. Ungeprüfte Kandidatenwinkel sind explorativ und
werden nicht als bestätigte Richtungsreferenz ausgewählt.

Ein Winkel allein beweist keinen Lernregelfehler: Die Änderungsnormen können
stark verschieden sein, Clipping verändert Richtungen, und eine einzelne
künstliche Lösung ist nicht die einzige mögliche Lösung. Erfolg mit
Sensor→Core-Eingriffen bestätigt zudem keinen rein rekurrenten Speicherpfad.
Die Suche nutzt Kalibrieraktivität derselben Bilder und Seeds; sie ist keine
unabhängige Bestätigung und kein Nachweis natürlich gelernten Gedächtnisses.

## Elektrische Pfadanalyse

Optionale native [Dynamikbeobachtungen](../../NERVA/docs/dynamics-audit.md)
erfassen zeitgenau kontinuierliche Potenzialentwicklung und
Integrationsgrenzen einschließlich neuromodulatorisch ausgelöster Entwicklung.

Pro Neuron und Probe werden erregende/hemmende Eingänge, Refraktärverwerfung,
Unterschwelligkeit, Spikes, Reset, kontinuierliche Entwicklung und
Additionsrundung summiert. Die gespeicherten Potenziale mit ihren tatsächlichen
lokalen Zeitstempeln erfüllen mit Toleranz `1e-9`:

```text
Endpotenzial − Anfangspotenzial
  = kontinuierliche Entwicklung + akzeptierter Netzeingang
    + Additionsrundung + Spike-Reset
```

Kontinuierliche Entwicklung umfasst Leck, intrinsische Ströme und Modulation;
sie wird nicht als isolierter passiver Leckverlust bezeichnet.
Die drei Integrationsklassen Spike / refraktär / unterschwellig sind
vollständig und disjunkt. Die Hemmungszählung ist eine Teilmenge:
Erregung allein würde am gleichen Ausgangspotenzial die Schwelle erreichen,
die tatsächliche Integration bleibt darunter. Das ist ein lokaler Vergleich,
kein simulierter Hemmungs-Knockout.

Pro Synapse stehen Ankunftszahlen, positive/negative Amplituden und Zuordnung
zu refraktären oder unterschwelligen Integrationen bereit. Der zentrale Bericht
verknüpft diese Daten mit tatsächlich veränderten Gewichten und nennt Neuronen
mit veränderten Schwellenabständen beziehungsweise Spikezahlen.
Gleiche Summen belegen keine identischen Zeitverläufe.

## Aufwand und Nachprüfbarkeit

Pro Seed: ein natürlicher Zweig, ein Anfangsgewichtszweig und zwölf Kandidaten;
14 native Trainingsdurchläufe, mit vollständigem Replay 28. Der Zweierpilot
enthält zwei Records, vier vollständige Ausführungen, 56 Trainingsdurchläufe
und höchstens 67.256 Modellschritte. Es gibt keine unbeschränkte Optimierung,
keine neuen Python-Pakete und keinen separaten Simulationsrunner.

Rohereignisse werden pro 50-ms-Fenster abgeholt. Die Cue-Messung ist auf
200.000 Ereignisse pro Fenster und 2 Millionen pro Zweig begrenzt;
Budgetüberschreitung macht den Versuch ungültig. Ruheereignisse werden
fensterweise geprüft und verworfen. Alle Aggregationen sind vollständig;
nur die ersten 128 Integrationsbeispiele pro Bild werden zusätzlich erhalten.
Der unabhängige Python-Report prüft Potenzialbilanzen, Kandidatenraster,
rekurrente Eingriffsgrenzen, feste Zielauswertung und Richtungswinkel erneut.

Wie bei den anderen Suiten entstehen Markdown, JSON, CSV, SQLite, Rohrecords,
Manifest, Binärdatei, Quellenarchiv und SHA-256-Beleg.
Ergebnisordner sind von Git ausgeschlossen und separat zu sichern:
[Ablage und Reproduktion](ARCHIVING.md).

