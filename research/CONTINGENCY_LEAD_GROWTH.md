# Vorlaufende Aktivität als lokale Bildungsregel im A/B-Test

Dosisgleiche Eingriffe an zwei archivierten Zielzellen änderten, ob
und wann ein Zielspike auftrat. Der folgende prospektive Test prüft,
ob eine lokale Auswahl neuer Kanten anhand wiederholter vorlaufender
Aktivität den Erwerb und Erhalt sensorischer Zuordnungen verbessert.
[Ausgangsbefund](CONTINGENCY_A_TIMING_RESULTS_20261124.md)

## Lokale Information und Kontrollregel

Für positive Netto-Integrationsbatches außerhalb der Refraktärzeit
bewertet die Regel zu höchstens einem Zeitpunkt pro Ziel und 10 ms
die vorherigen Spikes benachbarter erregender Zellen. Sie verwendet
je Quelle den letzten Spike mit mindestens 1 ms Vorlauf. Ein Vorlauf
bis 15, 40 oder 75 ms gibt dem Quellen–Ziel-Paar jeweils 3, 2 oder
1 Punkt. Der Wert fällt zwischen Beobachtungen exponentiell mit
2 s Zeitkonstante ab. Die Regel bezieht ihre Information ausschließlich
aus den nativen Spikes und Integrationsbatches. Sie erhält weder
Zuordnungslabels noch Rewards oder Probe-Ergebnisse.

Drei Regeln wählen unter identischen lokalen Voraussetzungen eine
Quelle. `recent` wählt wie bisher nach letzter Aktivität. `lead`
priorisiert den akkumulierten Wert. `lagged` berechnet denselben Wert
mit dem um 157 ms verzögerten Verlauf der Quellspikes. Dabei bleibt
die Reihenfolge der Quellspikes erhalten und der zeitliche Bezug
zum Zielbatch ändert sich. Beide neuen Arme sammeln in den ersten
157 ms noch keine Punkte. Bei Gleichstand gilt die bestehende
Reihenfolge nach jüngster Aktivität, Abstand und Zell-ID.

Alle drei Regeln behalten native Bildung, Lernregeln und lokale
Homöostase bei. Falls die native Regel keine Kante vorsieht, können
sie in derselben lokalen Nachbarschaft aus zuletzt aktiven erregenden
Quellen wählen. Zielberechtigung, Gewichte, Verzögerungen und die
Obergrenze von einer explorativen Kante pro Ziel über A und B sind
gleich. Pruning ist wie im vorigen A/B-Protokoll ausgesetzt. Die
Vergleiche messen tatsächliche Bildungszahlen separat. Rückkopplung
nach einer anderen Quellenwahl kann spätere Bildungsgelegenheiten
verändern.

## Unabhängige Reize und gekreuzte Verläufe

Sechs neue Seeds sind festgelegt. Die 8×8-Bilder der Seeds 34409,
34421 und 34429 werden um 90 Grad im Uhrzeigersinn gedreht.
Auf 34439, 34447 und 34459 werden die Spalten gespiegelt. Ihre
Zeilen werden von 0 bis 7 auf 0, 2, 4, 6, 1, 3, 5 und 7 abgebildet.
Die Umformungen sind fest und erhalten Bildenergie und Disjunktheit
aller acht Cue- und Folgebilder. [Konfiguration](config/visual_contingency_lead_growth_20261124.toml)

Jede Regel startet aus demselben entwickelten Zustand des Seeds. Sie
läuft während A und B. A wird vorwärts, umgekehrt und balanciert
trainiert. Von jeder dieser drei Historien zweigen am exakten A-Ende
alle drei B-Zuordnungen ab. Gewichte, laufende Ereignisse und lokale
Evidenz gehen unverändert von A nach B über. Eingaben, Wiederholungen
und Reizzeiten sind relativ zur jeweiligen A/B-Phase für alle Regeln
gepaart. Die Probe nutzt getrennte eingefrorene Kopien und dieselbe
elfsekündige Ruhe wie der vorige A/B-Test.

## Festgelegte Auswertung

Das ursprüngliche Erwerbskriterium vergleicht für jeden Cue den
passenden Zielpool mit dem falschen, dem Ausgangszustand und den
umgekehrten und balancierten Zuordnungen. Die Leerprobe und die
Aktivität bis zum Cue bleiben im vereinbarten Ruhebereich. Für B
wird zusätzlich gegen den Zustand nach A verglichen. Der Erhaltswert
bezieht sich auf dieselbe A-Zuordnung vor und nach B. Ein vollständiger
Pfad verlangt beide erworbenen A-Cues, beide weiter abrufbaren A-Cues
und beide erworbenen B-Cues im jeweiligen Zweig. Vergleiche ohne
auswertbaren Zielpool bleiben sichtbar.

Für jede Regel werden Erwerb A, Erwerb B, bedingter A-Erhalt und
vollständige Pfade ausgewiesen. `lead` wird fallgepaart mit `recent`
und `lagged` verglichen. Als technische Prüfung werden Änderungen
der Quellenwahl gegenüber der reinen Aktualitätsentscheidung
innerhalb desselben Entwicklungsbatches gezählt. Die Rohdaten
enthalten pro explorativer Kante die gewählte Quelle, die
Aktualitätsalternative und deren Evidenzwerte. Ein Vorteil gegenüber
beiden Kontrollen stützt die Hypothese, dass der zeitliche Vorlauf
für die Bildung nützlich ist. Entscheidungen über einen allgemeinen
Lernfortschritt hängen vom Erwerb und Erhalt auf unabhängigen
Netzrealisierungen ab.

## Reproduktion

Der Runner wiederholt jeden Record vollständig. Eine unabhängige
Auswertung prüft Saatwerte, Bilder, Root-Ereignisse, Topologie,
Auswahlnachweise und Ziel-Spikes. Vor der Ausführung werden
Prüfsummen für Konfiguration, Quellen und Werkzeugkette festgehalten.

```bash
config=research/config/visual_contingency_lead_growth_20261124.toml
root=research/results/visual_contingency_lead_growth_20261124
research/run.sh run "$config" --output "$root"
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_lead_growth_analysis.py "$root"
```
