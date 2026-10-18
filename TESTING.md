# Testen

```bash
rustfmt --edition 2021 --check src/main.rs
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline     # Logik und Attrappen, ohne Display
```

GUI nur isoliert (eigener headless labwc 0.9.8, eigener D-Bus, eigenes
XDG-Profil, virtuelle Eingaben; nie die laufende Sitzung):

```bash
bash werkzeug/bauen.sh                        # einmalig: qeingabe, qausgabe
bash werkzeug/gui-pruefung.sh 1024x768        # Release-Binary, sonst Debug
bash werkzeug/gui-pruefung.sh 1920x1080
```

Ergebnis je Auflösung in `nachweis/gui/<Auflösung>/`: `ergebnis.txt`,
Bilder, `software.log` (Debug-Ausgabe `QS_DEBUG=1`, daraus werden Zustände
und Positionen gelesen), `software.err`, `lauf-*.log`. Wiederholungsläufe in
`nachweis/gui/wiederholung/`.

Das Szenario (`werkzeug/szenario.sh`, 60 Fälle) prüft: Start und
Fenstergröße, Katalog und Statuszeile, Seitenleiste, Kategorienwechsel (alle
10), Sofortsuche mit Verzögerung, Detailseite (Eingabetaste, Escape,
Paketdetails, Homepage), Installieren bis Fertig mit Zustandswechsel,
Bedienbarkeit während eines Vorgangs, Abbrechen, Entfernen mit Bestätigung
(Abbruch und Bestätigung), belegte Sperre mit Warten, KeinAgent,
NichtBerechtigt, Aktualisierungen, Menü Ansicht/Datei/Bearbeiten,
Expertenliste mit Suche und Details, Info-Dialog dreimal mit Tastaturfokus
geschlossen (Eingabetaste, Escape, Alt+S), zweite Instanz, Strg+Q.

Menüs werden per Maus bedient (Tastaturnavigation in Popover-Menüs ist im
headless labwc unzuverlässig, Befund G2-H).
