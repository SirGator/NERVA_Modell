# Lokale Synapsensuche rekrutiert stille Zellen im Training

Die Auswahl nach jüngster präsynaptischer Aktivität erhöhte die
Ankünfte und rekrutierte mehr zuvor stille Zellen als die räumliche
Kontrolle. Das galt in allen acht Seed–Zuordnungs-Vergleichen bei
beiden Entfernungsbedingungen. Bei Erhalt der Kanten wurde ein
zuvor stilles Relais auf einem archivierten terminalen Pfad aktiv.
Kein Cue bestand die Abrufkriterien unter beiden Zuordnungen.
[Versuchsplan](CONTINGENCY_STRUCTURAL_EXPLORATION.md)

## Bildung, Übertragung und Rekrutierung

Alle Arme nutzten dieselben Startzustände, Root-Spikefolgen und
Lernregeln. Die bestehende Kandidatenauswahl blieb in allen Armen
aktiv. Beide Zusatzregeln durften je Ziel eine weitere Kante
bilden und nutzten dieselbe Gelegenheitsschwelle. Die Auswahl nach
jüngster Aktivität und die räumliche Kontrolle realisierten leicht
verschiedene Bildungszahlen. Der Erhalt aller Kanten veränderte
die Netztrajektorie und die nachfolgenden Bildungsgelegenheiten.

| Bildungsregel | Entfernung | Explorative Kanten | Davon erhalten | Positive explorative Ankünfte | Zuvor stille Zellen mit Spike |
| --- | --- | ---: | ---: | ---: | ---: |
| Nativ | Aktiv | 0 | 0 | 0 | 1 |
| Räumlich nächste Quelle | Aktiv | 927 | 45 | 1.850 | 10 |
| Jüngste Aktivität | Aktiv | 961 | 110 | 6.208 | 23 |
| Nativ | Unterdrückt | 0 | 0 | 0 | 4 |
| Räumlich nächste Quelle | Unterdrückt | 928 | 928 | 4.977 | 18 |
| Jüngste Aktivität | Unterdrückt | 992 | 992 | 28.242 | 46 |

Die letzte Spalte zählt Zell–Zuordnungs-Fälle auf vier Seeds. Es
gab 1.720 Fälle, in denen die Zelle im archivierten festen Arm
keinen Spike abgegeben hatte. Bei nativer Entfernung lieferte die
Auswahl nach jüngster Aktivität 6,46 positive Ankünfte pro
gebildeter Kante, die räumliche Kontrolle 2,00. Bei Erhalt lagen
die Werte bei 28,47 und 5,36. In der Auswahl nach jüngster
Aktivität lag der erste Spike in 22 von 23 beziehungsweise
44 von 46 rekrutierten Fällen bei oder nach einer positiven
explorativen Ankunft. Die Reihenfolge beschreibt die beobachtete
Trajektorie.

## Terminale Plastizität

Mit jüngster Aktivität und Erhalt der Kanten feuerte Relais 153 auf
Seed 34123 in der Vorwärtszuordnung siebenmal, jeweils während
des Folgebilds 1. Die neue Kante 111 → 153 lieferte fünf positive
Impulse vor dem ersten Spike, den letzten 305 µs davor. Im
auslösenden Integrationsbatch traf ein vorhandener Eingang von
Zelle 105 ein. Der Mitschnitt lässt offen, wie die Zelle ohne
die frühere explorative Übertragung reagiert hätte.

Der erhaltene Ausgang 153 → 105 übertrug sieben Prä-Impulse und
erzeugte 18 neue STDP-Tag-Änderungen. Sein sichtbares Endgewicht
war identisch mit dem Startgewicht. Die balancierte Kontrolle
rekrutierte dasselbe Relais ebenfalls siebenmal auf Folgebild 1.
Die übrigen fünf Relais–Zuordnungs-Fälle blieben still. Der
Vergleich der Entfernungsarme betrifft das ganze Netz, da in
den Erhaltungsarmen alle Core-Kanten erhalten blieben.

## Selektiver Abruf

Sechs der 96 Cue–Zuordnungs-Vergleiche lösten balancierte
Kontrollen aus. Drei Teiltreffer überstanden diese Kontrolle.
Sie entfielen auf denselben Seed 34157, denselben Cue und dieselbe
umgekehrte Zuordnung in den drei Armen mit erhaltenen Kanten. Alle
drei Antworten bestanden aus demselben Zielspike von Zelle 80
bei 43,668 ms nach Cue-Beginn. Die Regel nach jüngster Aktivität
übertraf die gepaarten Kontrollen in diesem Fall nicht. Damit
blieb der vorab festgelegte Auslöser für eine Akutblockade aus.

Der nächste Versuch kann die verzögerten Gewichtsänderungen auf
dem rekrutierten Pfad bei gleicher Aktivitätshistorie nachspielen.
Die Vorwärts- und balancierten Trainingsverläufe erlauben zu
prüfen, ob Paar-Timing tatsächlich unterschiedliche Information
trägt und ob ein möglicher Unterschied bei der lokalen Umsetzung
in Gewichte verloren geht. Die Befunde betreffen die bekannten
Seeds und die ursprünglichen Cues. Ein Nachweis dauerhafter und
übertragbarer Assoziation steht aus.

## Reproduktion

Alle vier Records wurden vollständig wiederholt. Der Python-Prüfer
rekonstruierte die Entscheidungen über explorative Quellen und
sämtliche dazugehörigen Ankünfte. Die Projektprüfung umfasste
467 Rust-, 111 Harness-Python- und 10 Dashboard-Tests. Ein
unabhängiger Build der archivierten Quellen reproduzierte
den Kurzlauf bytegenau. Modell `9b0d525` und Library `9a3a5fb`
waren vor dem Hauptlauf veröffentlicht.
[Kompakte Daten](dokumentation/visual_contingency_structural_exploration_20261123.json)

Das geprüfte lokale ZIP unter
`research/results/archives/visual_contingency_structural_exploration_20261123.zip`
enthält Eingaben, Rohdaten, Quellen, Binary und Prüfsummen.
Die nachträgliche deskriptive Aggregation und die Zeitreihen der
rekrutierten Relais sind mit `result_index.py` reproduzierbar.

```bash
root=research/results/visual_contingency_structural_exploration_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_exploration_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/result_index.py" "$root" | cmp - "$root/result-index.json"
sha256sum -c research/results/archives/visual_contingency_structural_exploration_20261123.zip.sha256
```
