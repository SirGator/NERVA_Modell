# M4: Sensorik, Zeitabstände und Lernmechanismen

Die M4-JSON-CLI ist integriert und die Versuchsreihen sind ausgeführt. Zuerst
liefen die vier einfachen Sensor-/Zeitfälle, danach direkte Signal- und
STDP-Prüfungen, anschließend die vollständige 20er-Matrix auf drei Seeds und
auf zehn neuen Seeds. Alle vier Kontrollarme, beide Tastenbelegungen sowie
Ruhe- und Interferenzproben wurden ausgeführt. NERVAs Lernregeln bleiben unverändert.

**Befund: Keine der 20 Bedingungen bestätigt funktionales Wirkungslernen auf
zehn neuen Seeds.** Größere Cores, andere Kodierungen und längere Zeitfenster
reichen unter diesen Einstellungen nicht aus. Zwei Engpässe sind messbar:
Die Aktionskopie bleibt im neutralen 2-Kanal-Fall unterschwellig; mit aktiver
Homöostase sinkt außerdem die tatsächlich ankommende sensorische Anregung.

[Zehn-Seed-Bericht](../../Messungen/Experimente/m4-confirm/summary.md),
[gepaarte Kennzahlen](../../Messungen/Experimente/m4-confirm/summary.csv),
[alle Belegungs-/Ruhe-/Zeitbin-Kontraste](../../Messungen/Experimente/m4-confirm/contrasts.csv),
[unabhängiger Datenaudit](../../Messungen/Experimente/m4-confirm/diagnostics.json),
[direkte Mechanismendiagnose](../../Messungen/Experimente/m4-initial-gate/mechanisms.json).
CLI, Eingangs-/Ergebnisvertrag und Grenzen stehen in [m4/README.md](m4/README.md).

## Ausgeführte Reihen

| Reihe | Bedingungen | Seeds | Abgeschlossene Fall-/Seed-Läufe |
|---|---:|---|---:|
| [Erstes Gate](../../Messungen/Experimente/m4-initial-gate/summary.md) | 4 | 0–2 | 12 |
| [Vollständiger Pilot](../../Messungen/Experimente/m4-pilot/summary.md) | 20 | 0–2 | 60 |
| [Unabhängige Prüfung](../../Messungen/Experimente/m4-confirm/summary.md) | 20 | 100–109 | 200 |

Jeder Lauf enthält acht Modellinstanzen: vier Arme × zwei Belegungen.
Die Gate-Fälle bei 8 ms werden im Pilot wiederholt und erhöhen nicht die Zahl
unabhängiger Seeds. Die Reihen werden getrennt ausgewertet.

Pro Instanz: 256 balancierte Trainingstrials (außer `before`), 128 rein visuelle
Kalibriertrials und je 128 Prüftrials in der ersten Probe, nach zwei Sekunden Ruhe und
nach weiteren 1,4 Sekunden visueller Interferenz. Die Probe bleibt eingefroren.
Der Auswerter gibt keine Informationen an das neuronale Modell zurück.

Die erste Probe folgt auf **20,48 Sekunden visuelle Kalibrierung** am bereits
eingefrorenen Modell. Sie ist keine Messung direkt am Trainingsende. Die
Membranzustände am Trainingsende werden separat gespeichert. Kurzlebige
Repräsentationen, die während der Kalibrierung verschwinden, sind durch den
negativen Vorhersagebefund nicht ausgeschlossen. Die Ruhephase zeigt das
neutrale Bild und ist bei Pixelsensorik weiterhin ein sensorischer Eingang.
Zwischen Ruhe und Interferenz liegt außerdem die vollständige Ruheprobe;
der Vergleich isoliert daher nicht ausschließlich die 1,4 Sekunden Interferenz.

## Was die zehn neuen Seeds zeigen

Auswahl aus der vollständigen Matrix; alle Werte sind balancierte Genauigkeiten,
über zehn Seeds und beide Belegungen gemittelt. Primär gilt der Spikezählungs-Auswerter.

| Bedingung | Vorhersage vor Bild | Visuelle Auslesbarkeit | Vorhersage nach Ruhe | Nach Interferenz |
|---|---:|---:|---:|---:|
| 2 Kanäle, 32 Core, 8 ms | 50,0 % | 50,0 % | 50,0 % | 50,0 % |
| 7 Kanäle, 32 Core, 8 ms | 49,8 % | 49,9 % | 50,4 % | 50,4 % |
| 16×16, 128 Core, 8 ms | 49,6 % | 53,1 % | 50,2 % | 50,6 % |
| 16×16, 256 Core, 8 ms | 51,8 % | 88,6 % | 50,9 % | 50,6 % |
| 16×16, 128 Core, 20 ms | 50,0 % | 95,6 % | 50,4 % | 49,8 % |
| 16×16, 128 Core, 50 ms | 50,2 % | 100,0 % | 50,0 % | 49,6 % |
| 16×16, 128 Core, ohne Homöostase | 49,8 % | 85,5 % | 50,4 % | 51,4 % |

Der nominell höchste Mittelwert der ersten Probe ist 54,3 % bei 2 ms. Der Vorteil
gegenüber gleich alt eingefroren beträgt 3,2 Prozentpunkte, sein gepaartes
95-%-Intervall reicht aber von −4,1 bis +11,8 Punkten. Gegenüber entkoppeltem
Training sind es 5,7 Punkte [−0,4; +16,7]. Die visuelle Auslesbarkeit liegt bei
49,7 %. Dieser Fall ist kein belastbarer Lernerfolg.

Die separaten Zeitbin-Auswerter sind vollständig gespeichert. Auch ihre
Ergebnisse müssen beide Belegungen und alle Kontrollen bestehen; ein einzelner
positiver Kontrast in der großen Matrix genügt nicht. Intervalle sind
deskriptive Bootstrap-Intervalle über gepaarte Seeds, ohne Korrektur für die
vielen Vergleiche. Testsamples und Belegungen werden nicht als unabhängige Seeds gezählt.
Weder der primäre Zählreadout noch die Zeitbins liefern in der ersten Probe einen
strikt positiven 95-%-Intervallunterrand gegenüber allen drei
Kontrollen, auch nicht im über beide Belegungen gemittelten Vergleich.

## Lernmechanismen L1–L4

Identische Sensorik (16×16), 128 Core, deterministisch, 8 ms und gleicher
Seed/Anfangsaufbau. L1 ist der vollständig eingefrorene, gleich alte Replay-Arm
von `b01`; L2–L4 sind die gekoppelten Arme der jeweiligen Bedingungen.
Für L2–L4 existieren zusätzlich entkoppelte und eingefrorene Kontrollen.

| Variante | STDP / Homöostase / Entwicklung | Vorhersage | Visuelle Auslesbarkeit | Gemessener sensorischer Strom je Core/Sekunde im Training |
|---|---|---:|---:|---:|
| L1 | aus / aus / aus | 49,5 % | 87,6 % | 9,995 |
| L2 (`e03`) | an / aus / aus | 49,8 % | 85,5 % | 9,627 |
| L3 (`b01`) | an / an / aus | 49,6 % | 53,1 % | 5,394 |
| L4 (`e04`) | an / an / an | 48,9 % | 53,1 % | 5,393 |

L4 bildet im Mittel pro Instanz 28,1 Verbindungen und entfernt 396,7. Ein
Startdrive von 2,5 erhöht die Bildung auf 144,65; auch das ergibt keinen
bestätigten Vorhersagevorteil. Die Strukturänderungen sind real, ihre Anzahl
ist kein Qualitätsmaß.

Der Stromvergleich zeigt, warum das anfängliche Normierungsbudget allein
nicht genügt. Die vorhandene Homöostase gilt für alle Populationen einschließlich
Sensorneuronen. Veränderte Erregbarkeit und Gewichte verändern die spätere
Weiterleitung. Es wurde weder nachgeregelt noch anhand der Testergebnisse
ein anderer Stromfaktor ausgewählt. Der Vergleich rechtfertigt eine getrennte
Untersuchung der Sensorweiterleitung; er isoliert nicht jede einzelne Ursache.
Auch die räumliche Versorgung bleibt ungleich: Beim 256er-Core mit 16×16
erhalten im Anfangsbudget im Mittel rund 83 % der Core-Zellen keinen direkten
sensorischen Strom aus den hier verwendeten Bildern. Rekurrente Eingänge sind
darin nicht enthalten. Mehr Neuronen bedeuten bei diesem lokalen Gerüst daher
nicht automatisch gleichmäßig mehr visuell versorgte Verarbeitungskapazität.

## Details des ersten Gates (Seeds 0–2)

256 balancierte Trainingstrials, 128 rein visuelle Kalibriertrials, jeweils 128
Prüftrials pro Belegung und Prüfzeitpunkt. Die vier Arme sind vorher eingefroren,
gleich alt eingefroren, gekoppelt gelernt und mit zerstörter Zuordnung gelernt.
Es wurden nach Kalibrierung, nach zwei Sekunden Ruhe und nach zusätzlicher visueller
Interferenz geprüft. Jede Probe bleibt eingefroren und verwendet denselben
visuell kalibrierten Auswerter.

Vier Bedingungen × drei Seeds × vier Arme × zwei Belegungen ergeben **96
Modellinstanzen**. Pro Bedingung und Arm enthält die erste Probe
768 Testfälle über drei Seeds und zwei Belegungen. Ruhe- und Interferenzprüfung
haben jeweils ebenso viele Fälle, werden aber separat berichtet.

| Sensorik / Abstand | Vor Training | Gleich alt eingefroren | Gekoppelt gelernt | Zuordnung zerstört | Visuelle Auslesbarkeit nach gekoppeltem Training |
|---|---:|---:|---:|---:|---:|
| 2 Kanäle / 8 ms | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 50,0 % |
| 7 Kanäle / 8 ms | 54,9 % | 57,4 % | 50,9 % | 49,0 % | 55,5 % |
| 2 Kanäle / 20 ms | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 100,0 % |
| 7 Kanäle / 20 ms | 53,1 % | 50,0 % | 50,0 % | 49,9 % | 84,6 % |

Primär sind dies balancierte Genauigkeiten aus Core-Spikezählungen.
Keine Bedingung liefert einen beständigen Vorteil gegenüber untrainierten,
gleich alten eingefrorenen und entkoppelten Netzen. Die 7-Kanal-/8-ms-Bedingung
liegt gegenüber gleich alt eingefroren im Mittel 6,5 Prozentpunkte niedriger;
das deskriptive 95-%-Intervall ist breit (−22,7 bis +3,9 Punkte).

Die 20-ms-Bedingung trennt zwei Dinge: Visuelle Wirkungen können aus dem
trainierten Core erkannt werden, eine bessere Vorhersage **vor** ihrem Eintreffen
entsteht daraus bisher nicht. Bei 8 ms sinkt die visuelle Auslesbarkeit der
trainierten 2-Kanal-Netze von 100 % in den eingefrorenen Kontrollen auf 50 %.
Das längere Merkmalsfenster bei 20 ms ist eine zusätzliche Erklärungsmöglichkeit;
dieser Unterschied allein beweist keinen besseren STDP-Zeitabstand.

Nach Ruhe und Interferenz bleiben beide 2-Kanal-Fälle bei 50 %. Der gekoppelte
7-Kanal-/8-ms-Fall erreicht nach Ruhe rund 48,8 %, nach Interferenz 50,9 %;
der 20-ms-Fall bleibt in beiden Prüfungen bei 50 %. Alle eingefrorenen Lernparameter
bleiben erhalten.

## Konkreter Signalengpass

In sämtlichen **18.432 Vorhersagefenstern** der beiden 2-Kanal-Bedingungen,
einschließlich aller Arme, Belegungen, Seeds sowie Ruhe-/Interferenzproben:

- Die erwarteten drei Aktionskopie-Ankünfte pro Trial sind vorhanden.
- Vor dem Wirkungsbild entstehen **null Core-Spikes**.
- Die Spikezählungs- und Zeitbin-Merkmale sind damit für A und D leer.

Auf den zehn neuen Seeds bestätigt sich der 2-Kanal-/8-ms-Befund in weiteren
**30.720 Vorhersagefenstern** über alle vier Arme: ebenfalls null Core-Spikes
bei vorhandenen Aktionskopie-Ankünften.

Die gesonderte Feedbackdiagnose mit gleichen neutralen Bildern bestätigt für
Seeds 0–2: Korrekte und vertauschte Aktionskopien erzeugen unterschiedliche
Membranverteilungen, aber keine Core-Spikes. Ohne Kopie sind die entsprechenden
Ankünfte null. Eine um 4 ms verzögerte Kopie verschiebt ihre erste Core-Ankunft
von **1.501 auf 5.501 µs**.

Beispiel Seed 0: Beim gemeinsamen analytischen Stichtag von 20 ms liegt das
maximale Core-Potential nach korrekter A-Kopie bei rund **0,228**, bei verzögerter
Kopie bei rund **0,279**. Die Schwelle ist **1,0**. Die Information über A/D ist
also unterschwellig vorhanden; der primäre Spike-Auswerter kann sie in diesem
Zustand nicht lesen. Ein Membranauswerter könnte A/D wiederum nur extern nachschlagen;
auch das allein wäre noch kein in NERVA gelernter Wirkungszusammenhang.

Der feste Anschluss der Aktionskopie ist im bestehenden Modell nicht plastisch.
Die plastische Core-Rekurrenz benötigt eigene Core-Spikes. Der direkte Befund
rechtfertigt deshalb zuerst einen vorab festgelegten Test der Stärke/Integration
des Aktionssignals und seiner Überschwellung. Die Gewichte wurden im gemessenen
Gate nicht nachträglich erhöht, um einen Erfolg zu erzeugen.

## STDP und Weiterleitung direkt geprüft

Neun isolierte Synapsentests verwenden dieselben STDP-Parameter wie das Modell:
±2, ±8 und ±20 ms zwischen **tatsächlicher synaptischer Ankunft** und Post-Spike,
zwei Paarungen außerhalb des 100-ms-Fensters und ein STDP-off-Fall. Alle Gewichte
stimmen innerhalb 1e−7 mit der vorhandenen Regel überein. Diese Bench-Pulse werden
nicht in die M4-Trainingsmodelle eingespeist.

Zwölf Aktuatorprüfungen über drei Seeds bestehen: Stille, nur A, nur D und
zeitgleiche A/D-Aktivierung. Einzelmotoren erzeugen die passende Taste und
sichtbare Bewegung; der exakte Gleichstand enthält sich.

Visueller Input erreicht den Core und teilweise auch Motoren. In neun getrennten
250-ms-Durchläufen über drei Bilder und drei Seeds erzeugen linkes/rechtes Bild
109–119 beziehungsweise 111–121 Core-Spikes; das neutrale 2-Kanal-Bild bleibt
still. Ein konkreter visueller Pfad liefert bei Seed 1 für die rechte Bildseite
zwar Core-Spikes, aber keinen Motor-Spike. Nullpfade werden mitberichtet.

Damit ist ein grundlegender Implementierungsfehler im Vorzeichen oder in der
Ankunftszeit der isoliert geprüften STDP-Paarungen nicht erkennbar. Das ist kein
Nachweis, dass diese Regel im rekurrenten Gesamtnetz bereits brauchbare Assoziationen
organisiert.

## Anregung und Lernen

Die feste Normierung erreicht beim gleich alten eingefrorenen 2-Kanal-Netz
beispielsweise **9,999** sensorische Stromeinheiten pro Core-Neuron und Sekunde
bei Ziel 10. Nach gekoppeltem Training liegt derselbe gemessene Mittelwert
über Seeds etwa bei **7,65–7,80**; im 7-Kanal-Fall etwa bei **8,48–8,57**.
Gelernte Gewichte und Zellzustände verändern somit trotz gleichem anfänglichen
Budget die spätere effektive Anregung. Dies wird protokolliert und nicht
leistungsabhängig nachgeregelt.

Bei Seed 0 im 2-Kanal-Fall sinkt die Summe aller Gewichte von etwa **61,404 auf
59,413**. Zehntausende lokale Gewichtsupdates sind messbar, erzeugen aber keinen
nachgewiesenen Vorhersagevorteil. Veränderung und Lernqualität bleiben getrennte
Messgrößen.

## Abnahme und verbleibende Forschungsfragen

Alle Auditflags der vollständigen Reihen bestehen. Der unabhängige Auswerter
prüft zusätzlich die Trial- und Phasen-CSV auf Zeitgrenzen, Aktionskopien,
balancierte Klassen, identische Replays, Freeze und ausgeschaltete Mechanismen.
Die ursprünglichen 20 Konfigurationen wurden zuvor technisch mit acht Trials
geprüft; diese Kurzläufe werden nicht als Leistungsdaten verwendet.
[Technische Verifikation](../../Messungen/Experimente/m4-confirm/verification.md).

Offen bleibt ein positiver Lernnachweis. Als nächste getrennte Parameterstudie
bieten sich die Überschwellung/Integration der Aktionskopie und die Wirkung der
Homöostase auf die sensorische Weiterleitung an. Werte und Erfolgskriterien
sollten vor der nächsten Ausführung feststehen. Lange Vergessensreihen und
zielgerichtete Nutzung folgen erst bei nachgewiesenem Wirkungslernen.
Der negative Befund gilt für diese Konfigurationen und Auswerter; er beweist
weder prinzipielle Lernunfähigkeit noch das Fehlen jeder internen Darstellung.
