# Visuelle Kontingenz auf entwickelter Struktur

MLD-03 fand bei natürlich gelernten Gewichten trotz veränderter Core-Erregbarkeit
keine Änderung der Cue-Spikefolgen. In den bisherigen Bildtests enthielten
die Vollbilder jeweils auch ihre Cue-Pixel. Hier testen wir, ob ein Cue
eine räumlich getrennte, zeitlich folgende Seheingabe aus der Erfahrung
vorhersagt und ob rekurrente Gewichte diese Zuordnung tragen.

## Versuchsplan

Pro Seed startet jede Trainings- und Abrufinstanz am gleichen vollständig
verglichenen MLD-02-Checkpoint. Dieser entsteht durch eine Sekunde native
Entwicklung mit Lernplastizität und Homöostase. Die vier neuen Hauptbilder
verwenden je acht helle, paarweise disjunkte Pixel. Ihre Pixel sind außerdem
von den aktiven Entwicklungsbildern disjunkt. Das bestehende Pixelmodell
verwendet die native deterministische Ratekodierung bei 120 Hz Maximalrate.
Im Training bleibt die akkumulierte Drei-Faktor-STDP aktiv. Entwicklung,
Homöostase, Konkurrenz und Aktionsrückkopplung sind ausgeschaltet.

Je Trainingsarm folgen 16 Trials mit jeweils fünf 50-ms-Fenstern. Cue 0
und Cue 1 kommen in jeder der acht Wiederholungen einmal vor, abwechselnd
in der Reihenfolge 0/1 und 1/0. In `forward` folgt Cue 0 das Folgebild 0
und Cue 1 das Folgebild 1. In `reverse` wird diese Zuordnung vertauscht.
In `balanced` wechselt die Zuordnung alle zwei Wiederholungen, sodass
jeder Cue gleich oft mit jedem Folgebild verbunden ist. Die drei Arme
haben gleiche marginale Bildexpositionen. Die unabhängige Prüfung zählt
die tatsächlich erzeugten Root-Spikes pro Kanal.

Im primären Arm `immediate` startet das Folgebild 50 ms nach dem Cue-Beginn.
Der vorab festgelegte zweite Arm `gap` beginnt das Folgebild nach 150 ms.
Der Abstand zwischen Cue-Beginnzeiten bleibt in beiden Armen 250 ms.
Dadurch ändern sich mit der Cue-Folgebild-Lücke auch die Ruhezeit nach dem
Folgebild und mögliche Wechselwirkungen mit dem nächsten Trial.
Es werden fünf neue Modellseeds 33207, 33211, 33223, 33247 und 33287
mit 256 Core-Zellen und vollständigem Replay verwendet.

## Abruf und Entscheidung

Jede Probe verwendet eine eigene eingefrorene Instanz desselben entwickelten
Ausgangszustands. `reset` verwendet dessen Gewichte. Die anderen Instanzen
erhalten die unverstärkten sichtbaren Endgewichte jeweils eines Trainingsarms,
einmal nur für Core→Core (`_core`) und einmal für alle Kanten (`_all`).
Der Rücktausch und der Zustandsvergleich prüfen den kausalen Eingriff.
Der Abruf folgt nach elf Sekunden Schwarzbildruhe auf der jeweiligen Instanz.
Das 200-ms-Messfenster enthält 50 ms Reiz und anschließend 150 ms Schwarz.
Das erwartete Folgebild wird nur in einer eigenen eingefrorenen Probe gezeigt.

Für jedes Folgebild bilden wir aus dem Reset-Abruf eine feste Menge von
Core-Zellen. Dafür zählen nur Zellen, die auf dieses Folgebild feuern und weder
auf das jeweils andere Folgebild noch auf die ursprünglichen Cues, die
vier einzelnen Cue-Hälften oder Schwarz reagieren. Die Hälften werden im
Training nie isoliert gezeigt. Die Zielmengen und Zuordnungen sind reine
Auswertung und gehen nicht in die Lernregel ein.

Eine Bedingung besteht das Kriterium für Original-Cues auf einem Seed, wenn
beide Zielmengen besetzt sind und für jeden Cue unter **beiden** trainierten
Zuordnungen folgende Bedingungen gelten:

- Die korrekte Zielmenge feuert öfter als die falsche.
- Sie feuert häufiger als bei demselben Cue unter Reset, der balancierten
  Zuordnung und der gegensätzlichen Zuordnung.
- Schwarz- und Ruheaktivität steigen gegenüber Reset nicht.

Die Cue-Hälften werden mit demselben Kriterium separat ausgewertet. Eine
Replikation des primären Befunds erfordert fünf von fünf bestandene
Core-only-Ergebnisse im Arm `immediate`. Seedweise Treffer und der
zweite zeitliche Arm werden vollständig berichtet. Leere Zielmengen
begrenzen die Auswertbarkeit und werden separat gezählt.

Der Gewichtsübertrag testet, ob sichtbare, persistente Gewichte für
Abrufwirkung ausreichen. Er lässt trainingsspezifische chemische Zustände,
Zellzustände und Kurzzeitspuren am ursprünglichen entwickelten Checkpoint.
Die Studie testet zwei visuelle Zuordnungen und bekannte Bildbestandteile.
Sie untersucht weder Abruf im weiterlaufenden trainierten Actor noch
fortlaufende Anpassung an neue Aufgaben oder autonome Aktionswahl.

## Ausführung und Daten

Vom Verzeichnis `main/NERVA_Modell/`:

```bash
./research/run.sh run research/config/visual_contingency_smoke.toml --output /tmp/nerva-visual-contingency-smoke
./research/run.sh plan research/config/visual_contingency_20261122.toml
./research/run.sh run research/config/visual_contingency_20261122.toml
./research/run.sh report research/config/visual_contingency_20261122.toml
python3 research/verify_contingency.py research/results/visual_contingency_20261122
```

Der Kurzlauf dient der technischen Prüfung. Die Hauptergebnisse, nativen
Spikereihen und Prüfbelege liegen im [lokalen Ergebnisarchiv](ARCHIVING.md).
