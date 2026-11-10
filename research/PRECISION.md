# Kontrollierter Vergleich: Direktupdates und erhaltene Lernbeiträge

Diese Suite korrigiert genau einen Mechanismus: die numerische Veröffentlichung
lokaler Lernbeiträge. Der bestehende Drei-Faktor-STDP-Ablauf erhält wahlweise
einen synapsenlokalen f64-Restbetrag. Neuronen, Topologie, Lernraten,
Tagdynamik und Neuromodulation werden nicht neu abgestimmt.
[Library-Mechanismus](../../NERVA/docs/weight-accumulation.md).

[Geprüfter Pilot und Befunde](PRECISION_RESULTS_20261110.md): Der numerische
Verlust wird stark reduziert; bei den beiden geprüften Seeds ändern sich
dadurch weder die Abruf-Spikes noch der gewichtsbedingte Assoziationsgewinn.

## Starten

Alle Befehle beginnen in `DSVLM/main/`. Der vorhandene Kontrollzentrum-Einstieg
verwendet Rust/Cargo mit lokal verfügbaren Paketen und Python 3.11+:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/precision_smoke.toml --output /tmp/nerva-precision-smoke
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/precision_smoke.toml --output /tmp/nerva-precision-smoke
./NERVA_Modell/research/run.sh control status --output /tmp/nerva-precision-smoke
./NERVA_Modell/research/run.sh control report --output /tmp/nerva-precision-smoke
```

Fester Pilot mit 256 Zellen und den bisherigen Seeds 17011/17027:

```bash
./NERVA_Modell/research/run.sh control plan NERVA_Modell/research/config/precision_pilot.toml
./NERVA_Modell/research/run.sh control run NERVA_Modell/research/config/precision_pilot.toml
./NERVA_Modell/research/run.sh control status NERVA_Modell/research/config/precision_pilot.toml
```

`suite = "precision"` wählt diesen Vergleich. Die alten Suiten bleiben
erreichbar; versiegelte ältere Archive verwenden bei einem geänderten
Controller ihre eigene archivierte `producer.py`.

## Feste Kontrollen und Startzustände

| Kontrolle | Numerische Veröffentlichung | Gewichtslernen | Bildzuordnung |
| --- | --- | --- | --- |
| `direct` | Historische f32-Direktupdates | an | gekoppelt |
| `accumulated` | f64-Restbetrag je Synapse | an | gekoppelt |
| `direct_frozen` | f32-Direktupdates | aus | gekoppelt |
| `accumulated_frozen` | f64-Restbetrag | aus | gekoppelt |
| `direct_unpaired` | f32-Direktupdates | an | vertauschte Folgebilder |
| `accumulated_unpaired` | f64-Restbetrag | an | vertauschte Folgebilder |

Alle Modellarme verwenden dieselben Hippocampus-Profile und eingeschaltete
Neuromodulation. Strukturentwicklung, Homöostase und Aktionsrückführung bleiben
aus. Es gibt weder künstliche Modulatorpulse noch eine Gewichtsverstärkung
im Modellvergleich. Die neue Auswahl erfolgt über
`LearningRule::ThreeFactorAccumulated`; die ursprünglichen Aufrufe behalten
ihre automatische Direktupdate-Auswahl.

Pro Seed müssen die initialen Netzwerk-Digests und vollständigen
Anfangsgewichtssnapshots übereinstimmen. Die gekoppelten Kontrollen müssen
identische zeitgestempelte Eingangsdaten erhalten. Ein nativer Modelltest
vergleicht die initialen Netzwerke beider Veröffentlichungsvarianten direkt.

Der spätere Verlauf darf durch tatsächliche Lernwirkung auseinanderlaufen.
Insbesondere wird nicht vorausgesetzt, dass beide Varianten nach veränderten
Spikes noch dieselben Tags, Modulatorsignale oder Updateanforderungen erzeugen.

## Gemeinsame numerische Kalibrierung

Eine zusätzliche deterministische Referenz wird einmal pro Gesamtplan
ausgeführt. Sie vergleicht die native Drei-Faktor-Regel mit beiden
Veröffentlichungsvarianten bei einem Intervall und 1.000 Intervallen über
dieselben 100 ms.

Gemessen werden angeforderte signierte Masse, sichtbare Gewichtsänderung,
Restbetrag, begrenzte Masse und numerischer Fehler. Anders als in der früheren
Diagnose ist der Akkumulator hier der tatsächlich aktive Library-Mechanismus.
Die Tagdynamik bleibt f32; die angeforderte Masse darf deshalb geringfügig von
der Intervallaufteilung abhängen.

## Drei getrennte Endpunkte

1. **Erhaltene Masse:** angeforderte und veröffentlichte Bruttomasse,
   Restbetrag-L1, begrenzte Masse und Rundungsfehler-L1. Pro Synapse wird die
   Bilanz gegen die tatsächlichen Anfangs-/Endgewichte geprüft.
2. **Neuronale Wirkung:** genaue Core-/Motor-Spike-Digests und Potenziale beim
   natürlichen Abruf, sowohl zwischen den beiden Präzisionsvarianten als auch
   gegen den jeweiligen Gewichtsreset.
3. **Assoziation:** Teilbild-Ergänzungskontraste und ihr gewichtsbedingter Anteil
   gegenüber Anfangsgewichten am selben trainierten Checkpoint, eingefrorenen
   Kontrollen und entkoppelten Bildzuordnungen.

Jeder Modellarm rekonstruiert seinen trainierten Zustand mit und ohne Audit.
Vor dem Gewichtsreset muss der volle Replayzustand identisch sein.
Beim Eingriff werden ausschließlich die sichtbaren Gewichte ersetzt;
die ungenutzten Restbeträge bleiben im eingefrorenen Zustand erhalten.
Ein Rücktausch prüft die unveränderten übrigen Zustände.

Das Gewicht bleibt bei der Ausbreitung f32. Ein im Restbetrag erhaltener
Beitrag ist deshalb nicht automatisch eine wirksame synaptische oder
funktionale Gedächtnisänderung.

## Bilanz und Interpretation

Für jede Synapse werden angeforderte und tatsächlich veröffentlichte
signierte Änderungen sowie Restbetragsänderung, Begrenzung und numerischer
Fehler getrennt erfasst. Die maximale Bilanzabweichung muss unter
`1e-12` liegen.

```text
angefordert = sichtbare Änderung + Restbetragsänderung + begrenzt + Rundungsfehler
```

Die Bruttomassen summieren absolute Einzelschritte; die Netto-L1-Masse summiert
absolute bleibende Endänderungen pro Verbindung. Diese Größen dürfen nicht
vertauscht werden. Gegenläufige Beiträge können sich auf derselben Synapse
aufheben. Bei Akkumulation bedeutet eine Anfrage ohne sofortige sichtbare
Änderung nicht, dass die Anfrage verloren ging.

Acht vorab feste einseitige Vorzeichentests prüfen je Teilbild:

- akkumuliert minus direkt,
- akkumuliert minus akkumuliert-eingefroren,
- akkumuliert minus akkumuliert-entkoppelt,
- positiven eigenen gewichtsbedingten Ergänzungsgewinn.

Gleichstände bleiben als Nichtgewinne im vollständigen Nenner.
Die gemeinsame strikte Schwelle ist `p < 0.05 / 8`; beide Teilbildantworten
müssen aktiv sein. Der Vergleich auf zwei bekannten Seeds bleibt explorativ.
Ein Assoziationsvorteil wird getrennt von besserer numerischer Bilanz und
veränderten Spikes berichtet. Ein allgemeiner Gedächtnisnachweis wird daraus
nicht abgeleitet.

## Effizienz und Ablage

Zwei Seeds ergeben zwölf Modellkontrollen und eine gemeinsame numerische
Kalibrierung: **13 Records, 26 Replay-Ausführungen, 48 Trainingsdurchläufe**
einschließlich Rekonstruktion. Der feste Pilot weist 65.952 Modellschritte aus.
Die Modellseeds laufen begrenzt parallel; der Runner wird einmal gebaut.

Ergebnisse liegen unter `research/results/<control.name>/`. Wie bei den
anderen Suiten sichern archivierte Binärdatei, Quellen, Konfiguration,
atomare Records und SHA-256-Belege die Ausführung und Wiederaufnahme.
Die kompakten Massentabellen bleiben pro Synapse aggregiert.
Große Gewichtssnapshots werden nicht nochmals in den Zentralbericht kopiert.

`CONTROL_REPORT.md` und JSON enthalten alle drei Endpunkte und die gemeinsame
Kalibrierung. CSV und die Tabelle `trials` in `control.sqlite` enthalten die
zwölf Modellkontrollen; die numerische Kalibrierung steht getrennt in JSON
und den SQLite-Metadaten. Der Status zählt alle 13 nativen Records.

[Kontrollzentrum](CONTROL.md) · [Diagnosekette A–D](CAUSAL_CHAIN.md) ·
[Ablage und Sicherung](ARCHIVING.md).
