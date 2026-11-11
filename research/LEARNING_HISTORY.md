# Learning History & Causal Trace

`hippocampal_learning_history` verknüpft die vorhandenen Lern-, Chemie- und
Elektroaudits. Die Beobachtung verändert keine neuronale Regel. Der Adapter
verwendet das vorhandene Bildassoziationsprotokoll und dessen Kontrollgruppen;
Drei-Faktor-Gruppen verwenden den vorhandenen synapsenlokalen f64-Restbetrag,
Pair-Gruppen natives Pair-STDP. Die bisherigen Adapter behalten ihre Defaults.

## Ausführen und untersuchen

Alle Befehle beginnen in `DSVLM/main/`:

```bash
./NERVA_Modell/research/run.sh plan NERVA_Modell/research/config/learning_history_smoke.toml --output /tmp/nerva-history
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/learning_history_smoke.toml --output /tmp/nerva-history
```

Die Beispielkonfiguration prüft vier Kontrollen bei 16 Core-Zellen, einem Seed
und einer Trainingswiederholung, einschließlich vollständiger Wiederholung.
Das ist eine technische Prüfung, kein Gedächtnisnachweis. Ein Trial benötigt
drei Trainingsdurchläufe: beobachtet, Gewichts-Reset und unbeobachtete Referenz.
Mit vollständigem Replay sind es sechs Durchläufe.

Ein Record lässt sich anschließend ohne neue Simulation abfragen:

```bash
# Konkreten Dateinamen unter records/ einsetzen:
./NERVA_Modell/research/run.sh history /tmp/nerva-history/records/TRIAL.json
./NERVA_Modell/research/run.sh history /tmp/nerva-history/records/TRIAL.json --neuron 64
./NERVA_Modell/research/run.sh history /tmp/nerva-history/records/TRIAL.json --synapse 812 --phase recall --limit 500
```

Ohne Filter zeigt die Abfrage die netzweite Zusammenfassung und bis zu 200
Synapsen, sortiert nach wirkungslosen verzögerten Gewichtsanforderungen und
gegenläufiger Änderungsmasse. Die Abfrage erhält das Archiv unverändert;
`report` übernimmt wie bisher die Integritätsprüfung des gesamten Archivs.
Das Ausgabe-JSON benennt jede Begrenzung und die tatsächliche Trefferzahl.

## Auswahl und Aufwand

Für ein erstes Screening beide Listen leeren:

```toml
[base.history]
neurons = []
synapses = []
max_rows = 100000
```

Damit werden nur netzweite und synapsenbezogene Statistiken gespeichert. Nach
Sichtung auffälliger Synapsen dieselbe Konfiguration mit ausgewählten IDs und
einem **neuen Ausgabeordner** erneut ausführen. Die Auswahl beeinflusst weder
Training noch Abruf. `neurons = [64]` erfasst diese Zelle und ihre ein- und
ausgehenden Verbindungen. `synapses = [812]` erfasst diese Verbindung sowie die
zellulären Ereignisse ihrer beiden Endpunkte. Es erfolgt keine rekursive
Ausweitung auf deren Nachbarn. Ungültige oder doppelte IDs werden abgewiesen.

Die Runtime-Audits werden nach jedem 50-ms-Modellfenster entleert; es gibt keine
periodischen Vollkopien des Netzwerks. Maximal 200.000 Quellzeilen pro Fenster
und standardmäßig 20.000.000 pro Trial (Training und beide Abrufzweige) begrenzen
den Beobachtungsaufwand. `history.max_source_rows` setzt dieses
Verarbeitungsbudget ausdrücklich auf 1 bis 100.000.000 Quellzeilen. Zusätzlich begrenzt
`history.max_rows` die gespeicherten Detailereignisse **einschließlich** der
verzögerten Intervallbilanzen auf 1 bis 1.000.000 Zeilen. Eine Überschreitung
macht den Trial ungültig; es werden keine unvollständigen Historien als negative
wissenschaftliche Ergebnisse veröffentlicht. Auch das Screening verarbeitet
die nativen Ereignisse im Arbeitsspeicher pro Fenster und ist nicht kostenlos.

## Inhalt des Records

`details.history` enthält:

- `summary`: netzweite Ereigniszahlen, tatsächliche Gewichtsänderungen je
  Lernhook, wirkungslose verzögerte Anforderungen und Gewichtsänderungen bei
  positiver Eligibility und negativem chemischen Faktor.
- `synapse_summary`: Topologie, Gewichte vor/nach Training, positive/negative
  Änderungsmasse, gegenläufige Masse `min(positive, abs(negative))`, verzögerte
  Anforderungen ohne sichtbare Änderung, Nullfaktoren und Ankünfte im Abruf.
- `neurons`: Rollen, Rezeptorausstattung, Schwelle, intrinsischer Strom und
  struktureller Antrieb der ausgewählten Zellen am Trainingsanfang.
- `events`: gemeinsame chronologische Zeitachse ausgewählter Ereignisse mit
  Mikrosekundenzeit, Phase, Mechanismus, Neuron-/Synapsen-ID und stabiler Ereignis-ID. `branch` trennt die
  tatsächlichen Trajektorien `learned` und `weight_reset`.
  Lernereignisse enthalten Eligibility, Faktor, Gewicht vorher/nachher und
  Rundungsrest; Chemieereignisse Konzentrationen und Rezeptorwirkungen;
  elektrische Ereignisse Ankünfte, kontinuierliche Potenzialänderungen,
  Integration, Schwelle, Refraktärzeit und Spikes.
- `delayed_windows`: exakte native Bilanz pro ausgewählter Synapse und
  Trainingsfenster, einschließlich kleiner Anforderungen ohne f32-Änderung.
- `neuron_index`, `synapse_index`: Positionen in der sortierten Ereignisliste;
  ein Ereignis wird gemeinsam referenziert und nicht pro Zelle dupliziert.

Ereignisse mit gleicher Zeit bewahren die Reihenfolge innerhalb ihres nativen
Auditstroms. Zwischen Lern- und Elektroaudit gibt es keine gemeinsame native
Sequenznummer; deren Gleichzeitigkeit ist keine belegte Kausalreihenfolge.
Unveränderte verzögerte Integrationen liegen nur aggregiert vor, nicht als
Einzelzeitpunkte. Eine angeforderte Pair-STDP-Änderung vor Begrenzung/Rundung
wird von dieser API nicht geliefert und bleibt `null`; tatsächliche Änderungen
sind erfasst. Die exakten einzelnen Prä-/Post-Paardifferenzen werden nicht
behauptet; Ankünfte und Post-Spikes stehen mit ihren nativen Zeitpunkten bereit.

## Wirkung und Aussagegrenzen

Der Adapter prüft nach dem Training die Gleichheit des **vollständigen
Replayzustands** zwischen beobachtetem und unbeobachtetem Modell. Anschließend
setzt er an einer identisch rekonstruierten Kopie ausschließlich die Gewichte
auf ihre Anfangswerte zurück. Die bestehende Prüfung stellt sicher, dass alle
anderen Zustandskomponenten und ausstehenden Ereignisse erhalten bleiben.

Der beobachtete Abruf einschließlich Ruhephasen wird außerdem gegen einen
unbeobachteten Abruf geprüft: gleiche Antworten und gleicher vollständiger
Endzustand. Beide Proben frieren gelernte Parameter ein. Die Lernhistorie
verbindet Trainingsupdates über dieselbe Synapsen-ID mit späteren tatsächlichen
Ankünften, Integrationen und Spikes. `training_update_id` verweist auf das letzte
gespeicherte Gewichtsupdate dieser Verbindung. Beide Abrufzweige werden
protokolliert und lassen sich mit `--branch learned` bzw. `--branch weight_reset`
getrennt abfragen; sie teilen denselben Ausgangszustand und dieselben Eingaben,
können danach aber kausal auseinanderlaufen. `recall_rest` trennt die Ruhephase
vom bewerteten `recall`.

Eine Ankunft auf einer beim Training veränderten Verbindung belegt deren Nutzung,
aber noch keine veränderte Übertragung: Amplituden werden bei der **Emission**
festgehalten; ausstehende Ankünfte können ältere Gewichte tragen. Der Gewichts-
Reset zeigt die gemeinsame Wirkung der Gewichtsänderungen auf Spikefolgen,
Potenziale und Cue-Komplettierung. Er isoliert keine einzelne Lernmarkierung.
Gegenläufige Masse beweist ebenso wenig, dass ein Mechanismus schädlich war.
Die bestehenden Chemie-, Pair-/Drei-Faktor-, Frozen- und Unpaired-Kontrollen
liefern gezielte Vergleiche bei gleichen Seeds und Eingaben; sie sind keine
Mechanismus-Abschaltung aus einem gemeinsamen vollständigen Zwischenzustand.

Entwicklung, Homöostase und synaptische Konkurrenz sind im zugrunde liegenden
Assoziationsassay deaktiviert und im Manifest ausdrücklich als inaktiv markiert.
Es werden dafür keine erfundenen Ereignisse oder Attributionen erzeugt. Ein
Adapter mit aktiven Mechanismen benötigt deren eigene native Beobachtungen und
einen entsprechend definierten Gegenversuch. Dieser Adapter lädt auch keine
beliebigen alten Netzwerkzustände: Er rekonstruiert sein Trainingsprotokoll
reproduzierbar. `passed` bleibt `null`; die Historie ist ein Beobachtungs- und
Diagnosewerkzeug.

[Gezielter Seed-7-Vergleich der stärksten Abschwächungen](LEARNING_HISTORY_FOLLOWUP_RESULTS_20261111.md):
negative Eligibility bei positivem Faktor, einschließlich Pair- und Chemiekontrollen.
