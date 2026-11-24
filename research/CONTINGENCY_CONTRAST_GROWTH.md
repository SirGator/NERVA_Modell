# Lokale Quellenwahl nach der Differenz zweier Vorlaufwerte

Im bisherigen A/B-Test änderte lokale Vorlauf-Evidenz die Quellen neu
gebildeter Kanten. Die Zielantworten blieben auf dem Niveau der
balancierten Reizexposition. Hier prüfen wir, ob der Abzug eines
zeitversetzten Referenzwerts die Quellenwahl selektiver macht.
[Ausgangsbefund](CONTINGENCY_LEAD_GROWTH_RESULTS_20261124.md)

## Eingriff und Kontrollen

Die vier Arme verwenden Aktualität (`recent`), beobachteten Vorlauf
(`lead`), um 157 ms verzögerten Vorlauf (`lagged`) oder die
vorzeichenbehaftete Differenz aus beobachtetem und verzögertem Vorlauf
(`contrast`). Im Differenzarm stammen beide Werte aus derselben
lokalen Quellhistorie und werden bei denselben positiven
Zielintegrationsbatches außerhalb der Refraktärzeit aktualisiert.
Alle Vorlaufarme beginnen nach 157 ms, nutzen höchstens eine
Stichprobe pro Ziel und 10 ms, gewichten den letzten Vorläuferspike
bei 1–15, 15–40 oder 40–75 ms mit 3, 2 oder 1 und lassen den Wert
mit 2 s Zeitkonstante abklingen. Die Verzögerung verschiebt die
Zuordnung eines historischen Quellspikes zum Zielbatch. Beide
Zeitfenster liegen in der Vergangenheit. Die verzögerte Stichprobe
kann weiterhin stimulusgebundene Aktivität enthalten.

Die Differenz kann negativ sein. Unter den vorhandenen erregenden
Kandidaten wählt der Arm den größten Wert. Auch bei ausschließlich
negativen Werten bleibt die explorative Bildungsgelegenheit
bestehen. Bei Gleichstand gelten Aktualität, Abstand und Zell-ID.
Die native Bildung hat Vorrang. Alle Arme verwenden dieselbe
Kandidatenberechtigung, dieselbe Obergrenze von einer explorativen
Kante pro Ziel über A und B und dieselben nativen Gewichts- und
Homöostaseregeln. Pruning ist im Test ausgesetzt. Die Bildungsregel
erhält ausschließlich Netzereignisse. Zuordnungslabels und
Probeausgänge werden bei der Auswertung verwendet.

## Unabhängiger A/B-Test

Die sechs Seeds 34501, 34513, 34525, 34531, 34543 und 34559 sowie die
beiden Bildabbildungen sind vor dem Lauf festgelegt. Mit
nullbasierten Pixelkoordinaten `(r,c)` bilden die ersten drei Seeds
auf `(r,(c+r+1) mod 8)` und die letzten drei auf
`((c+2r+1) mod 8,7-r)` ab. Beide Abbildungen sind bijektiv und
erhalten die Zahl aktiver Pixel und die Disjunktheit der acht
Cue- und Folgebilder.
[Konfiguration](config/visual_contingency_contrast_growth_20261124.toml)

Jeder Arm startet am exakt gleichen entwickelten Zustand des jeweiligen
Seeds. Von vorwärts, umgekehrt und balanciert trainierten A-Verläufen
zweigen alle drei B-Zuordnungen ab. Der Zustand der lokalen Evidenz geht
mit den laufenden Ereignissen und den Gewichten von A nach B über.
Die Eingaben und Reizzeiten sind phasenweise gepaart. Die Abrufprobe
läuft nach elf Sekunden Ruhe auf einer eingefrorenen Kopie.

## Endpunkte und Reproduktion

Erwerb verlangt für den passenden Zielpool bei ruhiger Grundaktivität
mehr Spikes als in der falschen Zuordnung, im Ausgangszustand und in
der umgekehrt und balanciert trainierten Kontrolle. B wird zusätzlich
mit dem Zustand vor B verglichen. A-Erhalt setzt vorangegangenen
A-Erwerb voraus. Der gemeinsame Endpunkt verlangt beide A-Cues,
ihren Erhalt und beide B-Cues auf demselben gerichteten Pfad.
Die Auswertung weist Erwerb, bedingten Erhalt und vollständige Pfade
je Arm aus und vergleicht die Differenzregel fallgepaart mit allen
drei Kontrollen. Ein reiner Wechsel von Kanten oder Spikefolgen
belegt die technische Manipulation. Ein Erwerbsvorteil erfordert
zusätzliche bestandene Abrufvergleiche gegenüber den Kontrollen.
Mehrere Pfade eines Seeds teilen dieselbe A-Historie.

Die Rohdaten enthalten Stück für Stück die neu gebildeten Kanten,
die gewählte und die nach Aktualität bevorzugte Quelle sowie
deren Evidenzwerte. Für den Differenzarm werden zusätzlich beide
Komponenten jedes Wertes archiviert. Die unabhängige Auswertung
prüft diese Entscheidungen, Reizereignisse, Topologie und die
Erwerbs- und Erhaltskriterien. Der native Runner wiederholt jeden
Record vollständig.

```bash
config=research/config/visual_contingency_contrast_growth_20261124.toml
root=research/results/visual_contingency_contrast_growth_20261124
research/run.sh run "$config" --output "$root"
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_contrast_growth_analysis.py "$root"
```
