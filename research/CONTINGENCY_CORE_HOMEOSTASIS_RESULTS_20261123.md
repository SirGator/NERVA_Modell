# Core-Homöostase rekrutiert die stillen Relais auf festem Graphen nicht

Alle acht zuvor stillen terminalen Kante–Zuordnung-Paare blieben
in beiden Core-Homöostase-Armen ohne Prä-Ankunft und ohne neue
Paaränderung. Bei ihren drei Quellzellen erhöhte der Regler den
strukturellen Antrieb um **0,346 bis 0,356** in Modelleinheiten.
Der intrinsische Strom blieb bei jeder Aktualisierung gleich.
[Versuchsplan](CONTINGENCY_CORE_HOMEOSTASIS.md)

## Wirkung im Training

Auf vier bekannten Seeds wurde der lokale Core-Regler während
vier Sekunden Training ein- oder ausgeschaltet und die Entwicklungs-
Eligibility jeweils erhalten oder vorher gelöscht. Jede der acht
Trainingsrekonstruktionen je aktivem Arm enthielt 10.240 belegte
Regleränderungen an allen 256 Core-Zellen. Außerhalb des Core wurden
keine Änderungen durch den Regler beobachtet. Die ausgeschalteten
Arme reproduzierten die archivierten Spikes und Endgewichte.

| Endpunkt | Ohne Core-Homöostase | Mit Core-Homöostase |
| --- | ---: | ---: |
| Task-Prä-Ankünfte auf den acht zuvor stillen Paaren | 0 | 0 |
| Neue Tag-Änderungen aus Task-Paaren auf diesen acht Paaren | 0 | 0 |
| Task-Prä-Ankünfte über alle 23 terminalen Paare | 277 | 259 |
| Neue Tag-Änderungen aus Task-Paaren über alle 23 Paare | 294 | 289 |
| Terminale Paare mit mindestens einer neuen Tag-Änderung | 15 | 15 |
| Core-Spikes im gesamten Training | 9.267 | 8.606 |

Die Tabellenwerte sind innerhalb jeder Homöostase-Bedingung für
erhaltene und entfernte Start-Eligibility gleich. Die Interaktion
der beiden Faktoren auf diese Zählungen ist null. Unterschiede in
den Gewichtssummen bleiben separat messbar. Die postsynaptischen
Zellen der acht zuvor stillen Paare feuerten in allen Armen.
Der fehlende neue Paarbeitrag an diesen Kanten lag an der
ausgebliebenen Prä-Ankunft.

Bei den drei stillen Quellen 93, 103 und 153 auf Seed 34123 erhöhte
die Regelung den strukturellen Antrieb. In der implementierten Regel
kann dieser Anstieg nur auftreten, wenn die lokale Eingangsschätzung
unter ihrem Soll liegt. Der Regler beantwortet diesen Zustand mit einem
strukturellen Suchsignal. Der Versuch hielt die Topologie eingefroren.
Damit war der Mechanismus, der auf dieses Signal mit einer
Synapsenneubildung reagieren kann, im Versuch deaktiviert.

## Selektiver Abruf

Beim anschließenden Transfer der sichtbaren Core→Core-Gewichte
war **keiner der 16 Cue–Zuordnungs-Vergleiche je Arm** erfolgreich.
Alle 32 Cue-Spikefolgen der zwei Core-Homöostase-Arme stimmten mit
den jeweils passenden ausgeschalteten Armen überein. Der Abruf
verwendete den gemeinsamen entwickelten Ausgangszustand. Die
trainierten intrinsischen Zustände wurden im Gewichtsvergleich
dadurch kontrolliert. Ein möglicher Abruf bei Übernahme dieser
Zustände wurde hier nicht untersucht.

Der nächste Versuch lässt die vorhandene, lokale Strukturplastizität
während des Trainings auf das Suchsignal reagieren. Der eingefrorene
Graph bleibt Kontrolle. Die Analyse soll prüfen, ob neu gebildete
Eingänge die stillen Quellen tatsächlich rekrutieren und ob ein
möglicher Abruf kausal von den neuen Verbindungen abhängt. Das
strukturelle Signal allein belegt noch weder Synapsenbildung noch
eigenständig erworbenes Gedächtnis.

## Prüfung und Reproduktion

Vier Records mit vollständigem Replay. Beide archivierten Quellstände
reproduzierten den Kurzlauf bytegenau. Die Projektprüfung umfasste
464 Rust-, 99 Harness-Python- und 10 Dashboard-Tests. Die zwei neuen
gezielten Python-Tests wurden zusätzlich im Kontingenz-Testsatz
ausgeführt. Der unabhängige Prüfer berechnet die Herkunft der Ankünfte,
die zugehörigen Prä-/Post-Ereignisse, die Kontraste und den selektiven
Abruf aus den Rohdaten. Die nachträgliche Auditfassung prüft zusätzlich
die archivierte Ankunftsfolge und gibt die bereits verwendeten
Ruhezählungen explizit aus. Die primären Zahlen sind identisch.
[Kompakte Daten](dokumentation/visual_contingency_core_homeostasis_20261123.json)

Modell `3840cc2` und Library `9a3a5fb` waren vor dem Hauptlauf
veröffentlicht. Das geprüfte lokale ZIP unter
`research/results/archives/visual_contingency_core_homeostasis_20261123.zip`
enthält Eingaben, Rohdaten, Quellen, Binary und Prüfsummen.

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
root=research/results/visual_contingency_core_homeostasis_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
python3 "$root/contingency_homeostasis_analysis.py" "$root"
python3 "$root/audit_entry.py" "$root"
python3 "$root/result_index.py" "$root"
sha256sum -c research/results/archives/visual_contingency_core_homeostasis_20261123.zip.sha256
```
