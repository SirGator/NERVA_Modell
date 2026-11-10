# Hippocampus-Kontrollzentrum: erster gepaarter Pilot

Ausgeführt am **10.11.2026** mit dem neuen Szenario
`hippocampal_association` im bestehenden Research-Harness.
[Protokoll und Bedienung](CONTROL.md) ·
[Feste Konfiguration](config/hippocampal_control_pilot.toml).

## Design und technische Prüfung

256 Processing-Zellen, Seeds **17011 und 17027**, acht Trainingswiederholungen,
elf Sekunden Schwarzbildruhe vor jedem Abrufbild, acht Kontrollen pro Seed.
Strukturentwicklung, Homöostase und Aktionsrückführung sind aus.
Die gleichen initialen Zellen, Verbindungen und Gewichte werden geprüft;
Chemie, Lernregel und zeitliche Zuordnung sind explizite Interventionen.

16 Records enthalten vollständigen Replay. Einschließlich auditierter und
unauditierter Checkpointrekonstruktion sind es 64 Trainingsdurchläufe.
Der Plan weist 87.936 Modellschritte aus. Zwei begrenzte Worker verarbeiten
unabhängige Seeds; der Runner wird nur einmal gebaut und anschließend archiviert.

Die Learning Audits sind vollständig und verändern den trainierten Replayzustand
nicht. Vor dem Gewichtsreset stimmen beide Checkpoints vollständig überein;
ein Rücktausch der Gewichte stellt denselben Replayzustand wieder her.
Eingangs-Digests stimmen in den gekoppelten Kontrollen überein; alle acht Arme
haben identische initiale Gewichtssnapshots und gleich häufige Bildexpositionen.

## Priorität 1: Neuromodulation an/aus

Mittlere Ergänzungskontraste im Drei-Faktor-Arm:

| Chemie | Teilbild A | Teilbild B | Gewichtswirkung A | Gewichtswirkung B |
| --- | ---: | ---: | ---: | ---: |
| an | 0,314688 | 0,333853 | 0 | 0 |
| aus | 0,320681 | 0,337570 | 0 | 0 |

Gewichtswirkung bezeichnet die Differenz zwischen Abruf mit gelernten und mit
Anfangsgewichten ab demselben trainierten Checkpoint. Chemie erzeugt hier keinen
positiven gewichtsbedingten Abrufvorteil. Die kleinen Unterschiede der
unmittelbaren Ergänzungskontraste sind ebenfalls in den entsprechenden
eingefrorenen Kontrollen vorhanden. Sie sind kein Nachweis gelernten Gedächtnisses.

## Priorität 2: Drei-Faktor-STDP gegen Pair-STDP

Mit Chemie haben beide Regeln dieselben mittleren Ergänzungskontraste und
keinen gemessenen `weight_gain` für beide Teilbilder. Die mittlere Summe
absoluter bleibender Gewichtsänderungen beträgt etwa **0,00007986** für
Drei-Faktor-STDP und **0,04723** für Pair-STDP.

Größere Gewichtsänderungen unter Pair-STDP führen hier somit ebenfalls zu
keinem positiven Ergänzungsgewinn. Im Pair-STDP-Arm ohne Chemie ändert der
Gewichtsreset für einen Seed die präzisen Core-Spikes; die mittleren
Gewichtswirkungen auf die Ergänzungskontraste sind negativ. Motor-Spikes
ändern sich dabei nicht. Das ist eine diagnostische Gewichtswirkung und kein
sinnvoller Lernvorteil.

Die acht vorab festgelegten explorativen Vergleiche auf Gewichtswirkung
haben jeweils null positive Seedpaare und `p = 1`. Die zwei Seeds erlauben
keine unabhängige Bestätigung. Die positiven absoluten Ergänzungskontraste
sind auch ohne Gewichtslernen vorhanden.

## Priorität 3: beobachtete Grenzen entlang der Messkette

Eingangssignale und neuronale Aktivität sind vorhanden. Im Drei-Faktor-Arm
mit Chemie zeigen die vollständigen Trainingsaudits:

| Seed | Nichtnull-Updateanforderungen | Davon ohne Gewichtsänderung | Anteil | Tatsächliche Gewichtsereignisse |
| --- | ---: | ---: | ---: | ---: |
| 17011 | 176.955 | 176.920 | 99,9802 % | 35 |
| 17027 | 237.023 | 233.151 | 98,3664 % | 3.872 |

Die Zahl der Anfragen ist von ihrer Änderungsmasse zu unterscheiden.
Der kompakte Zähler allein trennt Rundung und Gewichtsbegrenzung nicht kausal.
Ohne Chemie gibt es Tags und Integrationen mit Nullfaktor, aber keine
Drei-Faktor-Gewichtsänderungen.

Native gelernte Gewichte verändern im Drei-Faktor-Arm mit Chemie gespeicherte
Core-Potenziale. Bei beiden Seeds bleiben die Core- und Motor-Spikes während
der Abrufbilder sowie die Teilbildkontraste gegenüber dem Gewichtsreset gleich.
Ein nichtnull motorischer dritter Faktor wird während des Trainings nicht beobachtet.

Diese Aussagen gelten für diesen festen Bildablauf. Die gespeicherten Potenziale
haben ihre nativen neuronalen Zeitanker. Fehlende Spikewirkung in diesen Proben
beweist keine allgemeine Informationsabwesenheit und keine Wirkungslosigkeit
bei beliebigen anderen Reizen.

## Effizienz und Reproduktion

Ein Datensatz bedient alle drei Prioritäten. Auditdaten werden nach jedem
Frame verarbeitet; kompakte Fenstersummen ersetzen die Speicherung sämtlicher
roher Lernereignisse. Die 16 Roh-Records umfassen zusammen **9.125.624 Bytes**.
Die Summe der nativen Joblaufzeiten einschließlich Replay beträgt etwa
**104,99 Sekunden**; das ist eine Hostmessung und keine allgemeine
Beschleunigungsbehauptung.

Nach vollständiger Ausführung wurde `control report` erneut aufgerufen.
Der Rohdatenbeleg blieb bytegleich. Ein unabhängiger Prüfer berechnet die
Ergänzungskontraste, Akquisition, Gewichtswirkung und Netto-Gewichtsänderungen
aus den gespeicherten Zählvektoren und Gewichtssnapshots erneut.
Maximaler gefundener numerischer Fehler: **0**.
Die nicht gespeicherten rohen Auditereignisse werden dabei nicht rekonstruiert.

```bash
./NERVA_Modell/research/run.sh control report NERVA_Modell/research/config/hippocampal_control_pilot.toml
python3 NERVA_Modell/research/results/hippocampal_control_pilot_20261110/verify_results.py
```

| Artefakt | SHA-256 |
| --- | --- |
| Gesamtplan | `9060845bc7324169fcba5c3b833d4271cc6ce5f11d622cad6d68ce4f833a5344` |
| Rohdatenbeleg | `b4f1261bb78955d94a8928113c7d3931d92f7151dbb30aaf86ebc1f75213b2ac` |
| Quellenarchiv | `0d34fd47c21f1590a80637c8257e516f7fe560fb692da1b0754d6058bd6fcd78` |

[Automatischer Bericht](results/hippocampal_control_pilot_20261110/CONTROL_REPORT.md) ·
[Alle Messwerte](results/hippocampal_control_pilot_20261110/control-metrics.csv) ·
[Gesamtplan](results/hippocampal_control_pilot_20261110/control-plan.json) ·
[Rohdatenbeleg](results/hippocampal_control_pilot_20261110/receipt.json) ·
[Unabhängige Prüfung](results/hippocampal_control_pilot_20261110/verification.json).

Validierung: **55 Rust-Tests** des Modells und Harness sowie
**60 Python-Harness-Tests** bestehen; Rustformat und Clippy mit Warnungen als
Fehlern bestehen. VM- und GPU-Prüfungen wurden für diese Änderung nicht erneut
ausgeführt.

## Gesamtbewertung

| Bereich | Stand |
| --- | --- |
| Architekturentwicklung | Explizit vergleichbare Lernregeln auf demselben Hippocampus-Netz; Live-Defaults bleiben erhalten |
| Forschungsinfrastruktur | Gepaarte Kontrollen, archivierter Runner, Wiederaufnahme, begrenzte Parallelität und kompakte Learning Audits |
| Nachgewiesene Lernfähigkeit | Weiterhin offen; kein positiver gewichtsbedingter Ergänzungsgewinn im Pilot |

Die nächste gezielte Untersuchung bleibt die numerische Konsolidierung kleiner
Beiträge; motorische Rezeptoren und Modulatorversorgung sind davon getrennte
Fragen. Dieser Pilot ändert keine Lernregel und optimiert keine Parameter.
