# Elektrische Diagnose des visuellen Abrufs

Der visuelle Kontingenzversuch fand trotz veränderter rekurrenter Gewichte
identische Spikefolgen. Er beobachtete direkte Ankünfte an der passenden
Zielgruppe in 32 von 40 Fällen. Solche Ankünfte lagen in 25 Fällen auf
mindestens einer Kante mit gelerntem Gewichtsdelta. Daraus ergeben sich drei
prüfbare Möglichkeiten. Relevante Amplitudenänderungen kommen selten an.
Sie kommen an, erzeugen jedoch eine kleine oder ausbleibende Änderung des
Schwellenabstands. Oder sie verschieben den Schwellenabstand, ohne die
gelernte Zuordnung zu begünstigen. [Ausgangsbefund](VISUAL_CONTINGENCY_RESULTS_20261123.md)

## Festgelegter Vergleich

Die Diagnose verwendet **alle zehn** geprüften Records der visuellen
Kontingenz, fünf bekannte Seeds bei 50 und 150 ms. Zielgruppen und
Gewichte werden unverändert aus diesen Records übernommen. Jeder Record
wird auf eingefrorenen Kopien seines entwickelten Checkpoints erneut
abgerufen. Für die beiden Original-Cues werden Reset, `forward_core`,
`reverse_core` und `balanced_core` verglichen. Die 200-ms-Messfenster
nach elf Sekunden Ruhe übernehmen den ursprünglichen Ablauf. Der Runner
prüft Stimuli, Checkpoint, Topologie, installierte Gewichte und die gesamte
bisherige Cue-Antwort gegen den archivierten Record. Die erweiterte
Beobachtung muss in einem zweiten Durchlauf exakt übereinstimmen.

Erfasst werden alle synaptischen Ankünfte und Integrationsgrenzen an Zellen
beider Zielgruppen. Eine Grenze enthält Vorpotenzial, kanonische Eingangssumme,
Schwelle, Nachpotenzial, Ende der Refraktärzeit und Spike. Die Auswertung
prüft die native Schwellenregel erneut. Sie paart Grenzen nach Zeit und Zelle
und Ankünfte nach Zeit, Zielzelle und Synapse. Sie berichtet auch Änderungen
der Eingangssumme und die größte beobachtete Annäherung relativ zum jeweiligen
Reset-Schwellenabstand, wenn dort die Schwelle im Reset unterschritten blieb.

Für die primäre Richtungsprüfung wird für jede erreichbare Zielzelle je
Cue **im Reset** die nichtrefraktäre Grenze mit dem höchsten Wert von
`f32(Vorpotenzial + Eingangssumme) − Schwelle` gewählt. Bei Gleichstand gilt
die früheste Grenze. Zellen ohne solche Grenze gehen in die gesondert
ausgewiesene Abdeckung ein. Fehlende Zielgruppen oder fehlende gemeinsame
Grenzen machen die betroffene Prüfung unauswertbar. Die übrigen Arme
werden an denselben Grenzen ausgewertet. Überschreitet der Versuch
die Vergleichbarkeit durch abweichende Spikefolgen, Schwellen oder
Refraktärzeiten, wird kein gepaarter Richtungsbefund abgeleitet.

Pro Zielgruppe wird der mittlere Unterschied zu Reset berechnet.
Für eine Zuordnung und einen Cue muss der Mittelwert in der passenden
Zielgruppe positiv sein. Er muss zugleich den Wert in der anderen Zielgruppe
und den Wert für dieselbe Zielgruppe in beiden Trainingskontrollen
übertreffen. Der Seed besteht einen Zeitabstand nur, wenn das für beide
Cues unter beiden gegensätzlichen Zuordnungen gilt. Der Befund wird für
beide Abstände und alle Seeds ausgewiesen. Der Versuch testet einen
elektrischen Vorläufer von Abruf auf bekannten Seeds. Er prüft weder
biologische Validität noch erfolgreiche Generalisierung.

## Ausführung

Der native Befehl erhält genau einen unveränderten Record und schreibt
die vollständige geprüfte elektrische Beobachtung als JSON nach stdout.
Die unabhängige Auswertung liest solche Dateien aus `raw/`.
Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
source_run=research/results/visual_contingency_20261122
new_run=research/results/visual_contingency_electrical_20261123
mkdir -p "$new_run/raw"
for record in "$source_run"/records/*.json; do
  name="${record##*/}"
  research/target/release/nerva-research contingency-electrical "$record" > "$new_run/raw/$name.partial" || exit
  mv "$new_run/raw/$name.partial" "$new_run/raw/$name"
done
python3 research/contingency_electrical_analysis.py "$new_run"
```

Für den Hauptlauf werden vorab Quellstände, Binary und alle zehn Eingabedateien
per SHA-256 gebunden. Rohdaten und Prüfbelege liegen im lokalen Ergebnisarchiv.
