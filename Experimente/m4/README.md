# M4: Versuche und JSON-Vertrag

Die bereitgestellte Testsuite wurde in die getrennte Projektstruktur integriert:
Der Python-Orchestrator liegt hier, der einzelne neuronale Durchlauf und die
JSON-CLI liegen in `NERVA_Modell/Modell/src/m4/`. Die ursprünglich gelesene Testsuite unter
`NERVA_M4_testsuite/` verschwand während der Integration aus dem Workspace.
Die 20 Konfigurationen und der Ergebnisvertrag wurden aus den bereits gelesenen
Dateien übernommen; der Runner ergänzt parallele Jobs und das anschließend
angeforderte erste Gate mit vier einfachen Sensor-/Zeitbedingungen.

Vom Verzeichnis `DSVLM`:

```bash
# Verfügbare ursprüngliche 20 Bedingungen, ohne Ausführung:
python3 NERVA_Modell/Experimente/m4/runner.py plan --stage pilot --output /tmp/m4-pilot.json

# Neue gewünschte Prüfreihenfolge: 2/7 Kanäle, deterministisch, 8/20 ms, drei Seeds:
python3 NERVA_Modell/Experimente/m4/runner.py run --stage gate --jobs 4 \
  --output Messungen/Experimente/mein-m4-gate \
  --command './NERVA_Modell/Modell/run-demo.sh m4 --config {config} --output {output}'

# Einen vom Runner geschriebenen Fall direkt ausführen:
./NERVA_Modell/Modell/run-demo.sh m4 --config /tmp/config.json --output /tmp/result.json

# Isolierte Signal-/Aktuator-/STDP-Diagnose:
./NERVA_Modell/Modell/run-demo.sh m4 --mechanistic-audit --output /tmp/mechanisms.json

# Vollständige Matrix auf neuen Seeds (neues Ausgabeverzeichnis verwenden):
python3 NERVA_Modell/Experimente/m4/runner.py run --stage confirm --jobs 8 \
  --output Messungen/Experimente/meine-m4-bestaetigung \
  --command './NERVA_Modell/Modell/run-demo.sh m4 --config {config} --output {output}'

# Vorhandene vollständige Messungen erneut auswerten, ohne neue Neuronenläufe:
python3 NERVA_Modell/Experimente/m4/runner.py report --output Messungen/Experimente/m4-confirm
python3 NERVA_Modell/Experimente/m4/diagnose.py --output Messungen/Experimente/m4-confirm

python3 -m unittest discover -s NERVA_Modell/Experimente/m4/tests -v
```

`--stage pilot` bietet die ursprünglichen 20 Fälle mit Seeds 0–2 an.
`--stage confirm` verwendet ausschließlich neue Seeds 100–109. `--cases` wählt
Fälle gezielt aus. Ausgeführt wurden zuerst `gate` und direkte Mechanismustests,
danach alle 20 Fälle als `pilot` und `confirm`. Die Resultate liegen getrennt
unter `Messungen/Experimente/m4-initial-gate`, `m4-pilot` und `m4-confirm`.
Keine Bedingung bestätigt bisher Wirkungslernen. Ein sichtbares Ziel wird erst
nach einem belastbaren Wirkungslern-Nachweis untersucht.

## Konfigurationsvertrag

Der Runner schreibt pro Fall/Seed ein JSON mit exakt diesen Pflichtfeldern:

```json
{
  "protocol": "NERVA-M4-v1",
  "case_id": "a01_minimal_2ch",
  "seed": 0,
  "params": {
    "sensor": "effect_2ch", "resolution": 0,
    "encoding": "deterministic", "processing": 32,
    "action_effect_delay_ms": 8,
    "motor_feedback": true, "stdp": true,
    "homeostasis": true, "development": false,
    "initial_drive": 0, "pruning_min_us": 5000000,
    "normalization": "equal_input_current"
  },
  "arms": ["before", "aged_frozen", "paired", "scrambled"],
  "train_trials": 256,
  "test_trials": 128,
  "calibration_trials": 128
}
```

Unbekannte Felder, Sensornamen, Kodierungen und Auflösungen werden mit konkreter
Fehlermeldung und Exitcode ungleich null abgewiesen. `resolution: 0` ist der
explizite Suite-Sentinel für diagnostische 2-/7-Kanal-Sensoren; diese verwenden
das kanonische 64-Raster. Für `pixels` sind ausschließlich 8, 16 und 64 zulässig.
Trialzahlen müssen durch vier teilbar und im Bereich 8–4096 liegen. Kurze Läufe
mit acht Trials dienen technischen Tests, nicht dem Lernnachweis.

## Ergebnisvertrag

Die Pflichtfelder sind `protocol`, `case_id`, `seed`, `checks`, `arms`.
`checks` enthält `same_initial_network`, `matched_experience`, `no_leakage`,
`frozen_probe`, `balanced_labels`. Der Runner akzeptiert nur tatsächlich wahre
Prüfergebnisse; der Rust-Lauf bricht bei einem fehlgeschlagenen Audit ab.

Jeder der vier Arme enthält mindestens `balanced_accuracy`, `visual_decodability`,
`action_selection_rate`, `test_count`. Hinzu kommen die geforderten Spike-,
Refraktär-, Struktur-, Gewichts-, Feedback- und Aktionszählungen, Konfusionsmatrix,
visuelle Kalibrierqualität und vollständige `normal_mapping`/`reversed_mapping`-
Unterergebnisse. Primär ausgewertet wird der Zählreadout; acht relative Zeitbins
stehen in jedem Unterergebnis zusätzlich zur Verfügung.

`test_count` beträgt bei 128 Trials pro Belegung insgesamt **256**. Konfusionsmatrizen
haben Zeilen für wahres links/rechts und Spalten für vorhergesagtes links/rechts.
Es gibt keine Auffüllung unbekannter Klassen mit dem wahren Label.

Zählwerte auf Arm-Ebene summieren beide Belegungsinstanzen. `spike_count`,
`weight_updates`, Formation und Pruning dort beziehen sich auf das Training;
`before` hat deshalb null Trainingseinträge. Die einzelnen Phasen enthalten
separate Diagnosen. `synapse_count` ist ebenfalls über beide Instanzen summiert;
deren Einzelgrößen stehen in den Unterergebnissen.

`sensory_spikes` zählt erzeugte Eingangskanal-Spikes vor dem sensorischen
Neuron, `core_spikes` und `motor_spikes` tatsächliche neuronale Emissionen.
`spike_count` ist die Summe dieser drei Zähler und enthält keine separate
Aktionskopie-Population. Der tatsächlich weitergeleitete sensorische Strom
wird unabhängig aus synaptischen Ankünften gemessen. Kanalraten allein
garantieren somit keine entsprechende Sensor→Core-Weiterleitung.

`action_selection_rate` misst vorhandene Decoder-Vorschläge im bekannten
Aktionsfenster. Diese Vorschläge werden im kontrollierten Versuch nicht
ausgeführt; die Kennzahl misst weder richtige Zielentscheidungen noch Erfolg.

## Statistische Auswertung

`summary.csv` enthält gepaarte Bootstrap-Kontraste des primären Zählreadouts.
Die Einheit des Bootstraps ist der Seed; beide Belegungen werden für diesen
Überblick zuerst pro Seed gemittelt. `diagnose.py` ergänzt `contrasts.csv` mit
allen separaten Belegungen, drei Prüfzeitpunkten und beiden Auswertern sowie
`diagnostics.json` mit Strom-, Struktur- und Signalbefunden. Es prüft außerdem
jede Trial-/Phasen-CSV unabhängig auf Kausalität, Replay, Balance und Ablationen.
Bei fehlenden Fall-/Seed-Dateien bricht diese vollständige Diagnose ab.

`preliminary_all_three_controls` ist ein erster aggregierter Hinweis.
`m4_criteria_met` erfordert mindestens zehn Seeds, vollständige Ergebnisse,
positive untere 95-%-Intervallgrenzen gegen `before`, `aged_frozen` und
`scrambled` in beiden Belegungen und allen drei Prüfzeitpunkten sowie mindestens
80 % visuelle Auslesbarkeit je Belegung. Die Intervalle sind deskriptiv und
nicht für multiple Vergleiche korrigiert. Einzelne positive Kontraste oder hohe
Motoraktivität erfüllen diese Kriterien nicht.

## Phasen und Kausalität

Alle Trials dauern **160 ms**, unabhängig vom konfigurierten Abstand:

| Abschnitt | Dauer | Eingang/Aktion |
|---|---:|---|
| Baseline | 50 ms | Aktuelles mittiges Startbild |
| Aktion bis Wirkung | 2/8/20/50 ms | Bereits ausgeführte Taste als Kopie; weiterhin Startbild |
| Vorhersage | Am Ende dieses Fensters | Nur abgeschlossene Core-Spike-Merkmale |
| Physische Bewegung | Direkt danach | Die verzögerte Welt führt den vorgemerkten Schritt aus |
| Wirkungsbild | 50 ms | Jetzt erst links/rechts präsentieren |
| Neutrale Auffüllung | Rest bis 160 ms | Startbild, keine weitere Taste |

Im Mikrosekundenraster ist der Vorhersageabschluss strikt vor Bewegung und
erstem Wirkungspixel. Bewegung und Präsentation können denselben Mikrosekundenwert
haben: Die Welt wird zuerst fortgeschrieben, danach wird ihr Bild präsentiert.
Die Diagnose prüft zusätzlich, dass vor dem Bewegungstermin noch das unveränderte
Startbild vorliegt. Es werden echte Zustandsübergänge ausgeführt und protokolliert.

Der Modellport besitzt ausschließlich das neuronale Modell und Beobachtungsdaten.
Er bekommt aktuelle Frames und bereits ausgeführte Tasten. Welt, Labels,
Zielkoordinaten, korrekte Aktion und künftige Frames werden nicht darin gespeichert.
Die Zukunftsbilder liegen nur beim externen Versuchsaufbau.

`paired` und `aged_frozen` erhalten nachweislich dieselben tatsächlich an den
Port übergebenen Trainingsbilder, Tasten und Zeiten. `scrambled` behält exakt
dieselbe Tastenfolge und Bildhäufigkeiten; in jedem Viererblock kommt jede der
vier Aktion-/Bildkombinationen einmal vor. Das veränderte Trainingsbild ist ein
absichtlicher Replay-Eingriff, nicht die Behauptung einer anderen realen Tastenwirkung.
Probe-Bilder entsprechen bei allen Armen wieder der echten jeweiligen Belegung.

## Sensorik, Inputbudget und Auswerter

Das kanonische 256×256-Bild ist eine exakte 4×-Rasterung des bestehenden
64×64-Renderers. Es fügt keine künstlichen Details hinzu. Rechteckige Flächenmittelung
mit Fließkommawerten verkleinert es vor der sensorischen Merkmalsbildung. Alle
sieben Positionen und beide Wirkungsbilder werden automatisch auf Unterscheidbarkeit
geprüft. Der diagnostische 2-Kanal-Sensor soll nur links/rechts unterscheiden;
dass mehrere Positionen dabei dieselben Merkmale haben, ist beabsichtigt.

Die Eingangsrate wird vor dem ersten Neuronenereignis mit einem festen Faktor
skaliert. Der Faktor ergibt sich ausschließlich aus Anfangsgewichten, geometrischer
Dämpfung und gleich gewichteten visuellen Trials. Ziel sind **10 Einheiten
anfänglicher sensorischer synaptischer Strom pro Sekunde und Core-Neuron**, als
Populationsmittel über den **gesamten** Trial einschließlich neutraler Phasen.
Es werden weder neue Verbindungen ergänzt noch Gewichte angepasst, um dieses
Budget zu erzwingen. Null versorgte Core-Zellen bleiben null versorgt.

Diese Normierung garantiert keinen identischen Strom an jeder einzelnen Zelle.
Die erwartete Verteilung wird gespeichert; tatsächlicher Eingang, Refraktärverluste
und Abweichungen werden pro Phase gemessen. Während Lernen und Prüfung erfolgt
keine erfolgsabhängige Nachregelung. ON/OFF bleibt bei statischen Bildern still.
Die STDP-Abschaltung verwendet den bestehenden Runtime-Schalter, der auch
Utility-Lernupdates einfriert; dies ist keine neue Lernregel.

Der aus M3 wiederverwendete standardisierte Zentroid-Auswerter wird ausschließlich
auf getrennten rein visuellen Kalibriertrials angepasst. Die erste Hälfte prüft
chronologisch an der zweiten Hälfte die visuelle Auslesbarkeit; der Auswerter für
Vorhersagen nutzt anschließend alle Kalibrierbeispiele. Training, Kalibrierung
und die drei Prüfprotokolle haben getrennte, dokumentierte Seeds. Pro Belegung
verwenden alle Arme dieselbe jeweilige Testreihenfolge.

Die Merkmalfenster sind so lang wie der konfigurierte Aktions-/Wirkungsabstand.
Das wird ausdrücklich gespeichert: Verbesserte visuelle Auslesbarkeit bei 20
gegenüber 8 ms kann auch am längeren Beobachtungsfenster liegen. Der Versuch
isoliert deshalb nicht jede mögliche Wechselwirkung zwischen Delay und Auswerter.
Die Gesamtdauer und das anfängliche erwartete Inputbudget bleiben vergleichbar.

Die zusätzliche Pixel-Baseline verwendet einen separat visuell kalibrierten
8×8-Pixel-Zentroid-Auswerter auf dem noch unveränderten Startbild. Ihre gemessene
balancierte Genauigkeit beträgt erwartungsgemäß 50 %. Sie liest keine Aktion oder
künftigen Pixel.

## Freeze, Ruhe und Interferenz

Die laufende trainierte Instanz wird eingefroren. Danach folgen Kalibrierung und
erste Prüfung, zwei Sekunden ohne Aktionen und erneute Prüfung, anschließend
1,4 Sekunden zusätzliche visuelle Aktivität über alle sieben Positionen und eine
dritte Prüfung. Der Auswerter wird dabei nicht neu angepasst. Lernparameter,
Endpunkte, Delays, gespeicherte Traces/Utility/Eligibility und Struktur müssen
unverändert bleiben; dynamische Zustände dürfen weiterlaufen.

Die erste Prüfung beginnt nach der visuellen Kalibrierung (128 × 160 ms =
20,48 Sekunden); sie misst keine Vorhersage direkt am Trainingsende. Die
Trainingsende-Membranzustände stehen separat im Ergebnis. Ein durch diese
Kalibrieraktivität verdrängtes kurzlebiges Muster wird durch das Protokoll
nicht ausgeschlossen. Ruhe bedeutet neutrales Bild ohne Aktionen, nicht
garantierte neuronale Stille. Die Ruheprobe liegt vor der zusätzlichen
Interferenz; deren Einfluss ist deshalb nicht vollständig isoliert.

## Rohdaten und Grenzen

Neben `result.json` entstehen `result.json.details.csv` und
`result.json.audit.csv`; `--raw-events` ergänzt `result.json.events.csv`.
Die gemessenen Gate-Rohereignisse wurden anschließend verlustfrei nach
`.events.csv.gz` komprimiert. Sie enthalten die tatsächlichen NERVA-Spike-/Ankunfts-
und Gewichtsereignisse. Der separate Mechanismusaudit schreibt
`mechanisms.json.signal-events.csv`.

Die Membranschnappschüsse des Hauptlaufs zeigen NERVAs gespeicherte lokale
Zustände nach dem jeweiligen Ereignisfenster. Bei ereignisgesteuerter Integration
können die letzten Aktualisierungszeiten verschiedener Neuronen verschieden sein;
diese Werte sind kein zusätzlich trainierter Membranauswerter. Die gesonderte
Feedbackdiagnose projiziert dagegen unveränderliche Neuronenkopien analytisch
auf denselben Stichtag bei 20 ms und führt keine Werte ins Modell zurück.

Die Gate-Fälle auf drei Seeds und die vollständige Matrix auf zehn neuen Seeds
bestätigen keine allgemeine Handlungsfähigkeit oder Übertragung auf andere
Umgebungen. Nach dem negativen Gate wurden entsprechend der ergänzten Vorgabe
zuerst Signalwege und STDP geprüft. Die anschließenden Reihen verändern keine
neuronalen Lernregeln und wählen keine Parameter anhand von Testergebnissen nach.

[Gemessener Lauf und Befund](../M4_EXPERIMENT.md)
