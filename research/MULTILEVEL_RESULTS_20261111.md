# Mehrstufige Diagnose: Hauptseed 17027 und neuer Seed 23003

Die natürliche chemische Kopplung begrenzt die Größe der Gewichtsänderungen,
aber ihr Ersatz durch einen konstant positiven Lernfaktor löst das Abrufproblem
nicht. Auf beiden geprüften Seeds verbessern die unverstärkten natürlichen,
konstanten und Pair-STDP-Gewichte keinen Cue. Bei gleicher hoher L2-Stärke
verbessert die feste künstliche Referenz beide Cues; die geprüften Lernrichtungen
verschlechtern mindestens einen Cue. Das spricht innerhalb dieses Protokolls
für einen Engpass bei **funktional geeigneter Richtung und ausreichender Stärke**,
nicht allein für einen zu kleinen chemischen Faktor.

Die zuvor an Seed 7 beobachtete fast ausschließlich negative lokale Eligibility
ist dabei **nicht allgemein bestätigt**: Im gesamten B-Netz beträgt der negative
Anteil am absoluten Eligibility-Zeitintegral 51,66 % bzw. 66,79 %.

[Protokoll und Grenzen](MULTILEVEL_DIAGNOSIS.md) ·
[Hauptkonfiguration](config/multilevel_seed17027_20261111.toml) ·
[Vorab festgelegte Bestätigung](config/multilevel_confirmation_23003_20261111.toml) ·
[Unabhängiger Nachrechner](multilevel_report.py) ·
[Maschinenlesbare Ergebnisse und Recordhashes](dokumentation/multilevel_results_20261111.json).

## Ablauf und technische Prüfung

256 Verarbeitungsneuronen, acht Trainingswiederholungen, elf Sekunden Ruhe
vor jeder der fünf eingefrorenen Proben. Entwicklung, Homöostase und
Rückführung bleiben inaktiv. A friert Lernen ein; B verwendet die vorhandene
akkumulierende Drei-Faktor-Regel; C hält **nur den Lernfaktor auf +1**; D verwendet
native unmittelbare Pair-STDP. E konstruiert die zuvor bekannte
Sensor+Core-Referenz mit fester Dosis 1 und vertauschter Gegenrichtung.

Seed 23003 wurde vor Ausführung des Hauptlaufs reserviert und in keinem der
214 geprüften vorhandenen Manifeste/Kontrollpläne gefunden. Seine Konfiguration
und das Entscheidungskriterium wurden vor dem Hauptlauf gehasht. Nach dem
positiven Referenz- und unterscheidenden Normbefund wurde genau dieser Folgelauf
ohne Regel-, Parameter-, Methoden- oder Dosisänderung ausgeführt.

Beide nativen Trials bestehen vollständiges Replay, neutrale Beobachtung und
Gewichts-Rücktauschprüfungen. Die Originalregel auf der festgehaltenen Historie
reproduziert ihre nativen Gewichte, Eligibility, Prä-Traces und Rundungsreste
exakt. Der gesonderte Nachrechner prüft Gewichte/Restbeträge, berechnet die
Cue-Kontraste aus den nativen Spikezählungen neu und kontrolliert Norm und
Richtung aller ausgeführten normierten Arme. Die native `report`-Prüfung
kontrolliert Manifest, Identität und Record-Prüfsummen.

Die vollständige Projektprüfung bestand: **453 Rust-Tests**, **75 Harness-
Python-Tests** und **10 Dashboard-Tests**, Formatierung, Clippy und lokale
Dokumentationslinks. Drei zusätzliche Tests prüfen den Nachrechner, insbesondere
die Trennung von Membranpotenzial- und sonstigen Bilanzänderungen. Die Tests für
lokale QEMU-/Socketadapter liefen mit den dafür erforderlichen lokalen
Socketrechten; im eingeschränkten Sandbox-Modus scheiterten zunächst deren
Socket-Erzeugungen, nicht die Simulationsprüfungen.

Tatsächlich benötigt wurden je Ausführung 26 bzw. 20 Trainingsrekonstruktionen,
inklusive Replay insgesamt **92**, gegenüber dem vorab begrenzten Maximum 120.
Identische Gewichtsvektoren verwenden ihre vollständige Probe erneut. Die
aufgezeichneten Host-Laufzeiten einschließlich Replay betragen etwa 136,5 bzw.
94,2 Sekunden; sie sind keine belastbare Leistungsbenchmark.

## 1. Was die unveränderte Ereignisfolge zeigt

Die lokale Nachrechnung erzeugt keine neuen Netzwerkspikes. Sie verarbeitet
3.760 bzw. 3.233 tatsächliche Integrationsgrenzen, 156.420 bzw. 153.780
chemische Proben, 6.917 bzw. 6.338 Prä-Ankünfte und 1.738 bzw. 1.581 Post-Spikes.

| Seed | Regel auf derselben B-Historie | L2 sichtbarer Nettodeltas | Negativer Anteil ausgeführter Änderungsmasse |
| --- | --- | ---: | ---: |
| 17027 | Original | 0,00006707498 | 59,865 % |
| 17027 | Faktor +1 | 0,00340508828 | 51,662 % |
| 17027 | Pair-STDP | 0,00866747334 | 50,282 % |
| 23003 | Original | 0,00003337434 | 71,246 % |
| 23003 | Faktor +1 | 0,00265410697 | 66,786 % |
| 23003 | Pair-STDP | 0,00676675929 | 52,844 % |

Der konstante Faktor vergrößert die L2-Nettodifferenz auf dieser Historie um
**50,8×** bzw. **79,5×**. Das isoliert eine starke Größenwirkung des Lernfaktors
auf derselben Ereignisfolge. Es ist kein Beleg, dass eine pauschale Erhöhung die
Aufgabe lösen würde.

In den vollständigen chemischen Proben beider B-Läufe ist kein negativer
Lernfaktor enthalten; die gemessenen Faktoren liegen zwischen 0 und 0,139948
bzw. 0,108187. Dopamin ist im Gesamtfeld vorhanden; die früheren Nullbefunde
betrafen einzelne untersuchte Zellen. Die Daten sprechen hier nicht für eine
Umkehr positiver Markierungen durch einen negativen Faktor. Die räumlich und
zeitlich unterschiedliche positive Gewichtung erhöht jedoch den relativen
negativen Anteil gegenüber dem ungewichteten Eligibility-Integral.

Rundungsreste bleiben erhalten: Die Original-Nachrechnung endet mit insgesamt
6,89014 × 10⁻⁷ bzw. 6,15131 × 10⁻⁷ absoluter Restmasse. Sichtbare Änderungen und
Restbeträge werden getrennt geführt. Unverfügbare ungerundete Pair-Anforderungen
werden nicht aus f32-Deltas erfunden.

## 2. Echte Trainingsgegenversuche

| Seed | Arm | L1 Enddelta | Core-Spikeeffekt im Abruf gegenüber eigenem Reset | Abrufgewinn A / B |
| --- | --- | ---: | --- | ---: |
| 17027 | A: eingefroren | 0 | nein | 0 / 0 |
| 17027 | B: Original | 0,000319913 | nein | 0 / 0 |
| 17027 | C: Faktor +1 | 0,022757933 | nein | 0 / 0 |
| 17027 | D: Pair-STDP | 0,057744950 | nein | 0 / 0 |
| 23003 | A: eingefroren | 0 | nein | 0 / 0 |
| 23003 | B: Original | 0,000162244 | nein | 0 / 0 |
| 23003 | C: Faktor +1 | 0,018514499 | nein | 0 / 0 |
| 23003 | D: Pair-STDP | 0,047384232 | nein | 0 / 0 |

Auch die Motor-Spikefolgen verändern sich in diesen Abrufvergleichen nicht.
Potenziale können sich bereits ändern, ohne einen Spike zu beeinflussen:
Beim natürlichen B-Gewichtseingriff zeigen 54 von 1.030 bzw. 53 von 796 gemeinsam
beobachteten Neuron-/Bildpaaren andere gespeicherte Membranpotenziale; die
lokalen Abtastzeiten stimmen in diesen Vergleichen überein.

C und D erzeugen auf Seed 17027 jeweils Änderungen an vier Core→Motor-Synapsen,
während B dort keine sichtbaren Gewichtsänderungen erzeugt. Das zeigt verfügbare
Lernmarkierungen bei schwacher bzw. fehlender nativer Kopplung, aber keinen
funktionalen Motor-Lernerfolg. Auch dieser zusätzliche Effekt behebt den Cue-Abruf
nicht.

Die echte C- und D-Simulation stimmt auf Seed 23003 in allen Endgewichten mit
der jeweiligen Nachrechnung überein. Auf Seed 17027 weichen dagegen zwei bzw.
sechs Gewichte ab (L2-Abstand 0,0000397312 bzw. 0,000484657). Die Gesamtzahl der
Trainings-Core-Spikes bleibt jeweils gleich (737 bzw. 597); diese Zählungen
beweisen keine global identischen Spikezeiten. Auch die explizit verglichenen
Spikefolgen der jeweils ausgewählten Zellen stimmen überein. Die begrenzte
Detailauswahl lokalisiert die Ursache der verbleibenden Gewichtsabweichungen
nicht vollständig. Gerade deshalb ersetzt die feste Historie keinen echten
Gegenversuch.

## 3. Gleiche Stärke unterscheidet die Richtungen

Alle folgenden Eingriffe verändern ausschließlich die Gewichte am gemeinsamen
vollständigen B-Checkpoint. Gewinne beziehen sich auf dieselben festen Ziele
und Anfangsgewichte dieses Checkpoints. Die hohen Normen sind wegen der Grenzen
aller geprüften Richtungen gemeinsam auf 1,05294415 bzw. 0,72941966 begrenzt.
Jeder ausgeführte Arm erfüllt 1 % Normtoleranz und Kosinus ≥ 0,999 nach f32-
Publikation; die tatsächlich gemessenen Normen stehen im JSON-Bericht.

| Seed | Richtung bei gleicher hoher Stärke | Gewinn Cue A | Gewinn Cue B |
| --- | --- | ---: | ---: |
| 17027 | B: natürliche Richtung | −0,07327567 | −0,00097396 |
| 17027 | C: live Faktor +1 | −0,09214099 | −0,12549690 |
| 17027 | D: live Pair-STDP | −0,09214099 | −0,12549690 |
| 17027 | E: feste künstliche Referenz | +0,02501715 | +0,04221688 |
| 23003 | B: natürliche Richtung | −0,04456889 | 0 |
| 23003 | C: live Faktor +1 | −0,17702213 | 0 |
| 23003 | D: live Pair-STDP | −0,17702213 | 0 |
| 23003 | E: feste künstliche Referenz | +0,04072609 | +0,17719181 |

Die beiden Offline-Richtungen erzielen bei hoher Stärke dieselben Cue-Kontraste
wie C bzw. D. Bei natürlicher kleiner B-Stärke bleibt **jede** geprüfte Richtung,
einschließlich E, ohne Cue-Gewinn. Der reine Richtungswechsel bei dieser kleinen
Stärke reicht somit ebenfalls nicht.

Die unveränderte feste E-Dosis 1 ist zusätzlich auf beiden Seeds erfolgreich:
+0,02501715 / +0,03927678 bei Norm 1,11803399 und
+0,04588829 / +0,18153208 bei Norm 1,32664992. Sie besteht die Kriterien gegen
cuefreie Aktivität und die vertauschte Kontrolle. Die gemeinsame hohe Norm ist
eine separate Installation derselben tatsächlich ausgeführten E-Richtung.

## Diagnose und Aussagegrenzen

Die erste messbare Wirkung geht nicht erst beim Rundungsrest verloren: Schon
natürliche Nettogewichte verändern gespeicherte Potenziale. Ihre Stärke reicht
hier nicht für andere Abrufspikes. Ein konstanter Faktor und unmittelbare
Pair-Updates vergrößern die Änderungen stark, lösen aber ebenfalls keinen
Abrufgewinn aus. Selbst bei gleicher, funktional wirksamer hoher Stärke sind
diese Richtungen ungeeignet, während E beide Cue-Kontraste verbessert.

Die Daten grenzen deshalb mindestens zwei Engpässe ab: kleine natürliche
Änderungen **und** fehlende funktionale Richtung der geprüften Lernvarianten.
Der dritte Faktor allein ist keine hinreichende Reparatur. Die Beobachtung einer
negativen Markierung erklärt außerdem nicht automatisch, ob ihre Abschwächung
für die Aufgabe schädlich ist.

E ist eine künstliche **Sensor+Core**-Referenz, keine bestätigte rein rekurrente
Gedächtnisspur. Die Zielkonstruktion verwendet die gleichen Bilder; der neue
Seed bestätigt die Übertragbarkeit dieser Diagnose auf eine weitere
Modellinitialisierung, keine unabhängige Aufgabe oder natürlich gelernte
Erinnerung. Die Aussage gilt für diese beiden Seeds, diese Referenzmethode,
Stärken und Probeabfolge. Andere Regeln oder Richtungen sind nicht ausgeschlossen.

## Historien und Reproduktion

Die vollständigen Ablagen liegen im Projekt:

```text
research/results/multilevel_seed17027_20261111/
research/results/multilevel_confirmation_23003_20261111/
```

Je Ablage: Originalrecord unter `records/`, vollständiges Band und lokale
Nachrechnung im Record unter `details.offline`, Detailhistorien unter
`details.B_history` und `details.live.<Arm>.history`, Quellenarchiv, ausführbare
Datei, Manifest, SQLite-Katalog, Vorab-Provenienz und abschließender SHA-256-
Prüfbeleg. Unter `histories/` liegen separat lesbare, als **abgeleitet** markierte
Auszüge für B und A/C/D mit dem Hash ihres Originalrecords. Der Hauptlauf hat
acht ausgewählte B-Synapsen: 53, 56, 180, 182, 183, 184, 314, 392; der neue Seed:
53, 73, 106, 187, 292, 351, 373, 376.

`diagnosis.json` des Hauptlaufs bleibt unverändert an die Entscheidung für den
Folgelauf gebunden. `diagnosis-v2.json` ergänzt dessen Nachauswertung mit lokalen
Abtastzeiten und dem expliziten Vergleich ausgewählter Trainingsspikes. Der
Bestätigungsordner enthält diese Auswertung direkt als `diagnosis.json`.
Die Quellen-/Binaryhashes wurden vor Ausführung erfasst. Berichte, aktuelle
Nachrechner und Prüfprotokolle sind zusätzlich archiviert.

Beispielbefehle ab `DSVLM/main/`:

```bash
./NERVA_Modell/research/run.sh report NERVA_Modell/research/config/multilevel_seed17027_20261111.toml --binary NERVA_Modell/research/results/multilevel_seed17027_20261111/runner
python3 NERVA_Modell/research/multilevel_report.py NERVA_Modell/research/results/multilevel_seed17027_20261111
./NERVA_Modell/research/run.sh history NERVA_Modell/research/results/multilevel_seed17027_20261111/histories/B_original.json --synapse 53 --phase training --limit 20
```

Quellcode, Konfigurationen, kompakte Resultate und Berichte werden versioniert.
Große Rohdaten bleiben gemäß Projektvertrag außerhalb von Git; das Gesamtpaket
unter `research/results/archives/multilevel_study_20261111.zip` enthält beide
vollständigen Ablagen mit separater SHA-256-Datei.
