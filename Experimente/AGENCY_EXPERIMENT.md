# Minimaltest für eigenständige Handlungsfähigkeit

Der Versuch ist eine geschlossene Bild–Aktion–Bild-Schleife. Die Testwelt läuft
zunächst direkt in Rust, verwendet aber dieselben `Frame`- und `Key`-Typen wie
die QEMU-Schnittstelle. NERVA und seine neuronalen Regeln bleiben unverändert.

```bash
# Vom Projektverzeichnis; keine VM und keine Netzwerkverbindung erforderlich.
./NERVA_Modell/Experimente/run.sh agency

# Gemessener Zehn-Seed-Lauf, in einem neuen Ergebnisordner wiederholbar:
./NERVA_Modell/Experimente/run.sh agency --motor-decoder count --jobs 10 --output Messungen/Experimente/mein-handlungstest

# Kurzer technischer Test; ein Seed reicht nicht für den Machbarkeitsnachweis.
./NERVA_Modell/Experimente/run.sh agency --seeds 7 --processing 16 \
  --train-episodes 2 --adapt-episodes 2 --eval-episodes 2 --episode-steps 3
```

Standard: Seeds 0–9, 32 Processing-Neuronen, 32 Trainings- und 32
Anpassungsepisoden, vier Prüfphasen mit jeweils 18 Episoden, sechs Aktionen pro
Episode. Fünf Varianten ergeben **6.800 Episoden insgesamt**, keine 6.800
Trainingsfälle pro Modell. `--jobs` führt unabhängige Seeds parallel aus;
Standard sind vier Worker. Zusammengeführte Messwerte sind bei serieller und
paralleler Ausführung bytegleich. `--processing`, `--encoding`, `--initial-drive`
und `--pruning-min-us` erlauben weitere, ausdrücklich zu berichtende Konfigurationen.

## Welt und Eingangsgrenze

Die Welt hat sieben Positionen. Eine ausgefüllte weiße Figur steht auf einer
grauen Bahn; ein hohler weißer Rahmen markiert das Ziel. Beide Merkmale bleiben
im bestehenden Graustufenencoder sichtbar. Das Bild misst 64×64 Pixel.
A und D bewegen die Figur um eine Position, an den Rändern wird die Bewegung
begrenzt. Bei vertauschter Belegung kehrt sich die Tastenwirkung um. Die
Veränderung wird nicht durch ein zusätzliches Signal angekündigt.

Nur die zwei Motoren A/D sind angeschlossen. Ihre bisherigen Kanalnummern
bleiben erhalten. Im hier dokumentierten historischen Lauf bewirken Stille
und gleiche Motorzählungen **keine Taste** (`--motor-decoder count`);
es gibt keinen zufälligen oder zielabhängigen Ersatz für neuronale Aktionen.
Seit dem [M3-Folgeversuch](M3_EXPERIMENT.md) ist `--motor-decoder timing` Standard:
Nichtleere Zählgleichstände nutzen den früheren ersten Spike. Der historische
Zehn-Seed-Lauf wurde damit nicht erneut durchgeführt.
Die Live-VM-Demo behält standardmäßig die vollständige ISO-105-Tastatur.

Der Actor erhält ausschließlich `Frame` und seine eigenen ausgeführten Tasten.
Startposition, Zielkoordinate, Belegungsrichtung, Abstand, Erfolg, Belohnung und
korrekte Aktion werden nicht an `Model::step` oder die Lernregeln übergeben.
`MiniWorld::assess()` wird ausschließlich vom Auswerter benutzt. Seine Werte
stehen in den Ergebnis-CSV-Dateien, nicht in den neuronalen Eingängen.

Nach **jeder** ausgeführten Aktion wird das veränderte Bild ins Modell
eingespeist, auch bei Erfolg oder Ablauf der Episode. Der dadurch erzeugte
nächste Tastenvorschlag wird am Episodenende verworfen. Beim Zurücksetzen wird
keine weitere neuronale Aktion ausgeführt und die ausstehende eigene Tastenkopie
geleert. Das Netz selbst wird zwischen Episoden nicht neu aufgebaut.

## Ablauf

| Phase | Ziele | Tastenwirkung | Plastizität |
|---|---|---|---|
| `before` | 0,2,4,6 | Normal | Aus |
| `training` | 1,3,5 | Normal | Je Kontrollvariante |
| `after_frozen` | 0,2,4,6 | Normal | Aus |
| `reversed_before` | 0,2,4,6 | Vertauscht | Aus |
| `adaptation` | 1,3,5 | Vertauscht | Je Kontrollvariante |
| `reversed_after` | 0,2,4,6 | Vertauscht | Aus |

Damit erscheinen die Prüfziele nie in einer plastischen Trainingsphase.
Die anfängliche Leistungsmessung zeigt sie bereits ohne Lernen; sie sind also
gegenüber dem plastischen Training zurückgehalten, nicht völlig ungesehene Bilder.

Die 18 Prüfepisoden enthalten alle zulässigen Start/Ziel-Paare der vier Prüfziele
mit Anfangsabstand mindestens zwei. Je neun Aufgaben erfordern Bewegung nach
links beziehungsweise rechts. Alle sind innerhalb von sechs Aktionen lösbar.
Die Reihenfolge wird pro Seed vorab gemischt und für alle Varianten und
Prüfphasen identisch verwendet. Trainingsfolgen werden unabhängig davon erzeugt.
Das Ende einer Episode ist Erfolg oder Erreichen des festen Aktionsbudgets;
zufälliges, unbegrenztes Herumlaufen kann daher nicht jeden Versuch bestehen.

## Kontrollvarianten

Alle vier neuronalen Varianten beginnen pro Seed mit einem exakt identischen
Netzwerk. Danach wählen sie ihre Aktionen selbst und sammeln entsprechend
unterschiedliche Erfahrungen und Simulationszeiten.

| Variante | Gewichtslernen | Homöostase | Strukturentwicklung |
|---|---|---|---|
| `development` | Pair-STDP | An | Wachstum und Pruning |
| `fixed_topology` | Pair-STDP | An | Aus |
| `weight_only` | Pair-STDP | Aus | Aus |
| `frozen` | Aus | Aus | Aus |
| `random` | Kein neuronales Modell | – | Gleichverteiltes A/D |

`fixed_topology` kontrolliert gezielt den Entwicklungsanteil; `weight_only`
prüft tatsächlich rein gewichtsplastisches Lernen. Die zufällige
Anfangsverschaltung allein ist bereits in der eingefrorenen Kontrolle enthalten.
Eine handprogrammierte Zielpolitik wird weder trainiert noch verwendet.

Prüfphasen verwenden `Model::freeze()`. Gewichte, Topologie, STDP-/Utility-
Speicher und homöostatische Parameter werden vor und nach jeder Episode verglichen;
plastische Ereignisse müssen in jedem Beobachtungsschritt ausbleiben.
`Model::set_plasticity()` aktiviert für die nächste Trainingsphase dieselben
vorherigen Regeln wieder. NERVAs vorhandene API verwaltet die Homeostase-Uhren.
Während einer Lernpause können passive zeitabhängige Zustände weiterlaufen;
insbesondere kann die nur lesend projizierte Utility sinken. Es wird keine
zusätzliche Lernregel oder neue neuronale Fähigkeit implementiert.

## Kriterien und diagnostische Messungen

Die vorab gewählten Zielkriterien stehen im maschinenlesbaren Bericht:

- Mindestens zehn Seeds und mindestens 80 % Erfolg auf Prüfziele nach Einfrieren.
- Positiver gepaarter Vorteil gegenüber Vortraining, eingefrorenem Netz und
  Zufallsagent; das beschreibende 95-%-Bootstrapintervall über Seed-Differenzen
  muss vollständig oberhalb null liegen.
- Nach Belegungswechsel ein entsprechender Leistungseinbruch und nach erneuter
  Lernphase eine Erholung auf mindestens 80 %.

Ein zusätzlicher Vorteil gegenüber reiner Gewichtsplastizität und fester
Topologie wird separat gemessen. Er ist **kein** notwendiges Kriterium für
Handlungsfähigkeit selbst: Auch ein rein gewichtsplastischer Agent könnte sie
nachweisen. Die Bootstrapintervalle aus zehn Seeds sind eine kleine,
beschreibende Erstuntersuchung und kein allgemeiner Leistungsbeleg.

Zusätzlich werden A/D-Wahlen, Stille, Motor-Gleichstände, sichtbare Bewegung,
blockierte Bewegung, Bewegung zum/vom Ziel und neuronale Änderungen gezählt.
Eine externe Auswertung prüft, ob aktuelle Core-Zählungen die Richtung der
folgenden **Bildänderung** vorhersagen. Die Zielwerte stammen aus der Differenz
horizontaler Bildhelligkeitsmomente, nicht aus einem internen Positionssignal
an das Netz. Der unbewegte Zielrahmen trägt innerhalb einer Episode keine
Momentänderung bei. Eigene gewählte Aktionen allein und eine Mehrheitsvorhersage
dienen als Kontrollen. Klassenmittelwerte werden auf der ersten Hälfte jeder
Phase bestimmt und auf der späteren Hälfte geprüft; unbekannte Testklassen
zählen als Fehler. Diese Readouts werden niemals zur Aktionsauswahl verwendet.
Sie messen Vorhersagbarkeit, nicht neuronales Handlungsverständnis.

## Ergebnis des ersten vollständigen Laufs

[Gespeicherter Bericht](../../Messungen/Experimente/agency-ten-seeds-complete/agency-summary.md),
Seeds 0–9, Standardparameter, 32 Processing-Neuronen:

| Variante | Vor Training | Nach Training, eingefroren | Vertauscht, vor Anpassung | Nach Anpassung, eingefroren |
|---|---:|---:|---:|---:|
| Entwicklung | 3,3 % | 6,1 % | 3,9 % | 2,8 % |
| Feste Topologie mit Homöostase | 3,3 % | 4,4 % | 2,8 % | 3,9 % |
| Nur Gewichtslernen | 3,3 % | 5,0 % | 3,3 % | 3,3 % |
| Vollständig eingefroren | 3,3 % | 5,0 % | 3,3 % | 3,9 % |
| Zufälliges A/D | 31,1 % | 22,8 % | 30,6 % | 22,2 % |

Jede Prüfspalte enthält 180 Episoden je Variante, 18 pro Seed. Der
Machbarkeitsnachweis ist **nicht erbracht**. Die Entwicklungskontrolle erreicht
nach Training 11/180 Erfolge, die eingefrorene Kontrolle 9/180, der Zufallsagent
41/180. Die gepaarte Differenz Entwicklung minus eingefroren beträgt +1,1
Prozentpunkte, Intervall −2,8 bis +5,0. Gegenüber Zufall sind es −16,7 Punkte,
Intervall −23,3 bis −11,1. Auch Einbruch und Erholung zeigen keinen belastbaren
Lernverlauf; nach Anpassung bleiben nur 5/180 Erfolge.

Der erste klar gemessene Engpass liegt bei der Aktionsauswahl: Nach Training
führten **703 von 1.061** Entscheidungsfenstern (66,3 %) wegen gleicher,
nichtleerer A/D-Motorzählungen zu keiner Taste. Vollständige Motorstille trat
dabei nicht auf. Die übrigen Entscheidungen bewirkten 319 sichtbare Bewegungen;
die Bild-/Aktionsschleife arbeitet also. Millionen Gewichtsänderungen und
1.261 neue Verbindungen über die zehn ursprünglichen Trainingsläufe verbessern
die Zielverfolgung hier nicht hinreichend.

Von den 703 Gleichständen waren 664 Fenster mit **vier Spikes auf beiden
Motoren**, 33 mit jeweils fünf und sechs mit jeweils drei. Der bestehende
Zähldecoder lässt diese gleichzeitig aktiven Ausgänge unentschieden. Das grenzt
den beobachteten Explorationsengpass konkreter ein als die Vermutung, das Netz
würde überhaupt keine Motoraktivität erzeugen.

Auch der externe Core-Readout liefert keinen Nachweis gelernten Wirkungswissens:
Nach Training sagt er 245/533 Bildbewegungen richtig voraus (46,0 %), die
Mehrheitskontrolle 373/533 (70,0 %). Der Aktions-Readout erreicht 511/533, lernt
seine Zuordnung aber **außerhalb** von NERVA und erklärt keine neuronale
Zielverfolgung. Viele Stillstände machen die Mehrheitskontrolle stark.

Damit sind Wahrnehmungs-/Aktionspfad, Exploration, Vorhersage und Zielerfolg
getrennt messbar. Die Daten weisen auf unzureichende Aktionskonkurrenz beziehungsweise
Exploration hin. Eine zielgerichtete Motivation wird durch den sichtbaren Rahmen
und STDP allein weiterhin nicht begründet. Welche dieser Ursachen den Erfolg
entscheidend begrenzt, ist mit diesem einen Parametersatz noch nicht isoliert.
Der Befund widerlegt nicht den gesamten Ansatz.

Der nächste getrennte Test von Motorik und Wirkungslernen steht in
[SENSORIMOTOR_EXPERIMENT.md](SENSORIMOTOR_EXPERIMENT.md). Er nutzt eine Welt ohne
Ziel und untersucht die Information in den Motor-Spikezeiten, Konkurrenz und
einen ausdrücklich als Kontrolle gekennzeichneten zufälligen Tie-Break.

Weitere Welten und der QEMU-Transfer wurden entsprechend dem vorgeschlagenen
Stufenplan noch nicht begonnen: Zunächst müsste dieser Minimaltest einen
reproduzierbaren funktionalen Vorteil zeigen. Die bestehenden QEMU-Regressionstests
prüfen weiterhin den bisherigen Gast, nicht die neue Sieben-Positionen-Welt.

## Dateien und Prüfungen

- `NERVA_Modell/nerva-qemu/src/test_world.rs`: unabhängige Welt mit Bild-/Tasten-Schnittstelle.
- `NERVA_Modell/Experimente/src/agency.rs`: Actor, Episoden, Kontrollphasen, Readouts und Kriterien.
- `agency.json`, `agency-summary.md`: vollständige Messwerte und Überblick.
- `agency-episodes.csv`: Auswertung jeder Episode, einschließlich nur dort
  sichtbarer Start-/Zielkoordinaten.
- `agency-actions.csv`: jede gewählte Aktion, Motorzählungen und Bildwirkung.
- `agency-learning.csv`: Gewichtsanpassungen, Homöostase und Struktur pro Episode.
- `frames/`: exemplarische Start-/Endbilder; `worker-*/` enthält Teilberichte.

Die Tests prüfen Weltgrenzen, Umkehrung, sichtbare Ziele, getrennte Aufgabensätze,
Freeze/Resume, aus Motorzählungen abgeleitete Aktionen, deterministische
Wiederholung einschließlich CLI und serielle/parallele Gleichheit.
