# Lokale Modulation verändert die Gewichtsrichtung, der Abruf bleibt gleich

Auf identischen Trainingshistorien änderte ein konstanter lokaler Modulationsfaktor
die Stärke und Richtung der rekurrenten Gewichtsänderungen. Die L2-Norm war
**81- bis 406-mal größer** als mit dem nativen Faktor. Trotzdem änderte sich
in **keinem von 120** Cue-Vergleichen mit nativem Lernen eine Core-Spikefolge.
Das gilt sowohl für die direkt übertragenen Gewichte als auch für den
normkontrollierten Übertrag. [Versuchsplan](CONTINGENCY_CHEMICAL.md)

## Derselbe Lernablauf, ein anderer Faktor

Der Versuch rekonstruierte die zehn vorhandenen Kontingenz-Records am jeweiligen
entwickelten Checkpoint. In allen 30 Trainingsarmen stimmten Reizfenster,
beobachtete Spikes und native Endgewichte mit dem Archiv überein. Der
anschließende Offline-Replay reproduzierte die native Lernregel einschließlich
ihrer Paarzeitstempel, der gespeicherten Synapsenspuren und der Rundungsreste.
Positive und negative Eligibility-Integrale stimmten im nativen und konstanten
Arm je Synapse exakt überein. Beide verwendeten dieselben Ankünfte, Postsynapsen-
Spikes und aufgezeichneten chemischen Zustände.

Die Kosinusähnlichkeit zwischen nativem und konstantem Änderungsvektor lag
zwischen **0,357 und 0,783** auf den rekurrenten Kanten. Die Änderung des Faktors
wirkte also auch auf die Verteilung der Updates. Alle 30 Normierungen waren
numerisch zulässig. Die größte relative Normabweichung der installierten Gewichte
betrug 0,0295 %. Jede normierte Richtung hatte einen Kosinus von mindestens
0,99997 zur Richtung des konstanten Arms.

## Elektrische und funktionale Wirkung

| Abstand | Gewichte | Passende elektrische Einzelvergleiche | Größte absolute Verschiebung relativ zur Schwelle |
| --- | --- | --- | --- |
| 50 ms | Nativ | 8 / 16 | 0,00001609 |
| 50 ms | Konstant +1 | 8 / 16 | 0,001755 |
| 50 ms | Konstant +1 bei nativer L2-Norm | 7 / 16 | 0,00001106 |
| 150 ms | Nativ | 1 / 16 | 0,00000620 |
| 150 ms | Konstant +1 | 1 / 16 | 0,001825 |
| 150 ms | Konstant +1 bei nativer L2-Norm | 1 / 16 | 0,00001329 |

Der unnormierte Gegenversuch bewirkte größere elektrische Verschiebungen
in nativen Potenzialeinheiten, ohne eine Zielzelle zusätzlich zum Feuern zu
bringen. Bei gleicher L2-Norm änderten sich **65 von 108** gepaarten
Zielgruppenmittelwerten gegenüber nativ. Das weist in diesem festen
Trainingsablauf einen Richtungseffekt des Faktors unabhängig von der
Norm nach. Bei keinem auswertbaren Seed begünstigte er beide Cues unter
beiden Zuordnungen. Für jeden Zeitabstand waren drei Seeds vollständig
elektrisch auswertbar. Bei zwei Seeds fehlten für einen Cue Integrationsgrenzen
in einer Zielgruppe. Kein Seed erfüllte das vorab festgelegte Spike-Kriterium.

Der Eingriff ersetzt lokal variierende Faktoren durch +1 und ändert dabei
sowohl ihre Verteilung als auch ihre Größe. Die Aussage über Richtung beruht
auf dem normierten Übertrag. Der Versuch erfasst Gewichtswirkung aus der
beobachteten Historie. Er prüft keine Rückwirkung eines konstanten Faktors
auf späteres Training, keine biologische Wirksamkeit und keine Generalisierung
auf neue Seeds oder Bilder.

Der nächste kausale Test prüft, ob die vorhandene Rekurrenz überhaupt einen
selektiven Abruf der getrennten Folgebilder tragen kann. Dazu wird auf dem
festen Graphen ein diagnostischer Abrufpfad aus tatsächlich aktiven
Cue-Projektionen aufgebaut und gegen eine gleich starke gelernte Richtung
geprüft. Ein funktionierender Referenzpfad würde die elektrische Kapazität
der Topologie belegen und einen Maßstab für den Lernversuch liefern.

## Reproduktion und Prüfungen

Zehn native Records und vollständiges Replay der neuen Trainings- und
Abrufrekonstruktionen. Native und konstant gerechnete Endgewichte,
Normierung, Schwellenentscheidungen und Selektivitätskriterien wurden
unabhängig aus den Rohdaten geprüft. Der aus beiden archivierten
Quellständen neu gebaute Runner reproduzierte den Kurzlauf bytegenau.
Die Projektprüfung bestand mit 462 Rust-Tests, 97 Harness-Python-Tests,
10 Dashboard-Tests, Rustformat, Clippy und Linkprüfung.

Vor der Hauptmessung waren Library `cf4272e` und Modell `ff8e522`
veröffentlicht. Das lokale Archiv
`research/results/archives/visual_contingency_chemical_20261123.zip`
enthält Eingaben, unveränderte Rohrecords, beide Quellstände, Binary,
Prüfskripte und Prüfsummen. [Kompakte Daten und SHA-256 je Record](dokumentation/visual_contingency_chemical_20261123.json)

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
root=research/results/visual_contingency_chemical_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
python3 "$root/contingency_chemical_analysis.py" "$root"
python3 "$root/result_index.py" "$root"
sha256sum -c research/results/archives/visual_contingency_chemical_20261123.zip.sha256
```
