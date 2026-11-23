# Diagnostische Abrufkapazität des rekurrenten Graphen

Im Kontingenzversuch konnte der Wechsel des lokalen Modulationsfaktors
die rekurrente L2-Änderungsnorm um das 81- bis 406-Fache vergrößern. Die
Core-Spikefolgen blieben gleich. Dieser Versuch prüft, ob dieselbe
Modellarchitektur für die räumlich getrennten Folgebilder eine geeignete
rekurrente Route bereitstellt. [Ausgangsbefund](CONTINGENCY_CHEMICAL_RESULTS_20261123.md)

## Konstruktion vor der Abrufprüfung

Alle fünf neuen Seeds 34123, 34127, 34141, 34147 und 34157 verwenden
je 256 Core-Zellen und denselben Entwicklungsablauf wie die Kontingenz.
Die künstlichen Referenzen verwenden ausschließlich den eingefrorenen
entwickelten Zustand und dessen Reset-Proben. Jede Probe beginnt separat
nach elf Sekunden Schwarzbildruhe. Die beiden Original-Cues, beide
Folgebilder und Schwarz werden jeweils 50 ms gezeigt. Gezählt wird das
gesamte 200-ms-Fenster einschließlich anschließender 150 ms Schwarz.

Die Zielgruppe für ein Folgebild besteht aus Core-Zellen, die allein
auf dieses Folgebild feuern. Zellen mit Spikes auf einem Original-Cue,
Schwarz oder dem anderen Folgebild werden ausgeschlossen. Cue-Hälften
gehen weder in die Wahl der Zielzellen noch in die Routenkonstruktion ein.

Es werden zwei Gewichtskarten für entgegengesetzte Zuordnungen konstruiert.
Der direkte Pfad nutzt aktivierte, plastische erregende Core→Core-Kanten
unterhalb der Obergrenze mit einer Nichtnull-Ankunft beim jeweiligen Cue.
Die Kante darf bei Schwarz und beim anderen Cue keine Nichtnull-Ankunft
haben und muss in der passenden Zielgruppe enden. Der Relaisarm fügt
Ketten mit genau einer erregenden, auf beide Cues und Schwarz stillen
Relaiszelle außerhalb beider Zielgruppen hinzu. Die erste Kante hat
dieselben Exklusivitätsanforderungen wie im direkten Pfad. Die zweite
Kante muss im bestehenden Graphen plastisch, aktiviert, erregend und
unterhalb der Obergrenze sein und in der Zielgruppe enden.

Alle ausgewählten Kanten werden einmal pro Karte verstärkt. Bei halber
Dosis wird die Hälfte des Wegs von ihrem nativen Gewicht zu 2,0
zurückgelegt. Bei voller Dosis wird die Grenze 2,0 erreicht. Keine
neue Kante, direkte Zielstimulation, sensorische oder intrinsische
Änderung ist Teil der Referenz. Die Konstruktion nutzt Zielgruppen nur
für die diagnostische Gewichtskarte. Die native Lernregel erhält
weiterhin allein die zeitliche Folge der Bildreize.

## Kontrollen und Entscheidung

Alle vier Referenzbedingungen werden für beide Zuordnungen und Cues
ausgeführt. Eine der vier Zuordnungs/Cue-Kombinationen besteht, wenn
die Zahl der Spikes in der passenden Zielgruppe zugleich größer ist
als in der falschen Gruppe und in der passenden Gruppe beim Reset
und beim entgegengesetzt zugeordneten Arm. Schwarz- und Ruheaktivität
dürfen nicht steigen. Vier erfüllte Kombinationen belegen einen
zweiseitig selektiven Referenzabruf auf dem bestehenden Graphen.
Teiltreffer werden getrennt ausgewiesen.

Für die Folgediagnose wird die Referenz mit den meisten erfüllten
Kombinationen verwendet. Bei Gleichstand gilt die feste Reihenfolge
direkt halb, direkt voll, Relais halb, Relais voll. Bei null erfüllten
Kombinationen entfällt die Folgediagnose. Diese Auswahl dient der
technischen Kalibrierung und verwendet ausschließlich Original-Cues.

Beide Zuordnungen werden im Erfolgs- oder Teiltrefferfall zusätzlich
nativ mit acht Wiederholungen und 50 ms Abstand zwischen Reizanfängen
trainiert. Danach werden ausschließlich sichtbare Core→Core-Gewichte
auf eingefrorene Kopien übertragen. Die gelernte Richtung wird je
Zuordnung zum Vergleich auf die L2-Norm der Referenz verstärkt.
Umgekehrt wird die Referenzrichtung auf die native L2-Norm abgeschwächt.
Die veröffentlichte Norm darf höchstens 1 % vom Soll abweichen, bei
einem Richtungskosinus von mindestens 0,999. Unzulässige Vergleiche
werden ausgewiesen. Ein Erfolg künstlich verstärkter Gewichte zählt
als Diagnose der Richtung und bleibt vom nativen Lernergebnis getrennt.

Für jede ausgewählte positive Kombination werden die für ihren Pfad
verstärkten terminalen Kanten in die passende Zielgruppe auf die
Reset-Gewichte zurückgesetzt. Die Zahl der Zielspikes wird erneut
gemessen. Die vier Cue-Hälften werden erst jetzt geprüft. Ihre
Kontrollen verwenden dieselben Zielgruppen und jeweils ihren eigenen
Reset. Ein positiver Befund betrifft den getesteten künstlichen Pfad.
Ein negativer Befund erfasst die beiden Routenkonstruktionen, Dosen
und Zielgruppen, die in diesem Protokoll geprüft wurden.

## Ausführung und Daten

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
./research/run.sh run research/config/visual_contingency_capacity_smoke.toml --output /tmp/nerva-capacity-smoke
./research/run.sh plan research/config/visual_contingency_capacity_20261123.toml
./research/run.sh run research/config/visual_contingency_capacity_20261123.toml
./research/run.sh report research/config/visual_contingency_capacity_20261123.toml
python3 research/contingency_capacity_analysis.py research/results/visual_contingency_capacity_20261123
```

Jeder Seed wird vollständig wiederholt. Vor Ausführung werden die
Konfiguration, der Plan, Quellstände und Runner per SHA-256 gebunden.
Die unabhängige Prüfung rekonstruiert die Routen und Erfolgskriterien
aus den Rohdaten. Der Lauf erfordert keine VM.
