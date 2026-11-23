# Retrospektive Diagnose des A-Abrufs

Dieser Plan legt den Vergleich vor dem Hauptlauf fest.
Er untersucht die archivierten Seeds 34301, 34307,
34313, 34319, 34327 und 34333 mit jeweils allen drei A-Zuordnungen
vorwärts, umgekehrt und balanciert. Die bekannten Teiltreffer liegen
auf 34313 und 34327 jeweils bei `cue1` nach umgekehrtem A.
Die Auswahl und diese Kenntnis machen den Test retrospektiv.
[Archivierter Befund](CONTINGENCY_LIVE_GROWTH_RESULTS_20261123.md)

## Replay und Träger

Initialentwicklung und natives A-Training werden mit archiviertem
Root, Core, Bildungsregeln, Gewichten, Eingaben und Zeitplan exakt
wiederholt. Intakte A-Proben müssen die archivierten deterministischen
Felder reproduzieren. Neues B-Training gehört nicht zum Versuch.

Je Seed dient der entwickelte Zustand unmittelbar vor A als gemeinsamer
Transferträger für alle drei Zuordnungen. Zusätzlich dient jeder native
Zustand unmittelbar nach A als Referenzträger seiner Zuordnung.
Alle Träger werden exakt geklont und für Proben eingefroren.
Jeder Träger durchläuft einmal separat elf Sekunden Ruhe. Erst danach
werden aus seinem beruhigten Zustand die Eingriffs- und Probenkopien
erzeugt. Eingriffe erfolgen unmittelbar vor Probenbeginn. Damit wird
ihre Wirkung auf die Expression bei gleicher vorausgehender Ruhe geprüft.

## Native Referenzen

Für jeden nativen Nach-A-Träger gelten sechs Bedingungen.

| Bedingung | Eingriff unmittelbar vor der Probe |
| --- | --- |
| Intakt | Vollständiger nativer Zustand |
| Alle A-Kanten blockiert | Transmission aller während A geborenen Kanten und ihre bereits ausstehenden Ankünfte blockieren |
| A-Zieleingänge blockiert | Dieselbe Blockade für A-Kanten mit Ziel in der Vereinigung beider A-Zielpools |
| A-Komplement blockiert | Dieselbe Blockade für die übrigen A-Kanten |
| Alte Gewichte zurückgesetzt | Aktivierte, persistierende und während A veränderte Kanten auf ihr jeweiliges Vor-A-Gewicht setzen |
| Neue Gewichte zurückgesetzt | Alle A-Kanten auf ihr Geburtsgewicht 0,25 setzen |

Die drei Blockaden verändern keine übrigen Zustände. Mengen, deaktivierte
Transmissionen und entfernte Ankünfte werden vollständig protokolliert.

## Transfer auf den gemeinsamen Vor-A-Träger

Die unveränderte entwickelte Archivkopie ist die gemeinsame Basislinie.
Aus jeder A-Historie werden folgende sieben Bedingungen erzeugt.

| Bedingung | Übertragene Eigenschaften |
| --- | --- |
| Neue Orte, Geburtsgewicht | A-geborene Endpunkte mit Gewicht 0,25 |
| Ortskontrolle, Geburtsgewicht | Umverdrahtete A-Kanten mit Gewicht 0,25 |
| Neue Orte, trainierte Gewichte | A-geborene Endpunkte mit Nach-A-Gewichten |
| Ortskontrolle, trainierte Gewichte | Umverdrahtete A-Kanten mit Nach-A-Gewichten |
| Alte Gewichte | Nur Nach-A-Gewichte der persistenten Vor-A-Kanten |
| Alte und neue Gewichte | Alte Gewichte sowie A-geborene Endpunkte mit Nach-A-Gewichten |
| Alte und neue Gewichte mit Ortskontrolle | Alte Gewichte sowie umverdrahtete A-Kanten mit Nach-A-Gewichten |

Alle übertragenen neuen Kanten werden frisch erzeugt. Sie erhalten die
native, bereichsabhängige Kurzzeitkonfiguration und 1000 µs Verzögerung.
Vom Donor kommen ausschließlich die je Bedingung veröffentlichten
Endpunkte und Gewichte. Synaptische Spuren, Ressourcen, Zeitstempel,
ausstehende Ereignisse und weitere synaptische Zustände werden nicht
übertragen. Alte Gewichtsübertragungen verändern ausschließlich Gewichte
der anhand stabiler IDs zugeordneten persistenten Kanten.

## Deterministische Ortskontrolle

Ein gieriger Durchlauf in aufsteigender Kanten-ID wählt jeweils das erste
zulässige, noch unbenutzte Kantenpaar und tauscht dessen Quellen bei
festen Zielplätzen. Beide Quellen haben dieselbe Polarität. Quellen und
Ziele müssen paarweise verschieden sein. Selbstkanten, Duplikate und
bereits vorhandene Quell-Ziel-Paare sind ausgeschlossen. Beide neuen
Längen müssen innerhalb des Radius 0,25 liegen und dürfen höchstens
0,025 von der ursprünglichen Länge ihres jeweiligen Eingangsplatzes
abweichen. Jede Kante nimmt höchstens an einem Tausch teil.

Gewicht und Verzögerung bleiben am Zielplatz. Ein- und Ausgangsgrade,
Gewichtsmultimenge je Ziel und Verzögerung 1000 µs bleiben erhalten.
Ungepaarte Kanten bleiben unverändert. Der Prüfer rekonstruiert die
ersten zulässigen Paare und meldet Tauschabdeckung, unveränderte Kanten,
geänderte Endpunkte sowie sämtliche Grad-, Gewichts- und Längenprüfungen.

## Proben und Entscheidung

Jede Bedingung erhält getrennte Blank-, `cue0`- und `cue1`-Proben aus
demselben beruhigten Trägerzustand. Relative Reizzeiten und Root-Spikes
sind gepaart. Zielpools, Fenster und Ausschlüsse bleiben archivgetreu.
Primär werden rohe Zielpool-Spikes, Zellen und relative Spikezeiten
berichtet, zusätzlich Ruhe- und Blankantwort sowie Nichtauswertbarkeit.
Vergleiche umfassen die unveränderte Basis, die jeweilige Ortskontrolle
derselben Zuordnung, reine alte Gewichte und die entgegengesetzte sowie
balancierte A-Historie unter derselben Bedingung.

Das ausdrücklich benannte A-Erwerbsgate entspricht dem ursprünglichen
Erwerbskriterium einschließlich beider auswertbarer Pools, passendem
Pool, Ausgangszustand, Gegenrichtung, balancierter Exposition sowie
Ruhe- und Blankgrenzen. Ein Geometrieerfolg verlangt zusätzlich mehr
passende Zielspikes als die gepaarte Ortskontrolle bei identischem Cue
und Zielpool sowie tatsächlich geänderte Endpunkte der Kontrolle.
Kontrollen ohne geänderte Endpunkte bleiben für diese Aussage unauswertbar.

Native Blockaden prüfen bedingte Notwendigkeit beim Abruf im Nach-A-Zustand.
Gemeinsame Transfers prüfen bedingte Hinreichendheit auf dem Vor-A-Träger.
Die Gewichtsarme trennen Beiträge alter Gewichte, neuer Gewichte und
Endpunkte. Ein fehlgeschlagener Transfer lässt weitere Nach-A-Zustände
als mögliche Träger oder Voraussetzungen des Abrufs offen.
Alle Seeds, Zuordnungen, Cues und Rohantworten werden veröffentlicht.

## Versiegelung und unabhängige Prüfung

Vor dem Diagnoselauf werden Protokoll, Quellen, Binary und vollständige
Eingaben mit Prüfsummen versiegelt. Das Replay-Verzeichnis enthält
`input/` mit den unveränderten Archiveingaben und `raw/` mit vollständigen
Messdaten und Eingriffsmanifesten. Jeder vollständige Replay wird erneut
ausgeführt und in allen deterministischen Feldern verglichen.
Der Rust-Runner prüft die Zustandskopien. Ein unabhängiger Prüfer
rekonstruiert Kantenmengen, Transfers, Ortskontrollen, Proben und
sämtliche Kriterien aus Rohdaten.

```bash
root=research/results/visual_contingency_a_transfer_20261123
mkdir -p "$root/raw"
for input in "$root"/input/*.json; do
  name=${input##*/}
  research/target/release/nerva-research contingency-a-transfer "$input" \
    > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_a_transfer_analysis.py "$root"
```
