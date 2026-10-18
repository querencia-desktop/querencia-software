# Querencia Software – Schnittstellen zwischen den Teilprojekten (verbindlich)

Stand 18.10.2026, 20:20. Festgelegt von der Status-Session nach dem G2-I-Bericht
(, Abschnitt 5). Fünf Sessions bauen
parallel; damit es am Ende zusammenpasst, gelten diese Signaturen. Änderungen nur nach Absprache
über die Status-Session „Worker 1-6 Status“ (Nachricht mit Begründung und Vorschlag).

## Aufteilung

| Session | Ordner | Crate | Liefert |
|---|---|---|---|
| SW-1 | `~/projects/querencia-software-paketkit` | `qs_paketkit` (lib) | D-Bus-Schicht zu PackageKit |
| SW-2 | `~/projects/querencia-software-katalog` | `qs_katalog` (lib) | AppStream-Katalog, Kategorien, Suche, Symbole |
| SW-3 | `~/projects/querencia-software-ui` | `querencia-software` (bin) | Fenster, Ansichten, Dialoge; arbeitet gegen die Traits unten mit Attrappen |
| SW-4 | `~/projects/querencia-software-transaktion` | `qs_transaktion` (lib) + `qs-cli` (bin) | Transaktions-Zustandsmaschine, Fehlerabbildung, Wiederanbindung, Kommandozeilenwerkzeug |
| SW-5 | `~/projects/querencia-software-paket` | – | Prüfwerkzeuge (isolierte GUI-Prüfung), Desktop-Datei, Symbole, MetaInfo-Dateien für alle Querencia-Apps, Paketgerüst, Integrationsskript |
| Integration | `~/projects/querencia-software` (dieser Ordner) | Workspace | später: alle Crates zusammenführen, echte Backends einhängen, Abnahme |

Alle Crates: Rust 2021, `gtk4 0.11.5` / `glib 0.22.10` / `gio 0.22.10` (gleiche Versionen wie der
Editor-Prototyp), `zbus 5.19.0`, `tokio 1.52.1` (nur dort, wo zbus es braucht; GTK-Seite nutzt
`glib::MainContext::spawn_local`), `roxmltree 0.21.1`, `quick-xml 0.41.0`, `serde_json 1.0.151`,
`flate2 1.1.10`. **Kein serde_yaml im Cache** – DEP-11-YAML wird mit `appstreamcli convert` nach XML
gewandelt oder mit einem eigenen Minimalparser gelesen (Entscheidung SW-2). Alles `--offline`.
Deutsch zuerst, Bezeichner im Code deutsch oder englisch, aber einheitlich pro Crate.

## Gemeinsame Typen (`qs_typen`, ein kleines Crate, das SW-1 anlegt und alle anderen per `path` einbinden)

```rust
pub type PaketId = String;          // PackageKit-Format "name;version;arch;data"

#[derive(Clone, Debug, PartialEq)]
pub struct Paket {
    pub id: PaketId,
    pub name: String,               // Paketname, z. B. "querencia-bilder"
    pub version: String,
    pub arch: String,
    pub zusammenfassung: String,    // eine Zeile
    pub installiert: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Paketdetails {
    pub paket: Paket,
    pub beschreibung: String,       // mehrzeilig
    pub lizenz: String,
    pub groesse_bytes: u64,         // Download- oder Installationsgröße, je nach Backend
    pub url: Option<String>,
    pub gruppe: String,             // PackageKit-Group als Text
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dringlichkeit { Normal, Sicherheit, Fehlerbehebung, Erweiterung }

#[derive(Clone, Debug, PartialEq)]
pub struct Aktualisierung { pub paket: Paket, pub dringlichkeit: Dringlichkeit, pub neustart_noetig: bool }

#[derive(Clone, Debug, PartialEq)]
pub enum Vorgang { Installieren, Entfernen, Aktualisieren, QuellenAuffrischen }

#[derive(Clone, Debug, PartialEq)]
pub enum Fortschritt {
    Gestartet { vorgang: Vorgang, pakete: Vec<PaketId> },
    Prozent(u8),                                  // 0..=100, 101 = unbekannt (PackageKit-Konvention) → als None behandeln
    Status(String),                               // PackageKit-Status als deutscher Text ("Lade herunter …")
    PaketFortschritt { id: PaketId, prozent: u8 },
    WartetAufSperre,                              // cannot-get-lock → Oberfläche bietet Warten/Abbrechen
    BrauchtNeustart,
    Fertig,
    Abgebrochen,
    Fehler(Fehler),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fehler {
    NichtBerechtigt,                 // polkit abgelehnt oder kein Agent
    KeinAgent,                       // polkit-Agent fehlt (G2-I 1.4): eigene Meldung mit Hinweis
    SperreBelegt(String),
    NetzFehlt,
    PaketNichtGefunden(String),
    AbhaengigkeitenKonflikt(String),
    DaemonWeg,                       // PackageKit beendet/neu gestartet während der Transaktion
    Abgebrochen,
    Sonstiges { code: String, text: String },  // PackageKit-ErrorCode + Details
}

#[derive(Clone, Debug, PartialEq)]
pub struct Anwendung {               // aus AppStream
    pub id: String,                  // Component-ID, z. B. "org.querencia.Bilder"
    pub name: String,                // lokalisiert (de, sonst C)
    pub zusammenfassung: String,
    pub beschreibung: String,        // bereits als einfacher Text oder sehr kleines Markup (p, ul/li) – Entscheidung SW-2, dokumentieren
    pub paketname: Option<String>,   // <pkgname>; None = ohne Paket (z. B. Snap-Komponenten, werden ausgeblendet)
    pub kategorien: Vec<Kategorie>,
    pub symbol: Symbol,
    pub bildschirmfotos: Vec<String>,// URLs, v1 zeigt sie nicht an, aber sammelt sie
    pub lizenz: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Symbol { Thema(String), Datei(std::path::PathBuf), Keines }

/// Feste Kategorienliste nach GNOME 2 (Reihenfolge = Anzeige in der Seitenleiste).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kategorie { Zubehoer, Buero, Grafik, Internet, Multimedia, Spiele, Entwicklung, Bildung, System, Sonstiges }
impl Kategorie { pub fn name(self) -> &'static str; pub fn alle() -> &'static [Kategorie]; }
```

## Trait `Paketquelle` (liefert SW-1, konsumiert SW-3 und SW-4)

```rust
#[async_trait::async_trait(?Send)]   // falls async_trait nicht im Cache: Box<dyn Future> von Hand, gleiche Semantik
pub trait Paketquelle {
    async fn suchen(&self, text: &str, nur_installiert: bool) -> Result<Vec<Paket>, Fehler>;
    async fn paket_nach_name(&self, name: &str) -> Result<Option<Paket>, Fehler>;   // installierte Version bevorzugt
    async fn details(&self, id: &PaketId) -> Result<Paketdetails, Fehler>;
    async fn aktualisierungen(&self) -> Result<Vec<Aktualisierung>, Fehler>;
    /// Startet eine Transaktion; `melden` wird für jeden Fortschritt aufgerufen (auf dem Aufrufer-Thread).
    /// Rückgabe = Abbruchgriff; `abbrechen()` darf jederzeit gerufen werden.
    fn ausfuehren(&self, vorgang: Vorgang, ids: Vec<PaketId>, melden: Box<dyn Fn(Fortschritt)>) -> Box<dyn Abbruch>;
    /// Nur simulieren: was würde zusätzlich installiert/entfernt? (allow_deps=false, autoremove=false)
    async fn simulieren(&self, vorgang: Vorgang, ids: Vec<PaketId>) -> Result<Simulation, Fehler>;
    async fn laufende_transaktionen(&self) -> Result<Vec<String>, Fehler>;     // Objektpfade, für Wiederanbindung
}
pub trait Abbruch { fn abbrechen(&self); }
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Simulation { pub installieren: Vec<Paket>, pub entfernen: Vec<Paket>, pub aktualisieren: Vec<Paket>, pub download_bytes: u64 }
```

Regeln: `ausfuehren` ruft `melden` nur auf dem Thread auf, der `ausfuehren` gerufen hat (SW-1 löst das
über einen Kanal + `glib::MainContext` oder liefert einen `Receiver`, dann ist `melden` ein Adapter).
Entfernen immer mit `allow_deps=false`, nie `autoremove=true`. Jede Transaktion setzt
`SetHints(["locale=de_DE.UTF-8","interactive=true","background=false"])`.

## Trait `Katalog` (liefert SW-2, konsumiert SW-3)

```rust
pub trait Katalog {
    fn laden(&mut self) -> Result<Statistik, String>;                      // liest alle Quellen, baut Index; darf 1–2 s dauern, wird im Hintergrund-Thread gerufen
    fn kategorie(&self, k: Kategorie) -> Vec<&Anwendung>;                   // sortiert nach Name
    fn suchen(&self, text: &str) -> Vec<&Anwendung>;                        // Name, Zusammenfassung, Schlüsselwörter, Paketname; Treffer nach Gewicht
    fn nach_paket(&self, paketname: &str) -> Option<&Anwendung>;
    fn nach_id(&self, id: &str) -> Option<&Anwendung>;
    fn symbol_pfad(&self, a: &Anwendung, groesse: u32) -> Option<std::path::PathBuf>;   // 48 oder 64; Cache unter /var/lib/swcatalog/icons bzw. ~/.cache
    fn alle(&self) -> &[Anwendung];
}
pub struct Statistik { pub komponenten: usize, pub mit_paket: usize, pub mit_symbol: usize, pub deutsch: usize, pub quellen: Vec<String> }
```

Quellen in dieser Reihenfolge: `/var/lib/swcatalog/{yaml,xml}`, `/usr/share/swcatalog/{yaml,xml}`,
`/usr/share/metainfo/*.xml`, `~/.local/share/metainfo/*.xml` (hier liegen später die MetaInfo-Dateien
der Querencia-Apps aus SW-5). Nur Komponenten vom Typ `desktop-application` (und `console-application`
für die Expertenliste) mit `<pkgname>`; Snap/Flatpak-Bundles ausblenden. Locale: `de` → `de_DE` → `C`.

## Trait `Transaktionsmonitor` (liefert SW-4, konsumiert SW-3)

```rust
pub struct Auftrag { pub vorgang: Vorgang, pub pakete: Vec<Paket>, pub anwendung: Option<Anwendung> }
#[derive(Clone, Debug, PartialEq)]
pub enum Zustand { Simuliert(Simulation), WartetAufBestaetigung, Laeuft { prozent: Option<u8>, status: String }, WartetAufSperre { seit: std::time::Instant }, Fertig, Fehlgeschlagen(Fehler), Abgebrochen }
pub trait Transaktionsmonitor {
    fn starten(&self, auftrag: Auftrag, quelle: std::rc::Rc<dyn Paketquelle>, melden: Box<dyn Fn(Zustand)>) -> TransaktionsGriff;
    fn wieder_anbinden(&self, quelle: std::rc::Rc<dyn Paketquelle>, melden: Box<dyn Fn(Zustand)>) -> Vec<TransaktionsGriff>;  // nach App-Neustart
}
pub struct TransaktionsGriff { pub id: u64 } impl TransaktionsGriff { pub fn abbrechen(&self); pub fn bestaetigen(&self); pub fn sperre_erneut_versuchen(&self); }
```

Der Monitor kapselt: Simulation vor jedem Entfernen (Oberfläche zeigt, was mitgeht, Benutzer
bestätigt), Wiederholung bei `SperreBelegt` (Oberfläche fragt), `DaemonWeg` → Wiederanbindung über
`laufende_transaktionen`, `KeinAgent` → Erklärtext „Es fehlt der Berechtigungsdialog der Sitzung
(polkit-Agent); G2-K liefert ihn“, Zeitbegrenzungen, Logging nach `~/.local/state/querencia/software.log`.

## Oberfläche (SW-3), Pflichtumfang v1

Fenster „Querencia Software“ (App-ID `org.querencia.Software`, SSD-Rahmen, 960×640 Standard),
Menüleiste Datei (Aktualisierungen prüfen, Beenden), Bearbeiten (Suchen), Ansicht (Alle Programme /
Installiert / Aktualisierungen / Expertenliste), Hilfe (Info mit Querencia-Q). Links Seitenleiste
mit den 10 Kategorien + „Installiert“ + „Aktualisierungen (n)“; rechts Liste (Symbol 48, Name,
Zusammenfassung, Zustand installiert/verfügbar, Knopf), Suchfeld oben mit Sofortsuche; Detailseite
(Symbol 64, Name, Zusammenfassung, Beschreibung, Version, Größe, Lizenz, Homepage, Knopf
Installieren/Entfernen, Zurück); Fortschrittsdialog nicht modal, Fenster bleibt bedienbar;
Statuszeile. Attrappen `AttrappeQuelle` und `AttrappeKatalog` in `src/attrappen.rs` mit 30
Beispielanwendungen und simulierten Transaktionen (2 s), damit die GUI ohne Daemon prüfbar ist.
Umschaltung echte/Attrappen-Backends über Umgebungsvariable `QS_ATTRAPPE=1`.

## Gemeinsame Regeln

- Jede Session: Git von Anfang an im eigenen Ordner, kein Push, `cargo build -j2` unter `nice`,
  `df -h /` vorher, `target/` nach Abschluss leeren. Nichts live installieren; **keine
  Paketinstallation** (auch nicht als Test über PackageKit!) – Transaktionen nur gegen PackageKits
  `test_succeed`/`test_fail`/`test_spawn`-Backends einer privaten `packagekitd`-Instanz auf eigenem
  Bus (SW-1 baut dieses Prüfgerüst und beschreibt es in `PRUEFGERUEST.md`, SW-4 nutzt es) oder gegen
  die Attrappen. Lesende Aufrufe (Suche, Details, GetUpdates) gegen den echten PackageKit sind erlaubt.
- Symbolregel: nie 48x48 zusammen mit Scalable installieren.
- Am Ende jeder Session: Bericht , Übergabe
   mit sieben Abschnitten, Meldung an „Worker 1-6 Status“, anhalten.
