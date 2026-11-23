# Gezielter Quellentausch an den neuen A-Eingängen

Die bisherige Ortskontrolle ließ sämtliche neuen Zieleingänge in den
beiden Zweigen mit erhaltenem A-Abruf unverändert. Dieser Versuch greift
an diesen Eingängen ein. Er verwendet die archivierten Umkehrzuordnungen
und `cue1` auf Seeds 34313 und 34327. Die spikenden Zielzellen sind 72
beziehungsweise 73. Die Fallauswahl legt eine retrospektive Diagnose
zweier Antworten fest. [Ausgangsbefund](CONTINGENCY_A_TRANSFER_RESULTS_20261123.md)

## Eingriffe im nativen Zustand

Der Runner rekonstruiert den entwickelten Ausgangszustand und das
native A-Training aus dem versiegelten Archiv. Bildereignisse, Core-Spikes,
Bildungsentscheidungen und Topologie am Ende von A müssen übereinstimmen.
Nach dem Einfrieren bleiben elf Sekunden Ruhe vor jeder Eingriffsgruppe
identisch. Die unveränderten Proben müssen sämtliche bisherigen Spike-
und Ankunftsdaten reproduzieren.

Pro Zielzelle umfasst der Plan die drei während A entstandenen Kanten
mit nichtnull Ankunft in der archivierten `cue1`-Probe. Diese drei Kanten
werden einzeln und gemeinsam gesperrt. Die beiden neuen Eingänge ohne
Ankunft in dieser Probe bilden die gruppierte Blockadekontrolle. Eine
Blockade unterbindet auch ausstehende Ankünfte über die betroffene Kante.
Ein zusätzlicher Arm setzt gleichzeitig die während A veränderten älteren
Gewichte auf Vor-A-Werte und die Gewichte aller neuen Kanten auf ihren
Geburtswert. Die eingefrorenen Zellzustände und übrigen Kanten bleiben
bei diesen Eingriffen erhalten.

## Quellentausch auf dem gemeinsamen Träger

Auf demselben entwickelten Vor-A-Träger werden pro Seed zwei vorher
gemessene Anordnungen bei Geburtsgewicht 0,25 rekonstruiert. Eine übernimmt
die A-gewachsenen Endpunkte. Die andere übernimmt die bisherige Ortskontrolle.
Die Ziele behalten ihre Eingangsplätze, Gewichte und Verzögerungen.
Die Eingriffe setzen nach der Ruhephase ein. Neue Kanten beginnen im
nativen Ruhezustand mit ihrer bereichsabhängigen Kurzzeitkonfiguration.

Für jede Anordnung wählt der Plan eine kürzeste zulässige Rotation.
Sie umfasst genau einen der untersuchten Zieleingänge und weitere neue
Kanten zu anderen Zellen. Die vollständige Rotation erhält alle Ein- und
Ausgangsgrade. Zwei Teilbedingungen ändern nur den untersuchten Eingang
beziehungsweise nur die dabei mitrotierenden übrigen Verbindungen.
Damit lassen sich ihre Beiträge zur vollständigen Rotation vergleichen.

Jeder untersuchte Eingang erhält außerdem einzeln eine längenpassende
Ersatzquelle. Eine Gruppenbedingung wählt die größtmögliche Zahl
unterschiedlicher Ersatzquellen für die drei Plätze. Unter gleich großen
Gruppen entscheidet erst die kleinste summierte Längenänderung, dann die
aufsteigende Platz- und Quellen-ID. Eingangsgrad und Gewicht je Ziel
bleiben erhalten. Der Ausgangsgrad darf sich in den Einzel-, Gruppen-
und Teilbedingungen ändern. Blockadearme auf demselben Träger sperren
die in der Gruppenbedingung ersetzten Plätze beziehungsweise alle drei
untersuchten Eingänge.

Alle Ersatzkanten bleiben im ursprünglichen Bildungsradius von 0,25.
Die Länge je Platz darf sich um höchstens 0,025 ändern. Die Quellpolarität
bleibt erhalten. Ersatzpaare müssen in der jeweiligen Ausgangsanordnung
frei sein. Der Plan verwendet ausschließlich archivierte Topologie,
Positionen und Auswahl der beiden bekannten Antworten. Er wird vor dem
Lauf vollständig aus diesen Eingaben erzeugt und versiegelt.

## Messung und Entscheidung

Jeder Arm hat getrennte Blank-, `cue0`- und `cue1`-Proben bei identischen
Root-Ereignissen. Der Runner protokolliert die Zielspikes und alle
synaptischen Ankünfte sowie Integrationsbatches der untersuchten Zielzelle.
Als zusätzliche Dosisgröße zählt die Summe positiver Eingangsamplituden
bis einschließlich des ersten Zielspikes der intakten Referenz. Dieses
Zeitfenster bleibt für alle Eingriffe desselben Trägers fest. Bei einem
Träger ohne Zielspike dient die native Latenz als Referenz. Änderungen
von Inputmenge, Zeitpunkt und Zielantwort werden gemeinsam ausgewiesen.
Eine ausgebliebene Antwort nach Quellentausch kann durch verlorene
Ankünfte oder durch veränderte Spikezeiten und Rückkopplung vermittelt sein.

Der Hauptvergleich verwendet die beiden nativen Antwortfälle und ihre
zugeordneten eingefrorenen Transfers. Eine Einzelblockade prüft die
Notwendigkeit eines Eingangs im trainierten Zustand. Rotation, Teilrotation
und Ersatzquellen prüfen den Beitrag der Quellwahl auf dem gemeinsamen
Träger. Für eine Aussage über weitere Assoziationen braucht es anschließend
von diesen Fällen unabhängige Reize und Seeds.

## Reproduktion

Die Eingaben liegen unter `growth_input/` und `transfer_input/`, die
versiegelten Eingriffspläne unter `plans/` und Rohmessungen unter `raw/`.
Der Runner wiederholt jeden vollständigen Record. Die unabhängige Prüfung
erzeugt die Pläne erneut und validiert Topologie, Ankünfte, Integration,
Reizuhren und Zielspikezeiten.

```bash
prior=research/results/visual_contingency_a_transfer_20261123
root=research/results/visual_contingency_a_source_20261123
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_a_source_plan.py "$prior" "$root/plans"
for source in "$root"/growth_input/*.json; do
  name=${source##*/}
  research/target/release/nerva-research contingency-a-source "$source" "$root/plans/$name" > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_a_source_analysis.py "$root"
```
