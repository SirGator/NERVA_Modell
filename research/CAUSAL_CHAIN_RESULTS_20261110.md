# Diagnosekette A–D: geprüfter Pilot vom 10.11.2026

[Protokoll und Bedienung](CAUSAL_CHAIN.md) ·
[Feste Konfiguration](config/causal_chain_pilot.toml).

Der geprüfte Lauf liegt unter `results/causal_chain_pilot_20261110_v2/`.
A, B und D wurden einmal als deterministische gemeinsame Kalibrierungen
ausgeführt. C prüft die beiden vorhandenen Modellseeds **17011 und 17027**
mit **256 Processing-Zellen**, acht Trainingswiederholungen und elf Sekunden
Schwarzbildruhe vor jedem Abrufbild.

Die Suite enthält **fünf Records**, zehn Ausführungen mit vollständigem
Replay, 38 native Trainingsdurchläufe einschließlich Rekonstruktionen und
18.832 Hippocampus-Modellschritte. Alle Positiv- und Negativkontrollen bestehen.
Dies ist eine technische Diagnose, keine unabhängige Lernbestätigung.

## A: Ereignispartitionierung kann kleine Updates vollständig verlieren

Der native Drei-Faktor-Hook erhält dieselbe Anfangsmarkierung und denselben
gehaltenen Faktor über insgesamt 100 ms:

| Aufteilung | Angeforderte signierte Änderungsmasse | Tatsächliche Gewichtsänderung |
| --- | ---: | ---: |
| Ein Intervall | 0,00000862143229 | 0,00000864267349 |
| 1.000 Intervalle | 0,00000862143593 | 0 |

Im zweiten Arm werden alle 1.000 kleinen Anforderungen bei der f32-Addition
weggerundet. Die angeforderte Gesamtmasse ist nicht exakt gleich, weil auch der
Tag in f32 fortgeschrieben wird. Diese geringe Differenz erklärt jedoch nicht,
warum sämtliche tatsächlichen Gewichtsschritte ausbleiben.

Weitere feste Fälle trennen Rundung bei kurzem Intervall, kleinen Nichtnullfaktor,
exakten Nullfaktor, Gewichtsbegrenzung, fehlenden Tag und Freeze. Die
repräsentierbare positive Kontrolle verändert das Gewicht. Die beiden
Grenzfälle werden als Begrenzung erkannt und nicht fälschlich als reine
Rundungsverluste gezählt.

Der externe f64-Akkumulator dient nur der Diagnose; die neuronale Lernregel
wurde nicht verändert. Diese Kalibrierung klassifiziert nicht nachträglich
jede Einzelanforderung im großen Modell.

## B: Ein künstlich gespeicherter A→B-Pfad kann selektiv abrufen

Im nativen Drei-Zell-Testnetz bewirkt das vorab festgelegte Gewicht 1,5
nach elf Sekunden Wartezeit einen Spike der Zielzelle B genau eine
Millisekunde nach dem Cue-Spike von A. B erhält keinen direkten Stimulus.

Mit Nullgewicht, gesperrter Verbindung, ohne Cue oder mit Cue an der
unabhängigen dritten Zelle bleibt B still. Die Gewichte bleiben in allen
Kontrollen erhalten. Damit funktionieren Speicherung eines gesetzten Gewichts,
Weiterleitung und selektiver Abruf in diesem künstlichen Testnetz.

Das zertifiziert weder die Pixelkodierung noch den gesamten Hippocampuspfad
und ist kein natürlich gelernter Zusammenhang.

## C: Native Änderungen bleiben unter der beobachteten Abrufwirkung

Alle Gewichtsäste starten mit demselben vollständigen trainierten Checkpoint.
Nur die Gewichte werden nach der festen Formel skaliert und bei 0–2 begrenzt.
Ein Rücktausch stellt die übrige Replaygleichheit sicher.

| Seed | Gain | Installierte Gewichtsänderung L1 | Begrenzte Kanten | Core-Spikes geändert | Motor-Spikes geändert |
| --- | ---: | ---: | ---: | --- | --- |
| 17011 | 1 | 0,000000521541 | 0 | nein | nein |
| 17011 | 100 | 0,0000521541 | 0 | nein | nein |
| 17011 | 1.000 | 0,000521541 | 0 | nein | nein |
| 17027 | 1 | 0,000159204 | 0 | nein | nein |
| 17027 | 100 | 0,0159204 | 0 | nein | nein |
| 17027 | 1.000 | 0,159204 | 0 | ja | ja |

Gain 0 und 10 sind ebenfalls ausgeführt und in der vollständigen Kurve enthalten.
Für Seed 17027 ist 1.000 der erste **getestete** Gain mit Core-Spikewirkung.
Das ist keine exakt bestimmte kontinuierliche Schwelle und kein Nachweis
monotoner Wirkung. Für Seed 17011 reicht das gesamte feste Raster nicht.

Die separaten Mikro-Gewichtsdosen 0, 0,5 und 1 bleiben ohne Zielspike;
1,5 und 2 erzeugen einen. Dieser positive technische Bereich darf nicht auf
das große Netz übertragen werden. Insbesondere bedeutet ein fehlender Effekt
im Modellraster keine unendliche Wirksamkeitsschwelle.

## D: Lokale Information wirkt nur mit geeigneter räumlicher und zeitlicher Verfügbarkeit

Im positiven Arm trifft der Prä-Spike bei 2 ms an der Zielverbindung ein,
die postsynaptische Zelle feuert bei 10 ms und ein künstlicher naher
Modulatorpuls folgt bei 100 ms. Die native Lernmarkierung und der später
gehaltene lokale Faktor führen zu einer Gewichtsänderung von
**0,0000573434**.

Fehlender Prä- oder Post-Spike, Rezeptor-Knockout, weit entfernte
Modulatorquelle und Freeze verhindern die Gewichtsänderung. Der unmarkierte
Nachbar bleibt in allen Fällen unverändert.

Beim Puls nach zehn Sekunden gibt es noch 20 Nichtnull-Anforderungen, aber
keine tatsächliche Gewichtsänderung. „Zeitliche Überlappung vorhanden“ genügt
damit nicht: Die verbleibende Masse muss auch numerisch wirksam sein.

Das Feld `tag_before_release` ist der **gespeicherte Tag nach der ersten
50-ms-Phase**, nicht ein auf die spätere Pulszeit analytisch fortgeschriebener
Wert. Die tatsächlichen Integrationszeiten, Tags und gehaltenen Faktoren
stehen in den lokalen Audittraces; der verspätete Puls trifft einen stark
abgeklungenen Rest.

## Beobachtete Grenze und nächste gezielte Frage

Für beide Modellseeds sind die tatsächlichen Kanalzählmuster der beiden
Vollbilder unterscheidbar. Die Auswertung vergleicht beobachtete
Kanalzählmengen ohne absolute Zeitstempel; bloß nichtnull Aktivität reicht
für dieses Eingangskriterium nicht aus.

Die Core-Zählvektoren der Vollbilder unterscheiden sich ebenfalls. Lokale
Tags und bleibende Gewichtsänderungen sind vorhanden, die nativen
Gewichtsänderungen verändern aber die Spikes der Abrufbilder nicht.
Die erste nicht bestandene **beobachtete** Modellstufe ist deshalb
`native_recall_effect`.

Diese Stufentests belegen noch keine korrekt gelernte Bildassoziation.
Die Mikrotests garantieren ausgewählte Teilfunktionen ihrer eigenen
Schaltungen. Sie ersetzen keinen Nachweis, dass der große Actor einen
Zusammenhang sinnvoll speichert und nutzt.

Der numerische Partitionsvergleich rechtfertigt als nächsten getrennten
Engineering-Versuch eine Konsolidierung mit Erhalt kleiner Beiträge.
Eine solche Korrektur wäre ein eigener Quellstand mit Kontrollvergleich;
sie wurde hier nicht vorgenommen.

## Prüfung, Datenumfang und Reproduktion

Die fünf Roh-Records belegen **1.772.189 Bytes**. Der zentrale Bericht
dupliziert weder die vollständigen Gewichtssnapshots noch die Abrufvektoren.
Die native Binärdatei und Quellen sind einmal pro Gesamtplan archiviert.

Die unabhängige Prüfung berechnet die einteiligen f32-Updatefälle, installierten
Gewichtsdosen und beobachteten Eingangs-/Abrufmerkmale erneut. Maximaler
gefundener numerischer Fehler: **0**. Nicht gespeicherte rohe Auditereignisse
und die ursprünglichen Spikeereignisse hinter Fingerprints werden dabei nicht
rekonstruiert.

Die nativen Endgewichte und die Abrufzählvektoren bei Gain 1 stimmen für beide
Seeds exakt mit dem früheren `three_on`-Arm des Kontrollzentrum-Piloten überein.
Die Diagnoseinstrumentierung hat diese geprüften Modellgrößen nicht verändert.

```bash
./NERVA_Modell/research/run.sh control report NERVA_Modell/research/config/causal_chain_pilot.toml
python3 NERVA_Modell/research/results/causal_chain_pilot_20261110_v2/verify_results.py
```

| Artefakt | SHA-256 |
| --- | --- |
| Gesamtplan | `ead91a3a53eb097641071c44636c27d4192e38a9515720f112b32e79ac0b3898` |
| Rohdatenbeleg | `7fd6877d68e011faebb288435d6524779abee6e668244bc7656cf2b2e2917d32` |
| Quellenarchiv | `f71168f1fb9aa3dadb32ac66e5f29f85ddfd5f6744e6216674a51d63d74b0e95` |

Validierung: **32 Rust-Harness-Tests**, **62 Python-Harness-Tests**,
Rustformat und Clippy mit Warnungen als Fehlern bestehen.
Library, Modellregeln, GPU und VM wurden durch diese Änderung nicht verändert.

[Automatischer Bericht](results/causal_chain_pilot_20261110_v2/CONTROL_REPORT.md) ·
[Messwerte](results/causal_chain_pilot_20261110_v2/control-metrics.csv) ·
[Plan](results/causal_chain_pilot_20261110_v2/control-plan.json) ·
[Rohdatenbeleg](results/causal_chain_pilot_20261110_v2/receipt.json) ·
[Unabhängige Prüfung](results/causal_chain_pilot_20261110_v2/verification.json).
