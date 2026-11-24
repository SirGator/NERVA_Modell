# Vorlaufbasierte Kantenwahl änderte das Netz ohne selektiven Erwerb

Auf sechs neuen Seeds mit zwei neuen Bildlayouts bestand jede Regel
**0 von 24** auswertbaren A-Erwerbsvergleichen und **0 von 72** B-Vergleichen.
Die Vorlaufregel wählte bei **92 von 2.807** explorativen Entscheidungen
eine andere Quelle als die Aktualitätswahl im selben Zustand. Der Eingriff
änderte damit die Netzentwicklung. Ein Vorteil bei selektivem Erwerb fiel
unter den geprüften Bedingungen aus.
[Versuchsplan](CONTINGENCY_LEAD_GROWTH.md)

## Gepaarte Ergebnisse

| Quellenwahl | Neue Kanten insgesamt | Explorative Kanten | Abweichende Quellenwahl | A bestanden | B bestanden |
| --- | ---: | ---: | ---: | ---: | ---: |
| Jüngste Aktivität | 8.297 | 2.806 | — | 0 / 24 | 0 / 72 |
| Vorlauf-Evidenz | 8.288 | 2.807 | 92 | 0 / 24 | 0 / 72 |
| Um 157 ms verzögerte Evidenz | 8.290 | 2.804 | 47 | 0 / 24 | 0 / 72 |

Die Zählung der Kanten umfasst die 18 A- und 54 B-Phasen je Regel.
Abweichende Quellenwahl bedeutet, dass die berechnete Evidenz im jeweiligen
Entwicklungsbatch eine höher bewertete Quelle als die reine Aktualitätswahl
ergab. In 347 Vorlauf- und 236 verzögerten Entscheidungen war die Evidenz
der gewählten Quelle positiv. Die Vorlaufregel änderte gegenüber der
reinen Aktualitätswahl in **52 von 72** gepaarten Phasen die Menge neu
gebildeter Quell–Ziel-Paare. In 52 Phasen änderten sich die Core-Spikefolgen
im Training. Von 360 gepaarten Proben wichen 133 Core-Spikefolgen ab.
Gegenüber der verzögerten Kontrolle unterschieden sich die neu gebildeten
Paare in 51 Phasen und die Core-Spikefolgen in 120 Proben. Die tatsächliche
Zahl der Kanten blieb in beiden Vergleichen ähnlich.

## Was die Zielantworten begrenzte

Alle Zielpools waren auswertbar. Sämtliche Vergleiche erfüllten die
Ruhebedingungen. Bei jeder Regel gab es vier A- und acht B-Vergleiche
mit einem Spike im passenden Zielpool. Jede dieser Antworten entsprach
in der Zahl der Zielspikes der zugehörigen balancierten Reizkontrolle.
In B lagen drei der acht Antworten über dem jeweiligen Vor-B-Zustand.
Die umgekehrte und die balancierte B-Zuordnung erreichten dabei dieselbe
Zahl. Die Reizexposition und die laufende Adaptation reichten also für
vereinzelte Zielantworten. Die Antwort auf die gerichtete Zuordnung ging
in keinem Fall über die balancierte Exposition hinaus.

Der fehlende A-Erwerb ließ pro Regel **0 auswertbare Erhaltsfälle**.
Der gemeinsame Endpunkt aus A- und B-Erwerb und anschließendem A-Erhalt
wurde auf allen 24 gerichteten Pfaden je Regel verfehlt. Diese Resultate
betreffen den festen Score, die beiden Bildlayouts, die sechs Seeds und
die aktuelle Bildungsbegrenzung. Pro Seed teilen sich mehrere B-Zweige
dieselbe A-Historie.

Der nächste Versuch kann pro Kandidatenpaar die beobachtete Vorlauf-Evidenz
und eine zeitversetzte Referenz im selben Netz akkumulieren. Die
Quellenwahl nach ihrer Differenz prüft, ob sich wiederkehrender Vorlauf
von durch gemeinsame Reizexposition entstehender Aktivität abheben lässt.
Die bisherige zeitversetzte Regel bleibt dafür eine getrennte Kontrolle.
Ein neuer Satz von Seeds und Bildern bleibt für den Erwerbsvergleich nötig.

## Reproduktion

Der Runner wiederholte alle sechs Records vollständig. Die unabhängige
Auswertung erzeugte die Bildlayouts erneut und prüfte Reizereignisse,
Kantenverläufe, die protokollierten Evidenzvergleiche und die
Abrufkriterien. Zwei Offline-Kurzläufe aus den versiegelten Quellen
reproduzierten alle deterministischen Record-Felder. Das frühere
A-Zeitreplay blieb bytegleich. Der Projektcheck bestand 486 Rust-,
160 Harness-Python- und 10 Dashboard-Tests. Modell `3b97d29` und
Library `c13d9f7` waren vor dem Hauptlauf veröffentlicht.
[Kompakte Daten](dokumentation/visual_contingency_lead_growth_20261124.json)

Das lokale ZIP enthält Protokoll, Spezifikationen, Binary, Quellen,
Rohdaten und Auswertungen.
`research/results/archives/visual_contingency_lead_growth_20261124.zip`

```bash
root=research/results/visual_contingency_lead_growth_20261124
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_lead_growth_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/descriptive_index.py" "$root" | cmp - "$root/descriptive-index.json"
sha256sum -c research/results/archives/visual_contingency_lead_growth_20261124.zip.sha256
```
