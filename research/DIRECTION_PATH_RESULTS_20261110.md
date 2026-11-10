# Richtungs- und Abrufpfadanalyse: Pilot vom 10.11.2026

Der Harness führt beide Untersuchungen jetzt im bestehenden Kontrollzentrum
aus. Der Pilot trennt wirksame künstliche Bild-Ergänzung über Sensorprojektionen
von einer verifizierten rein rekurrenten Abruflösung. Letztere ist in diesem
Versuch weiterhin offen. Natürlich gelernte Gewichte ändern Ankünfte und
Potenziale, aber keine Abruf-Spikes oder Ergänzungswerte.

[Protokoll und feste Erfolgskriterien](DIRECTION_PATH.md) ·
[Konfiguration](config/direction_path_pilot.toml) ·
[Archivierter Gesamtbericht](results/direction_path_pilot_20261110_v3/CONTROL_REPORT.md).

## Versuchsstand

256 Core-Zellen, Seeds 17011/17027, acht Trainingswiederholungen und elf Sekunden
Ruhe vor jeder der fünf Proben. Native Drei-Faktor-STDP verwendet den
synapsenlokalen f64-Restbetrag. Neuronendynamik, Lernraten und Topologie wurden
nicht angepasst. Alle Gewichtseingriffe beginnen am gleichen vollständigen
trainierten Replayzustand; Entwicklung, Homöostase, Rückführung und
Abrufplastizität sind ausgeschaltet.

Je Seed werden ein natürlicher Zweig, ein Zweig mit Anfangsgewichten und zwölf
künstliche Kandidaten geprüft: Sensor+rekurrent oder ausschließlich rekurrent,
jeweils drei feste additive Dosen und eine Kontrolle mit vertauschter Richtung.
Insgesamt: zwei Rohrecords, vier vollständige Ausführungen einschließlich
Replay, 56 Trainingsdurchläufe, höchstens 67.256 Modellschritte.

Das feste Ziel ist der Ergänzungskontrast zwischen den Vollmusterantworten des
Anfangsgewichtszweigs. Eine Kandidatenreferenz muss beide Cues korrekt
verbessern, die vertauschte Richtung schlagen und ohne Cue zusätzliche
Core-Spikes vermeiden. Die Bilder dienen zugleich zur Kalibrierung der
künstlichen Gewichte: dies ist eine explorative Fähigkeitsdiagnose.

## Richtungsbefund

| Seed | Gültige künstliche Referenz | Erste gültige additive Dosis | Kontrastgewinn Cue A / B | Cosinus natürliche Deltas / Referenz |
| --- | --- | ---: | ---: | ---: |
| 17011 | Sensor+rekurrent | 1 | +0,13421820 / +0,01089915 | −0,17390354 |
| 17027 | Sensor+rekurrent | 1 | +0,02501715 / +0,03927678 | +0,00715331 |

Die natürlichen Kontrastgewinne sind bei beiden Seeds für beide Cues exakt null.

| Seed | Natürliche Nettoänderung L1 | Referenzänderung L1 | Natürliche L1-Masse auf Referenzsynapsen |
| --- | ---: | ---: | ---: |
| 17011 | 0,00001642108 | 3,99999987 | 0,00000260770 |
| 17027 | 0,00031991303 | 1,5 | 0,00000196695 |

Die natürlichen Richtungen stimmen mit diesen künstlichen Lösungen global
schlecht überein. Beim zweiten Seed wirkt die erfolgreiche Referenz
ausschließlich auf Sensor→Core-Verbindungen; beim ersten überwiegend dort.
Das zeigt eine künstlich verbesserbare Eingangsprojektion. Es bestätigt keinen
funktionierenden rein rekurrenten Gedächtnisspeicher.

Die beiden Änderungsnormen unterscheiden sich stark. Ein negativer oder nahezu
nuller Cosinus allein beweist deshalb nicht, dass ausschließlich die Lernregel
den ausbleibenden Abruf verursacht. Andere wirksame Richtungen können existieren.

## Getrennte rekurrente Kontrolle

Keiner der ausschließlich rekurrenten Kandidaten erfüllt die Kriterien:

- **Seed 17011:** Die Dosen 0,25 / 1 / 2 ändern Gewichte, verbessern Cue A
  aber nicht und verschlechtern Cue B geringfügig. Die vertauschte Kontrolle
  liefert ebenfalls keine beidseitige Verbesserung.
- **Seed 17027:** Die aus den Kalibrieraktivitäten berechnete rekurrente
  Richtung ist null. Alle sechs rekurrenten Kandidaten installieren daher
  die Anfangsgewichte. Dies ist eine Grenze dieser Konstruktionsmethode;
  es wurde hier keine nichttriviale passende rekurrente Lösung getestet.

Es wäre falsch, daraus einen unmöglichen rekurrenten Abruf oder eine generell
ungeeignete Netzwerkarchitektur abzuleiten. Für diesen Pfad gibt es weiterhin
keine nachgewiesen wirksame Richtung, mit der die natürliche Regel kausal
verglichen werden könnte.

## Wo natürliche Änderungen ankommen

Die Cue-Proben entsprechen Bildindizes 2 und 3. Gezählt werden vorhandene
Synapsen und tatsächliche native elektrische Übergänge.

| Seed | Cue | Geänderte Gewichtssynapsen mit Ankunft | Veränderte Ankunftssummen | Neuronen mit verändertem minimalem Schwellenabstand | Neuronen mit anderer Spikezahl |
| --- | --- | ---: | ---: | ---: | ---: |
| 17011 | A | 12 von 46 | 12 | 7 | 0 |
| 17011 | B | 11 von 46 | 11 | 8 | 0 |
| 17027 | A | 17 von 65 | 17 | 11 | 0 |
| 17027 | B | 15 von 65 | 15 | 11 | 0 |

Die Gewichtswirkung verschwindet damit nicht vollständig vor der Integration:
veränderte synaptische Amplituden und Schwellenabstände sind messbar.
Die genauen Core- und Motor-Spike-Digests bleiben trotzdem unverändert.

| Seed | Cue-Integrationen insgesamt | Spikes | Refraktär verworfen | Unterschwellig | Erregung allein über Schwelle, mit Hemmung darunter |
| --- | ---: | ---: | ---: | ---: | ---: |
| 17011 | 127 | 14 | 2 | 111 | 0 |
| 17027 | 2.195 | 17 | 10 | 2.168 | 0 |

Diese Summen sind für den natürlichen und den Anfangsgewichtszweig gleich.
Sie enthalten Core- und Motorneuronen und sind keine Zahl verlorener Bits.
Viele unterschwellige Integrationen sind für sich kein Nachweis einer
Fehlfunktion. Die Hemmungszählung prüft nur den gleichen momentanen
Ausgangszustand; ihr Nullwert schließt längerfristige Konkurrenz nicht aus.

Rohrecords enthalten pro Bild sämtliche Synapsen- und Neuronensummen,
Quell-Spikezahlen und bis zu 128 genaue Integrationsbeispiele.
Kontinuierliche Potenzialentwicklung, akzeptierter Netzeingang,
Additionsrundung und Spike-Reset sind separat bilanziert. Die kontinuierliche
Entwicklung enthält auch intrinsische Ströme und Neuromodulation.

## Verifikation und Ablage

Die vollständige Projektprüfung bestand: **441 Rust-Tests, 66 Python-Tests im
Harness und 10 Dashboard-Tests**, außerdem Formatierung und Clippy.
Die abschließende Ergänzung der Quell-Spikezahlen wurde zusätzlich mit den
betroffenen nativen Diagnose- und Python-Kontrolltests geprüft.
Der unveränderte CPU-Referenztest besteht.
Tests vergleichen außerdem den vollständigen Replayzustand chemisch aktiver
Modelle mit und ohne elektrische Beobachtung.

Der [unabhängige Standardbibliotheksprüfer](results/direction_path_pilot_20261110_v3/verify_results.py)
importiert den Controller nicht. Er prüft Archiv-/Rohdatenhashes, installierte
f32-Gewichte einschließlich Pfadmasken und Begrenzung, Richtungswinkel,
feste Zielkontraste, Referenzwahl, sämtliche Potenzialbilanzen und Zählklassen.
Maximale Potenzialbilanzabweichung: **7,77 × 10⁻¹⁶**.
[Prüfbeleg](results/direction_path_pilot_20261110_v3/verification.json).

| Beleg | SHA-256 |
| --- | --- |
| Kontrollplan | `611a86638d3300ce3b536225d7a3fe0f795c27777e9edf2f4a1bb89ff0a03c68` |
| Rohdatenbeleg | `7bb981e591854e4a4143b54f37dc1a41273cd4b8b6cb018badefeb37ef51cbdf` |
| Quellenarchiv | `625be10b8f4668beed5cb68590853c067a6b9148cd85bd710e7297eac5ada891` |

Rohrecords belegen 37.507.861 Bytes. Der vollständige Lauf liegt unter
`results/direction_path_pilot_20261110_v3/`. Frühere Entwicklungsarchive
`direction_path_pilot_20261110/` und `..._v2/` bleiben erhalten; sie werden
nicht als zusätzliche unabhängige Evidenz gezählt. Ein fehlgeschlagener früher
Smoke-Lauf wurde vor der vollständigen Erfassung chemisch ausgelöster
Potenzialentwicklung abgebrochen und nicht als negatives Forschungsergebnis
gewertet.

Die Ergebnisordner sind von Git ausgeschlossen. Ein Code-Commit sichert die
Rohdaten nicht; siehe [Archivierregeln](ARCHIVING.md).

## Aussagegrenze

Die Untersuchung zeigt jetzt konkret, wo kleine gespeicherte Änderungen im
geprüften Abrufpfad noch messbar sind, ohne das Spike-Muster zu ändern.
Sie liefert zudem kontrollierte künstliche Ergänzungen über Sensorprojektionen.
Eine wirksame rein rekurrente Referenz, eine normkontrollierte kausale
Trennung von Richtung und Stärke sowie unabhängige Bestätigungsseeds bleiben
offen. Natürliches Assoziationslernen ist weiterhin nicht nachgewiesen.

