# M7: Zuordnungsspuren in Gewichten, noch keine zusätzliche Spike-Weiterleitung

Abschlussbericht vom 16.10.2026. M7 führt den im neuen Auftrag vorgeschlagenen
Synapsentest nach M6 aus. **Die vorhandene STDP erzeugt belegungsabhängige
Gewichtsänderungen. Diese Änderungen beeinflussen Membranpotenziale, erzeugen
in den untersuchten Aktionsproben aber keine zusätzliche Core-Spikeweiterleitung.**
Damit ist „es verändert sich überhaupt nichts entsprechend der Erfahrung“ zu
stark. Ebenso wäre „NERVA hat funktionales Wirkungswissen gelernt“ zu stark.

[Gemessene Übersicht](../../Messungen/Experimente/m7-synapse-audit/summary.md),
[jede einzelne Synapse unter beiden Belegungen](../../Messungen/Experimente/m7-synapse-audit/edge-comparison.csv),
[Gewichtssummen](../../Messungen/Experimente/m7-synapse-audit/weights-summary.csv),
[Prä-/Post-Zeitpaare](../../Messungen/Experimente/m7-synapse-audit/pair-summary.csv),
[Gegenprüfungen](../../Messungen/Experimente/m7-synapse-audit/counterfactuals.csv),
[Datenaudit und deskriptive Kontraste](../../Messungen/Experimente/m7-synapse-audit/diagnostics.json).

## Aufbau: vorhandene M6-Erfahrungen erneut ausführen

Die Untersuchung verwendet **alle zehn vorhandenen Seeds 400–409** der separaten
M6-Core-Kontrolle ohne Core-Homöostase. Sie ist eine explorative Ursachenanalyse,
keine neue Bestätigung auf unbenutzten Seeds. Ein Software-Probelauf mit Seed 400
ging dem gespeicherten [Matrixplan](../../Messungen/Experimente/m7-synapse-audit/manifest.json) voraus.
Parameter, Arme und Eingriffe wurden anschließend nicht nach Ergebnissen einzelner
Seeds ausgewählt. M6 und seine historischen Messdateien bleiben erhalten.

Fest: 2-Kanal-Diagnosesensor aus aktuellen Pixeln, 32 räumliche Core-Zellen,
drei Aktionskopien, Gewicht 0,9, Core-Tau 20 ms, 50 ms zwischen Aktion und Wirkung,
256 Trainingstrials. Homöostase und Strukturentwicklung sind aus. Die dünne
zufällige Anfangsvernetzung bleibt bestehen. Sensor-, Motor-, Aktionskopie- und
Core-Populationen werden wie bisher zusammengesetzt; keine funktionalen internen
Module oder gewünschten Handlungswege werden vorgegeben.

Verglichen werden gekoppelt trainiert, entkoppelt trainiert und mit denselben
Erfahrungen gealtert eingefroren. Beide Belegungen erhalten dieselben ausgeführten
Tasten. Der gekoppelte Wirkungsstrom kehrt sich bei Belegungsumkehr um; die
entkoppelten Bilder bleiben unabhängig von der Taste. Drei Arme × zwei Belegungen
× zehn Seeds ergeben **60 protokollierte Trainingsläufe**.

Anfangsnetz-Fingerabdruck und sensorischer Verstärkungsfaktor müssen mit M6
übereinstimmen. Core-Spikes, Sensorimpulse, Kopienankünfte und Gewichtsupdatezahlen
des Trainings werden mit den M6-Diagnosen verglichen. Die abschließenden
Core-Membranwerte müssen bei der tatsächlich gespeicherten **f32-Präzision**
übereinstimmen. Dezimale JSON-Werte können beim f64-Einlesen geringfügig anders
gerundet werden; es wird deshalb auf den zugrunde liegenden f32-Zustand verglichen.
Das ist kein aus M6 serialisierter vollständiger Runtime-Checkpoint.

Die nachfolgenden Gegenprüfungen reproduzieren dagegen jeweils den vollständigen
ausführbaren M7-Zustand mit `Model::same_replay_state`, einschließlich ausstehender
Ereignisse, Spuren, Sensorphase, Decoder und Uhren. Erst danach wird ein Gewicht
auf der jeweiligen getrennten Instanz verändert. Das Elternmodell bleibt erhalten.

## Was an den Synapsen tatsächlich passiert

Jede beobachtete Gewichtsänderung wird der letzten tatsächlichen präsynaptischen
Ankunft und dem letzten postsynaptischen Spike zugeordnet. Positive und negative
Änderungen werden getrennt erfasst, einschließlich Paaren über Trialgrenzen.
Die Runtime benutzt die präsynaptische **Ankunft**, nicht den früheren
Emissionszeitpunkt. Es gilt die bestehende nächste-Paar-Regel:

```text
LTP: +0,0001  × exp(−Abstand / 20 ms)
LTD: −0,00012 × exp(−Abstand / 20 ms)
Paarfenster: 100 ms; Gewichte: 0 bis 2
```

Alle **1.834.289** beobachteten Updates stimmen in Rust exakt mit dieser
f32-Rechnung überein. Ein passiver Gewichtsledger stellt sicher, dass die Ereignisse
sämtliche Gewichtsänderungen erklären. Der externe Python-Audit rekonstruiert
die nächsten Partner unabhängig aus den Ereignissen und vergleicht Ledger,
Zeitpaare, Zwischenstände und Endgewichte. Inhibitorische Ausgangssynapsen werden
von dieser Pair-STDP nicht angenommen; `plastic=true` allein genügt nicht.

Mittelwerte je Modellinstanz über zehn Seeds und zwei Belegungen:

| Arm | Core-Kanten verstärkt | abgeschwächt | unverändert | Mittlere absolute Gewichtsänderung pro Core-Kante |
|---|---:|---:|---:|---:|
| gealtert eingefroren | 0,0 | 0,0 | 128,0 | 0 |
| gekoppelt | 7,4 | 8,8 | 111,8 | 0,000963 |
| entkoppelt | 7,5 | 8,8 | 111,7 | 0,000953 |

Diese 128 Core-Kanten umfassen auch inhibitorische und nicht aktiv gewordene
Verbindungen. Die größte absolute Änderung einer gekoppelten Core-Kante beträgt
0,047186. Die acht visuellen Eingänge zum Core werden dagegen **alle abgeschwächt**:
im Mittel sinkt ihre Gewichtssumme um 2,1162 pro Instanz. Es erreicht keine
untersuchte Core-, visuelle oder Core→Motor-Kante Gewicht null. Sensorimpulse
werden bei deaktivierter Homöostase weiterhin vollständig weitergegeben;
synaptische Abschwächung ist ein zusätzlicher, davon verschiedener Einfluss.

Im gekoppelten Arm enthalten die Core→Core-Updates über alle 20 Instanzen:

| Paartyp | Updates | Summe der Gewichtsänderungen | Mittlerer absoluter Zeitabstand |
|---|---:|---:|---:|
| LTP: Prä-Ankunft im Aktionsfenster, Post-Spike im Wirkungsfenster desselben Trials | 21.946 | +0,202260 | 52,38 ms |
| LTP: andere Paare desselben Trials | 36.422 | +2,890107 | 6,39 ms |
| LTD: andere Paare desselben Trials | 72.702 | −4,454494 | 25,59 ms |
| LTP über Trialgrenze | 7.724 | +0,011282 | 84,93 ms |
| LTD über Trialgrenze | 6.337 | −0,009693 | 87,73 ms |

Die Handlungs-/Wirkungspaare fehlen also nicht vollständig. Sie liegen häufig
deutlich jenseits der 20-ms-Zeitkonstante und erzeugen entsprechend kleine
Einzeländerungen. Andere Paarungen und Depression tragen wesentlich mehr zur
gesamten Gewichtsbewegung bei. Allein aus der Anzahl solcher Paare folgt noch
keine selektive oder wirksame Assoziation.

## Ändern sich die Gewichte entsprechend der Belegung?

Ja, in dieser Diagnose. Im gekoppelten Arm beträgt die mittlere Summe der absoluten
Core-Gewichtsunterschiede zwischen normaler und umgekehrter Belegung **0,041446**
pro Seed. Bei entkoppelten und eingefrorenen Erfahrungen ist sie exakt null.
Die einzelnen positiven und negativen Unterschiede stehen in `edge-comparison.csv`.

Eine zusätzliche, vor der Matrixauswertung festgelegte topologische Diagnose
betrachtet vorhandene Core-Kanten von direkt an die Aktionskopie angeschlossenen
Zellen zu direkt visuell angeschlossenen Zellen. Gewichtsänderungen in Richtung
der erfahrenen Wirkung werden positiv gezählt, Änderungen zur Gegenwirkung
negativ. Zellen mit beiden visuellen Eingängen tragen null zum Unterschied bei.
Nur von STDP angenommene Core-Kanten zählen. Je Seed existieren dafür insgesamt
ein bis sieben berücksichtigte Aktions-/Visualkanten; keine wird eigens angelegt.

Die mittlere Ausrichtung beträgt **+0,006224**, mit deskriptivem gepaartem
95-%-Bootstrapintervall **[+0,002838; +0,009995]** gegenüber jeweils entkoppeltem
und eingefrorenem Arm. Neun der zehn Seedmittelwerte sind positiv. Die beiden
Kontrollen haben bei Mittelung über die Belegungen exakt null. Es wird weder
für mehrere Vergleiche korrigiert noch mit neuen Seeds bestätigt.

Diese Ausrichtung beschreibt vorhandene Anschlusszellen und Gewichte. Sie ist
kein gelerntes Objekterkennungsmodul, keine Vorhersagegenauigkeit und kein Beweis
für die kausale Nutzung einer Wirkungsklasse. Auch unterschiedliche Bildreihenfolgen
ändern lokale Spikepaarungen. Die funktionale Bedeutung benötigt die folgende
getrennte Intervention.

## Beeinflussen diese Änderungen die Antwort auf A/D?

Nach Training werden die vorhandenen Regeln eingefroren und 2 Sekunden neutrales
Bild gezeigt. Für jede Taste beginnt jeder Eingriff auf einem eigenen exakten
Replay dieses ausführbaren Zustands. Es wird anschließend nur das neutrale Bild
gezeigt und dieselbe dreifache Aktionskopie eingespielt, **kein Wirkungsbild**.
Die Eingriffe ändern ausschließlich ausgewählte Gewichte:

- `sham`: unverändertes trainiertes Modell.
- `restore_core`: sämtliche Core→Core-Gewichte auf ihren Anfangswert setzen.
- `restore_all_plastic`: sämtliche als plastisch markierten Gewichte auf Anfangswerte setzen.
- `block_core`: sämtliche Core→Core-Gewichte null setzen, einschließlich Hemmung.
- `no_copy`: unveränderte Gewichte, aber keine Aktionskopie als Eingangskontrolle.

Je Eingriff im gekoppelten Arm: zwei Tasten × zwei Belegungen × zehn Seeds =
**40 separate Aktionsproben**. Über alle drei Arme sind es 600 Gegenprüfungen.

| Eingriff | Core-Spikes pro Probe | Spikes außerhalb der drei Kopie-Zielzellen | Geänderte Spikefolgen gegenüber Sham | Größte Potenzialänderung gegenüber Sham |
|---|---:|---:|---:|---:|
| unverändert | 3 | 0 | 0/40 | 0 |
| Core-Gewichte zurückgesetzt | 3 | 0 | 0/40 | 0,032855 |
| alle plastischen Gewichte zurückgesetzt | 3 | 0 | 0/40 | 0,032855 |
| Core-Rekurrenz blockiert | 3 | 0 | 0/40 | 0,591560 |
| keine Aktionskopie | 0 | 0 | 40/40 | 1,190755 |

Auch im entkoppelten und eingefrorenen Arm ändern die Gewichtseingriffe keine
Core-Spikefolge. Die normale Aktionsantwort besteht in dieser Probe vollständig
aus den drei direkt angeschlossenen Core-Zellen. Rekurrente Ströme beeinflussen
andere Membranpotenziale; das belegt die große Änderung beim Blockieren. Die
gelernten Gewichtsänderungen beeinflussen diese Potenziale ebenfalls, reichen
aber innerhalb des geprüften 50-ms-Aktionsfensters nicht für zusätzliche Spikes.

Damit ist hier erstmals experimentell getrennt: **zuordnungsabhängige Gewichte**,
**elektrisch wirksame Rekurrenz** und **zusätzliche Spike-Weiterleitung** sind
verschiedene Befunde. Letztere ist in diesen isolierten Aktionsproben nicht
vorhanden. Andere neuronale Zustandsmerkmale, andere Zeitfenster oder eine Probe
ohne die 2 Sekunden Ruhe sind dadurch nicht ausgeschlossen. M7 verwendet keinen
externen Zukunftsauswerter; die funktionalen Vorhersagegrenzen stammen weiterhin
aus [M6](M6_EXPERIMENT.md).

## Erhaltung und nächste offene Frage

Gewichte werden initial, nach jeweils 64 Trainingstrials, beim Freeze, nach
2 Sekunden Ruhe und nach 1,4 Sekunden zusätzlichen Bildern protokolliert.
Nach Freeze bleiben sie exakt erhalten. Das folgt aus abgeschalteter Plastizität
und **beweist keine Gedächtnisstabilisierung bei weiterlaufendem Lernen**.
Die Interferenzbilder laufen auf der unveränderten Elterninstanz; Eingriffe
erfolgen ausschließlich auf den Replayzweigen.

Der nächste gezielte Forschungsgegenstand ist somit die funktionale Weiterleitung
der vorhandenen Gewichtsassoziation: tatsächliche Ströme, Schwellen und zeitliche
Summation in kleinen vorhandenen Core-Pfaden. M7 rechtfertigt keine pauschale
Behauptung, die STDP sei funktionslos oder es müsse sofort eine neue Lernregel
hinzukommen. Kurzzeitplastizität, andere lokale Regeln oder Neuromodulation
wurden hier nicht implementiert. Auch das laufende VM-Modell wurde nicht umgestellt.

Software und Aufrufe: [m7/README.md](m7/README.md).
Vollständigkeit, Tests und Reproduzierbarkeit:
[verification.md](../../Messungen/Experimente/m7-synapse-audit/verification.md).
