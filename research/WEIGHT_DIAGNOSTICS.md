# Gewichtseffekt im unveränderten Bildpiloten (09.11.2026)

Der erste hippocampale Bildpilot zeigte Gewichtsereignisse, aber identische
Vorher-/Nachher-Zählvektoren. Version 2 desselben Produzenten ergänzt ausschließlich
lesende Messungen. Netzwerk, Parameter, Bilder, Zeiten und Erfolgskriterien bleiben
identisch. Es wird keine neue Lernregel und kein Parameterraster eingeführt.

```bash
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/hippocampal_weight_diagnostics.toml
```

Zwei feste Seeds (42,73), fünf Kontrollen und exakter Replay aller zurückgegebenen
Messungen entsprechen dem ersten Piloten. Die neue Ergebnisreihe verwendet einen
eigenen Namen; vorhandene Rohdaten und Quellenarchive werden nicht überschrieben.
Der Snapshot trägt `NERVA-Hippocampal-Probe/2`.

Direkt vor und nach dem Training werden alle vorhandenen Gewichte kopiert,
zusammen mit Synapsen-/Endpunkt-IDs, gespeicherter signierter Eligibility und
Zeitanker. Native f32-Werte werden exakt nach f64 erweitert, damit eine externe
Nachrechnung nicht an einer verkürzten f32-Dezimaldarstellung scheitert. Die Zahl
der Kanten ist bereits durch die feste Modelltopologie begrenzt. Eine Änderung
der Kantenidentitäten wird abgelehnt.

Gemessen werden die Summe absoluter Nettogewichtsänderungen, die maximale
Einzeländerung und die Zahl tatsächlich verschiedener Endgewichte. Zusätzlich
werden diese Größen nach physischen Sensor/Core/Motor-Verbindungsklassen und
presynaptischer Polarität getrennt. Die vorhandene Ereigniszahl bleibt erhalten:
viele einzelne Updates können dieselbe kleine Nettoänderung ergeben.

Gespeicherte Eligibility und die lokalen Lernfaktoren am Trainingsende sind
Diagnosen des dann vorliegenden Zustands. Sie sind keine Vorhersage darüber,
welche Gewichte bei weiterlaufender Aktivität entstehen würden. Der bisherige
Versuch friert vor der Nachherprobe Gewichte ein; die neue Messung ändert diesen
Ablauf nicht. Der Speicherort einer eventuellen Fähigkeit wird nicht interveniert.

Als Integritätsprüfung müssen sämtliche bisherigen neun Messwerte, nativen
Vorher-/Nachher-Vektoren, Trainingsfolgen und Netzwerkprüfsummen exakt mit dem
archivierten ersten Lauf übereinstimmen. Die neuen Gewichtssummen werden separat
in Python nachgerechnet. Eine Übereinstimmung belegt die Neutralität dieser
Messung für den festen Piloten, keinen funktionellen Gedächtnisgewinn.
