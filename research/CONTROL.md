# Harness als Forschungskontrollzentrum

Diese Seite beschreibt die Standardsuite `association`.
Die Suite [direction_strength](DIRECTION_STRENGTH.md) kreuzt Richtung und
L2-Stärke und erweitert die rekurrente Referenzsuche; identische Gewichtsvektoren
teilen eine Beobachtung, Plankosten sind dabei Obergrenzen.
Die [Richtungs- und Abrufpfadanalyse](DIRECTION_PATH.md), Suite `direction_path`,
ergänzt eine begrenzte künstliche Referenzsuche und native elektrische Bilanzen.
Die Suite [`precision`](PRECISION.md) vergleicht f32-Direktupdates und
erhaltene kleine Lernbeiträge mit denselben Anfangsnetzwerken und Eingängen.
Die zusätzliche [kausale Diagnosekette A–D](CAUSAL_CHAIN.md) verwendet denselben
Einstieg mit `suite = "causal_chain"`: numerische Plastizität, künstliches
Gedächtnis, feste Wirksamkeitsdosen und lokale Informationsverfügbarkeit.

Der Befehl `research/run.sh control` bündelt drei Prioritäten in einem festen,
gepaarten Versuch: Neuromodulation an/aus, Drei-Faktor-STDP gegen Pair-STDP und
die Diagnose der Kette Eingang → Aktivität → Gewicht → Abruf. Er verwendet
den vorhandenen Rust-Runner, dessen Replayprüfung und Ergebnisablage.

Alle folgenden Befehle beginnen in `DSVLM/main/`. Voraussetzungen:
Rust/Cargo mit lokal verfügbaren Paketen und Python 3.11+. Eine QEMU-VM,
CUDA oder zusätzliche Python-Pakete werden nicht benötigt.

## Planen, ausführen und prüfen

Zuerst den kleinen technischen Versuch verwenden:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/hippocampal_control_smoke.toml --output /tmp/nerva-control-smoke
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/hippocampal_control_smoke.toml --output /tmp/nerva-control-smoke
./NERVA_Modell/research/run.sh control status --output /tmp/nerva-control-smoke
./NERVA_Modell/research/run.sh control report --output /tmp/nerva-control-smoke
```

`plan` validiert alle Kontrollen und Budgets, baut einmal im Release-Modus und
archiviert den ausführbaren Stand. Es startet keine neuronalen Läufe.
`run` verwendet diese Binärdatei und setzt unvollständige Archive fort.
`report` validiert vorhandene Records mit dem archivierten Runner und erzeugt
Tabellen und Datenbank neu; es simuliert keine fehlenden Läufe.
`status` zeigt den Dateifortschritt und den Stand des abgeschlossenen Prüfbelegs.

Der feste 256-Zellen-Pilot hat eine eigene Konfiguration:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/hippocampal_control_pilot.toml
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/hippocampal_control_pilot.toml
./NERVA_Modell/research/run.sh control status NERVA_Modell/research/config/hippocampal_control_pilot.toml
```

Ohne `--output` liegt der Lauf unter `research/results/<control.name>/`.
Ergebnisse sind von Git ausgeschlossen und separat zu sichern.
Für geänderte Konfigurationen oder einen neuen Programmstand einen neuen
Ausgabeordner verwenden; bestehende Pläne bleiben an ihren archivierten Runner
gebunden. Es gibt keine automatische Parametersuche oder Modellübernahme.

## Feste Kontrollen

Alle acht Arme verwenden Hippocampus-Gewebeprofile mit derselben weichen
Arealgrenze, STP, Sensorik, Startgewichten und Topologie. Strukturentwicklung,
Homöostase und Aktionsrückführung sind ausgeschaltet.

| Kontrolle | Lernregel | Chemie | Gewichtslernen | Zeitliche Zuordnung |
| --- | --- | --- | --- | --- |
| `three_on` | Drei-Faktor-STDP | an | an | gekoppelt |
| `three_off` | Drei-Faktor-STDP | aus | an | gekoppelt |
| `pair_on` | Pair-STDP | an | an | gekoppelt |
| `pair_off` | Pair-STDP | aus | an | gekoppelt |
| `frozen_on` | Drei-Faktor-STDP | an | aus | gekoppelt |
| `frozen_off` | Drei-Faktor-STDP | aus | aus | gekoppelt |
| `three_unpaired` | Drei-Faktor-STDP | an | an | vertauschte Folgebilder |
| `pair_unpaired` | Pair-STDP | an | an | vertauschte Folgebilder |

Die chemische Ablation erhält Rezeptoren und Quellzellen. Bei Drei-Faktor-STDP
ohne chemischen Faktor können Tags entstehen, aber keine faktorgetriebene
Konsolidierung. Pair-STDP verändert die Gewichte unmittelbar. Chemie verändert
zusätzlich Erregbarkeit und Übertragung; ihr Gesamteffekt ist deshalb nicht
ausschließlich ein Lernregel-Effekt.

Die ausdrückliche Auswahl erfolgt über
`Model::with_sensor_and_learning(..., LearningRule)`.
Die bisherigen Live-Modell-Aufrufe behalten ihre automatische Regelwahl.

## Aufgabe und identische Startbedingungen

Zwei 8×8-Vollbilder haben sechs gemeinsame und je zwei eigene weiße Pixel.
Ein Teilbild enthält nur die zwei eigenen Pixel. Training zeigt Teilbild A →
Vollbild A → Schwarz und Teilbild B → Vollbild B → Schwarz; die Reihenfolge
wechselt seedabhängig zwischen AB und BA. Im entkoppelten Arm werden die
Vollbilder vertauscht. Zahl und Häufigkeit der Bilder bleiben gleich.

Die neuronalen Eingänge sind ausschließlich aktuelle Pixel. Labels,
Ähnlichkeitswerte und richtige Zuordnungen bleiben in der externen Auswertung;
der Prüfer erzeugt weder Belohnung noch künstliche Modulatorpulse.

Der Harness prüft pro Seed gemeinsame Anfangsnetz-Digests und bei den sechs
gekoppelten Kontrollen identische zeitgestempelte Eingangs-Digests. Ein nativer
Test vergleicht die vollständigen initialen Netzwerke aller acht Kontrollen.
Lernregel und Chemieschalter sind die erklärten Interventionen; diese
unterschiedlichen Einstellungen werden nicht als identischer Zustand ausgegeben.

Eine untrainierte, eingefrorene Instanz liefert die Vorherreferenz. Das Training
beginnt separat am unangetasteten Startzustand und wird nicht durch diese
Vorherprobe vorgeprägt.

## Abruf und Gewichtswirkung

Jeder Arm wird mit und ohne Learning Audit bis zum selben trainierten Zustand
ausgeführt. `Model::same_replay_state` muss vor dem Eingriff vollständig
übereinstimmen. Beide Instanzen werden eingefroren; eine behält die gelernten
Gewichte, die andere erhält ausschließlich die Anfangsgewichte zurück.
Ein Rücktausch prüft zusätzlich, dass alle übrigen Zustände gleich geblieben sind.

Beide Instanzen sehen dieselben vier Abrufbilder: Vollbild A, Vollbild B,
Teilbild A, Teilbild B, jeweils nach festgelegter Schwarzbildruhe. Spätere
Aktivitätsunterschiede dürfen als Folge des Gewichtseingriffs entstehen.
Es werden keine Zellzustände, STP-Zustände oder chemischen Felder zurückgesetzt.

Der Ergänzungskontrast eines Teilbildes ist seine Cosinusähnlichkeit zur
Core-Antwort auf das eigene Vollbild minus der Ähnlichkeit zum anderen Vollbild.
Stille erhält keine Ähnlichkeitsgutschrift.

- `completion_a/b`: Ergänzungskontrast mit gelernten Gewichten.
- `acquisition_a/b`: Nachher minus unabhängige untrainierte Referenz.
- `weight_gain_a/b`: Gelernter Abruf minus Abruf mit Anfangsgewichten am
  gleichen trainierten Checkpoint.
- `recall_core_effect`, `recall_motor_effect`: unterschiedliche genaue
  Spikeereignisse während der Abrufbilder.
- `recall_potential_effect`: unterschiedliche gespeicherte Core-Potenziale
  an den Abfragezeitpunkten; kein nachträgliches Fortschreiben auf gemeinsame
  neuronale Zeitanker.

Die acht vorab festen Tests vergleichen `weight_gain` für beide Teilbilder:
`three_on − three_off`, `three_on − pair_on`,
`three_on − frozen_on` und `three_on − three_unpaired`.
Einseitige exakte Vorzeichentests behalten Gleichstände als Nichtgewinne im
vollen Nenner; die gemeinsame Schwelle beträgt strikt `p < 0.05 / 8`.
Abrufdifferenzen werden zusätzlich deskriptiv ausgewiesen.

Die zwei Pilotseeds sind keine unabhängige Bestätigung. Auch ein bestandener
explorativer Test wäre kein allgemeiner Gedächtnis- oder Kognitionsnachweis.
`functional_memory_demonstrated` bleibt ausdrücklich `false`.

## Audits und Effizienz

Die Audits werden nach jedem Frame geleert und kompakt aggregiert:

| Stufe | Messung |
| --- | --- |
| Eingang | Erzeugte und tatsächliche sensorische Spikes, Eingangs-Digests, Ankünfte und Amplituden am Core |
| Aktivität | Core-/Motor-Spikes je Trainingsfenster, genaue Spike-Digests beim Abruf |
| Lernen | Tags, lokale Faktoren, angefragte Updates, Anfragen ohne Gewichtsänderung, tatsächliche Änderungsmasse, Nettoänderungen je Verbindungspfad |
| Abruf | Beide Teilbildkontraste, Gewichtsreset, Spike-/Potenzialwirkung und unveränderte Lernparameter |

Pro Frame sind maximal 100.000 Auditzeilen erlaubt, pro Training eine Million.
Ein Überlauf oder eine Abweichung zwischen Audit und nativen Gewichtsereignissen
macht den Lauf ungültig. Volle rohe Audittraces werden hier nicht auf Platte
dupliziert; für eine vertiefte Einzelereignisanalyse bleibt die vorhandene
[kausale Diagnose](CAUSALITY_256.md) verfügbar.

Die gemeldeten Engpässe sind Beobachtungshinweise: beispielsweise Tags ohne
Updateanforderungen, Anforderungen ohne tatsächliche Gewichtsänderung oder
Gewichtsänderungen ohne Spikewirkung im Abruf. Das beweist keine allgemeine
Informationsabwesenheit. Rundung und Clipping werden durch diesen kompakten
Hinweis allein nicht kausal unterschieden.

Angefragte Updateanforderungen und Nullfaktor-Integrationen betreffen die
verzögerte Drei-Faktor-Regel. Nullwerte bedeuten bei Pair-STDP keine fehlenden
direkten Updates. Ein fehlender motorischer dritter Faktor ist für
Drei-Faktor-Lernen relevant; Pair-STDP benötigt diesen Faktor nicht, und
eingefrorene Kontrollen sollen keine Gewichte lernen.

Unabhängige Seeds laufen mit begrenzter Parallelität; jeder Seed enthält alle
acht Kontrollen. Ein Datensatz bedient alle drei Prioritäten. Vollständige
Records werden beim Neustart geprüft und wiederverwendet. Der kleine und der
256-Zellen-Pilot umfassen jeweils 16 Records, 32 Replay-Ausführungen und
64 Trainingsdurchläufe einschließlich Checkpointrekonstruktion.

`max_trials`, `max_native_training_passes` und `max_wall_seconds` begrenzen
die Ausführung. Zusätzlich weist der Plan die maximale Zahl der Modellschritte
aus. Das Zeitbudget gilt für die Rust-Ausführung einschließlich Replay und
nativer Recordprüfung; Kompilierung und zentrale Python-Aufbereitung werden
separat ausgeführt. Die Workerzahl ist auf 1–8 begrenzt.

## Dateien und Sicherung

| Datei | Inhalt |
| --- | --- |
| `control-plan.json`, `control-plan.sha256` | Fester Gesamtplan, Prioritäten, native Teilpläne, Kosten und Artefakthashes |
| `source.toml`, `source-code.zip`, `producer.py`, `runner` | Originalkonfiguration, Rust-/Controller-Quellen und ausgeführte Binärdatei |
| `seeds/s<seed>/` | Native Manifeste, vollständige Records und Zusammenfassungen |
| `logs/` | Ausgabe der nativen Prozesse |
| `CONTROL_REPORT.md`, `control-report.json` | Vergleich der Prioritäten, Kontrollmittelwerte und beobachtete Engpässe |
| `control-metrics.csv`, `control.sqlite` | Messwerte zum eigenen Filtern und Auswerten |
| `receipt.json` | SHA-256 aller abgeschlossenen Roh-Records und Bezug zum Gesamtplan |

`status` zählt vorhandene Dateien; `run/report` prüft ihre native Integrität.
Nach Abschluss schützt der SHA-256-Beleg zusätzlich gegen unbemerkte
Änderungen der ursprünglichen Records. Prüfsummen erkennen Veränderungen,
ersetzen aber keine vertrauenswürdige externe Sicherung.

Ein Archiv mit älterem Controller wird in einer kompatiblen Laufzeitumgebung
über dessen archivierte `producer.py` erneut geprüft:

```bash
python3 /absoluter/laufpfad/producer.py report --output /absoluter/laufpfad
```

[Ablage und Reproduktion](ARCHIVING.md) · [Harness](README.md) ·
[Zentrale Dokumentation](../../Doku/README.md).
