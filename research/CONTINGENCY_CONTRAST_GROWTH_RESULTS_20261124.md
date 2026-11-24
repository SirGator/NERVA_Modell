# Vorlauf und Differenz lieferten dieselben zusätzlichen B-Teiltreffer

Auf sechs vorab festgelegten Seeds bestanden die Vorlauf- und
die Differenzregel dieselben **5 von 72** B-Erwerbsvergleichen.
Aktualität und verzögerter Vorlauf bestanden dieselben **3 von 72**.
Die zwei zusätzlichen Treffer betrafen denselben Cue auf Seed 34531
unter zwei A-Historien. Der Abzug der verzögerten Evidenz brachte
keinen zusätzlichen Erwerbs- oder Erhaltsfall. Alle vier Regeln
verfehlten den gemeinsamen Endpunkt auf allen 24 gerichteten Pfaden.
[Versuchsplan](CONTINGENCY_CONTRAST_GROWTH.md)

| Quellenwahl | Explorative Kanten | Andere Quelle als nach Aktualität | A-Erwerb | B-Erwerb | Bedingter A-Erhalt |
| --- | ---: | ---: | ---: | ---: | ---: |
| Jüngste Aktivität | 2.838 | 0 | 1 / 24 | 3 / 72 | 3 / 3 |
| Vorlauf | 2.834 | 137 | 1 / 24 | 5 / 72 | 3 / 3 |
| Verzögerter Vorlauf | 2.838 | 114 | 1 / 24 | 3 / 72 | 3 / 3 |
| Vorlauf minus verzögerter Vorlauf | 2.836 | 154 | 1 / 24 | 5 / 72 | 3 / 3 |

Alle Zielpools waren auswertbar, alle Erwerbsproben erfüllten die
Ruhekriterien. Der einzige A-Erwerbsfall lag auf Seed 34525.
Die Antwort blieb nach allen drei B-Zuordnungen erhalten. Die
drei B-Teiltreffer aller Regeln lagen auf Seed 34501. Damit
fielen erfolgreicher A-Erhalt und erfolgreicher B-Erwerb im
Versuch auf unterschiedliche Netzrealisierungen.

## Welche Quellenwahl sich tatsächlich änderte

Gegenüber der reinen Vorlaufregel änderte die Subtraktion in
53 von 72 gepaarten A- und B-Phasen die Menge der gebildeten
Quell–Ziel-Paare und in 120 von 360 Proben die Core-Spikefolge.
Bei 22 der 154 von Aktualität abweichenden Entscheidungen lag
die beobachtete Komponente der gewählten Quelle auf oder unter
derjenigen der Aktualitätsalternative. Hier kehrte der Abzug
die Reihenfolge dieser beiden Kandidaten um oder löste den
Gleichstand zugunsten der anderen Quelle auf.

Die nachträgliche Entscheidungsbilanz zeigt, dass in A
**1.577 von 2.063** explorativen Kanten innerhalb der 157 ms
Anlaufzeit entstanden. **173** A-Entscheidungen lagen danach
und hatten mindestens zwei Kandidaten. In B waren es **245
von 773**. Auf dieser Datengrundlage betrifft der Vergleich
eine Bildungsregel mit früh gebundenen Quellen. Die feste
Verzögerung definiert eine lokale Referenz im Modell. Aus dem
Versuch folgt keine biologische Validierung dieser Zeitkonstante.

## Gleicher Zielspike über zwei A-Historien

Bei Seed 34531 löste `cue_b0` nach vorwärts trainiertem B in
beiden Vorlaufarmen einen Spike in Zelle 93 nach **36,335 ms**
aus. Das galt nach balanciertem und nach vorwärts trainiertem A.
In allen vier betreffenden Zweigen entstand die explorative
Kante **87→93** während B und lieferte beim Abruf eine Ankunft.
Bei der Aktualitäts- und der verzögerten Regel entstand in
den gepaarten Zweigen stattdessen **100→93**. Diese Kante
lieferte dort beim Abruf keine Ankunft. Dasselbe galt für
die umgekehrte und balancierte B-Zuordnung beider Vorlaufarme.
Die beiden erfolgreichen B-Fälle gehen auf dieselbe Zelle und
denselben Seed zurück. Die Netzverläufe enthalten weitere
Unterschiede, sodass der Beitrag der Kante offen bleibt.

Der nächste kausale Test ist eine gezielte Deaktivierung
von 87→93 auf den eingefrorenen erfolgreichen B-Trägern und
ein dosengleicher Tausch der Quelle 100 gegen 87 auf den
gepaarten Kontrollträgern. Unveränderte Replays, gleich
starke lokale Ersatzquellen und Ankünfte an Zelle 93 prüfen,
ob die neue Kante die B-Antwort trägt. Eine Replikation des
Erwerbsvorteils braucht danach unabhängige Netzrealisierungen.

## Reproduktion

Das Modell `1ca4dd4` und die Library `c13d9f7` waren vor dem
Hauptlauf veröffentlicht. Alle sechs Records wurden vollständig
wiederholt und unabhängig ausgewertet. Ein Offline-Neubau aus
den versiegelten Quellen reproduzierte das Manifest und alle
deterministischen Felder der beiden Kurzläufe. Die früheren
Lead-Felder blieben exakt und der frühere A-Ankunftszeit-Record
war bytegleich. Der Projektcheck bestand 487 Rust-, 164
Harness-Python- und 10 Dashboard-Tests.
[Kompakte Daten](dokumentation/visual_contingency_contrast_growth_20261124.json)

Das lokale ZIP enthält Rohdaten, Eingaben, Quellen, Binary,
unabhängige Auswertung und das Skript für den kompakten Index.
`research/results/archives/visual_contingency_contrast_growth_20261124.zip`

```bash
root=research/results/visual_contingency_contrast_growth_20261124
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_contrast_growth_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/compact_report.py" "$root" | cmp - "$root/compact-index.json"
sha256sum -c research/results/archives/visual_contingency_contrast_growth_20261124.zip.sha256
```
