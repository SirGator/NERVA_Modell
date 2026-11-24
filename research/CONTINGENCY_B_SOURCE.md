# Kausaler Quellentausch beim archivierten B-Abruf

In zwei A-Historien von Seed 34531 löste `cue_b0` nach
vorwärts trainiertem B einen Spike in Zelle 93 aus. Beide
Vorlaufregeln hatten während B die Kante 87→93 gebildet.
In Aktualitäts- und verzögerter Vorlaufregel entstand an
dieser Stelle 100→93. Der Versuch prüft, ob die neue Quelle
den Abruf im eingefrorenen Zustand verursacht.
[Ausgangsbefund](CONTINGENCY_CONTRAST_GROWTH_RESULTS_20261124.md)

## Eingriffe auf demselben eingefrorenen Träger

Der Runner rekonstruiert die vier Quellenregeln, die balancierte
und die vorwärts trainierte A-Historie sowie je drei B-Zuordnungen.
Damit liegen 24 Träger vor. Die Trainingsereignisse und die
Endtopologien für A und B müssen mit dem nativen Archiv
übereinstimmen. Nach dem Einfrieren aller Plastizität folgen
elf Sekunden Ruhe. Alle Eingriffe setzen erst danach auf
exakten Kopien des jeweiligen Trägers an.

| Arm | Eingriff |
| --- | --- |
| Intakt | Unveränderter Abruf als exakte Archivkontrolle |
| Blockade | Den einzigen während B explorativ entstandenen Eingang zu Zelle 93 sperren |
| Tausch | Dieselbe Sperre und einen neuen Eingang von der jeweils anderen Quelle aus 87 und 100 einsetzen |
| Ortskontrolle | Dieselbe Sperre und einen neuen Eingang von einer weiteren nahe gelegenen erregenden Quelle einsetzen |

Beim Einsetzen bleiben Gewicht und Verzögerung des jeweiligen
gesperrten Eingangs exakt erhalten. Die Ersatzkante hat die
native Kurzzeitkonfiguration und startet in ihrem Ruhezustand.
Die tatsächlich gelieferte Zahl und Amplitude von Ankünften
hängen von der Quellaktivität ab und werden gemessen. Beide
Ersatzbedingungen verwenden einen im Träger freien Endpunkt
im nativen Bildungsradius. Unter den zulässigen Quellen der
Ortskontrolle haben beim intakten `cue_b0` aktive Zellen Vorrang.
Danach entscheiden der kleinste Abstand zur Länge der
Tauschkante und bei Gleichstand die kleinere Zell-ID. Falls
keine zulässige Quelle im Abruf aktiv war, wird dies ausgewiesen.

## Auswertung

Jeder Arm erhält getrennte Proben für Blank, `cue_b0` und
`cue_b1`. Root-Ereignisse, Probenzeit und Aktivität während
der Ruhe müssen je Träger übereinstimmen. Im intakten Arm
stimmen zudem die gesamten Core-Spikefolgen und Ankunfts-IDs
mit dem Archiv überein. Für Zelle 93 werden alle signierten
Ankünfte, Integrationsbatches und Spikes protokolliert.

Eine Blockade ohne Zielspike stützt die Notwendigkeit des
Eingangs auf dem jeweiligen erfolgreichen Träger. Der reziproke
Tausch prüft, ob 87→93 auf den stummen Trägern für die Antwort
ausreicht. Die Ortskontrolle prüft den Beitrag einer anderen
lokalen Quelle. Die gekreuzten B-Zuordnungen zeigen, ob ein
Tausch denselben Spike auch auf balancierten oder umgekehrt
trainierten Trägern auslösen kann. Diese Unterscheidung trennt
die Funktion eines Eingangs im eingefrorenen Abruf von der
Selektivität der Kantenbildung beim Lernen.

Die beiden A-Historien wurden aus den bekannten Trefferfällen
gewählt. Sie stammen aus einer Netzrealisierung. Der Test ist
eine retrospektive Mechanismusdiagnose. Neue Erwerbsfälle und
Fortsetzung des Lernens erfordern anschließend unabhängige
Seeds und Reize.

## Reproduktion

Der exakte Eingang ist der überprüfte Runner-Record
`v00000-s34531-i000.json` der Kontraststudie. Zwei getrennte
native Aufrufe müssen bytegleiche Ausgaben erzeugen.

```bash
input=research/results/visual_contingency_contrast_growth_20261124/records/v00000-s34531-i000.json
root=research/results/visual_contingency_b_source_20261124
cargo build --release --manifest-path research/Cargo.toml
mkdir -p "$root"
research/target/release/nerva-research contingency-b-source "$input" > "$root/source.json"
research/target/release/nerva-research contingency-b-source "$input" > "$root/replay.json"
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_b_source_analysis.py "$input" "$root/source.json" --replay "$root/replay.json"
```
