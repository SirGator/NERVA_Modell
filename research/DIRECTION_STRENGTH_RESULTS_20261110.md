# Richtung × Stärke: Pilot vom 10.11.2026

Der normkontrollierte Vierfelderversuch ist implementiert und vollständig
ausgeführt. Bei Seed 17027 reicht weder eine starke natürliche Richtung noch
eine schwache künstliche Referenzrichtung: Nur die Kombination aus
Referenzrichtung und Referenzstärke verbessert beide Cues. Bei Seed 17011
bleibt die volle Vierfelderdiagnose durch Gewichtsgrenzen eingeschränkt.

Die erweiterte Suche erzeugt jetzt bei beiden Seeds nichttriviale rekurrente
Eingriffe, findet aber weiterhin keine beidseitig erfolgreiche rein rekurrente
Referenz. Natürliches Assoziationslernen bleibt nicht nachgewiesen.

[Protokoll](DIRECTION_STRENGTH.md) ·
[Konfiguration](config/direction_strength_pilot.toml) ·
[Archivierter Gesamtbericht](results/direction_strength_pilot_20261110_v2/CONTROL_REPORT.md).

## Kontrollierter Ablauf

256 Core-Zellen, Seeds 17011/17027, acht Wiederholungen und elf Sekunden Ruhe
pro Probe. Native Drei-Faktor-STDP verwendet den bestehenden f64-Restbetrag.
Alle tatsächlich simulierten Gewichtseingriffe starten am vollständig gleichen
trainierten Modellzustand. Gewichte werden vom gemeinsamen Anfangsvektor aus
ersetzt; Tags, Restbeträge, Warteschlangen und Uhren bleiben erhalten.
Lernraten und Neuronendynamik sind unverändert.

Jeder Zweig durchläuft dieselben fünf Proben mit eingefrorener Plastizität.
Die Zielvektoren sind die Vollmusterantworten des Anfangsgewichtszweigs.
Stärke bezeichnet die **L2-Norm sichtbarer Gewichtsnettodifferenzen**.
Infeasible Skalierungen werden nicht durch Clipping als vermeintlich
richtungsgleich ausgegeben. Zusätzlich wird die hohe Stärke auf den gemeinsam
zulässigen Radius begrenzt.

18 feste Referenzkandidaten je Seed untersuchen drei Methoden, drei additive
Dosen und je eine Gegenrichtungskontrolle. Identische vollständige
f32-Gewichtsvektoren teilen eine Beobachtung. Einschließlich Replay entstanden
94 native Trainingsdurchläufe statt des Budgets von höchstens 100.

## Seed 17027: Richtung und Stärke wirken zusammen

Die ursprüngliche Referenzstärke ist auf beiden Richtungen zulässig.
Alle vier Arme erfüllen die festen Norm-/Richtungskriterien.

| Arm | Richtung | Gewünschte L2-Stärke | Tatsächliche L2-Stärke | Ergänzungsgewinn Cue A / B |
| --- | --- | ---: | ---: | ---: |
| NN | natürlich | 0,00006707498 | 0,00006707498 | 0 / 0 |
| NR | natürlich | 1,11803399 | 1,11803399 | −0,07327567 / −0,00097396 |
| RN | künstliche Referenz | 0,00006707498 | 0,00006709318 | 0 / 0 |
| RR | künstliche Referenz | 1,11803399 | 1,11803399 | +0,02501715 / +0,03927678 |

Gewinn bedeutet hier den Unterschied des festen Ergänzungskontrasts gegenüber
Anfangsgewichten. Die RN-Norm weicht nach numerischem Veröffentlichungsabgleich
um 0,0271 % ab und erfüllt damit die vorab feste 1-%-Grenze.

Die verstärkte natürliche Richtung ist in diesen beiden Cue-Kontrasten
schlechter als die unverstärkte. Der reine Richtungswechsel bei natürlicher
Stärke bewirkt keine messbare Kontraständerung. Bei hoher Stärke verbessert
der Richtungswechsel die Kontraste um +0,09829282 / +0,04025074.
Dieselbe Differenz erscheint als Interaktion, weil beide schwachen Arme
denselben Kontrast liefern.

Für diese Bilder, Stärkeauswahl und künstliche Lösung sprechen die Daten
damit für ein Zusammenspiel von Richtung und Stärke. Das beweist weder,
dass jede natürliche Verstärkung schadet, noch dass diese Referenz die einzige
funktionierende Richtung ist.

## Seed 17011: Gewichtsgrenzen begrenzen den vollen Vergleich

Die natürliche L2-Stärke beträgt 0,00000446423, die erfolgreiche künstliche
Referenzstärke 1,68522990. Die natürliche Richtung erlaubt ohne
Richtungsverzerrung jedoch höchstens 0,49497909.
Der Arm natürliche Richtung × volle Referenzstärke ist deshalb **nicht
ausgeführt**; die ursprüngliche 2×2-Matrix ist ungültig.

Die Referenzrichtung bei natürlicher Stärke bleibt wirkungslos. Die
unveränderte starke Referenz verbessert beide Cues um
+0,13421820 / +0,01089915.

Bei gemeinsamer zulässiger hoher Stärke ergibt sich eine gültige Matrix:

| Arm | Richtung | Gewünschte L2-Stärke | Ergänzungsgewinn Cue A / B |
| --- | --- | ---: | ---: |
| NN | natürlich | 0,00000446423 | 0 / 0 |
| NR | natürlich | 0,49497909 | 0 / 0 |
| RN | künstliche Referenz | 0,00000446423 | 0 / 0 |
| RR | künstliche Referenz | 0,49497909 | +0,07369066 / −0,00162049 |

Bei dieser gemeinsam zulässigen Stärke verbessert die Referenzrichtung Cue A,
verschlechtert Cue B aber leicht. Sie erfüllt hier also nicht mehr die
beidseitigen Referenz-Erfolgskriterien. Richtung und Stärke lassen sich
numerisch vergleichen; ein vollständiger erfolgreicher Ersatz des natürlichen
Abrufs ist bei dieser Stärke nicht gezeigt. Die fehlende ursprüngliche
NR-Variante wird nicht aus dem anderen Seed ergänzt oder extrapoliert.

Der RN-Veröffentlichungsabgleich erreicht 0,1282 % Normabweichung.
Unvermeidbare Quantisierung wird weiterhin sichtbar: Beim separaten
16-Zellen-Smoke-Seed 7 überschreitet derselbe schwache Vergleich die
1-%-Grenze und erzeugt folgerichtig keine gültigen Vierfelderkontraste.

## Rekurrente Suche: Die Nullrichtung ist behoben, Erfolg bleibt offen

| Seed | Rekurrenter Direktkontrast: Kanten mit Score ≠ 0 | Rekurrenter Relaiskontrast: Kanten mit Score ≠ 0 | Gültige rekurrente Referenz |
| --- | ---: | ---: | --- |
| 17011 | 6 | 31 | keine im Raster |
| 17027 | 11 | 45 | keine im Raster |

Die vorherige Nullrichtung bei Seed 17027 blockiert diese neue Suche nicht mehr.
Alle neuen Kandidaten verändern ausschließlich bestehende Core→Core-Gewichte;
Sensorprojektionen bleiben exakt auf Anfangsgewichten.

Die getesteten rekurrenten Richtungen verbessern bei keinem Seed beide
Cue-Kontraste. Beim zweiten Seed bleiben die gezielten Richtungen für Cue A
wirkungslos und verschlechtern Cue B leicht. Die vertauschten Kontrollen
schneiden ebenfalls nicht beidseitig besser ab.

Die gültigen Referenzen beider Seeds stammen daher weiterhin aus der
Sensor+Core-Methode. Die vier Gewichtskombinationen untersuchen deren
Richtung/Stärke; sie werden nicht als gelöster rekurrenter Speicherpfad
bezeichnet. Zusätzliche passende Wege oder andere künstliche
Konstruktionsmethoden werden durch diese endliche Suche nicht ausgeschlossen.

## Technische Prüfung und Reproduktion

Die vollständige Projektprüfung bestand: **444 Rust-Tests, 69 Python-Tests
des Harness und 10 Dashboard-Tests**, außerdem Formatierung und Clippy.
Tests decken exakte Vierfelderkontraste, Normierung, unzulässiges Clipping,
Nullrichtungen, Quantisierungsgrenzen, Replay, Beobachtungswiederverwendung,
rekurrente Eingriffsgrenzen und gehashte Berichtshelfer ab.

Der [unabhängige Nachrechner](results/direction_strength_pilot_20261110_v2/verify_results.py)
importiert weder Controller noch Berichtshelfer. Er prüft Archivhashes,
sichtbare Gewichte, Normen, Richtungstreue, feste Zielkontraste,
Referenzwahl, Vierfelderinteraktion und elektrische Potenzialbilanzen.
Die größte Bilanzabweichung beträgt **7,77 × 10⁻¹⁶**.
Die nativen trainierten Gewichte, Eingangsdigests und natürlichen
Core-/Motor-Abrufspikes stimmen auf beiden Seeds exakt mit dem
[vorherigen Richtungs-/Pfadpilot](DIRECTION_PATH_RESULTS_20261110.md) überein.
[Prüfbeleg](results/direction_strength_pilot_20261110_v2/verification.json).

| Beleg | SHA-256 |
| --- | --- |
| Kontrollplan | `41debae3a6516e275d88f01a9b4f7e28cb901e7d6f2803d3ccb6ee240dc51ac5` |
| Rohdatenbeleg | `48b045150f8240afce0c3ea7a2f640f3503bc56563427ca936b3a3908a4a9b2a` |
| Quellenarchiv | `6c89e89e38ed6ecbf246f7874f3e38c9a574d131df5508ed88466f5307a5e605` |

Der finale Lauf enthält 59.582.794 Bytes Rohrecords in zwei Records und vier
vollständigen Ausführungen einschließlich Replay. Tatsächlich wurden je
Ausführung 24 beziehungsweise 23 Trainingsdurchläufe und 23 beziehungsweise
22 unterschiedliche Gewichtsbeobachtungen benötigt. Das feste Modellschrittbudget
betrug höchstens 120.100; es ist keine gemessene Schrittzahl.

Das frühere Entwicklungsarchiv ohne numerische Radiuskalibrierung bleibt
erhalten und zählt nicht als unabhängige Evidenz. Die Kalibrierung sucht
ausschließlich einen numerischen Radius; sie verwendet keine neuronalen
Antworten und lockert die festen Auswertungstoleranzen nicht.

Die vollständigen Ergebnisordner sind von Git ausgeschlossen und separat
zu sichern. Die Code-Commits enthalten die Implementierung, Konfigurationen,
Tests und diesen Bericht, aber keine Rohdaten.
[Archivierregeln](ARCHIVING.md).

## Einordnung

Der Vierfeldervergleich trennt nun tatsächlich Richtung, Stärke und
Gewichtsgrenzen. Seed 17027 zeigt in diesem Assay, dass eine bloße Verstärkung
der natürlichen Änderung den künstlichen Referenzabruf nicht ersetzt.
Bei Seed 17011 bleibt die vollständige ursprüngliche Trennung unzulässig.
Ein beidseitig erfolgreicher rein rekurrenter Abrufpfad und unabhängige
Bestätigungsseeds bleiben offen. Ein allgemeiner Lernnachweis folgt daraus nicht.
