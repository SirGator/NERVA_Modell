# Richtung und Änderungsstärke getrennt prüfen

Die Suite `direction_strength` erweitert den
[Richtungs-/Pfadvergleich](DIRECTION_PATH.md) um einen normkontrollierten
Vierfelderversuch und zwei zusätzliche rekurrente Suchrichtungen.
[Gemessener Pilot](DIRECTION_STRENGTH_RESULTS_20261110.md).

## Ausführen

Alle Befehle beginnen in `DSVLM/main/`. Rust/Cargo mit lokal verfügbaren
Paketen und Python 3.11+ genügen. Der CPU-Referenzpfad braucht weder CUDA noch QEMU.

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/direction_strength_smoke.toml --output /tmp/nerva-factorial
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/direction_strength_smoke.toml --output /tmp/nerva-factorial
./NERVA_Modell/research/run.sh control status --output /tmp/nerva-factorial
./NERVA_Modell/research/run.sh control report --output /tmp/nerva-factorial
```

Der feste Pilot startet mit `config/direction_strength_pilot.toml`:
256 Core-Zellen, acht Wiederholungen, elf Sekunden Ruhe pro Probe und
Seeds 17011/17027. Es werden keine Lernraten oder Zellmechanismen angepasst.

## Gemeinsame Grundlage

Das Bildprotokoll und der native Drei-Faktor-Lernablauf mit f64-Restbetrag
bleiben erhalten. Entwicklung, Homöostase und Rückführung sind ausgeschaltet.
Ein auditierter trainierter Zustand dient als Referenz. Jeder tatsächlich
simulierte Gewichtseingriff rekonstruiert genau diesen vollständigen Zustand;
`same_replay_state` und ein Gewichts-Rücktausch kontrollieren die Gleichheit
einschließlich Warteschlangen, Uhren, Tags und Restbeträgen.

Abruf erfolgt mit eingefrorener Plastizität in der festen Sequenz Schwarzbild,
Vollmuster A, Vollmuster B, Cue A, Cue B; jeweils nach Ruhe. Die Zielvektoren
stammen ausschließlich aus den Vollmustern des Anfangsgewichtszweigs.
Auch spätere künstliche Zweige werden gegen diese festen Ziele bewertet.
Ruhe ist kein behaupteter vollständiger Zustandsreset: spätere Proben dürfen
durch die vorherigen Gewichtswirkungen unterschiedliche Vorgeschichten haben.

## Begrenzte Suche nach einer künstlichen Referenz

| Methode | Konstruktionsprinzip | Änderbare Synapsen |
| --- | --- | --- |
| `sensory_and_recurrent` | Bisherige Cue-Differenz × Differenz der fehlenden Ergänzungsaktivität | Sensor→Core und Core→Core |
| `recurrent_contrast` | Normierte Core-Cue-Differenz × normierte Vollmuster-Klassendifferenz | Ausschließlich Core→Core |
| `recurrent_two_hop` | Direkter Kontrast plus über vorhandene erregende Core-Verbindungen weitergereichte Cue-/Zielkontraste | Ausschließlich Core→Core |

Die neue Kontrastmethode verlangt keinen positiven
`Vollmuster − Teilmuster`-Unterschied an jedem Ziel. Die Relaismethode
berücksichtigt zusätzlich Wege über ein Zwischenneuron, ohne neue Kanten
oder erzwungene Spikes einzuführen. Für eine Kante i→j lautet ihr Rohscore:

```text
q[i] * t[j] + q[i] * backward_target[j] + forward_cue[i] * t[j]
```

q und t sind die auf den größten Absolutwert normierten Cue- beziehungsweise
Vollmuster-Klassendifferenzen. `forward_cue` summiert q über vorhandene
aktivierte erregende Eingangskanten; `backward_target` summiert t über deren
Ausgangskanten. Beide Karten werden erneut normiert. Inhibitorische
Quellpolarität kehrt das Vorzeichen des Gewichtsbetrag-Eingriffs um.
Verändert werden vorhandene aktivierte plastische Verbindungen.

Alle drei Methoden prüfen die festen additiven Dosen 0,25 / 1 / 2 und jeweils
die umgekehrte Richtung als Kontrolle vertauschter Ziele: 18 Kandidaten.
Die künstliche Suche darf ihre Gewichte auf [0,2] begrenzen; als spätere
Referenz zählt immer der tatsächlich installierte Gewichtsvektor.

Ein Kandidat muss beide Cues aktiv und korrekt ergänzen, ihre festen
Klassenkontraste gegenüber Anfangsgewichten um mehr als `1e-9` verbessern
und die vertauschte Richtung bei gleicher Dosis schlagen. Ohne Cue darf die
Core-Spikezahl nicht steigen. Pro Methode gilt die erste erfolgreiche Dosis.
Die Referenzwahl bevorzugt zuerst `recurrent_contrast`, dann
`recurrent_two_hop`, dann den Sensor+Core-Vergleich. Alle Methodenbefunde
werden unabhängig von dieser Wahl archiviert.

Keine erfolgreiche Referenz bedeutet: Vierfelderdiagnose offen.
Es wird keine lediglich veränderte Aktivität als korrekter Abruf gewertet.

## Der Vierfelderversuch

Für Anfangsgewichte w0 definieren wir die sichtbaren Nettodeltas
dN = wNatürlich − w0 und dR = wReferenz − w0.
**Stärke ist ihre L2-Norm**, Richtung der zugehörige Einheitsvektor.

| Arm | Richtung | Gewünschte Stärke |
| --- | --- | --- |
| NN | natürlich | ‖dN‖ |
| NR | natürlich | ‖dR‖ |
| RN | künstliche Referenz | ‖dN‖ |
| RR | künstliche Referenz | ‖dR‖ |

Alle Gewichtseingriffe gehen von w0 aus; der vollständige trainierte
Neuronenzustand bleibt die gemeinsame Ausgangsbasis.
Natürlicher Zweig, erfolgreiche Referenz und identische Kreuzvarianten
verwenden ihre bereits erfasste exakte Beobachtung wieder.

**Normierung wird nicht mit anschließendem Clipping vermischt.**
Pro Richtung wird der maximal zulässige Radius berechnet, bevor ein Gewicht
die Grenzen [0,2] überschreitet. Unzulässige Varianten werden als solche
archiviert und nicht ausgeführt.

Zusätzlich wird dasselbe 2×2 bei gemeinsamer hoher Stärke gerechnet:

```text
common_high = min(Referenznorm, zulässiger Radius natürliche Richtung,
                  zulässiger Radius Referenzrichtung)
```

Die natürliche Stärke bleibt die andere Stufe. Fehlt der nötige Abstand
zwischen den Stärken, ist der Vergleich ungültig.

Da die Ausbreitung f32-Gewichte nutzt, wird die Norm nach Veröffentlichung
erneut berechnet. Bei mehr als 0,01 % Rundungsabweichung kann eine auf
48 Schritte begrenzte Bisektion den skalaren Radius vor Veröffentlichung
kalibrieren. Diese rein numerische Suche nutzt keine neuronalen Antworten
und ändert die mathematische Richtung nicht. Gewünschte Stärke,
verwendeter Radius, tatsächliche Stärke, Rundungsnorm und Richtungscosinus
bleiben getrennt sichtbar. Nicht vermeidbare Quantisierung kann einen Arm
weiterhin ungültig machen.

Gültige Arme verlangen höchstens 1 % relativen Normfehler und
Richtungscosinus ≥ 0,999. Nullrichtungen sind undefiniert.
Diese festen Grenzen werden nicht nach einem günstigen Ergebnis gelockert.

## Auswertung und Grenzen

Nur vollständig gültige Matrizen liefern getrennte Kontraste pro Cue:

- Richtung bei natürlicher Stärke: RN − NN.
- Richtung bei hoher Stärke: RR − NR.
- Stärke auf natürlicher Richtung: NR − NN.
- Stärke auf Referenzrichtung: RR − RN.
- Zusammenspiel: RR − NR − RN + NN.

Es werden Ergänzungskontraste und ihre Differenzen ausgewertet, getrennt von
Spikes, elektrischen Potenzialbilanzen und Gewichtsänderungsnormen.
Die Python-Auswertung berechnet feste Zielantworten, tatsächlich installierte
f32-Gewichte, Normen, Richtungsfidelität und Interaktion aus Rohdaten erneut.

Die künstlichen Gewichte sind auf denselben Bildern und bekannten Seeds
kalibriert. Das ist eine Fähigkeits-/Ursachendiagnose und keine unabhängige
Bestätigung natürlich gelernten Gedächtnisses. Eine endliche erfolglose
rekurrente Suche beweist keinen unmöglichen Speicherpfad. Auch die erste
erfolgreiche Lösung ist nicht die einzig mögliche richtige Richtung.

## Effizienz und Reproduktion

Jeder unterschiedliche vollständige f32-Gewichtsvektor wird nur einmal
simuliert. Kandidaten und Matrixarme verweisen auf diese Beobachtungen.
Das vermeidet insbesondere Wiederholungen bei Nullrichtungen und identischen
Stärkestufen und dupliziert keine großen elektrischen Rohdaten.

Das Budget beträgt höchstens 25 native Trainingsdurchläufe je Seed und
Ausführung, einschließlich des gehaltenen Referenzzustands. Mit zwei Seeds
und vollständigem Replay sind das höchstens 100 Trainingsdurchläufe und
120.100 Modellschritte. Der tatsächliche Aufwand wird separat berichtet;
die Plankosten sind bei dieser Suite ausdrücklich Obergrenzen.

Die bestehende elektrische Beobachtung bleibt fensterweise begrenzt.
Jede Beobachtung enthält Synapsen-/Neuronensummen, Quell-Spikezahlen und
Potenzialbilanzen; Beispiele bleiben auf 128 je Bild begrenzt.
`factorial_report.py` wird zusammen mit `producer.py`, Runner,
Konfiguration und Quellenarchiv abgelegt und durch den Kontrollplan gehasht.
Ein geänderter Berichtshelfer macht ein Archiv nicht stillschweigend gültig.

Markdown, JSON, CSV, SQLite und SHA-256-Belege entstehen über den vorhandenen
Controller. Ältere Archive verwenden ihren archivierten Producer.
[Ergebnissicherung](ARCHIVING.md): Rohdaten sind von Git ausgeschlossen.
