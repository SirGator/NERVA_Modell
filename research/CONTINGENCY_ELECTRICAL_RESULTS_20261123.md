# Messbare Gewichtswirkung bleibt unterhalb der Feuerschwelle

Rekurrente Gewichtsänderungen verschieben beim visuellen Abruf messbar
Ankunftsamplituden und Schwellenabstände. Einzelne Cues zeigen eine zur
Trainingszuordnung passende Verschiebung. Kein auswertbarer Seed zeigt
dieses Muster für beide Cues unter beiden gegensätzlichen Zuordnungen.
Die beobachteten Spikefolgen bleiben exakt gleich.
[Versuchsplan](CONTINGENCY_ELECTRICAL.md)

## Zuordnung und Zeitabstand

Die Diagnose rekonstruiert alle zehn vorhandenen Records auf eingefrorenen
Kopien ihrer entwickelten Ausgangszustände. Gewichte und Zielgruppen stammen
aus dem ursprünglichen Versuch. Pro Cue und Zielzelle wird die nichtrefraktäre
Integrationsgrenze mit dem höchsten Reset-Kandidatenpotenzial relativ zur
Schwelle gewählt. An denselben Grenzen werden die passend trainierte, die
gegensätzlich trainierte und die balancierte Zuordnung verglichen.

| Abstand zwischen Reizanfängen | Vollkriterium bestanden | Vollkriterium verfehlt | Unauswertbare Seeds | Bestandene auswertbare Cue–Zuordnungs-Vergleiche |
| --- | --- | --- | --- | --- |
| 50 ms | 0 | 3 | 2 | 8 von 16 |
| 150 ms | 0 | 3 | 2 | 1 von 16 |

Für 33223 und 33287 fehlt bei Cue 0 unter beiden Zeitabständen jede
Integrationsgrenze für die zweite Zielgruppe. Für Cue 1 bleiben beide Seeds
auswertbar. Alle 60 Paarungen mit Reset haben identische Root- und
Core-Spikefolgen, Integrationszeitpunkte, Schwellen und Refraktärzeiten.
Pro Seed stimmen auch die Reset-Beobachtungen beider Zeitabstände überein.

Die Einzelvergleiche sind innerhalb der fünf bekannten Seeds abhängig.
Die Häufung bei 50 ms beschreibt diese Läufe. Mit dem Cue-Folgebild-Abstand
änderte sich auch die Zeit bis zum nächsten Cue. Die Häufung isoliert daher
keinen bestimmten Lernzeitparameter.

## Wie viel an der Zielzelle ankam

In 25 von 40 Original-Cue-Fällen mit fester Trainingszuordnung erreichte
mindestens eine geänderte Ankunftsamplitude die passende Zielgruppe.
Das sind 13 von 20 Fällen bei 50 ms und 12 von 20 bei 150 ms. Über beide
Zielgruppen und einschließlich der balancierten Kontrolle änderten sich
364 von 1.026 gepaarten Ankunftsamplituden. Die größte Betragsänderung einer
Amplitude und einer kanonischen Eingangssumme betrug jeweils
`5,871 × 10⁻⁶` in nativen Modelleinheiten.

Von 27.612 gepaarten Integrationsgrenzen rückten 1.475 näher an die Schwelle
und 2.267 weiter von ihr weg. Die größte Annäherung betrug
`1,609 × 10⁻⁵` bei Seed 33223, Cue 1, 50 ms, `forward_core`, Neuron 91.
Am selben Zeitpunkt lag das Reset-Kandidatenpotenzial um `0,462255`
unter der Schwelle. Die Änderung schloss **0,00348 %** dieses Abstands.
Eine nachträgliche Suche über alle gepaarten Grenzen fand den größten
relativen Anteil an derselben Stelle. Diese Messung bei nativen Gewichten
liefert keinen Skalierungsfaktor für eine veränderte Netztrajektorie.

Die Daten lokalisieren für einen Teil der passenden Zielgruppen eine
elektrisch messbare Gewichtswirkung. Die Verschiebungen zeigen noch keine
stabile Zuordnungsrichtung. Ein funktionaler Gedächtnisabruf oder die
biologische Gültigkeit der modellierten Mechanismen ist damit offen.
Der nächste kausale Vergleich setzt während des Trainings an und spielt
dieselbe Spike- und Ankunftshistorie mit nativem gegenüber konstantem
lokalem Modulationsfaktor ab. Das trennt vorhandene Prä-/Post-Information
von der Gewichtung durch Chemie. Der Richtungsbefund muss danach auf
unabhängigen Seeds und visuellen Zuordnungen geprüft werden.

## Prüfungen und Rohdaten

Zehn neue elektrische Records mit vollständigem Replay. Die alte Antwort
einschließlich Ruheaktivität, Root- und Core-Spikefolgen und IDs der leitenden
Kanten wurde für jeden erneut beobachteten Cue exakt reproduziert.
Der unabhängige Prüfer berechnet die kanonische Eingangssumme, die
f32-Schwellenentscheidung und alle Richtungskontraste aus den Rohdaten.
49 Rust-Harness- und 92 Python-Harness-Tests, Rustformat, Clippy und lokale
Linkprüfung bestanden. [Kompakte Daten und SHA-256 je Record](dokumentation/visual_contingency_electrical_20261123.json)

Der Code und der Versuchsplan wurden vor dem Lauf auf `NERVA_Modell 507481e`
veröffentlicht. Library-Stand war `NERVA f973a24`. Das lokale Archiv unter
`research/results/archives/visual_contingency_electrical_20261123.zip`
enthält die verifizierten Eingaberecords, neuen Rohdaten, Quellen, Binary,
Skripte und Prüfsummen.

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
root=research/results/visual_contingency_electrical_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
python3 "$root/contingency_electrical_analysis.py" "$root"
python3 "$root/result_index.py" "$root"
sha256sum -c research/results/archives/visual_contingency_electrical_20261123.zip.sha256
```
