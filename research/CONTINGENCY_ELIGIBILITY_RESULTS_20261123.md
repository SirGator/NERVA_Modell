# Geerbte Eligibility erklärt die Gewichtsänderungen stiller Abrufkanten

Auf Seed 34123 veränderten sich acht terminale Kanten des später künstlich
wirksamen Abrufpfads, obwohl ihre Quellen im Training nie feuerten.
Die vollständige Historie zeigt **null Prä-Ankünfte** an diesen Kanten.
Das Löschen der Eligibility am Ende der Entwicklung beseitigte bei allen
acht sowohl die Gewichtsanforderungen als auch die veröffentlichten
Änderungen. Das Löschen der regelinternen Paarhistorie hatte an diesen
Kanten keinen Einfluss auf die gemessenen Beiträge.
[Versuchsplan](CONTINGENCY_ELIGIBILITY.md)

## Woher die Gewichtsänderungen kamen

Die vier untersuchten Seeds hatten im Kapazitätsversuch mindestens
einen Referenzteiltreffer. Für beide Zuordnungen wurden das archivierte
Training exakt rekonstruiert und dieselben Ereignisse mit vier
Anfangszuständen der lokalen Lernregel verarbeitet. Die terminale
Auswahl aus den früheren Abrufablationen umfasst 23 Kante–Zuordnung-Paare.
Alle acht Paare mit stiller Quelle gehören zu Seed 34123.

| Intervention bei Trainingsbeginn | Stille Quellen mit veröffentlichter Änderung | Absolute angeforderte Masse an stillen Quellen | Absolute angeforderte Masse an aktiven Quellen |
| --- | --- | ---: | ---: |
| Nativer Checkpoint | 8 / 8 | 2,700 × 10⁻⁶ | 3,349 × 10⁻⁵ |
| Eligibility auf null | 0 / 8 | 0 | 2,445 × 10⁻⁵ |
| Paarhistorie geleert | 8 / 8 | 2,700 × 10⁻⁶ | 3,349 × 10⁻⁵ |
| Beide Eingriffe | 0 / 8 | 0 | 2,445 × 10⁻⁵ |

Die Zahlen summieren die Beträge der angeforderten Beiträge vor der
Gewichtspublikation. Die absolute Summe der veröffentlichten
Nettoänderungen an den acht stillen Kanten betrug nativ 2,757 × 10⁻⁶.
Der unabhängige Prüfer reproduzierte die angeforderten Beiträge aus dem
geerbten Tag, seinem Zeitanker, der lokal gehaltenen Chemie und der
f32-Rundung bei jedem Zeitschritt. Die Prüfung ordnet die Änderung der
Integration vorhandener Tags in der Trainingsphase zu. Bei den acht
Kanten war auch kein bereits während der Entwicklung emittierter Impuls
unterwegs, der während des neuen Trainings eintraf.

Bei den 15 Paaren mit aktiver Quelle wurden **277 Prä-Ankünfte** aus
Emissionen der Trainingsphase beobachtet. Alle 15 Gewichte änderten
sich auch nach Entfernen der geerbten Eligibility. Das Netto der
angeforderten Beiträge über diese Gruppe wechselte von −2,611 × 10⁻⁶
auf +5,612 × 10⁻⁶. Dieser Summenwechsel beschreibt die untersuchten
Kanten. Er misst keine Verbesserung ihrer Zuordnungsspezifität.
Die aufgezeichnete Paarhistorien-Ablation ergab auf den terminalen
Kanten für die angeforderten und veröffentlichten Beiträge einen
Interaktionsterm von null. Auch die vollständigen Gewichtsvektoren
aller acht Trainingsläufe blieben durch die Paarhistorien-Ablation
in beiden Eligibility-Bedingungen exakt gleich.

## Nativer Eingriff und Abruf

Der eingefrorene Checkpoint enthielt je nach Seed zwischen 92 und 105
synaptische Eligibility-Werte ungleich null. Als diese Werte vor der
erneuten Freigabe der Lernregel auf null gesetzt wurden, blieben in
allen acht Paarungen die beobachteten Lernzeitpunkte, Prä-Ankünfte,
Post-Spikes und chemischen Proben identisch zum nativen Training.
Alle veröffentlichten Endgewichte entsprachen exakt dem Offline-Arm
`clear_eligibility`.

In den 16 Cue–Zuordnungs-Vergleichen mit dem nativen Abruf änderte sich
keine Core-Spikefolge. Der Offline-Arm verwendete dieselben Core-Gewichte
und damit dieselben Proben. Natives Training und Eligibility-Eingriff
erfüllten jeweils 0 von 16 Spike-Kriterien. Der Eingriff trennt damit
die Herkunft der Gewichtsänderungen, ohne einen funktionalen Lerngewinn
zu zeigen. Die Datenauswahl stammt aus technisch wirksamen künstlichen
Abrufpfaden auf bekannten Seeds. Sie erlaubt keine Aussage zur
Häufigkeit des Befunds auf beliebigen Pfaden oder Aufgaben.

Der nächste Test setzt an den fehlenden Prä-Ereignissen an. Lokale
Core-Homöostase während des Trainings kann mit dem bestehenden Regler
auf identischem Graphen gegen den Standardarm geprüft werden. Der
Faktor mit und ohne Start-Eligibility trennt neu entstandene
Paarereignisse von der weiterwirkenden Entwicklungsspur. Entscheidend
bleibt, ob das System daraus selektiven Abruf ohne künstliche
Gewichtssetzung erwirbt.

## Reproduktion

Vier Records, vollständiges Replay, 464 Rust- und 97 Harness-Python-Tests
in der Projektprüfung plus zwei neue gezielte Python-Tests. Beide
archivierten Quellstände erzeugten beim Neubau einen bytegleichen
Kurzlauf. Der native Runner wurde vor der Messung mit Library
`9a3a5fb` und Modell `25d56c8` veröffentlicht.
Die Auswertung für das Archiv überspringt dessen zusätzlich enthaltene
Eingabe-`manifest.json` beim Vergleich der Record-Dateinamen.
Rechenregeln, Auswahlkriterien und native Rohdaten blieben bei dieser
Verpackungskorrektur unverändert. [Kompakte Messdaten pro Kante und Record](dokumentation/visual_contingency_eligibility_20261123.json)

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
root=research/results/visual_contingency_eligibility_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
python3 "$root/analysis_entry.py" "$root"
python3 "$root/result_index.py" "$root"
sha256sum -c research/results/archives/visual_contingency_eligibility_20261123.zip.sha256
```
