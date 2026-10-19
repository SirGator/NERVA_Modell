# M5: Aktionskopie und sensorische Homöostase

Abschlussbericht vom 16.10.2026. Zwei getrennte Ursachenversuche mit bestehenden
LIF-, STDP- und Homöostaseregeln. Parameter und Endpunkte stehen in den vor der
Ausführung gespeicherten Manifesten. Nach dem Pilot wurden keine Parameter geändert.

**Beide vermuteten Engpässe sind unter den untersuchten Bedingungen bestätigt:
Zeitliche Integration macht die bisherige Aktionskopie überschwellig; Homöostase
an Sensorneuronen schwächt deren Weiterleitung deutlich. Funktionales
Wirkungslernen ist damit weiterhin nicht nachgewiesen.**

[Zehn-Seed-Ergebnisse](../../Messungen/Experimente/m5-confirm/summary.md),
[Pilot](../../Messungen/Experimente/m5-pilot/summary.md),
[gepaarte Populationskontraste und Datenaudit](../../Messungen/Experimente/m5-confirm/diagnostics.json),
[Vorhersagekontraste](../../Messungen/Experimente/m5-confirm/prediction-contrasts.csv),
[technische Verifikation](../../Messungen/Experimente/m5-confirm/verification.md).

## A: Wann wird die Aktionsinformation überschwellig?

32 Core-Neuronen, 2-Kanal-Diagnosesensor, stets dasselbe neutrale Bild. Keine
Homöostase oder Strukturentwicklung. Je eine frische Instanz für A, D und keine
Kopie, jeweils mit und ohne STDP. Innerhalb einer Bedingung ist der Anfangszustand
bitgleich. Es werden Aktionskopien diagnostisch eingespeist; es wird keine VM
bewegt und kein Wirkungsbild gezeigt.

Vorab festgelegt: Kopiegewichte **0,45 / 0,9 / 1,8 / 3,6** und Core-Membranzeitkonstanten
**5 / 20 / 50 ms**. Die drei Kopien derselben gehaltenen Taste liegen immer bei
**1, 10.001 und 20.001 µs**. Alle Bedingungen haben denselben 50-ms-Stichtag.
Die Membranzeitkonstante verändert die bestehende LIF-Integration; die Zahl und
Zeitpunkte der Eingangsimpulse bleiben gleich. Die Kopieneuronen behalten ihre
bisherige Refraktärzeit. Die Anschlussverbindungen der Kopie bleiben nichtplastisch.

Beide Signalkriterien bedeuten: A und D erzeugen jeweils Core-Spikes und tatsächliche
Ankünfte auf plastischen Core→Core-Verbindungen; ihre Core-Spikezählmuster unterscheiden
sich. Das prüft eine Voraussetzung des Lernens und keine gelernte Wirkungszuordnung.

| Gewicht | Core-Tau | Beide Signalkriterien auf neuen Seeds | Erste Core-Spikes |
|---:|---:|---:|---:|
| 0,45 | 5 / 20 / 50 ms | jeweils 0/10 | keine |
| 0,9 | 5 ms | 0/10 | keine |
| 0,9 | 20 ms | 10/10 | 21.501 µs, nach der dritten Kopie |
| 0,9 | 50 ms | 10/10 | 11.501 µs, nach der zweiten Kopie |
| 1,8 | 5 / 20 / 50 ms | jeweils 10/10 | 1.501 µs, nach der ersten Kopie |
| 3,6 | 5 / 20 / 50 ms | jeweils 10/10 | 1.501 µs, nach der ersten Kopie |

Bei Gewicht 0,9 und Tau 20 ms entstehen durchschnittlich drei Core-Spikes und
zwölf Ankünfte auf plastischen Core-Kanten je Aktion. Bei Gewicht 1,8 und Tau
20 ms sind es 9,1 beziehungsweise 36,4. Das bisherige Gewicht ist bei drei
Kopien also bereits ausreichend; M4 hatte nur eine Kopie vor dem Wirkungsbild.
Das zusätzliche Gewicht 3,6 verbessert diese Signalwerte hier nicht gegenüber 1,8.

Auch echte lokale Gewichtsänderungen treten auf. Bei Gewicht 1,8 und Tau 20 ms
sind es im Mittel 3,5 Core-Gewichtsupdates je Aktion; **nur neun der zwanzig
A-/D-Durchläufe** über zehn Seeds enthalten solche Updates. Ankünfte auf
plastischen Kanten garantieren somit noch keine geeigneten Post-Spikes und
STDP-Paarungen in jedem Netz. Alle Rohereignisse und betroffenen Synapsen stehen
in den jeweiligen JSONs und `.events.csv`.

Die eingefrorenen Kontrollen verändern keine Lernparameter. Ohne Kopie bleiben
die neutralen Netze still. Kein Auswerter oder Lehrsignal erzeugt Core-Spikes.

## B: Welche Homöostase-Population schwächt die Sensorik?

16×16 Pixel, 128 Core-Neuronen, deterministische Kodierung, 8 ms Aktions-/Wirkungsabstand.
Der unveränderte M4-Ablauf enthält alle vier Arme, beide Belegungen, visuelle
Kalibrierung sowie erste, Ruhe- und Interferenzprobe. Verglichen werden:

- keine Homöostase,
- nur Processing-Core,
- Processing-Core und visuelle Sensorneuronen,
- alle Populationen als zusätzliche Referenz zum bisherigen Modell.

„Core und Sensorik“ umfasst die visuellen Neuronen, ausdrücklich nicht die
gesonderten Aktionskopie-Zellen oder Motoren. Alle Bereiche haben pro Seed
dasselbe Anfangsnetz, denselben Normierungsfaktor und dieselben erzeugten
Eingangsimpulse. Die eingefrorenen Kontrollarme stimmen über die vier Bereiche
vollständig überein. Es erfolgt keine Nachregelung anhand der Leistung.

Mittelwerte der zehn neuen Seeds, jeweils über beide Belegungen:

| Homöostase | Weitergegebene Sensorimpulse | Sensorstrom je Core/s | Visuelle Auslesbarkeit | Vorhersage vor Wirkungsbild |
|---|---:|---:|---:|---:|
| Keine | 100,0 % | 9,647 | 87,7 % | 51,1 % |
| Nur Core | 100,0 % | 9,649 | 82,7 % | 48,4 % |
| Core + visuelle Sensoren | 57,3 % | 5,419 | 53,0 % | 47,6 % |
| Alle Populationen | 57,3 % | 5,419 | 52,8 % | 47,0 % |

Hier zählt Weiterleitung tatsächliche Sensorneuronen-Spikes geteilt durch erzeugte
Eingangskanal-Spikes. Sie wird getrennt vom am Core ankommenden synaptischen Strom
gemessen. STDP kann dessen Amplituden zusätzlich verändern.

Der gezielte Kontrast **Core + Sensorik minus nur Core** ergibt:

- Weiterleitung: **−42,69 Prozentpunkte**, gepaartes 95-%-Intervall [−42,75; −42,64].
- Visuelle Auslesbarkeit: **−29,69 Punkte**, Intervall [−33,44; −25,94].
- Sensorischer Strom: **−4,231 Einheiten je Core/s**, Intervall [−4,258; −4,199].
- Vorhersage: −0,78 Punkte, Intervall [−4,73; +2,11]; kein abgesicherter Unterschied.

Die zusätzliche Einbeziehung der Motor- und Kopieneuronen erklärt den großen
Sensorverlust nicht: Der Weiterleitungsunterschied zwischen `all` und `core_sensor`
ist null. Die sensorischen intrinsischen Ströme bleiben bei `none` und `core`
exakt null. Bei `core_sensor` und `all` werden sie in den aktiven visuellen Zellen
negativ, teilweise bis zur konfigurierten Grenze −10. Über alle 256 Sensorzellen
gemittelt liegt ihr Wert am Trainingsende bei −0,823. Das stützt die Erklärung
über veränderte sensorische Erregbarkeit.

Core-Homöostase allein erhält zwar die Sensorweiterleitung, verringert hier die
visuelle Auslesbarkeit gegenüber keiner Homöostase ebenfalls etwas: −5,08 Punkte
[−9,30; −1,25]. Das Ergebnis rechtfertigt keine pauschale Aussage, jede Variante
von Homöostase sei falsch oder allein der Core sei bereits optimal eingestellt.

## Umfang und Grenzen

Pilot: zwölf Motorbedingungen und vier Sensorbedingungen, jeweils Seeds 0–2.
Neue Prüfung: dieselben Bedingungen mit **Seeds 200–209**, die weder im M4-Pilot
noch in dessen Bestätigung verwendet wurden. Insgesamt 208 Fall-/Seed-Ergebnisse:
156 Motorläufe mit jeweils sechs Interventionen und 52 Sensorläufe mit jeweils
acht Modellinstanzen. Pilot und neue Seeds werden getrennt ausgewertet.

Die gepaarten Bootstrap-Intervalle verwenden Seeds als Einheit. Sie sind deskriptiv
und nicht für multiple Vergleiche korrigiert. Einzelne Spikes, Updates oder
positive Kontraste sind kein funktionaler Lernnachweis.

Die erste Vorhersageprobe in B folgt weiterhin auf 20,48 Sekunden visuelle
Kalibrierung. Kurzlebige Repräsentationen direkt am Trainingsende sind damit
nicht ausgeschlossen. Ruhe bedeutet neutrales Bild, nicht neuronale Stille.
Zwischen Ruhe und Interferenz liegt die vollständige Ruheprobe.

**A und B korrigieren die beiden Engpässe nicht bereits gemeinsam.** B behält
zur Isolation der Homöostase die bisherige einzelne Aktionskopie mit Gewicht
0,9 bei. Keine B-Bedingung besteht die Vorhersagekriterien gegenüber allen
Kontrollen in beiden Belegungen und den drei Prüfzeitpunkten.

Der nächste begrenzte Wiederholungstest kann nun eine vorab festgelegte
überschwellige Aktionskopie mit erhaltener Sensorweiterleitung kombinieren,
erneut mit eingefrorenen und entkoppelten Kontrollen sowie unbenutzten Seeds.
Dieser kombinierte Wirkungslerntest ist in M5 noch nicht ausgeführt. Die
Standardparameter des laufenden VM-Modells wurden nicht anhand dieser Messung
automatisch umgestellt; neue Lernregeln oder größere Netze wurden nicht eingeführt.
