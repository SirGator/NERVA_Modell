# Neue Kanten tragen den einzelnen B-Abruf

Auf Seed 34223 verlor der bekannte B-Teiltreffer beide
Zielspikes, sobald die während B entstandenen Eingänge
in die B-Zielpools gesperrt wurden. Der Reset der 393
älteren Gewichte auf den Stand direkt vor B erhielt beide
Spikezeiten exakt. Auf diesem eingefrorenen Zustand war
die Übertragung über die neuen Zieleingänge für den
Abruf nötig. [Versuchsplan](CONTINGENCY_B_FACTOR.md)

## Getrennte Eingriffe am gleichen Zustand

Die Eingriffe setzten direkt vor `cue_b0` ein, nach elf
Sekunden unveränderter Ruhe und nach demselben A/B-Training.
Im Umkehrzweig feuerte Zelle 94 nach 27,001 und 52,001 ms
im passenden B-Zielpool. Das Sperren der neuen Kanten schloss
auch bereits eingeplante Ankünfte ein. Der Reset älterer
Gewichte behielt bereits eingeplante Amplituden bei.

| Eingriff auf Seed 34223, Umkehrzuordnung | Gesperrte Kanten | Zurückgesetzte Gewichte | Passende Zielspikes | Alle Core-Spikes | B-Kriterium |
| --- | ---: | ---: | ---: | ---: | ---: |
| Ausgangszustand nach B | 0 | 0 | 2 | 26 | erfüllt |
| Ältere Gewichte zurücksetzen | 0 | 393 | 2 | 26 | erfüllt |
| Alle während B gebildeten Kanten sperren | 51 | 0 | 0 | 25 | verfehlt |
| Beide Eingriffe | 51 | 393 | 0 | 25 | verfehlt |
| Neue Kanten in die Zielpools sperren | 16 | 0 | 0 | 24 | verfehlt |
| Übrige neue Kanten sperren | 35 | 0 | 2 | 27 | erfüllt |

Im unbehandelten Zweig trugen vier der 16 neuen
Zieleingänge mindestens eine Ankunft in der Probe bei.
Drei führten in die Zelle 94. Die Blockade der übrigen
35 Kanten ließ beide Zielspikes bei identischen Zeiten
bestehen. Die Blank-Probe blieb unter allen Bedingungen
still. Beide Strukturblockaden, die den Treffer aufhoben,
ahmen der Zielantwort selbst zwei Spikes. Die Änderung
des B-Kriteriums beruht damit auf der Antwort des
passenden Pools.

## Reichweite und nächste Hypothese

Über die vier archivierten Seeds hinweg waren je Bedingung
16 gerichtete Cue–Zuordnungs-Vergleiche auswertbar. In vier
Vergleichen gab der passende Pool im unveränderten Zustand
mindestens einen Spike ab. Das Sperren aller neuen Kanten
und das Sperren allein der neuen Zieleingänge unterdrückten
alle diese Antworten. Die Zielspikezahlen blieben in allen
16 Vergleichen gleich, wenn ältere Gewichte zurückgesetzt
oder nur die übrigen neuen Kanten gesperrt wurden. Beim
Reset älterer Gewichte blieben auch die vollständigen
Core-Spikefolgen aller 36 Proben identisch. Unter jeder
Bedingung lag die Zahl selektiver Treffer bei null oder
eins. Nur der bereits bekannte Fall auf Seed 34223 erfüllte
das Kriterium, und zwar in drei Bedingungen.

Der Eingriff prüft die Übertragung beim Abruf nach dem
Training. Ob die Strukturbildung die Zuordnung während B
speichert oder einen vorhandenen Unterschied erst lesbar
macht, bleibt offen. Eine prospektive Prüfung kann nach
derselben A-Historie auf neuen Seeds drei B-Zuordnungen
verzweigen. Drei Bedingungen während B vergleichen die
Bildung nach jüngster Aktivität, räumliche Kandidatenauswahl
und ausgesetzte Bildung. Paarregel, Modulation und
Core-Homöostase bleiben dabei aktiv. Der unabhängige
Abruf von A bleibt Voraussetzung für eine Aussage über
Gedächtniserhalt.

## Reproduktion

Alle vier nativen Eingangsrecords wurden aus dem versiegelten Archiv
rekonstruiert. Der Runner wiederholte jeden vollständigen Ausgabe-Record
intern. Der unabhängige Prüfer rekonstruierte Eingriffsmengen, Endgewichte,
Schaltzustände und Erfolgskriterien. Der Offline-Neubau aus den archivierten
Quellen reproduzierte den technischen Kurzlauf bytegenau. Modell `ea82ac8`
und Library `9d60220` waren vor dem Hauptlauf veröffentlicht.
Der Projektcheck bestand 474 Rust-, 129 Harness-Python- und 10 Dashboard-Tests.
[Kompakte Daten](dokumentation/visual_contingency_b_factor_20261123.json)

Das lokale ZIP enthält versiegelte Eingaben, reproduzierte Rohdaten,
Quellen, Binary und beide Analysen.
`research/results/archives/visual_contingency_b_factor_20261123.zip`

```bash
root=research/results/visual_contingency_b_factor_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_b_factor_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/descriptive_index.py" "$root" | cmp - "$root/descriptive-index.json"
sha256sum -c research/results/archives/visual_contingency_b_factor_20261123.zip.sha256
```
