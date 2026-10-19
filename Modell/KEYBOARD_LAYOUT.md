# Vollständige deutsche Tastatur

Das laufende Modell besitzt 105 motorische Tastenkanäle für eine deutsche
ISO-Tastatur. Die bisherigen Kanäle 0–4 bleiben W, A, S, D und Leertaste.
Die übrigen Kanäle ergänzen die gesamte Tastatur.

```text
Esc    F1 F2 F3 F4   F5 F6 F7 F8   F9 F10 F11 F12       Druck Rollen Pause
^  1 2 3 4 5 6 7 8 9 0 ß ´  Rücktaste                 Einfg Pos1 Bild↑   Num / * -
Tab  Q W E R T Z U I O P Ü +  Enter                     Entf Ende Bild↓   7 8 9 +
Fest   A S D F G H J K L Ö Ä #  Enter                                    4 5 6 +
Shift  < Y X C V B N M , . -   Shift                          ↑          1 2 3 Enter
Strg Win Alt      Leertaste      AltGr Win Menü Strg          ← ↓ →        0 , Enter
```

## Verwendung

```bash
./NERVA_Modell/Modell/run-demo.sh layout
./NERVA_Modell/Modell/run-demo.sh demo --keyboard full
./NERVA_Modell/Modell/run-demo.sh demo --keyboard wasd
./NERVA_Modell/Modell/run-demo.sh smoke --keyboard full
```

`full` ist die Standardbelegung. Jeder Lauf speichert `keyboard-layout.csv`
mit Kanal, deutscher Beschriftung und QMP-Code. Die Rohzählungen pro Kanal
stehen in `motor-activity.csv`; ausgeführte Tasten stehen in `trace.csv`.

Shift, Strg, Alt/AltGr und Windows links/rechts sind unabhängige Modifier.
Der Motor-Decoder kann sie mit einer eindeutig gewählten normalen Taste
kombinieren. Stille lässt los. Bei nichtleeren Zählgleichständen entscheidet
standardmäßig der eindeutig früheste erste Spike; exakt gleiche erste
Spikezeiten wählen keine normale Taste. `--motor-decoder count` enthält sich
bei jedem Zählgleichstand. Modifier können selbstständig gehalten werden.
Der Adapter drückt Modifier zuerst und lässt sie nach den normalen Tasten los.

QMP sendet physische Tastaturpositionen. Die deutschen Namen setzen eine
passend konfigurierte deutsche Tastaturbelegung im Gast voraus: `Z` wird
über die QMP-Position `y` gesendet, `Y` über `z`, und beispielsweise `Ä`
über `apostrophe`. Es wird kein Unicode-Text eingefügt und die Gastbelegung
wird nicht verändert. Der kleine BIOS-Demogast verwendet seine BIOS-Belegung
und verarbeitet für Bewegung A, D und Leertaste. BIOS-Tastenkombinationen
wie Strg+Alt+Entf können den Gast neu starten; die Demo-QEMU-Instanz bleibt dabei aktiv.

Die Codes folgen [QEMUs QKeyCode-Definition](https://www.qemu.org/docs/master/interop/qemu-qmp-ref.html#enum-qkeycode).

## Kanalbelegung

| Kanal | Taste | QMP-Code |
|---|---|---|
| 0 | W | `w` |
| 1 | A | `a` |
| 2 | S | `s` |
| 3 | D | `d` |
| 4 | Leertaste | `spc` |
| 5 | Esc | `esc` |
| 6 | F1 | `f1` |
| 7 | F2 | `f2` |
| 8 | F3 | `f3` |
| 9 | F4 | `f4` |
| 10 | F5 | `f5` |
| 11 | F6 | `f6` |
| 12 | F7 | `f7` |
| 13 | F8 | `f8` |
| 14 | F9 | `f9` |
| 15 | F10 | `f10` |
| 16 | F11 | `f11` |
| 17 | F12 | `f12` |
| 18 | Druck | `sysrq` |
| 19 | Rollen | `scroll_lock` |
| 20 | Pause | `pause` |
| 21 | ^ | `grave_accent` |
| 22 | 1 | `1` |
| 23 | 2 | `2` |
| 24 | 3 | `3` |
| 25 | 4 | `4` |
| 26 | 5 | `5` |
| 27 | 6 | `6` |
| 28 | 7 | `7` |
| 29 | 8 | `8` |
| 30 | 9 | `9` |
| 31 | 0 | `0` |
| 32 | ß | `minus` |
| 33 | ´ | `equal` |
| 34 | Rücktaste | `backspace` |
| 35 | Tab | `tab` |
| 36 | Q | `q` |
| 37 | E | `e` |
| 38 | R | `r` |
| 39 | T | `t` |
| 40 | Z | `y` |
| 41 | U | `u` |
| 42 | I | `i` |
| 43 | O | `o` |
| 44 | P | `p` |
| 45 | Ü | `bracket_left` |
| 46 | + | `bracket_right` |
| 47 | Enter | `ret` |
| 48 | Feststelltaste | `caps_lock` |
| 49 | F | `f` |
| 50 | G | `g` |
| 51 | H | `h` |
| 52 | J | `j` |
| 53 | K | `k` |
| 54 | L | `l` |
| 55 | Ö | `semicolon` |
| 56 | Ä | `apostrophe` |
| 57 | # | `backslash` |
| 58 | Shift links | `shift` |
| 59 | < | `less` |
| 60 | Y | `z` |
| 61 | X | `x` |
| 62 | C | `c` |
| 63 | V | `v` |
| 64 | B | `b` |
| 65 | N | `n` |
| 66 | M | `m` |
| 67 | , | `comma` |
| 68 | . | `dot` |
| 69 | - | `slash` |
| 70 | Shift rechts | `shift_r` |
| 71 | Strg links | `ctrl` |
| 72 | Windows links | `meta_l` |
| 73 | Alt | `alt` |
| 74 | AltGr | `alt_r` |
| 75 | Windows rechts | `meta_r` |
| 76 | Menü | `menu` |
| 77 | Strg rechts | `ctrl_r` |
| 78 | Einfg | `insert` |
| 79 | Pos1 | `home` |
| 80 | Bild auf | `pgup` |
| 81 | Entf | `delete` |
| 82 | Ende | `end` |
| 83 | Bild ab | `pgdn` |
| 84 | Pfeil hoch | `up` |
| 85 | Pfeil links | `left` |
| 86 | Pfeil runter | `down` |
| 87 | Pfeil rechts | `right` |
| 88 | Num | `num_lock` |
| 89 | Num / | `kp_divide` |
| 90 | Num * | `kp_multiply` |
| 91 | Num - | `kp_subtract` |
| 92 | Num 7 | `kp_7` |
| 93 | Num 8 | `kp_8` |
| 94 | Num 9 | `kp_9` |
| 95 | Num + | `kp_add` |
| 96 | Num 4 | `kp_4` |
| 97 | Num 5 | `kp_5` |
| 98 | Num 6 | `kp_6` |
| 99 | Num 1 | `kp_1` |
| 100 | Num 2 | `kp_2` |
| 101 | Num 3 | `kp_3` |
| 102 | Num Enter | `kp_enter` |
| 103 | Num 0 | `kp_0` |
| 104 | Num , | `kp_decimal` |
