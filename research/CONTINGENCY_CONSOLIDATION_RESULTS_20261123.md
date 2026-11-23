# Native Gewichte erhalten einen Teil der zeitlichen Zuordnungsinformation

Auf **10 von 23** archivierten terminalen Kante–Zuordnung-Paaren
war die native Gewichtsänderung positiv und größer als auf
derselben Kante im umgekehrten und im balancierten Training.
Die native Nettoänderung im zugeordneten Training erreichte
höchstens 5,66 × 10⁻⁶ Modelleinheiten im Betrag. Am wieder aktivierten Relais 153
begünstigte das Paar-Timing die balancierte Folge.
[Versuchsplan](CONTINGENCY_CONSOLIDATION.md)

## Was aus den Paaren im Gewicht ankommt

Für jede der drei Trainingsgeschichten auf vier bekannten Seeds
wurde dieselbe Vereinigung terminaler Kanten ausgewertet. Die
native lokale Regel reproduzierte Gewichte, Eligibility,
Rundungsreste und Uhren. Die vier Offline-Varianten nutzten pro
Geschichte dieselben Ankünfte, Postsynapsen-Spikes und chemischen
Zeitreihen. Alle Live-Läufe hatten dieselben Root-Spikezahlen je
Kanal. Bereits archiviertes Training wurde exakt reproduziert.

| Replay-Variante | Positives Tag-Integral über beiden Kontrollen | Positiver Gewichtsauftrag über beiden Kontrollen | Positive sichtbare Änderung über beiden Kontrollen | Sichtbares Endgewicht verändert |
| --- | ---: | ---: | ---: | ---: |
| Nativer Faktor mit Rest | 11 | 10 | 10 | 15 |
| Nativer Faktor mit direktem f32 | 11 | 10 | 1 | 4 |
| Faktor +1 mit Rest | 11 | 11 | 11 | 16 |
| Faktor +1 mit direktem f32 | 11 | 11 | 11 | 16 |

In allen vier Varianten wurden auf 16 Paaren Gewichtsbeiträge
angefordert. Bei konstantem Faktor +1 war die Summe ihrer Beträge
118-mal so groß wie unter nativer Modulation. Die Vorzeichen
beider Beitragsrichtungen blieben im Replay aufgeschlüsselt,
und es gab keine abgeschnittenen Beiträge an Gewichtsgrenzen.
Die Akkumulation erhöhte bei nativer Modulation die Zahl
sichtbarer und im Zuordnungsvergleich positiver Ergebnisse.
Alle zehn dieser nativen Ergebnisse stammen von terminalen
Quellen, die bereits im früheren Training feuerten.

## Relais 153 → 105 auf Seed 34123

Die Quellzelle feuerte siebenmal in der Vorwärtszuordnung und
siebenmal im balancierten Training. Der Ausgang bildete dabei
18 beziehungsweise 19 neue Tag-Änderungen. In der umgekehrten
Zuordnung blieb die Quelle still. Das vorzeichenbehaftete
Tag-Integral im balancierten Training war 9,2-mal so groß
wie in der Vorwärtszuordnung.

| Geschichte | Netto-Tag-Integral | Nativer Netto-Gewichtsauftrag | Sichtbare Nettoänderung | Änderung des Rundungsrests |
| --- | ---: | ---: | ---: | ---: |
| Vorwärts | +1,40 × 10⁻⁶ | −2,24 × 10⁻⁹ | 0 | −2,24 × 10⁻⁹ |
| Umgekehrt | 0 | 0 | 0 | 0 |
| Balanciert | +1,29 × 10⁻⁵ | +1,05 × 10⁻⁷ | +1,04 × 10⁻⁷ | +9,40 × 10⁻¹⁰ |

In der Vorwärtszuordnung hoben sich 99,39 % der angeforderten
Beiträge auf. Das sichtbare Gewicht änderte sich zwischenzeitlich
24-mal und kehrte zum Startwert zurück. Der verbleibende Betrag
liegt im Rundungsrest. Mit +1 und Rest wurden vorwärts
+1,42 × 10⁻⁶ und balanciert +1,29 × 10⁻⁵ veröffentlicht.
Die direkte f32-Variante veröffentlichte in der Vorwärtszuordnung
mit +1 sogar eine negative Nettoänderung von −5,86 × 10⁻⁶ bei
positivem Auftrag.

## Nächster kausaler Vergleich

Die Ergebnisse trennen zwei Engpässe. Am reaktivierten Ausgang
trägt das Paar-Timing keinen Vorteil für die geprüfte Zuordnung.
Andere terminale Wege tragen einen schwachen Vorteil bis ins
sichtbare native Gewicht. Ein eingefrorener Test auf den
tatsächlich gewachsenen Graphen kann nun bei gleicher Dosis
erlernte Richtungen gegen balancierte und vertauschte
Gewichtsrichtungen prüfen. Die bisherige Kapazitätsreferenz
stammt aus entwickelten Graphen vor dem neuen Wachstum.
Für dauerhaft selektiven Abruf und späteres Weiterlernen fehlt
weiterhin eine prospektive Bestätigung.

## Reproduktion und Prüfung

Alle vier Records wurden vollständig wiederholt. Der
unabhängige Quellen-Build reproduzierte den Kurzlauf bytegenau.
Die Projektprüfung umfasste 467 Rust-, 114 Harness-Python- und
10 Dashboard-Tests. Der gezielte Kontingenztestsatz umfasste
nach dem ergänzenden Audit 30 Tests.
[Kompakte Daten](dokumentation/visual_contingency_consolidation_20261123.json)

Modell `e825bcf` und Library `9a3a5fb` waren vor dem Hauptlauf
veröffentlicht. Im versiegelten Python-Prüfer war die Kanten-ID
1394 im beschreibenden Relaisfeld als Zell-ID behandelt worden.
Die Auditfassung löst die Kante korrekt zu Quellzelle 153 auf.
Der Vergleich beider Ausgaben bestätigte identische
Lernarithmetik, Gewichte und Zuordnungskontraste. Beide
Prüffassungen und Ausgaben liegen im Archiv.

Das geprüfte lokale ZIP liegt unter
`research/results/archives/visual_contingency_consolidation_20261123.zip`.

```bash
root=research/results/visual_contingency_consolidation_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_consolidation_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_consolidation_analysis_audit.py" "$root" | cmp - "$root/analysis-audit.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/result_index.py" "$root" | cmp - "$root/result-index.json"
sha256sum -c research/results/archives/visual_contingency_consolidation_20261123.zip.sha256
```
