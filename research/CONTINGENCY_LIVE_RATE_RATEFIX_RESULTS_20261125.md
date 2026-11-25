# Lokale Ratenkorrektur — Ratefix, 25.11.2026

[Protokoll](CONTINGENCY_LIVE_RATE_RATEFIX.md) ·
[Erstlauf 23.11.2026](CONTINGENCY_LIVE_RATE_RESULTS_20261123.md) ·
[Versuchsreihen](dokumentation/README.md)

## Fragestellung

Die lokale Ratenkorrektur in `main/NERVA/src/learning/pair_stdp.rs` zog den
bedingten Erwartungswert des **ungemaskten** Exponentialkerns ab und
dividierte durch `P = 1-e^{-λW}`. Die Lernregel selbst wendet den
**maskierten** Kern `K(t)=M(t)·e^{-t/τ}` an und zieht die Korrektur an
**jedem** Ereignis ab. Dieser Lauf wiederholt den
[Live-Rate-Versuch vom 23.11.2026](CONTINGENCY_LIVE_RATE_RESULTS_20261123.md)
mit derselben Konfiguration und denselben Seeds, aber mit der korrigierten
Korrektur (`masked_kernel_expectation`, keine Division durch `P`).

## Konfiguration

| Parameter | Wert |
| --- | --- |
| Config | `research/config/visual_contingency_live_rate_20261125.toml` |
| Spec | `research/results/visual_contingency_live_rate_20261125/spec.json` |
| Seeds | 34211, 34223, 34237, 34253 |
| Runs | 192 (`4 × 12 × 4`) |
| NERVA-Commit | `c13d9f733d474fe1516f3c170a0440b7c294c535` |
| Modell-Commit | `6feed9923144d916aa8bb4122024cd1e9ba4d53a` |

Die Seeds, Stimuli, Störmodi und Zeitfenster sind identisch zum Lauf vom
23.11.2026. Nur die lokale Ratenkorrektur im Lernpfad wurde geändert.

## Ergebnis

| Arm | A | B | Erhalt | gemeinsam |
| --- | ---: | ---: | ---: | ---: |
| Native | 0/16 | 1/16 | 0/0 | 0 |
| Korrigiert | 0/16 | 1/16 | 0/0 | 0 |

Die Abrufzahlen sind identisch zum Erstlauf. Der einzige B-Treffer bleibt
Seed 34223, Forward, nativ und korrigiert (14 B-Trefferspikes, 3,69×
Gegenstimulus, 2 aktivierte B-Zielzellen). Kein Fall erfüllt A und B
gemeinsam.

### Pro Seed

| Seed | A-Ziele | B-Ziele | Native A | Native B | Korrigiert A | Korrigiert B |
| ---: | --- | --- | ---: | ---: | ---: | ---: |
| 34211 | [7, 8] | [2, 6] | 0/4 | 0/4 | 0/4 | 0/4 |
| 34223 | [4, 6] | [2, 7] | 0/4 | 1/4 | 0/4 | 1/4 |
| 34237 | [6, 4] | [7, 2] | 0/4 | 0/4 | 0/4 | 0/4 |
| 34253 | [5, 8] | [4, 12] | 0/4 | 0/4 | 0/4 | 0/4 |

## Technischer Vergleich

### Innerhalb des neuen Laufs

Die korrigierte Korrektur verändert 29–138 ursprünglich aktive Core→Core-Kanten
gegenüber dem nativen Arm bei identischen Trainings-Core-Spikefolgen. Die
RMS-Paardifferenz liegt bei 1,34·10⁻⁷ bis 7,77·10⁻⁷ (A) und 7,27·10⁻⁷ bis
2,06·10⁻⁶ (B). Das ist dieselbe Größenordnung wie im Erstlauf.

### Gepaarter Vergleich mit dem Erstlauf (23.11.2026)

| Vergleich | RMS | abweichende Kanten | Core-Spikes gleich |
| --- | ---: | ---: | --- |
| alt nativ vs. neu nativ | 2,6·10⁻⁹ … 1,14·10⁻⁶ | 1–79 | meist nein |
| alt korrigiert vs. neu korrigiert | 1,8·10⁻⁹ … 1,07·10⁻⁶ | 5–77 | — |
| neu nativ vs. neu korrigiert | 1,34·10⁻⁷ … 2,06·10⁻⁶ | 29–138 | ja |

Die nativen Arme der beiden Läufe sind nicht bitidentisch: die Maskenberechnung
läuft jetzt über `f64` (`window_mask_real`), was die Gewichtsupdates minimal
verschiebt und in den chaotischen Spikefolgen zu divergierenden Trajektorien
führt (Core-Spikefolgen meist unterschiedlich, aber Kantenmengen meist gleich).
Die Differenz zwischen altem und neuem Korrekturterm ist kleiner als diese
numerische Divergenz.

## Warum der Fix im Modellregime klein ist

Die alte Korrektur teilte durch `P=1-e^{-λW}`. Im Modellregime ist die
geschätzte Partnerereignisrate hoch genug, dass `λW` groß wird und `P≈1`.
Die folgende Tabelle zeigt die alte (bedingte, unmaskierte) und neue
(maskierte, unbedingte) Korrektur für `W=100000 µs`, `τ_fast=20000 µs`:

| λW | alt (bedingt) | neu (maskiert) | Verhältnis alt/neu |
| ---: | ---: | ---: | ---: |
| 0,005 | 0,198957 | 0,000991 | 200,82 |
| 0,05 | 0,201710 | 0,009822 | 20,54 |
| 0,5 | 0,230101 | 0,090440 | 2,54 |
| 1,0 | 0,263009 | 0,166134 | 1,58 |
| 5,0 | 0,503369 | 0,499965 | 1,01 |
| 20,0 | 0,800000 | 0,800000 | 1,00 |

Der Fix beseitigt die starke Überkorrektur bei niedriger Rate. Im hier
gemessenen Regime liegt die Partnerwahrscheinlichkeit nahe 1, daher ist der
praktische Unterschied zur alten Korrektur klein; der beobachtete
Abrufunterschied bleibt null.

## Grenzen

- Nur vier Seeds, ein Gewebesatz, eine feste Dosis.
- Die Korrektur nutzt weiterhin den lokalen Ratenzähler (`count/τ_correction`),
  nicht die wahre Spurverteilung.
- Die nativen Arme divergieren zwischen den Läufen durch die `f64`-Maskenpfad-
  Änderung; der Vergleich ist deshalb nicht bitweise, sondern gepaart nach Seed
  und Störmodus.
- Kein gemeinsamer A/B-Abruf; der Befund betrifft nur die technische
  Korrektheit der lokalen Korrektur, nicht die Abrufkapazität.

## Reproduktion

```bash
cd main/NERVA_Modell

# Lauf ausführen
bash research/results/visual_contingency_live_rate_20261125/run.sh

# Versiegelung prüfen
cd research/results/visual_contingency_live_rate_20261125
sha256sum -c preregistration.sha256

# Deskriptiver Index und gepaarter Vergleich
cd /path/to/main/NERVA_Modell
PYTHONDONTWRITEBYTECODE=1 python3 research/results/visual_contingency_live_rate_20261125/descriptive_index.py \
  research/results/visual_contingency_live_rate_20261125 > /tmp/live-rate-ratefix-index.json
PYTHONDONTWRITEBYTECODE=1 python3 research/results/visual_contingency_live_rate_20261125/rate_correction_comparison.py \
  > /tmp/live-rate-ratefix-compare.json
```
