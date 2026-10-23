# Testen

*Seit 0.1.1 unter `docs/`; Pfade wie `werkzeug/…`, `data/…`, `src/…` gelten relativ zu `programm/`.*

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

Das Szenario (`werkzeug/szenario.sh`, 67 Fälle) prüft: Start und
Fenstergröße, eingebettete Symbole, Symbolwahl (Thema, Datei, ohne Symbol,
unbekannter Themenname → Ersatzsymbol), Katalog und Statuszeile, Seitenleiste, Kategorienwechsel (alle
10), Sofortsuche mit Verzögerung, Detailseite (Eingabetaste, Escape,
Paketdetails, Homepage), Installieren bis Fertig mit Zustandswechsel,
Bedienbarkeit während eines Vorgangs, Abbrechen, Entfernen mit Bestätigung
(Abbruch und Bestätigung), belegte Sperre mit Warten, KeinAgent,
NichtBerechtigt, Aktualisierungen, Menü Ansicht/Datei/Bearbeiten,
Expertenliste mit Suche und Details, Info-Dialog mit Programmsymbol und Q dreimal mit Tastaturfokus
geschlossen (Eingabetaste, Escape, Alt+S), zweite Instanz, Strg+Q.

Menüs werden per Maus bedient (Tastaturnavigation in Popover-Menüs ist im
headless labwc unzuverlässig).

## Prüfprotokoll (`QS_DEBUG=1`)

Tab-getrennte Zeilen auf stdout, Koordinaten relativ zum Fensterinhalt
(x, y, Breite, Höhe). Das Szenario liest nur dieses Protokoll.

| Zeile | Bedeutung |
|---|---|
| `katalog n mit_paket mit_symbol deutsch quellen` | nach `Katalog::laden` |
| `symbole fenstersymbol true/false ersatzsymbol true/false` | Ressourcen im Symbolthema |
| `ansicht kennung n suchtext` | Liste neu aufgebaut (`alle`, `kategorie:<Name>`, `installiert`, `aktualisierungen`, `experten`) |
| `status text` | Statuszeile |
| `seitentext aktualisierungen n` | Zähler in der Seitenleiste |
| `details titel zustand knopf` / `details zu` | Detailseite geöffnet / geschlossen |
| `detailsymbol titel quelle`, `zeilensymbol i titel quelle` | Bildquelle `thema:<name>`, `datei:<pfad>` |
| `paketdetails name bytes` | Paketdetails nachgeladen |
| `transaktion gestartet vorgang paket` / `laeuft prozent status` / `simuliert pakete` / `wartet-auf-bestaetigung` / `wartet-auf-sperre` / `fertig paket` / `abgebrochen` / `fehler Fehler` / `wieder-angebunden n` | Transaktionszustände |
| `dialog kennung` / `dialog zu kennung` | `info`, `entfernen`, `sperre`, `kein-agent`, `nicht-berechtigt`, `fehler`, `laeuft`, ab 0.1.3 `netz-offline-frage` (vor dem Vorgang, „Trotzdem versuchen“), `netz-offline` (nach `no-network`), `netz-pk` |
| `netz pruefe` / `netz online/offline stufe pk monitor archiv host` / `netz vor-vorgang` / `netz frage-offline` / `netz trotzdem` / `netz meldung-entfernt` | Erreichbarkeitsprüfung (0.1.3): Start, Ergebnis (`stufe` = `packagekit`, `netzmonitor`, `archiv` oder `-`; `monitor` `an`/`aus`/`-`; `archiv` `erreichbar`/`nicht-erreichbar`/`-`), frische Prüfung vor einem Vorgang, Frage gezeigt, „Trotzdem versuchen“ gewählt, veraltete `no-network`-Meldung nach dem Wechsel auf online entfernt |
| `netzstatus text` | Netzwerktext rechts in der Statuszeile |
| `info-symbole programmsymbol querencia-q` | Inhalt des Info-Dialogs |
| `snap-huellen namen` | ab 0.1.6: nach jedem Paketstand die ausgeblendeten Snap-Hüllen (Paketnamen, durch Komma getrennt; leer, wenn keine) |
| `uri adresse` | ab 0.1.5: Adresse, die das Programm im Standardprogramm öffnen lässt (Hilfe → Online-Hilfe, Hilfe → Fehler melden, Website im Info-Dialog) |
| `fokus suche` | Suchfeld hat den Fokus |
| `zeiger x y` | Zeigerbewegung (Kalibrierung) |
| `layout anfang` … `layout ende` | Block mit `menue i …`, `seite kennung …`, `suche …`, `liste …`, `zeile i titel …`, `knopf i text …`, `zurueck …`, `aktion text …`, `homepage …`, `abbrechen …`, `statuszeile …`, `netzfeld …`, `netzknopf …`, `fenster w h`, `toplevel typ titel sichtbar w h` |

SW-5 hat in `werkzeug/faelle/FAELLE.md` (Repo `querencia-software-paket`)
ein eigenes Format für seinen Platzhalter vorgeschlagen. Laut FAELLE.md gilt
das Format von SW-3; die Angleichung der SW-5-Fälle an diese Tabelle ist
Sache der Integration.

## Erreichbarkeitsprüfung (0.1.3)

Unit-Tests in `src/netz.rs`: Stufenlogik mit Attrappen (welche Stufe gefragt
wird, Entscheidung, Texte), Quellenwahl aus `sources.list`/`*.list`/deb822
`*.sources` (Distribution vor Fremdquelle, `Enabled: no`, `deb-src`),
URI-Zerlegung, `os-release`, HTTP-Statuszeile.

Isoliert mit Attrappen (`QS_ATTRAPPE=1`, `QS_NETZ_ATTRAPPE`), aus dem
Workspace-Wurzelverzeichnis:

```bash
bash werkzeug/gui-netz.sh online    1024x768   # online,online,online
bash werkzeug/gui-netz.sh nm-archiv 1024x768   # offline,offline,online (Pflichtprobe B2)
bash werkzeug/gui-netz.sh offline   1024x768   # offline,offline,offline, am Ende online
```

Ergebnis in `nachweis/gui/netz-<Fall>-<Auflösung>/`, Bilder zusätzlich unter
. Echte Stufen ohne Fenster:
`querencia-software --netz-pruefen` (fragt alle drei Stufen, nur lesend;
`QS_APT_QUELLE=http://192.0.2.1/ubuntu` prüft das Zeitlimit).

## Dunkle Darstellung (0.1.4)

`src/style.css` setzt nur noch Farben, die das Thema nicht hat; Ansichtsgrund
und Ränder kommen über `@theme_base_color` und `@borders` aus dem Thema, die
übrigen dunklen Werte stehen im Block `@media (prefers-color-scheme: dark)` am
Ende. Damit der Block
greift, bindet `fenster::load_css` den eigenen `GtkCssProvider` an
`gtk-interface-color-scheme`; `main` meldet die Einstellung nach dem Start
einmal (GTK 4.22 wendet das dunkle Thema beim Start sonst nicht an).

Geprüft wird isoliert mit einem Ersatz für das Settings-Portal
(`org.freedesktop.appearance color-scheme` 0 bzw. 1, im Lauf umschaltbar),
eigenem HOME, Bus und labwc: Bilder hell gegen die Live-Ablage (pixelgleich,
de/en, 1024×768 und 1920×1080), dunkel beim Start und nach einem Wechsel im
Lauf, Kontrast dunkel AA. Werkzeuge und Ergebnisse liegen im
Integrationsordner der Version (`werkzeug/gui-dunkel.sh`,
`szenario-dunkel.sh`, `regression.sh`, `vergleich.py`,
`kontrast-software.py`, `messung-bindung.sh`).
