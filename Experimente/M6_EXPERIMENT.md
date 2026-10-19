# M6: Aktionskopien und Sensorweiterleitung gemeinsam prüfen

Abschlussbericht vom 16.10.2026. **M6 liefert keinen belastbaren Nachweis gelernter
Handlungswirkungen.** Das gilt für alle vier Kombinationen, beide Sensorvarianten
und die getrennt bewerteten unmittelbaren und späteren Proben. Die visuelle
Auslesbarkeit beträgt dabei durchgehend 100 %. Eine zusätzliche Ursachenprüfung
zeigt: Core-Homöostase kann die zuvor ausreichende motorische Integration nach
dem Training wieder unterdrücken. Ihre Abschaltung erhält die Aktionssignale,
erzeugt aber ebenfalls keinen abgesicherten Lernvorteil.

[Bestätigung auf Seeds 300–309](../../Messungen/Experimente/m6-confirm/summary.md),
[Pilot auf Seeds 0–2](../../Messungen/Experimente/m6-pilot/summary.md),
[alle Vorhersagekontraste](../../Messungen/Experimente/m6-confirm/prediction-contrasts.csv),
[unabhängiger Datenaudit und faktorielle Kontraste](../../Messungen/Experimente/m6-confirm/diagnostics.json),
[Core-Aktivität aus den Trialprotokollen](../../Messungen/Experimente/m6-confirm/signal-diagnostics.json),
[technische Verifikation](../../Messungen/Experimente/m6-confirm/verification.md).

## Vorab festgelegter kombinierter Versuch

M5 hatte motorische Integration und sensorische Homöostase getrennt untersucht.
M6 kombiniert beide Faktoren ohne neue neuronale Lernregeln:

| Variante | Kopien derselben ausgeführten Taste | Visuelle Homöostase |
|---|---:|---|
| A: Referenz innerhalb von M6 | 1 | an |
| B: motorische Integration | 3 | an |
| C: erhaltene Sensorweiterleitung | 1 | aus |
| D: Kombination | 3 | aus |

Alle Varianten verwenden Kopiegewicht **0,9**, Core-Membranzeitkonstante **20 ms**,
STDP an und Strukturentwicklung aus. **Core-Homöostase bleibt in dieser Matrix an.**
Motor- und Aktionskopie-Zellen erhalten keine Homöostase. Die weitere Core-Kontrolle
ist unten ausdrücklich als eigener Folgeversuch ausgewiesen.

Die primäre Sensorbedingung ist ein aus dem aktuellen Bild abgeleiteter
2-Kanal-Diagnosesensor mit 32 Core-Neuronen. Eine zweite Bedingung verwendet
16×16 Graustufenpixel und 128 Core-Neuronen. Beide kodieren deterministisch.
Das ist eine Replikation unter anderer Sensorik und Kapazität, kein isolierter
Vergleich ausschließlich der Netzwerkgröße. Pro Fall laufen vier Arme und beide
Belegungen: untrainiert eingefroren (`before`), mit Erfahrungen gealtert, aber
eingefroren (`aged_frozen`), gekoppelt gelernt (`paired`) und unabhängig gepaarte
Aktions-/Wirkungsbilder (`scrambled`).

Pro Seed haben A–D dasselbe Anfangsnetz, denselben vor Training festgelegten
sensorischen Verstärkungsfaktor und dieselben erzeugten visuellen Eingangsimpulse.
Das initiale erwartete Sensorstrombudget ist über das ganze 160-ms-Trial auf den
Core-Populationsmittelwert 10 pro Neuron/s normiert. Es wird nach dem Training
nicht nachgeregelt. Beim Belegungswechsel bleibt die tatsächlich ausgeführte
Tastenfolge identisch; nur ihre erfahrene Wirkung kehrt sich um.

Der [Plan der Bestätigung](../../Messungen/Experimente/m6-confirm/manifest.json) enthält acht Fälle mit
256 Trainingstrials, 128 visuellen Kalibriertrials und 128 laufenden Testtrials je
Probe. Pilot und Bestätigung bleiben getrennt; die neuen Seeds wurden nicht zum
Nachstellen der Parameter verwendet.

## Zeitprotokoll und unmittelbare Messung

Ein Trial enthält 50 ms neutrales Startbild, 50 ms Aktionsintegration, 50 ms
Wirkungsbild und 10 ms neutrales Bild. Das Aktionsfenster ist in 10/10/30 ms
unterteilt, auch in den Einzelkopie-Varianten. Drei Kopien werden bei Aktionsbeginn,
nach 10 ms und nach 20 ms eingespeist. Ihre tatsächlich protokollierten
synaptischen Ankünfte liegen nach **1,5 / 11,5 / 21,5 ms**, jeweils an drei
angeschlossenen Core-Zellen. Die Vorhersage erfolgt am Ende des Aktionsfensters,
**eine Mikrosekunde vor Bewegung und Wirkungsbild**. Keine Kopie trifft erst nach
der Wirkung ein.

Nach dem Training wird die laufende Elterninstanz eingefroren. Zur Kalibrierung
des externen visuellen Auswerters wird eine getrennte Instanz durch dieselben
Frames, Tasten und Zeitabschnitte exakt bis zum Checkpoint reproduziert. Jeder
neuronale Schritt und der anschließende ausführbare Zustand werden verglichen:
einschließlich ausstehender Ereignisse, STDP-Spuren, lokaler Uhren,
Sensorphasenzustand, Decoder und Modellzeit. Die 20,48 Sekunden Kalibrierung
vergehen ausschließlich auf diesem Zweig. Die Elterninstanz bleibt am Checkpoint
und wird weder kalibriert noch zurückgesetzt.

Zwei verschiedene Messungen trennen einen einzelnen Ausgangszustand von einem
fortlaufenden Test:

- **Checkpoint:** A und D werden auf jeweils einem eigenen Replay desselben
  Checkpoints geprüft. Das sind zwei Beispiele je Belegung und Seed.
- **Online sofort:** 128 Trials beginnen unmittelbar am unveränderten Checkpoint.
  Die ersten 16 balancierten Trials werden zusätzlich separat berichtet.
- **Nach Ruhe:** Anschließend 2 Sekunden neutrales Bild und weitere 128 Trials.
- **Nach Interferenz:** Anschließend 1,4 Sekunden weitere Bilder und 128 Trials.

Zwischen Ruhe und Interferenz liegt somit die vollständige Ruheprobe. Der
Auswerter wird in diesen Proben nicht neu angepasst. Auch beim Freeze entwickeln
sich Membranpotenziale, Spuren und ausstehende Ereignisse weiter; nur die
Lernparameter bleiben erhalten. Die erste Prognose liegt **99.999 µs nach dem
Checkpoint**, weil sie ein neutrales Startbild und die Aktionsintegration benötigt.
Noch kürzere vorübergehende Zustände sind deshalb nicht ausgeschlossen.

Der primäre Auswerter verwendet visuell kalibrierte Core-Spikezählungen;
acht Zeitbins bilden einen sekundären Auswerter. Die visuelle Auslesbarkeit wird
mit chronologisch getrennten Kalibrierhälften geprüft. Die reine Pixelvorhersage
vor dem Wirkungsbild bleibt bei 50 %. Weltzustand, Wirkungsklassen, Zukunftsbilder
und Auswerterwerte gelangen nicht in den neuronalen Actor. Die Tasten werden für
diesen Ursachenversuch kontrolliert ausgeführt; er prüft Wirkungsrepräsentation,
keine selbstständige Zielverfolgung.

## Bestätigung: kein reproduzierbarer Lernvorteil

Mittelwerte über zehn neue Seeds und beide Belegungen, gekoppelt trainierter Arm:

| Fall | Checkpoint | Online sofort | Erste 16 | Nach Ruhe | Nach Interferenz | Visuell | Sensorweiterleitung |
|---|---:|---:|---:|---:|---:|---:|---:|
| 2 Kanäle / A | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 100 % | 96,9 % |
| 2 Kanäle / B | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 100 % | 96,9 % |
| 2 Kanäle / C | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 100 % | 100 % |
| 2 Kanäle / D | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 100 % | 100 % |
| 16×16 Pixel / A | 50,0 % | 50,8 % | 51,2 % | 50,7 % | 52,2 % | 100 % | 57,3 % |
| 16×16 Pixel / B | 50,0 % | 50,6 % | 51,2 % | 50,3 % | 51,9 % | 100 % | 57,3 % |
| 16×16 Pixel / C | 50,0 % | 50,2 % | 49,1 % | 50,0 % | 50,0 % | 100 % | 100 % |
| 16×16 Pixel / D | 50,0 % | 50,1 % | 49,4 % | 49,7 % | 50,0 % | 100 % | 100 % |

Weiterleitung bedeutet tatsächliche visuelle Sensorneuronen-Spikes geteilt durch
erzeugte Sensorimpulse während des Trainings. Der in M5 beobachtete starke
Verlust ist unter der Pixelbedingung erneut sichtbar. Beim 2-Kanal-Sensor fällt
er hier wesentlich kleiner aus; 43 % Verlust sind keine allgemeine Konstante
für alle Sensoren und Zeitprotokolle.

Lernen erfordert für jeden Prüfzeitpunkt gesondert mindestens zehn Seeds,
visuelle Auslesbarkeit mindestens 80 % in beiden Belegungen und einen positiven
gepaarten Spikezähl-Vorhersagevorteil gegenüber Zufall sowie **allen drei**
Kontrollen in beiden Belegungen. Zusätzlich müssen Belegungswechsel und
gleichzeitig richtige Vorhersagen für dieselbe Aktion unter beiden Belegungen
gegenüber allen Kontrollen zunehmen. Die unteren Grenzen der gepaarten
95-%-Bootstrapintervalle müssen jeweils strikt positiv sein. Die ersten 16
Online-Trials sind eine zusätzliche Zustandsdiagnose, kein eigener bestätigender
Endpunkt. Ein späterer Verlust würde einen bestandenen unmittelbaren Endpunkt
nicht nachträglich aufheben.

**Keine Bedingung besteht diese Kriterien**, weder am Checkpoint noch online,
nach Ruhe oder nach Interferenz. Der Zeitbin-Auswerter liefert ebenfalls keinen
Vorteil gegenüber allen Kontrollen in beiden Belegungen. Im 2-Kanal-Test ändern
sich die Vorhersagen beim Belegungswechsel überhaupt nicht. Unter Pixeln wechseln
sie bei 7,7–9,1 % der Aktionen; nur 3,9–5,3 % sind unter beiden Belegungen zugleich
richtig. Diese Werte liefern keinen lernabhängigen Vorteil.

Die Intervalle verwenden Seeds als gepaarte Einheit. Sie sind deskriptiv und
nicht für multiple Vergleiche korrigiert. A verwendet ebenfalls das neue
Zeitprotokoll und die Replay-Kalibrierung. Daher ist A eine Referenz **innerhalb
von M6**, keine unveränderte Wiederholung der alten M4-Bedingung.

## Zusätzliche Ursache: Core-Homöostase unterdrückt Integration

Die feste M6-Matrix bewahrt den Signalerfolg aus einem frischen M5-Netz nicht
automatisch während des Lernens. Bei den 2-Kanal-Varianten B und D entstehen nach
Training im gekoppelten wie im entkoppelten Arm **null Core-Spikes in allen
2.560 unmittelbaren Aktionsfenstern je Arm/Variante**. In den untrainierten und
gealtert eingefrorenen Kontrollen entstehen dagegen jeweils drei Spikes pro
Aktionsfenster. Die drei Kopien kommen in allen Fällen korrekt an.

Die anschließend geplante [Einzelzelldiagnose](../../Messungen/Experimente/m6-core-diagnostic/plan.json)
prüft dieses Signalproblem auf Seeds 0–2, Variante D, beiden Belegungen und
beiden Tasten. Für die drei tatsächlichen Zielzellen jeder Aktion werden dieselben
beobachteten Eingangsankünfte auf unabhängige Zellkopien angewandt. Die unveränderte
Zellkopie muss die Spikezeiten der echten Netzinstanz exakt reproduzieren. Nur
auf einer zweiten Zellkopie wird der konstante intrinsische Strom am Aktionsbeginn
auf null gesetzt. Der gelernte Elternzustand bleibt unverändert.

| Messung an 36 Zielzellen | Ergebnis |
|---|---:|
| Intrinsischer Strom nach Training | −10,0 bis −5,469 |
| Unveränderte Zellkopie | 36/36 ohne Spike |
| Gleiche Zelle, nur intrinsischer Strom null | 36/36 mit einem Spike |
| Potenzial einschließlich drittem Eingang, unverändert | 0,913 bis 0,993 |
| Potenzial einschließlich drittem Eingang, Strom null | 1,042 bis 1,105 |
| Schwelle | 1,0 |

Diese Intervention isoliert die elektrische Wirkung des negativen intrinsischen
Stroms. Sie ist kein vollständiger rekurrenter Gegenlauf und kein Lernnachweis.
Die Trainingsendzustände stimmen mit dem ursprünglichen Pilot überein.
Rohdaten: [Seed 0](../../Messungen/Experimente/m6-core-diagnostic/seed-000.json),
[Seed 1](../../Messungen/Experimente/m6-core-diagnostic/seed-001.json),
[Seed 2](../../Messungen/Experimente/m6-core-diagnostic/seed-002.json).

## Getrennte Folgeprüfung: Core-Homöostase an und aus

Um die Core-Homöostase als eigene Variable zu prüfen, folgt eine begrenzte,
explorative Zusatzreihe mit **neuen Seeds 400–409**. Ihr
[vor Ausführung gespeicherter Plan](../../Messungen/Experimente/m6-core-control/manifest.json) hält
Variante D mit 2-Kanal-Sensor, 32 Core-Zellen, drei Kopien, Sensorhomöostase aus
und alle übrigen M6-Parameter fest. Nur Core-Homöostase wird an-/abgeschaltet.
Alle vier Arme, beide Belegungen und dieselben Erfolgskriterien bleiben erhalten.

| Core-Homöostase | Checkpoint | Online | Nach Ruhe | Nach Interferenz | Visuell | Core-Spikes im Aktionsfenster |
|---|---:|---:|---:|---:|---:|---:|
| an | 50,0 % | 50,0 % | 50,0 % | 50,0 % | 100 % | 0 |
| aus | 52,5 % | 52,5 % | 52,5 % | 52,5 % | 100 % | 3 |

Die Abschaltung erhält die Integration im laufenden Netz: **0/2.560** stille
Online-Fenster statt **2.560/2.560**. Anfangsnetze, Normierung und eingefrorene
Kontrollen stimmen exakt überein. Der gepaarte Vorhersageunterschied aus minus an
beträgt jedoch nur **+2,5 Prozentpunkte, 95-%-Intervall [0,0; +7,5]** an allen vier
Prüfzeitpunkten. Auch ohne Core-Homöostase besteht kein Lernnachweis gegenüber
den Kontrollen und bei Belegungsumkehr.

[Zusatzbericht](../../Messungen/Experimente/m6-core-control/summary.md),
[Vorhersagekontraste](../../Messungen/Experimente/m6-core-control/prediction-contrasts.csv),
[Datenaudit und gepaarter An-/Aus-Vergleich](../../Messungen/Experimente/m6-core-control/diagnostics.json).
Diese nach dem M6-Pilot motivierte Reihe wird nicht als ursprüngliche
Bestätigung der Vierervarianten ausgegeben.

## Bedeutung und Grenzen

M6 umfasst 24 Pilot- und 80 Bestätigungsergebnisse. Hinzu kommen 20 Ergebnisse
der getrennten Core-Kontrolle und 36 Einzelzellinterventionen in drei weiteren
Diagnoseergebnissen. Der unabhängige Auswerter liest sämtliche zugehörigen
Trial-, Phasen-, Sensor-, Kopien- und Replayprotokolle; keine fehlenden Ergebnisse
wurden durch Ersatzwerte ergänzt.

Die negative ursprüngliche Matrix allein könnte man noch durch nachträglich
unterdrückte Aktionssignale erklären. Die zusätzliche Core-Kontrolle zeigt dagegen:
Im 2-Kanal-Test ohne Homöostase sind Sensorweiterleitung,
visuelle Auslesbarkeit und überschwellige Aktionssignale sind dort vorhanden,
ein belastbarer Zuordnungsvorteil fehlt weiterhin. Das beweist nicht, dass STDP
grundsätzlich ungeeignet ist. Es begründet als nächsten Forschungsgegenstand
die tatsächlichen Prä-/Post-Zeiten, Gewichtswirkungen und rekurrenten Pfade
kleiner Handlungs-Wirkungs-Schaltungen.

GPU-Optimierung, neue Lernregeln, größere Netze oder automatische Umstellung des
Live-Modells wurden daraus nicht abgeleitet. Dessen Standardwerte bleiben
Kopiegewicht 0,9, Core-Tau 20 ms und globale Homöostase. Die neue Technik dient
der Zustandsprüfung und kontrollierten Ausführung des bestehenden Modells.
