# Kante 87→93 trägt den B-Zielspike auf Seed 34531

Die Blockade von 87→93 löschte den B-Zielspike in **4 von 4**
erfolgreichen eingefrorenen Trägern. Der reziproke Ersatz von
100→93 durch 87→93 erzeugte ihn in **20 von 20** stummen
Trägern bei unverändertem Gewicht und unveränderter Verzögerung
des Eingangsplatzes. Darunter waren alle 16 balanciert und
umgekehrt trainierten B-Zweige. Die während B gebildete Kante
trägt damit auf dieser Netzrealisierung kausal den einzelnen
Abrufspike. Der Quellentausch kann die Antwort auch ohne
passende B-Trainingshistorie herstellen.
[Versuchsplan](CONTINGENCY_B_SOURCE.md)

## Eingriff und abrufbare Zuordnung

| B-Zuordnung | Intakt mit Zielspike | Verlust durch Blockade | Zuwachs durch 87→93 |
| --- | ---: | ---: | ---: |
| Vorwärts | 4 / 8 | 4 / 4 | 4 / 4 |
| Balanciert | 0 / 8 | — | 8 / 8 |
| Umgekehrt | 0 / 8 | — | 8 / 8 |

Die vier intakten Antworten stammen aus zwei A-Historien und
den beiden Vorlaufregeln. Der Ersatz durch 100→93 löschte sie
ebenfalls. Die vier geretteten Vorwärtszweige erfüllten nach
dem Tausch das ursprüngliche B-Kriterium, solange die
Vergleichszweige intakt blieben. Mit demselben Quellentausch
in den Vergleichszweigen fiel dieser Vorteil weg.
`cue_b1` und Blank lösten in keinem der 96 Eingriffsarme
einen Spike in Zelle 93 aus.

Die Ortskontrollen blieben in allen Trägern stumm. Unter den
zulässigen zusätzlichen erregenden Quellen lag jeweils keine
Zelle mit einem Spike in der unveränderten `cue_b0`-Probe.
Die eingesetzten Ortskontrollen prüften daher Gewicht,
Verzögerung und Einbau eines lokalen Eingangs. Ob ein anderer
aktiver Eingang bei identischen Ankünften die gleiche Wirkung
hätte, bleibt offen.

## Elektrischer Beitrag

Alle 24 Antworten mit 87→93 traten bei **36,335 ms** auf.
Der Eingang lieferte zuvor oder gleichzeitig zwei erregende
Ankünfte. Die erste lag stets bei **19,668 ms**. Die zweite
lag 23-mal bei **35,335 ms** und einmal direkt im auslösenden
Batch bei **36,335 ms**. Alle übrigen Ankünfte an Zelle 93
bis einschließlich dieses Batches und alle früheren
Core-Spikes stimmten mit dem jeweiligen blockierten Träger
exakt überein. Das lokalisiert den Unterschied vor dem
Zielspike auf die beiden zusätzlichen Ankünfte.

Im auslösenden Batch lag das Potential einschließlich
Nettoinput bei aktiver 87→93-Kante 0,01433 bis 0,02804
über der Schwelle. Die gepaarten blockierten Zustände lagen
0,29347 bis 0,29494 darunter. Die Ankünfte und das
Überschreiten der Schwelle sind im Rohprotokoll getrennt
ausgewiesen.

## Quellenwahl während des Lernens

In allen acht Vorwärtszweigen entstand der Eingang bei
derselben Zeit nach B-Beginn, bei **3.099,793 ms**. Es gab
drei zuletzt aktive Kandidaten. Die jüngsten Spikes von
87 und 100 lagen jeweils 22,792 bis 23,792 ms zurück.
Die Aktualitätsalternative war durchgehend 100. Beide
Vorlaufregeln wählten 87, die zeitversetzte Regel blieb
bei 100. In den Kontrollzuordnungen bildete die native
Regel 100→93. Damit lokalisiert der Versuch die in diesem
Fall wirksame Unterscheidung in der Quellenwahl beim
Training. Die nachträgliche Wahl nur beim Abruf stellt
die Antwort auch auf den anderen B-Historien her.

Die Kausaldiagnose ist auf einen bekannten Seed und eine
Zielzelle beschränkt. Der nächste prospektive Test sollte auf
unabhängigen Seeds kreuzen, ob die erste explorative
Kantenbildung sofort oder erst nach einer festen Phase
lokaler Evidenzsammlung erlaubt ist. Die Aktualitätsregel
mit demselben Aufschub kontrolliert den geänderten
Bildungszeitpunkt. So lässt sich prüfen, ob sich der
beobachtete Beitrag der Vorlaufwahl wiederholt, wenn sie
mehr der früh entstehenden Kanten beeinflussen kann.

## Reproduktion

Die erste technische Ausführung brach vor einer Rohdatenausgabe
ab, da sie auch in den Kontrollzuordnungen eine explorative
Kante voraussetzte. Die gegen alle 24 Archivzweige geprüfte
Korrektur `0dd6505` war vor dem ausgewerteten Lauf veröffentlicht.
Zwei native Ausführungen und ein Offline-Neubau aus den
versiegelten Quellen lieferten bytegleiche Rohdaten. Ein
unabhängiger Prüfer rekonstruierte Ursprung, exakte
Topologieänderungen, Root-Ereignisse, elektrische Zielspuren
und B-Abrufkriterien. Alle 72 unveränderten Proben
reproduzierten das Archiv. Der frühere A-Ankunftszeit-Record
blieb bytegleich. Der Projektcheck bestand 487 Rust-,
168 Harness-Python- und 10 Dashboard-Tests.
[Kompakte Daten](dokumentation/visual_contingency_b_source_20261124.json)

Das lokale Archiv enthält den Originalrecord, versiegelte
Quellen, Binary, alle drei Replays und abgeleitete Auswertungen.
`research/results/archives/visual_contingency_b_source_20261124.zip`

```bash
root=research/results/visual_contingency_b_source_20261124
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
cmp "$root/source.json" "$root/rebuilt.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_b_source_analysis.py" "$root/input.json" "$root/source.json" --replay "$root/replay.json" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/formation_index.py" "$root/input.json" | cmp - "$root/formation-index.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/compact_report.py" "$root" | cmp - "$root/compact-index.json"
sha256sum -c research/results/archives/visual_contingency_b_source_20261124.zip.sha256
```
