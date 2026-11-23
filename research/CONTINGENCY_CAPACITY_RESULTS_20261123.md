# Selektiver rekurrenter Abruf ist auf dem bestehenden Graphen möglich

Die künstlich verstärkte Relaisreferenz erzeugte auf **einem von fünf neuen
Seeds** einen selektiven Abruf für beide Cues unter beiden gegensätzlichen
Zuordnungen. Sie verwendete nur vorhandene rekurrente Kanten. Auf vier Seeds
erreichte mindestens eine Referenz einen Teiltreffer. Im nativen Training
bestand keiner der anschließend geprüften vier Seeds einen Cue-Vergleich.
[Vorregistrierter Versuchsplan](CONTINGENCY_CAPACITY.md)

## Abruf auf fünf neuen Netzen

| Seed | Zielgruppengrößen | Direkt halb | Direkt voll | Relais halb | Relais voll | Cue-Hälften der gewählten Referenz |
| --- | --- | --- | --- | --- | --- | --- |
| 34123 | 12 / 5 | 2 / 4 | 2 / 4 | 3 / 4 | **4 / 4** | 6 / 8 |
| 34127 | 5 / 5 | 0 / 4 | 1 / 4 | 0 / 4 | 1 / 4 | 1 / 8 |
| 34141 | 5 / 8 | 2 / 4 | 2 / 4 | 2 / 4 | 2 / 4 | 1 / 8 |
| 34147 | 6 / 5 | 0 / 4 | 0 / 4 | 0 / 4 | 0 / 4 | – |
| 34157 | 4 / 10 | 2 / 4 | 2 / 4 | 2 / 4 | 2 / 4 | 1 / 8 |

Jede Zelle zählt bestandene Cue–Zuordnungs-Kombinationen. Die Zielgruppen
und Referenzkarten wurden aus Reset-Proben mit den vollständigen Bildern
gebildet. Die vier Cue-Hälften wurden erst nach Wahl der Referenz geprüft.
Die Referenz setzte nur plastische, erregende Gewichte auf dem bestehenden
Core-Graphen und wurde nach elf Sekunden Ruhe abgerufen. Ihre vorgegebene
Zuordnung belegt die elektrische Kapazität der getesteten Schaltung.
Der Versuch liefert damit keinen Nachweis eines selbst erworbenen Gedächtnisses.

Beim besten Seed 34123 erzeugte der volle Relaisarm in den vier Fällen
6, 12, 2 und 6 korrekte Zielspikes. Das Zurücksetzen nur der zum jeweiligen
Pfad gehörenden terminalen Gewichte reduzierte diese Zählungen auf
0, 0, 0 und 1. Über alle ausgewählten Teiltreffer sank die korrekte
Spikezahl in **9 von 9** terminalen Eingriffen, davon acht auf null.
Das ist eine gemeinsame Wirkung der zurückgesetzten Gewichte je Fall.
Die Notwendigkeit einzelner Kanten wurde nicht getestet.

Die nachträgliche Pfadauswertung fand im vollen Relaisarm in **9 von 20**
Fällen eine für die jeweilige Zuordnung ausgewählte terminale Kante.
In allen neun Fällen lieferte sie tatsächlich mindestens einen Impuls und
der Referenzabruf war selektiv. Vier Fälle enthielten Ankünfte von zuvor
ruhigen Relaiszellen. In den anderen elf Fällen blieb die Auswahl terminaler
Kanten leer. Die Zählung beschreibt die feste Regel mit cue-exklusiver
Ankunft und höchstens einer Relaiszelle. Andere Pfade und Eingriffe bleiben
offen.

## Native Lernrichtung und Vergleichsnormen

Der native Übertrag bestand auf den vier weiter untersuchten Seeds
0 von 16 Cue–Zuordnungs-Vergleichen. Drei Referenzen konnten bis zur
nativen Änderungsnorm abgeschwächt werden und bestanden dort ebenfalls
keinen Vergleich. Der Gegenversuch mit verstärkter Lernrichtung bei
Referenznorm blieb in allen vier Fällen unauswertbar. Die Gewichtsgrenzen
verhinderten bei mindestens einer Zuordnung das richtungstreue Hochskalieren.
Auf 34123 lagen die Referenznormen bei 5,82 und 4,95, die zulässigen
Obergrenzen entlang der gelernten Richtungen bei 0,367 und 0,361.
Der Unterschied zwischen Richtung und Stärke wurde damit bei der
funktionalen Referenzstärke noch nicht isoliert.

Auf 34123 hatten die präsynaptischen Zellen von **8 der 11** terminalen
Kanten in der jeweils zugehörigen nativen Trainingsaufzeichnung keinen
Spike. Ihre Gewichte änderten sich dennoch. Die Aufzeichnung enthält
Core-Spikes pro Trainingsfenster. Verzögerte Ankünfte aus der Entwicklung,
bereits vorhandene Eligibility und neu angelegte Paarungen wurden hier
nicht getrennt. Der nächste kausale Test beobachtet genau diese Pfade und
spielt dieselbe Trainingshistorie mit und ohne übernommene Eligibility
ab. Damit lässt sich prüfen, ob die nativen Updates für die später
benötigten Relais hauptsächlich aus der Entwicklung stammen.

## Rohdaten und Reproduktion

Fünf Records und zehn Ausführungen einschließlich vollständigem Replay.
Unabhängig geprüft wurden Reizfolgen, installierte Gewichte, Sollnormen,
Routenauswahl, Spike-Kriterien, Cue-Fragmente und terminale Eingriffe.
Mit beiden archivierten Quellständen ließ sich das Manifest bytegenau
und der deterministische Kurzlauf exakt reproduzieren. Die Projektprüfung
bestand mit 463 Rust-, 97 Harness-Python- und 10 Dashboard-Tests sowie
Rustformat, Clippy und Linkprüfung. [Maschinenlesbare Daten und Record-Hashes](dokumentation/visual_contingency_capacity_20261123.json)

Modellstand `0c30cee` und Library `cf4272e` waren zu Messbeginn veröffentlicht.
Das lokal geprüfte ZIP liegt unter
`research/results/archives/visual_contingency_capacity_20261123.zip`.
Es enthält die vollständigen Messdaten, das Binary, beide Quellarchive,
die getrennten Seed-Pläne und die unabhängige Auswertung.

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
root=research/results/visual_contingency_capacity_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
./research/run.sh report research/config/visual_contingency_capacity_20261123.toml --binary "$root/nerva-research"
python3 "$root/contingency_capacity_analysis.py" "$root"
python3 "$root/describe.py" "$root"
sha256sum -c research/results/archives/visual_contingency_capacity_20261123.zip.sha256
```
