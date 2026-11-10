# Kausale Diagnosekette A–D

Diese Suite erweitert das [Forschungskontrollzentrum](CONTROL.md) um feste
Experimente zur Kette Eingang → Repräsentation → lokale Verknüpfung →
Speicherung → Abruf. Ausgefallene Teilfunktionen werden durch ausdrücklich
künstliche Positivkontrollen ersetzt. Die normale Lernregel wird dabei nicht
repariert, optimiert oder durch einen künstlichen Erfolg ersetzt.

## Start und gemeinsame Infrastruktur

Alle Befehle beginnen in `DSVLM/main/`:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/causal_chain_smoke.toml --output /tmp/nerva-chain-smoke
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/causal_chain_smoke.toml --output /tmp/nerva-chain-smoke
./NERVA_Modell/research/run.sh control status --output /tmp/nerva-chain-smoke
./NERVA_Modell/research/run.sh control report --output /tmp/nerva-chain-smoke
```

Für den festen 256-Zellen-Vergleich:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/causal_chain_pilot.toml
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/causal_chain_pilot.toml
./NERVA_Modell/research/run.sh control status NERVA_Modell/research/config/causal_chain_pilot.toml
```

`suite = "causal_chain"` wählt die neue Kette. Ohne dieses Feld bleibt der
bisherige Acht-Kontrollen-Vergleich aktiv. Beide Suiten verwenden denselben
nativen Runner, Replay, atomare Records, Sperren, begrenzte Worker,
Budgetprüfung und Wiederaufnahme. `report` prüft vorhandene Daten und startet
keine fehlenden Simulationen.

A, B und D sind deterministische Kalibrierungen ohne zufällige Seedvariation.
Sie werden gemeinsam einmal unter dem ersten Seed abgelegt. Nur C wird für
jeden festgelegten Modellseed ausgeführt. Zwei Seeds ergeben deshalb **fünf
Records**, zehn vollständige Replay-Ausführungen und **38 Trainingsdurchläufe**:
24 Hippocampus-Trainingsrekonstruktionen und 14 kleine D-Kontrollen inklusive
Replay. A und B trainieren kein Netz.

Der 256-Zellen-Plan weist 18.832 Modellschritte für C aus. Die zusätzlichen
Mikrotests haben feste Fallzahlen und kleine native Ereigniswarteschlangen.
Es gibt keine adaptive Suche und kein automatisch vergrößertes Raster.

Der [geprüfte Pilot vom 10.11.2026](CAUSAL_CHAIN_RESULTS_20261110.md) enthält
die numerischen Befunde, Positivkontrollen und vollständigen Modellkurven.

## A – Numerische Plastizität

Die vorhandenen öffentlichen `PairStdp`-Hooks erzeugen einen lokalen Tag.
Die native verzögerte Integration wird mit ausdrücklich gesetztem
postsynaptischem Faktor ausgeführt. Der Prüfer trennt:

| Kontrollfall | Gezielt festgelegte Ursache |
| --- | --- |
| `representable` | Ausreichend große Nichtnull-Anforderung, Gewicht innerhalb der Grenzen |
| `sub_ulp_interval` | Sehr kurzer Integrationsabstand; Anfrage unter der Auflösung der f32-Addition |
| `tiny_factor` | Kleiner Nichtnull-Faktor bei längerem Integrationsabstand |
| `zero_factor` | Tag vorhanden, Faktor exakt null |
| `upper_bound`, `lower_bound` | Update zeigt aus einer erreichten Gewichtsgrenze heraus |
| `no_tag` | Faktor vorhanden, aber keine zeitliche Markierung |
| `frozen` | Lernen ausdrücklich deaktiviert |

Die Referenz verwendet `a_plus = 0.0001`, `a_minus = 0.00012`, die vorhandenen
STDP-Zeitskalen und Gewichtsgrenzen 0–2. Pro Schritt werden ungerundete Anfrage,
f32-Delta, f32-Summe, Begrenzung und tatsächliche Gewichtsänderung getrennt
geprüft. Nullanforderungen werden nicht als Rundungsverlust gezählt.

Ein zusätzlicher Vergleich verarbeitet dieselben 100 ms und denselben
gehaltenen Faktor einmal als einzelnes Intervall und einmal als 1.000
Intervalle. Die angeforderte Masse wird in beiden Ästen separat gemessen:
Auch der gespeicherte Tag wird in f32 fortgeschrieben und kann deshalb leicht
von der Ereignispartitionierung abhängen.

Ein externer f64-Akkumulator zeigt zum Vergleich, was durch Erhalt kleiner
Beiträge möglich wäre. Er wird **nicht** in die Library oder das Modell
eingebaut. Dieser Versuch isoliert Mechanismen; er klassifiziert nicht
rückwirkend jede Anfrage eines beliebigen früheren Modelllaufs.

## B – Künstliches Gedächtnis

Ein kleines Netz aus A, B und einer unabhängigen Cue-Zelle wird mit der nativen
Library aufgebaut. Alle Zellen verwenden LIF-Dynamik mit Ruhe/Reset 0,
Schwelle 1 und 1-ms-Refraktärzeit. Die einzige Verbindung ist A→B mit 1-ms-Verzögerung
und räumlicher Abschwächung. Lernen ist ausgeschaltet.

Das vorab feste Gewicht 1,5 muss nach der konfigurierten Wartezeit einen
Cue an A in einen verzögerten Spike von B übertragen. Die Zielzelle erhält
keinen direkten Stimulus. Nullgewicht, gesperrte Verbindung, fehlender Cue
und ein Cue an der unabhängigen Zelle dürfen B nicht aktivieren.
Gewichtserhalt, genaue Spikezeiten und tatsächliche Ankünfte werden gespeichert.

Damit werden unterscheidbare direkte Eingänge, unterschiedliche Zellantworten,
künstliche Speicherung und selektiver Abruf garantiert geprüft. Die
Pixelkodierung und der große Hippocampus werden durch diesen Mikrotest nicht
zertifiziert.

## C – Erzwungene Lernwirkung

Zusätzlich zu einer festen Gewichtskurve im Drei-Zell-Testnetz wird das
bestehende Hippocampus-Modell geprüft. Es lernt mit den bisherigen Bildpaaren,
Neuromodulation und Drei-Faktor-STDP; Strukturentwicklung, Homöostase und
Aktionsrückführung bleiben aus.

Ein vollständig auditierter Trainingslauf liefert den Referenzcheckpoint.
Fünf unabhängig rekonstruierte Checkpoints müssen vor jedem Eingriff mit
`Model::same_replay_state` übereinstimmen. Ein kontrollierter Rücktausch
verifiziert, dass anschließend ausschließlich Gewichte verändert werden.

Die fünf festen Dosen lauten:

```text
w(gain) = clamp(w_initial + gain × (w_learned − w_initial), 0, 2)
gain ∈ {0, 1, 10, 100, 1000}
```

Gain 0 ist die Anfangsgewichtsreferenz, Gain 1 enthält die native Lernwirkung.
Größere Gains sind künstliche Gewichtsinterventionen. Installierte L1-Masse,
maximales Kantendelta, begrenzte Kanten, genaue Core-/Motor-Spikewirkungen,
Potenziale und Abrufzählvektoren werden erfasst.

Alle Äste sehen dieselben vier Abrufbilder nach derselben Schwarzbildruhe.
Die Referenz bleibt unangetastet; Zellzustände, chemische Felder und
Kurzzeitplastizität werden nicht nach einer Probe zurückgesetzt oder zwischen
verschiedenen Gains weiterverwendet.

Ausgewiesen wird der **erste getestete Gain** mit Core-Spikewirkung.
Ein leeres Ergebnis bedeutet nur: keine solche Wirkung im festen Raster.
Es ist weder eine unendliche Schwelle noch ein Nachweis allgemeiner
Wirkungslosigkeit. Bei fehlenden nativen Enddeltas kann deren Verstärkung
keinen neuen Inhalt erzeugen. Nichtmonotone Wirkungen und Clipping bleiben
als vollständige Kurve sichtbar.

Die separate Mikro-Gewichtskurve prüft 0, 0,5, 1, 1,5 und 2. Ihr positives
Ergebnis darf nicht als Nachweis eines funktionierenden Hippocampus-Abrufpfads
übertragen werden.

## D – Lokale Informationsverfügbarkeit

Ein kontrolliertes natives Vier-Zell-Netz verwendet zwei eingehende Synapsen:
eine markierte Zielverbindung und einen unmarkierten Nachbarn. Die
Library-Standardlernparameter sind hier bewusst eine gut messbare
Mechanismenkalibrierung; sie ersetzen nicht die Parameter des laufenden Modells.

Im positiven Arm trifft der Prä-Spike an der Zielverbindung bei 2 ms ein;
die postsynaptische Zelle feuert bei 10 ms. Ein ausdrücklich künstlicher
Dopaminpuls aus einer nahegelegenen Quellzelle folgt bei 100 ms.
Die drei Ereignisse sind nicht gleichzeitig; der Tag muss den zeitlichen
Abstand überbrücken.

Kontrollen entfernen Prä- oder Post-Spike, Rezeptoren oder Lernen,
verschieben den Puls auf zehn Sekunden oder die Quelle räumlich weit weg.
Die Auditdaten enthalten:

- Tatsächliche Prä-Ankünfte und postsynaptische Spikezeiten.
- Tags und Gewichte vor/nach lokalen Hooks an derselben Synapse.
- Konzentration und tatsächlich gehaltenen Faktor am postsynaptischen Zellort.
- Nichtnull-Anfragen, angeforderte Masse, ausbleibende und tatsächliche Updates.
- Den unveränderten unmarkierten Nachbarn.

Ein später Puls kann noch einen winzigen nichtnull Tagrest treffen. Lokale
Überlappung, ausreichend große Anforderung und wirksame Gewichtsänderung werden
deshalb getrennt berichtet. Eine fehlende Rezeptorantwort wird nicht mit
fehlender zeitlicher Markierung verwechselt.

`tag_before_release` bezeichnet den gespeicherten Tag nach der ersten
50-ms-Phase. Er wird nicht auf die spätere Pulszeit hochgerechnet. Die Tags
und Faktoren zu den tatsächlichen Integrationszeiten stehen in den Audittraces.

## Auswertung und Grenzen

`CONTROL_REPORT.md`, JSON, CSV und SQLite enthalten alle vier Experimente.
Große Gewichtssnapshots und Abrufvektoren bleiben ausschließlich in den nativen
Roh-Records; sie werden nicht nochmals in den zentralen Bericht kopiert.

Stufenmetriken verwenden 1 für beobachtet, 0 für nicht beobachtet und **−1 für
nicht geprüft**. Ein nicht geprüfter Schritt wird nicht als bestanden oder
gescheitert ausgegeben. Die erste nicht bestandene beobachtete Stufe wird
ausschließlich aus den Modellmessungen C abgeleitet. Fehlt eine erwartete
Diagnosekontrolle, wird diese Interpretation als ungültig markiert.

Das Eingangskriterium in C verlangt disjunkte beobachtete Kanalzählmengen
für Vollbild A und B, ohne absolute Zeitstempel. Nichtnull-Aktivität allein
genügt nicht. Das Repräsentationskriterium vergleicht die vollständigen
Core-Zählvektoren der Vollbilder. Beide Kriterien sind technische
Unterscheidbarkeitstests und noch kein Nachweis korrekt gelernter Assoziationen.

Die künstlichen Positivkontrollen zeigen Fähigkeiten des jeweils geprüften
Pfads. Sie sind keine natürlich gelernten Assoziationen, keine unabhängige
Bestätigung und kein Nachweis biologischer Gedächtnisfunktion.
`functional_memory_demonstrated` bleibt falsch.

Neue Pläne archivieren Binärdatei, Controller, Konfiguration und Rust-Quellen.
Bereits versiegelte ältere Läufe werden mit ihrer archivierten `producer.py`
geprüft; ihre Quellen- und Datenbelege werden nicht überschrieben.
[Ablage und Sicherung](ARCHIVING.md).
