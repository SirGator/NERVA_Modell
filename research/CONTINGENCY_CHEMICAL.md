# Lokale Modulation bei identischer visueller Trainingshistorie

Die elektrische Diagnose der visuellen Kontingenz fand eine kleine,
unzuverlässig zur gelernten Zuordnung passende Spannungskomponente.
Ein Eingriff in die Chemie während des laufenden Netztrainings könnte
gleichzeitig Erregbarkeit, Übertragung und die Folge von Prä- und Post-Spikes
ändern. Dieser Versuch ändert allein den lokalen Lernfaktor auf einer
festen Trainingshistorie. [Elektrischer Ausgangsbefund](CONTINGENCY_ELECTRICAL_RESULTS_20261123.md)

## Festgelegter Vergleich

Es werden alle zehn bisherigen Records der visuellen Kontingenz und deren
elektrische Abrufkontrollen verwendet. Das sind fünf bekannte Seeds mit zwei
Abständen zwischen Reizanfängen von 50 und 150 ms. Pro Record werden die
vorwärts, umgekehrt und balanciert zugeordneten Sequenzen am originalen
entwickelten Checkpoint mit vollständiger Lernbeobachtung rekonstruiert.
Reizfenster, Root- und Core-Spikes sowie native Endgewichte müssen den
archivierten Varianten exakt entsprechen.

Aus dem Checkpoint werden die Synapsenzustände und die installierte
Lernregel samt lokalen Zeitstempeln kopiert. Der native Offline-Arm muss
Endgewichte, angesammelte Rundungsreste, Prä-Traces, Eligibility, deren
Zeitstempel und den Zustand der Lernregel reproduzieren. Der Gegenversuch
ersetzt bei denselben Ankünften, Postsynapsen-Spikes und chemischen Zuständen
nur den Faktor für angeforderte Gewichtsänderungen durch konstant +1.
Vorzeichen und Größe des Faktors können dadurch zugleich geändert werden.
Es findet in keinem Offline-Arm eine erneute neuronale Simulation statt.

Im Abruf werden nur Core→Core-Gewichte ausgetauscht. Die auf diese Weise
erhaltenen Gewichte werden zum einen direkt eingesetzt (`constant`).
Zum anderen wird pro Zuordnung die Änderungsrichtung des Gegenversuchs
auf die L2-Norm der natürlichen Änderung aus demselben Trainingsarm
gebracht (`matched`). Die installierte Norm darf höchstens 1 % abweichen.
Der installierte Richtungsvektor muss einen Kosinus von mindestens 0,999
zur ursprünglich berechneten Richtung haben. Bei unmöglicher Normierung
einer Zuordnung bleibt der normkontrollierte Gesamttest explizit
unauswertbar. Gewichte und Normen werden ohne Auswertung von Abrufspikes
oder Zielgruppen festgelegt.

## Endpunkte

Die beiden Original-Cues und Schwarzbild werden separat an eingefrorenen
Kopien desselben entwickelten Checkpoints nach elf Sekunden Ruhe beobachtet.
Reset und die nativen Gewichte verwenden die archivierten Beobachtungen.
Zielgruppen und Spike-Kriterium entsprechen dem ursprünglichen
Kontingenzversuch, hier auf Original-Cues begrenzt. Ein Erfolg verlangt
unter beiden gegensätzlichen Zuordnungen einen Anstieg des korrekten
Zielgruppen-Spikezählers gegenüber Reset, balancierter und gegensätzlicher
Zuordnung sowie der falschen Zielgruppe. Schwarz- und Ruheaktivität
dürfen nicht steigen. Die Analyse weist veränderte Spikefolgen und
Kriterienerfolg separat aus.

Solange die Trajektorien und Grenzen gepaart bleiben, wird zusätzlich
der elektrische Richtungsendpunkt der vorigen Diagnose berechnet.
Bei Trajektorienwechsel ist die gepaarte elektrische Richtung für den
betroffenen Vergleich unauswertbar. Wenn die unnormierte Variante
größere Wirkung zeigt, können Stärke und Richtung beitragen.
Ein Unterschied bei gleicher Norm belegt einen Einfluss der
Modulationsform auf den Gewichtsvektor und dessen elektrische Wirkung
im fixierten Ablauf. Ein funktionaler Vorteil verlangt das Spike-Kriterium.
Eine unabhängige Bestätigung, lebendes Training mit konstantem Faktor,
Abruf von Cue-Fragmenten und biologische Validierung liegen außerhalb
dieses diagnostischen Experiments.

## Ausführung

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
cargo build --offline --release --manifest-path research/Cargo.toml
source_run=research/results/visual_contingency_20261122
electrical_run=research/results/visual_contingency_electrical_20261123
new_run=research/results/visual_contingency_chemical_20261123
mkdir -p "$new_run/raw"
for record in "$source_run"/records/*.json; do
  name="${record##*/}"
  research/target/release/nerva-research contingency-chemical \
    "$record" "$electrical_run/raw/$name" > "$new_run/raw/$name.partial" || exit
  mv "$new_run/raw/$name.partial" "$new_run/raw/$name"
done
python3 research/contingency_chemical_analysis.py "$new_run"
```

Vor der Hauptmessung werden beide Quellstände, alle Eingabedateien,
Auswertung und Binary per SHA-256 gebunden. Der native Befehl
wiederholt jede vollständige Rekonstruktion einschließlich Abruf und
verlangt identische elektrische Rohdaten.
