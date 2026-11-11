# Lernhistorie: gezielter Vergleich auf Seed 7

Die drei stärksten Abschwächungen des beobachteten 256-Zellen-Laufs entstehen
an den ausgewählten Synapsen aus **negativer Eligibility bei positivem
chemischem Faktor**. Eine Umkehr positiver Markierungen durch negative Faktoren
ist für diese Änderungen nicht der Auslöser. Dieselben drei Verbindungen werden
auch mit Pair-STDP ohne Chemie schwächer. Kein geprüfter Arm erzielt einen
gewichtsbedingten Abrufgewinn.

## Versuchsaufbau

Unverändert: Seed 7, 256 Verarbeitungsneuronen, acht Trainingswiederholungen,
11 Sekunden Ruhe vor jedem Abrufbild, native Eingaben und biologische Parameter.
Auswahl: Synapsen 52 (13 → 91), 57 (14 → 84), 360 (90 → 84), einschließlich
zellulärer Ereignisse ihrer Endpunkte. Die drei Synapsen wurden **nach Sichtung
des ursprünglichen Ergebnisses** gewählt; dies ist eine gezielte Diagnose,
keine unabhängige Bestätigung.

Fünf Gruppen, jeweils mit vollständigem Replay und internem Gewichts-Reset:
Drei-Faktor-STDP mit Akkumulation, dieselbe Regel ohne Chemie, Pair-STDP mit und
ohne Chemie, eingefrorene Gewichte mit Chemie. Es wurden keine Lernraten,
chemischen Konzentrationen, Schwellen oder Gewichte künstlich erhöht.

[Konfiguration](config/learning_history_followup_seed7.toml) ·
[Reproduzierbare Auswertung](analysis/learning_history_followup.py) ·
[Maschinenlesbare Ergebnisse und Rohdatenhashes](dokumentation/learning_history_followup_seed7_20261111.json).

Die fünf nativen Records bestehen die Integritätsprüfung des Harness. Alle
Eingabefingerabdrücke stimmen überein (`01b0a80c25c2f305`); Anfangsgewichte und
Synapsenidentitäten sowie die archivierten Parameter der ausgewählten Zellen
sind identisch. Die vollständigen Modellzustände bleiben durch die Beobachtung
unverändert. Die netz- und synapsenweiten Zusammenfassungen von `three_on`
reproduzieren den ursprünglichen 256-Zellen-Lauf exakt, trotz anderer Detailauswahl.

## Netzweiter Vergleich

`L1 Enddelta` summiert die absoluten Differenzen zwischen Anfangs- und
Endgewicht. `Negativer Massenanteil` bezieht sich dagegen auf **alle ausgeführten
Einzeländerungen**, einschließlich gegenläufiger Beiträge.

| Gruppe | Gewichtsereignisse | Geänderte Synapsen | L1 Enddelta | Negativer Massenanteil | Abrufgewinn A / B |
| --- | ---: | ---: | ---: | ---: | ---: |
| Drei-Faktor + Chemie | 1.392 | 34 | 0,0000493377 | 98,526 % | 0 / 0 |
| Drei-Faktor ohne Chemie | 0 | 0 | 0 | nicht definiert | 0 / 0 |
| Pair-STDP + Chemie | 4.345 | 60 | 0,0496669 | 50,539 % | 0 / 0 |
| Pair-STDP ohne Chemie | 4.338 | 60 | 0,0497450 | 50,568 % | 0 / 0 |
| Frozen + Chemie | 0 | 0 | 0 | nicht definiert | 0 / 0 |

Kein Arm verändert gegenüber seinem eigenen Gewichts-Reset die Core- oder
Motor-Spikefolge im bewerteten Abruf. Pair-STDP erzeugt ein etwa 1.007-mal größeres
L1-Enddelta als Drei-Faktor-STDP mit Chemie, aber ebenfalls keinen Abrufgewinn.
Das belegt keine hinreichende Lernrichtung und keine funktionale Assoziation.

Die eingefügte Auswertung des ursprünglichen Drei-Faktor-Laufs wird bestätigt:
1.320 Synapsen insgesamt, 26 abgeschwächte und acht verstärkte; je 17 veränderte
Sensor→Core- und erregende Core→Core-Verbindungen; keine veränderten hemmenden
oder Core→Motor-Verbindungen. Nur zwei Synapsen enthalten gegenläufige
Einzeländerungen. Die gegenläufige Masse beträgt 1,04308 × 10⁻⁷ gegenüber
4,95464 × 10⁻⁵ gesamter absoluter Änderungsmasse.

Bei Pair-STDP sind gegenläufige Updates dagegen häufig (58 Synapsen mit Chemie,
57 ohne). Das ist ein Unterschied zwischen den Regeln, kein Nachweis, dass diese
Gegenläufigkeit das fehlende Abruflernen verursacht.

## Die drei Verbindungen

| Synapse | Drei-Faktor + Chemie | Drei-Faktor ohne Chemie | Pair + Chemie | Pair ohne Chemie |
| --- | ---: | ---: | ---: | ---: |
| 52: 13 → 91 | −0,00000804663 | 0 | −0,00139558 | −0,00139558 |
| 57: 14 → 84 | −0,00000917912 | 0 | −0,00126064 | −0,00127476 |
| 360: 90 → 84 | −0,00000680983 | 0 | −0,000857919 | −0,000481516 |

Die nativen Diagnosen liefern für alle **648 ausgeführten verzögerten Updates**
dieser drei Verbindungen denselben Vorzeichenfall:

```text
negatives Eligibility-Integral × positiver gehaltener Lernfaktor
→ negative angeforderte Gewichtsänderung
→ negative ausgeführte Gewichtsänderung
```

Auf Synapse 52 entfallen 135, auf 57 weitere 152 und auf 360 weitere 361 solche
Updates. Positive/negative Aktualanforderungen und Rundungsreste werden getrennt
geführt; die Auswertung prüft das native Produkt bei unveränderter Lernrate 1
sowie die Bilanz zwischen ausgeführten Änderungen und Endgewichten.

Auch die **vollständigen Intervallbilanzen einschließlich unsichtbarer Updates**
zeigen bereits vor chemischer Gewichtung eine stark negative Eligibility:

| Synapse | Negativer Anteil am absoluten Eligibility-Zeitintegral | Negativer Anteil an angeforderter Änderungsmasse |
| --- | ---: | ---: |
| 52 | 99,772 % | 99,843 % |
| 57 | 99,601 % | 99,697 % |
| 360 | 100 % | 100 % |

Diese ungewichteten Integrale sind eine Beschreibung derselben aufgezeichneten
Trajektorie, kein simulierter Gegenversuch mit konstantem Lernfaktor.
Die Chemie verstärkt hier die relative negative Gewichtung leicht; sie erzeugt
nicht erst das negative Vorzeichen dieser Integrale.

## Konkrete Zeitfolge: Synapse 57

Die Zeitangaben stammen aus demselben nativen Lern-Auditstrom:

| Zeit | Ereignis | Beobachtete Eligibility / Änderung |
| --- | --- | --- |
| 34,336 ms | Post-Spike-Hook | Eligibility von 0 auf +0,000100000 |
| 42,669 ms | Prä-Ankunfts-Hook | von +0,0000983472 auf +0,0000192370 |
| 51,002 ms | Prä-Ankunfts-Hook | von +0,0000189191 auf −0,0000332345 |
| 59,336 ms | Prä-Ankunfts-Hook | von −0,0000326851 auf −0,0000670657 |
| 89,336 ms | Erstes sichtbares verzögertes Gewichtsupdate | negativer Tag, positiver Faktor +0,0285880 |

Beim letzten Ereignis beträgt das integrierte Eligibility-Signal
−2,81305 × 10⁻⁷ Sekunden; der angeforderte Beitrag ist −8,04196 × 10⁻⁹.
Zusammen mit dem vorherigen Rest −2,68993 × 10⁻⁸ wird eine f32-Änderung von
−5,96046 × 10⁻⁸ veröffentlicht. Der neue Rest ist +2,46634 × 10⁻⁸.
Die größere sichtbare Einzeländerung erklärt sich durch die vorher akkumulierten
Beiträge. Dieselben ersten vier Eligibility-Hooks erscheinen auch ohne Chemie;
dort entsteht kein Gewichtsupdate, weil der dritte Faktor fehlt.

Bei den Zielzellen 84 und 91 ist der **gemessene** chemische Lernfaktor in allen
Trainingsproben positiv: 0,00188105–0,0458930 bzw. 0,00159917–0,0332696.
Gemessenes Dopamin ist jeweils null, Acetylcholin und Noradrenalin sind vorhanden.
Dies beschreibt die beobachteten Konzentrationen und Rezeptorwirkungen und
belegt keine allgemeine Aussage über die Nützlichkeit einzelner Botenstoffe.

## Einordnung und Grenzen

Der konkrete Verdacht einer schädlichen Vorzeichenumkehr durch die Chemie wird
an diesen drei Synapsen **nicht bestätigt**. Die Zeitregel erzeugt schon negative
Markierungen; deren zeitliche Integration ist überwiegend negativ. Pair-STDP
zeigt dort ebenfalls Abschwächung ohne Chemie. Der Unterschied zwischen sofortigen
Pair-Updates und zeitlich integrierter Eligibility bleibt für die Größenordnung
und Verteilung der Änderungen relevant.

Die Chemie-Abschaltung schaltet neben dem Lernfaktor auch die übrigen chemischen
Wirkungen ab und kann dadurch Spikezeiten verändern. Der Pair-Vergleich allein
isoliert deshalb keine reine Multiplikation mit einem Faktor. Der Gewichts-Reset
isoliert die **gemeinsame** Wirkung der gelernten Gewichte innerhalb eines Arms;
er beweist keinen schädlichen Einzelmechanismus. Ausstehende Ankünfte können
weiterhin bei früheren Gewichten emittiert worden sein.

Die fehlenden Motor-Lernfaktorproben und unveränderten Core→Motor-Gewichte werden
bestätigt, erklären den fehlenden Bildabruf aber nicht allein. Entwicklung,
Homöostase und Konkurrenz sind wie zuvor deaktiviert. Es wurde ein Seed geprüft;
keine Mechanik wurde entfernt und keine biologische Regel verändert.

## Ablage und erneute Prüfung

Alle Rohdaten bleiben jetzt im Projekt, nicht ausschließlich unter `/tmp`:

```text
research/results/learning_history_followup_seed7_20261111/
  records/v00000-s7-i000.json   Drei-Faktor + Chemie
  records/v00000-s7-i001.json   Drei-Faktor ohne Chemie
  records/v00000-s7-i002.json   Pair + Chemie
  records/v00000-s7-i003.json   Pair ohne Chemie
  records/v00000-s7-i004.json   Frozen + Chemie
  followup-analysis.json       vollständige abgeleitete Auswertung
  source-code.zip, runner     vor Ausführung gesicherter Programmstand
  pre-run-provenance.json     vor Ausführung erfasste Hashes
  verification-receipt.json   Prüfbeleg mit SHA-256 der Artefakte
```

Die ursprüngliche Detailhistorie für Neuron 64 liegt zusätzlich unter
`research/results/learning_history_seed7_baseline_20261111/`, unverändert kopiert
und mit einem eigenen Importbeleg. Die Rohdaten sind wie üblich von Git
ausgeschlossen; Konfiguration, Auswertungscode, kompakte Ergebnisse und dieser
Bericht werden versioniert. Unter `research/results/archives/` liegen ZIP-Pakete
beider vollständiger Ablagen mit separaten SHA-256-Dateien.

Befehle ab `DSVLM/main/`:

```bash
./NERVA_Modell/research/run.sh report NERVA_Modell/research/config/learning_history_followup_seed7.toml --binary NERVA_Modell/research/results/learning_history_followup_seed7_20261111/runner
python3 NERVA_Modell/research/analysis/learning_history_followup.py NERVA_Modell/research/results/learning_history_followup_seed7_20261111
./NERVA_Modell/research/run.sh history NERVA_Modell/research/results/learning_history_followup_seed7_20261111/records/v00000-s7-i000.json --synapse 57 --phase training --limit 20
```

Die Analyse verändert die Rohdaten nicht. Der native `report`-Aufruf prüft die
Records gegen Manifest, Identität und FNV-Prüfsumme; der zusätzliche Prüfbeleg
bindet die Originaldateien kryptografisch an die Auswertung.
