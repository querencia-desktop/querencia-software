//! Attrappen für den Betrieb ohne PackageKit und AppStream (`QS_ATTRAPPE=1`
//! oder solange die echten Backends fehlen).
//!
//! `AttrappeQuelle`: 30 Anwendungspakete (8 installiert, 3 mit
//! Aktualisierung) plus 4 reine Pakete für die Expertenliste; simulierte
//! Transaktionen mit Fortschritt 0 → 100 in 2 s. Drei Pakete zeigen die
//! Fehlerwege:
//!
//! | Paket          | Verhalten                                                        |
//! |----------------|------------------------------------------------------------------|
//! | `gnome-chess`  | erster Versuch: `WartetAufSperre`, dann `SperreBelegt`; zweiter Versuch gelingt |
//! | `wireshark`    | `Fehler::NichtBerechtigt`                                         |
//! | `virt-manager` | `Fehler::KeinAgent`                                               |
//!
//! `AttrappeKatalog`: 30 Anwendungen über alle 10 Kategorien, deutsche und
//! (drei) englische Texte, Themensymbole aus Adwaita/hicolor, die auf der VM
//! existieren; ein Symbol als Datei (htop).
//!
//! INTEGRATION: `main.rs` wählt zwischen Attrappen und echten Backends; die
//! echten liegen in `qs_paketkit` (Quelle) und `qs_katalog` (Katalog).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use qs_typen::{
    Aktualisierung, Anwendung, Dringlichkeit, Fehler, Fortschritt, Kategorie, Paket, PaketId,
    Paketdetails, Symbol, Vorgang,
};

use crate::schnittstellen::{Abbruch, Katalog, Paketquelle, Simulation, Statistik};

/// Dauer einer simulierten Transaktion.
pub const TRANSAKTIONSDAUER: Duration = Duration::from_millis(2000);
const SCHRITT: Duration = Duration::from_millis(100);

pub const PAKET_SPERRE: &str = "gnome-chess";
pub const PAKET_NICHT_BERECHTIGT: &str = "wireshark";
pub const PAKET_KEIN_AGENT: &str = "virt-manager";

const ARCH: &str = "amd64";
const QUELLE: &str = "ubuntu-resolute-universe";

// ---------------------------------------------------------------- Daten

/// Rohdaten einer Anwendung: id, Name, Zusammenfassung, Paketname, Version,
/// Kategorien, Symbol, Größe in KiB, Lizenz, Homepage, Beschreibung, installiert.
struct Roh {
    id: &'static str,
    name: &'static str,
    zusammenfassung: &'static str,
    paket: &'static str,
    version: &'static str,
    kategorien: &'static [Kategorie],
    symbol: Symbol,
    kib: u64,
    lizenz: &'static str,
    homepage: &'static str,
    beschreibung: &'static str,
    installiert: bool,
}

use Kategorie as K;

fn thema(n: &str) -> Symbol {
    Symbol::Thema(n.into())
}

fn rohdaten() -> Vec<Roh> {
    vec![
        // Zubehör
        Roh { id: "org.gnome.gedit", name: "Texteditor", zusammenfassung: "Texte bearbeiten", paket: "gedit", version: "48.1-1", kategorien: &[K::Zubehoer], symbol: thema("accessories-text-editor-symbolic"), kib: 5_120, lizenz: "GPL-2.0-or-later", homepage: "https://gedit-technology.github.io", beschreibung: "gedit ist ein einfacher Texteditor für den GNOME-Desktop.\n\nEr bietet Syntaxhervorhebung, mehrere Reiter, Suchen und Ersetzen und Erweiterungen.", installiert: true },
        Roh { id: "org.gnome.Calculator", name: "Taschenrechner", zusammenfassung: "Berechnungen durchführen", paket: "gnome-calculator", version: "48.0-1", kategorien: &[K::Zubehoer], symbol: thema("accessories-calculator-symbolic"), kib: 7_300, lizenz: "GPL-3.0-or-later", homepage: "https://apps.gnome.org/Calculator", beschreibung: "Der GNOME-Taschenrechner beherrscht einfache, wissenschaftliche und finanzmathematische Berechnungen sowie Umrechnungen.", installiert: false },
        Roh { id: "org.gnome.Charmap", name: "Zeichentabelle", zusammenfassung: "Sonderzeichen nachschlagen und einfügen", paket: "gucharmap", version: "16.0.2-1", kategorien: &[K::Zubehoer], symbol: thema("accessories-character-map-symbolic"), kib: 2_800, lizenz: "GPL-3.0-or-later", homepage: "https://wiki.gnome.org/Apps/Gucharmap", beschreibung: "Die Zeichentabelle zeigt alle Unicode-Zeichen nach Schriftsystem und erlaubt das Kopieren einzelner Zeichen.", installiert: true },
        // Büro
        Roh { id: "org.libreoffice.LibreOffice.writer", name: "LibreOffice Writer", zusammenfassung: "Dokumente erstellen und bearbeiten", paket: "libreoffice-writer", version: "4:25.8.1-1", kategorien: &[K::Buero], symbol: thema("x-office-document"), kib: 38_400, lizenz: "MPL-2.0", homepage: "https://www.libreoffice.org", beschreibung: "Writer ist die Textverarbeitung von LibreOffice: Briefe, Berichte, Bücher mit Formatvorlagen, Tabellen, Bildern und Serienbriefen.", installiert: true },
        Roh { id: "org.libreoffice.LibreOffice.calc", name: "LibreOffice Calc", zusammenfassung: "Spreadsheet application", paket: "libreoffice-calc", version: "4:25.8.1-1", kategorien: &[K::Buero], symbol: thema("x-office-spreadsheet"), kib: 41_000, lizenz: "MPL-2.0", homepage: "https://www.libreoffice.org", beschreibung: "Calc is the spreadsheet program of LibreOffice. It offers functions, charts, pivot tables and compatibility with Excel files.\n\n(Keine deutsche Übersetzung im Katalog.)", installiert: false },
        Roh { id: "org.libreoffice.LibreOffice.impress", name: "LibreOffice Impress", zusammenfassung: "Präsentationen erstellen", paket: "libreoffice-impress", version: "4:25.8.1-1", kategorien: &[K::Buero], symbol: thema("x-office-presentation"), kib: 22_500, lizenz: "MPL-2.0", homepage: "https://www.libreoffice.org", beschreibung: "Impress erstellt Bildschirmpräsentationen mit Folienvorlagen, Übergängen und Notizen.", installiert: false },
        // Grafik
        Roh { id: "org.gimp.GIMP", name: "GIMP", zusammenfassung: "Bilder bearbeiten und Grafiken erstellen", paket: "gimp", version: "3.0.4-1", kategorien: &[K::Grafik], symbol: thema("applications-graphics-symbolic"), kib: 120_000, lizenz: "GPL-3.0-or-later", homepage: "https://www.gimp.org", beschreibung: "GIMP ist ein freies Bildbearbeitungsprogramm für Fotoretusche, Bildkomposition und Bilderstellung.\n\nEbenen, Masken, Filter, Skripte und viele Dateiformate.", installiert: true },
        Roh { id: "org.inkscape.Inkscape", name: "Inkscape", zusammenfassung: "Vektorgrafiken zeichnen", paket: "inkscape", version: "1.4.2-1", kategorien: &[K::Grafik, K::Buero], symbol: thema("x-office-drawing"), kib: 95_000, lizenz: "GPL-3.0-or-later", homepage: "https://inkscape.org", beschreibung: "Inkscape ist ein Vektorgrafikprogramm für Logos, Diagramme, Illustrationen und SVG-Dateien.", installiert: false },
        Roh { id: "org.gnome.eog", name: "Bildbetrachter", zusammenfassung: "Bilder anzeigen und durchblättern", paket: "eog", version: "47.0-2", kategorien: &[K::Grafik], symbol: thema("image-x-generic"), kib: 3_600, lizenz: "GPL-2.0-or-later", homepage: "https://wiki.gnome.org/Apps/EyeOfGnome", beschreibung: "Eye of GNOME zeigt Bilder in vielen Formaten, mit Diaschau, Zoom und einfacher Drehung.", installiert: false },
        // Internet
        Roh { id: "org.mozilla.firefox", name: "Firefox", zusammenfassung: "Im Internet surfen", paket: "firefox", version: "143.0+build1-0ubuntu1", kategorien: &[K::Internet], symbol: thema("firefox"), kib: 260_000, lizenz: "MPL-2.0", homepage: "https://www.mozilla.org/firefox", beschreibung: "Firefox ist ein schneller, sicherer Webbrowser mit Reitern, Erweiterungen, Synchronisation und Tracking-Schutz.", installiert: true },
        Roh { id: "org.mozilla.Thunderbird", name: "Thunderbird", zusammenfassung: "E-Mails lesen und schreiben", paket: "thunderbird", version: "1:140.3.0-1", kategorien: &[K::Internet], symbol: thema("x-office-addressbook"), kib: 230_000, lizenz: "MPL-2.0", homepage: "https://www.thunderbird.net", beschreibung: "Thunderbird ist ein E-Mail-Programm mit Kalender, Adressbuch, Newsgroups und Chat.", installiert: false },
        Roh { id: "com.transmissionbt.Transmission", name: "Transmission", zusammenfassung: "Dateien über BitTorrent laden", paket: "transmission-gtk", version: "4.0.6-1", kategorien: &[K::Internet, K::Sonstiges], symbol: thema("folder-download"), kib: 4_200, lizenz: "GPL-2.0-or-later", homepage: "https://transmissionbt.com", beschreibung: "Transmission ist ein schlanker BitTorrent-Client mit Verschlüsselung, Bandbreitenbegrenzung und Web-Oberfläche.", installiert: false },
        // Multimedia
        Roh { id: "org.videolan.VLC", name: "VLC Media Player", zusammenfassung: "Videos und Musik abspielen", paket: "vlc", version: "3.0.21-6", kategorien: &[K::Multimedia], symbol: thema("video-x-generic"), kib: 48_000, lizenz: "GPL-2.0-or-later", homepage: "https://www.videolan.org", beschreibung: "VLC spielt nahezu alle Audio- und Videoformate, DVDs, Streams und Webcams ab.", installiert: true },
        Roh { id: "org.gnome.Rhythmbox3", name: "Rhythmbox", zusammenfassung: "Musiksammlung verwalten und abspielen", paket: "rhythmbox", version: "3.4.8-1", kategorien: &[K::Multimedia], symbol: thema("audio-x-generic"), kib: 16_000, lizenz: "GPL-2.0-or-later", homepage: "https://wiki.gnome.org/Apps/Rhythmbox", beschreibung: "Rhythmbox verwaltet die Musiksammlung, spielt Internetradio und Podcasts und synchronisiert tragbare Geräte.", installiert: false },
        Roh { id: "org.audacityteam.Audacity", name: "Audacity", zusammenfassung: "Audio aufnehmen und bearbeiten", paket: "audacity", version: "3.7.3-1", kategorien: &[K::Multimedia], symbol: thema("microphone"), kib: 52_000, lizenz: "GPL-2.0-or-later", homepage: "https://www.audacityteam.org", beschreibung: "Audacity ist ein Mehrspur-Audioeditor für Aufnahme, Schnitt, Effekte und Export in viele Formate.", installiert: false },
        // Spiele
        Roh { id: "org.gnome.Chess", name: "Schach", zusammenfassung: "Gegen den Rechner oder Freunde Schach spielen", paket: PAKET_SPERRE, version: "48.0-1", kategorien: &[K::Spiele], symbol: thema("applications-games-symbolic"), kib: 2_900, lizenz: "GPL-3.0-or-later", homepage: "https://wiki.gnome.org/Apps/Chess", beschreibung: "GNOME Schach bietet ein 2D- und 3D-Brett, Spielstärken und Partien gegen Schachprogramme wie GNU Chess.\n\n(Attrappe: erster Versuch trifft auf eine belegte Paketsperre.)", installiert: false },
        Roh { id: "org.gnome.Mines", name: "Minen", zusammenfassung: "Verborgene Minen auf einem Spielfeld aufspüren", paket: "gnome-mines", version: "40.1-2", kategorien: &[K::Spiele], symbol: thema("input-gaming"), kib: 1_900, lizenz: "GPL-3.0-or-later", homepage: "https://wiki.gnome.org/Apps/Mines", beschreibung: "Das klassische Minensuchspiel mit drei Feldgrößen und eigener Größenwahl.", installiert: false },
        Roh { id: "net.supertuxkart.SuperTuxKart", name: "SuperTuxKart", zusammenfassung: "A 3D open-source kart racing game", paket: "supertuxkart", version: "1.4+ds-2", kategorien: &[K::Spiele], symbol: thema("applications-games-symbolic"), kib: 820_000, lizenz: "GPL-3.0-or-later", homepage: "https://supertuxkart.net", beschreibung: "SuperTuxKart is a free kart racing game featuring Tux and friends, with story mode, multiplayer and many tracks.", installiert: false },
        // Entwicklung
        Roh { id: "org.vim.gvim", name: "Vim (GTK)", zusammenfassung: "Texteditor für Programmierer", paket: "vim-gtk3", version: "2:9.1.1800-1", kategorien: &[K::Entwicklung], symbol: thema("gvim"), kib: 9_800, lizenz: "Vim", homepage: "https://www.vim.org", beschreibung: "gVim ist die grafische Fassung von Vim mit Menüleiste, Maus und Zwischenablage.", installiert: true },
        Roh { id: "org.gnome.meld", name: "Meld", zusammenfassung: "Dateien und Verzeichnisse vergleichen", paket: "meld", version: "3.22.3-1", kategorien: &[K::Entwicklung], symbol: thema("text-x-script"), kib: 3_100, lizenz: "GPL-2.0-or-later", homepage: "https://meldmerge.org", beschreibung: "Meld zeigt Unterschiede zwischen zwei oder drei Dateien oder Verzeichnissen und hilft beim Zusammenführen, auch mit Git.", installiert: false },
        Roh { id: "org.gnome.Builder", name: "Builder", zusammenfassung: "An IDE for GNOME", paket: "gnome-builder", version: "48.2-1", kategorien: &[K::Entwicklung], symbol: thema("applications-engineering-symbolic"), kib: 64_000, lizenz: "GPL-3.0-or-later", homepage: "https://apps.gnome.org/Builder", beschreibung: "Builder is an integrated development environment for GNOME with Flatpak support, debugger, profiler and language servers.", installiert: false },
        // Bildung
        Roh { id: "org.stellarium.Stellarium", name: "Stellarium", zusammenfassung: "Planetarium für den Schreibtisch", paket: "stellarium", version: "25.2-1", kategorien: &[K::Bildung], symbol: thema("applications-science-symbolic"), kib: 390_000, lizenz: "GPL-2.0-or-later", homepage: "https://stellarium.org", beschreibung: "Stellarium zeigt den Sternenhimmel in Echtzeit, mit Sternbildern, Planeten, Nebeln und Teleskopsteuerung.", installiert: false },
        Roh { id: "org.kde.gcompris", name: "GCompris", zusammenfassung: "Lernspiele für Kinder von 2 bis 10", paket: "gcompris-qt", version: "25.1-1", kategorien: &[K::Bildung, K::Spiele], symbol: thema("ebook-reader"), kib: 210_000, lizenz: "AGPL-3.0-or-later", homepage: "https://gcompris.net", beschreibung: "GCompris enthält über 180 Lernaktivitäten: Lesen, Rechnen, Naturwissenschaft, Geografie und Spiele.", installiert: false },
        Roh { id: "net.ankiweb.Anki", name: "Anki", zusammenfassung: "Mit Karteikarten lernen", paket: "anki", version: "2.1.15+dfsg-4", kategorien: &[K::Bildung], symbol: thema("accessories-dictionary-symbolic"), kib: 18_000, lizenz: "AGPL-3.0-or-later", homepage: "https://apps.ankiweb.net", beschreibung: "Anki ist ein Karteikartenprogramm mit verteilter Wiederholung, Bildern, Audio und Synchronisation.", installiert: false },
        // System
        Roh { id: "org.htop.htop", name: "htop", zusammenfassung: "Prozesse im Terminal beobachten", paket: "htop", version: "3.4.1-1", kategorien: &[K::System], symbol: Symbol::Datei(PathBuf::from("/usr/share/icons/hicolor/scalable/apps/htop.svg")), kib: 450, lizenz: "GPL-2.0-or-later", homepage: "https://htop.dev", beschreibung: "htop ist ein interaktiver Prozessbetrachter für das Terminal mit Sortierung, Filter und Baumansicht.\n\n(Attrappe: Symbol als Datei.)", installiert: true },
        Roh { id: "org.virt-manager.virt-manager", name: "Virtuelle Maschinen", zusammenfassung: "Virtuelle Maschinen verwalten", paket: PAKET_KEIN_AGENT, version: "1:5.0.0-4", kategorien: &[K::System], symbol: thema("qemu"), kib: 8_700, lizenz: "GPL-2.0-or-later", homepage: "https://virt-manager.org", beschreibung: "Virt-Manager verwaltet virtuelle Maschinen über libvirt: anlegen, starten, Konsole, Speicher und Netzwerke.\n\n(Attrappe: liefert „KeinAgent“.)", installiert: false },
        Roh { id: "org.wireshark.Wireshark", name: "Wireshark", zusammenfassung: "Netzwerkverkehr mitschneiden und auswerten", paket: PAKET_NICHT_BERECHTIGT, version: "4.4.9-1", kategorien: &[K::System, K::Internet], symbol: thema("network-workgroup"), kib: 110_000, lizenz: "GPL-2.0-or-later", homepage: "https://www.wireshark.org", beschreibung: "Wireshark analysiert Netzwerkprotokolle Paket für Paket, live oder aus Mitschnittdateien.\n\n(Attrappe: liefert „NichtBerechtigt“.)", installiert: false },
        // Sonstiges
        Roh { id: "org.gnome.FontManager", name: "Schriftverwaltung", zusammenfassung: "Schriften ansehen und verwalten", paket: "font-manager", version: "0.9.1-1", kategorien: &[K::Sonstiges], symbol: thema("font-x-generic"), kib: 3_400, lizenz: "GPL-3.0-or-later", homepage: "https://fontmanager.github.io", beschreibung: "Font Manager zeigt installierte Schriften mit Vorschau, Sammlungen und Vergleich.", installiert: false },
        Roh { id: "system-config-printer.desktop", name: "Drucker", zusammenfassung: "Drucker einrichten", paket: "system-config-printer", version: "1.5.18-2", kategorien: &[K::Sonstiges, K::System], symbol: thema("printer"), kib: 2_600, lizenz: "GPL-2.0-or-later", homepage: "https://github.com/OpenPrinting/system-config-printer", beschreibung: "Druckerverwaltung für CUPS: Drucker hinzufügen, Treiber wählen, Warteschlangen und Testseiten.", installiert: false },
        Roh { id: "org.gnome.seahorse.Application", name: "Passwörter und Schlüssel", zusammenfassung: "Schlüssel und Passwörter verwalten", paket: "seahorse", version: "47.0.1-1", kategorien: &[K::Sonstiges], symbol: thema("application-certificate"), kib: 4_900, lizenz: "GPL-2.0-or-later", homepage: "https://wiki.gnome.org/Apps/Seahorse", beschreibung: "Seahorse verwaltet PGP- und SSH-Schlüssel sowie Passwörter im Schlüsselbund.", installiert: false },
    ]
}

/// Reine Pakete ohne AppStream-Komponente (nur in der Expertenliste).
const REINE_PAKETE: [(&str, &str, &str); 4] = [
    (
        "libqs-demo1",
        "0.3.1-1",
        "Beispielbibliothek für die Expertenliste (Attrappe)",
    ),
    (
        "python3-attrappe",
        "1.0.0-2",
        "Python-Modul ohne grafische Oberfläche (Attrappe)",
    ),
    (
        "qs-cli",
        "0.1.0-1",
        "Kommandozeilenwerkzeug von Querencia Software (SW-4)",
    ),
    (
        "fonts-attrappe",
        "2025.10-1",
        "Schriftpaket ohne Anwendung (Attrappe)",
    ),
];

fn paket_id(name: &str, version: &str, installiert: bool) -> PaketId {
    format!(
        "{name};{version};{ARCH};{}",
        if installiert { "installed" } else { QUELLE }
    )
}

// ---------------------------------------------------------------- AttrappeQuelle

struct Eintrag {
    paket: Paket,
    details: Paketdetails,
}

/// Veränderlicher Zustand der Attrappe; Timer simulierter Transaktionen
/// halten einen eigenen `Rc` darauf.
struct Innen {
    eintraege: Vec<Eintrag>,
    aktualisierungen: Vec<Aktualisierung>,
    /// Versuche je Paketname (für die Sperre-Attrappe).
    versuche: HashMap<String, u32>,
}

impl Innen {
    fn setze_installiert(&mut self, id: &str, installiert: bool) {
        let name = id.split(';').next().unwrap_or(id).to_string();
        // Nach einer Aktualisierung gilt die neue Version.
        let neue_version = if installiert {
            self.aktualisierungen
                .iter()
                .find(|a| a.paket.name == name)
                .map(|a| a.paket.version.clone())
        } else {
            None
        };
        for e in self.eintraege.iter_mut().filter(|e| e.paket.name == name) {
            if let Some(v) = &neue_version {
                e.paket.version = v.clone();
            }
            e.paket.installiert = installiert;
            e.paket.id = paket_id(&e.paket.name, &e.paket.version, installiert);
            e.details.paket = e.paket.clone();
        }
        if neue_version.is_some() {
            self.aktualisierungen.retain(|a| a.paket.name != name);
        }
    }
}

pub struct AttrappeQuelle {
    innen: Rc<RefCell<Innen>>,
}

impl Default for AttrappeQuelle {
    fn default() -> Self {
        Self::new()
    }
}

impl AttrappeQuelle {
    pub fn new() -> Self {
        let mut eintraege = Vec::new();
        for r in rohdaten() {
            let paket = Paket {
                id: paket_id(r.paket, r.version, r.installiert),
                name: r.paket.into(),
                version: r.version.into(),
                arch: ARCH.into(),
                zusammenfassung: r.zusammenfassung.into(),
                installiert: r.installiert,
            };
            let details = Paketdetails {
                paket: paket.clone(),
                beschreibung: r.beschreibung.into(),
                lizenz: r.lizenz.into(),
                groesse_bytes: r.kib * 1024,
                url: Some(r.homepage.into()),
                gruppe: r.kategorien[0].name().into(),
            };
            eintraege.push(Eintrag { paket, details });
        }
        for (name, version, zusammenfassung) in REINE_PAKETE {
            let paket = Paket {
                id: paket_id(name, version, false),
                name: name.into(),
                version: version.into(),
                arch: ARCH.into(),
                zusammenfassung: zusammenfassung.into(),
                installiert: false,
            };
            let details = Paketdetails {
                paket: paket.clone(),
                beschreibung: format!("{zusammenfassung}.\n\nKeine AppStream-Komponente."),
                lizenz: "GPL-3.0-or-later".into(),
                groesse_bytes: 256 * 1024,
                url: None,
                gruppe: "Sonstiges".into(),
            };
            eintraege.push(Eintrag { paket, details });
        }
        let mut aktualisierungen = Vec::new();
        for (name, dringlichkeit, neustart) in [
            ("firefox", Dringlichkeit::Sicherheit, true),
            ("vlc", Dringlichkeit::Fehlerbehebung, false),
            ("htop", Dringlichkeit::Normal, false),
        ] {
            let alt = eintraege
                .iter()
                .find(|e| e.paket.name == name)
                .expect("Aktualisierung für unbekanntes Paket")
                .paket
                .clone();
            let neue_version = format!("{}+neu", alt.version);
            aktualisierungen.push(Aktualisierung {
                paket: Paket {
                    id: paket_id(name, &neue_version, false),
                    version: neue_version,
                    installiert: false,
                    ..alt
                },
                dringlichkeit,
                neustart_noetig: neustart,
            });
        }
        AttrappeQuelle {
            innen: Rc::new(RefCell::new(Innen {
                eintraege,
                aktualisierungen,
                versuche: HashMap::new(),
            })),
        }
    }

    /// Alle Pakete (für die Unit-Tests).
    #[cfg(test)]
    pub fn alle_pakete(&self) -> Vec<Paket> {
        self.innen
            .borrow()
            .eintraege
            .iter()
            .map(|e| e.paket.clone())
            .collect()
    }

    #[cfg(test)]
    pub fn alle_aktualisierungen(&self) -> Vec<Aktualisierung> {
        self.innen.borrow().aktualisierungen.clone()
    }
}

/// Abbruchgriff der simulierten Transaktion: hält die Quelle des Timers.
struct AttrappeAbbruch {
    quelle: Rc<RefCell<Option<glib::SourceId>>>,
    melden: Rc<dyn Fn(Fortschritt)>,
    laeuft: Rc<RefCell<bool>>,
}

impl Abbruch for AttrappeAbbruch {
    fn abbrechen(&self) {
        if let Some(id) = self.quelle.borrow_mut().take() {
            id.remove();
        }
        if self.laeuft.replace(false) {
            (self.melden)(Fortschritt::Abgebrochen);
        }
    }
}

/// Leerer Abbruchgriff (Transaktion schon beendet).
struct KeinAbbruch;
impl Abbruch for KeinAbbruch {
    fn abbrechen(&self) {}
}

#[async_trait::async_trait(?Send)]
impl Paketquelle for AttrappeQuelle {
    async fn suchen(&self, text: &str, nur_installiert: bool) -> Result<Vec<Paket>, Fehler> {
        let t = text.trim().to_lowercase();
        Ok(self
            .innen
            .borrow()
            .eintraege
            .iter()
            .map(|e| &e.paket)
            .filter(|p| !nur_installiert || p.installiert)
            .filter(|p| {
                t.is_empty()
                    || p.name.to_lowercase().contains(&t)
                    || p.zusammenfassung.to_lowercase().contains(&t)
            })
            .cloned()
            .collect())
    }

    async fn paket_nach_name(&self, name: &str) -> Result<Option<Paket>, Fehler> {
        Ok(self
            .innen
            .borrow()
            .eintraege
            .iter()
            .map(|e| &e.paket)
            .find(|p| p.name == name)
            .cloned())
    }

    async fn details(&self, id: &PaketId) -> Result<Paketdetails, Fehler> {
        let name = id.split(';').next().unwrap_or(id);
        self.innen
            .borrow()
            .eintraege
            .iter()
            .find(|e| e.paket.id == *id || e.paket.name == name)
            .map(|e| e.details.clone())
            .ok_or_else(|| Fehler::PaketNichtGefunden(id.clone()))
    }

    async fn aktualisierungen(&self) -> Result<Vec<Aktualisierung>, Fehler> {
        Ok(self.innen.borrow().aktualisierungen.clone())
    }

    fn ausfuehren(
        &self,
        vorgang: Vorgang,
        ids: Vec<PaketId>,
        melden: Box<dyn Fn(Fortschritt)>,
    ) -> Box<dyn Abbruch> {
        let melden: Rc<dyn Fn(Fortschritt)> = Rc::from(melden);
        melden(Fortschritt::Gestartet {
            vorgang: vorgang.clone(),
            pakete: ids.clone(),
        });
        let namen: Vec<String> = ids
            .iter()
            .map(|id| id.split(';').next().unwrap_or(id).to_string())
            .collect();
        // Fehlerwege der Attrappe
        if namen.iter().any(|n| n == PAKET_KEIN_AGENT) {
            melden(Fortschritt::Fehler(Fehler::KeinAgent));
            return Box::new(KeinAbbruch);
        }
        if namen.iter().any(|n| n == PAKET_NICHT_BERECHTIGT) {
            melden(Fortschritt::Status("Warte auf Berechtigung …".into()));
            melden(Fortschritt::Fehler(Fehler::NichtBerechtigt));
            return Box::new(KeinAbbruch);
        }
        let sperre = namen.iter().any(|n| n == PAKET_SPERRE) && {
            let mut innen = self.innen.borrow_mut();
            let z = innen.versuche.entry(PAKET_SPERRE.into()).or_insert(0);
            *z += 1;
            *z == 1
        };
        let quelle_id: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
        let laeuft = Rc::new(RefCell::new(true));
        if sperre {
            melden(Fortschritt::Status("Warte auf die Paketsperre …".into()));
            melden(Fortschritt::WartetAufSperre);
            let m = melden.clone();
            let l = laeuft.clone();
            let q = quelle_id.clone();
            let id = glib::timeout_add_local_once(Duration::from_millis(500), move || {
                q.borrow_mut().take();
                if l.replace(false) {
                    m(Fortschritt::Fehler(Fehler::SperreBelegt(
                        "apt-get, PID 4242".into(),
                    )));
                }
            });
            *quelle_id.borrow_mut() = Some(id);
            return Box::new(AttrappeAbbruch {
                quelle: quelle_id,
                melden,
                laeuft,
            });
        }
        // Normaler Lauf: 0 → 100 in 2 s, Status wechselt bei 50 %.
        let schritte = (TRANSAKTIONSDAUER.as_millis() / SCHRITT.as_millis()) as u32;
        let zaehler = Rc::new(RefCell::new(0u32));
        let status_anfang = match vorgang {
            Vorgang::Installieren => "Lade herunter …",
            Vorgang::Entfernen => "Entferne …",
            Vorgang::Aktualisieren => "Lade Aktualisierung herunter …",
            Vorgang::QuellenAuffrischen => "Lade Paketlisten …",
        };
        let status_ende = match vorgang {
            Vorgang::Installieren => "Installiere …",
            Vorgang::Entfernen => "Räume auf …",
            Vorgang::Aktualisieren => "Installiere Aktualisierung …",
            Vorgang::QuellenAuffrischen => "Lese Paketlisten …",
        };
        melden(Fortschritt::Status(status_anfang.into()));
        melden(Fortschritt::Prozent(0));
        let m = melden.clone();
        let l = laeuft.clone();
        let q = quelle_id.clone();
        let innen = self.innen.clone();
        let id = glib::timeout_add_local(SCHRITT, move || {
            let mut z = zaehler.borrow_mut();
            *z += 1;
            let prozent = (*z * 100 / schritte).min(100) as u8;
            if *z == schritte / 2 {
                m(Fortschritt::Status(status_ende.into()));
            }
            m(Fortschritt::Prozent(prozent));
            for id in &ids {
                m(Fortschritt::PaketFortschritt {
                    id: id.clone(),
                    prozent,
                });
            }
            if *z >= schritte {
                q.borrow_mut().take();
                l.replace(false);
                {
                    let mut innen = innen.borrow_mut();
                    for id in &ids {
                        match vorgang {
                            Vorgang::Installieren | Vorgang::Aktualisieren => {
                                innen.setze_installiert(id, true)
                            }
                            Vorgang::Entfernen => innen.setze_installiert(id, false),
                            Vorgang::QuellenAuffrischen => {}
                        }
                    }
                }
                m(Fortschritt::Fertig);
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        *quelle_id.borrow_mut() = Some(id);
        Box::new(AttrappeAbbruch {
            quelle: quelle_id,
            melden,
            laeuft,
        })
    }

    async fn simulieren(&self, vorgang: Vorgang, ids: Vec<PaketId>) -> Result<Simulation, Fehler> {
        let mut sim = Simulation::default();
        let innen = self.innen.borrow();
        let eintraege = &innen.eintraege;
        let finde = |id: &str| {
            let name = id.split(';').next().unwrap_or(id);
            eintraege
                .iter()
                .map(|e| &e.paket)
                .find(|p| p.id == id || p.name == name)
                .cloned()
        };
        let abhaengig = |p: &Paket| Paket {
            id: paket_id(&format!("{}-data", p.name), &p.version, p.installiert),
            name: format!("{}-data", p.name),
            version: p.version.clone(),
            arch: "all".into(),
            zusammenfassung: format!("Datendateien für {}", p.name),
            installiert: p.installiert,
        };
        for id in &ids {
            let Some(p) = finde(id) else {
                return Err(Fehler::PaketNichtGefunden(id.clone()));
            };
            match vorgang {
                Vorgang::Installieren => {
                    sim.download_bytes += eintraege
                        .iter()
                        .find(|e| e.paket.name == p.name)
                        .map(|e| e.details.groesse_bytes)
                        .unwrap_or(0);
                    sim.installieren.push(abhaengig(&p));
                    sim.installieren.push(p);
                }
                Vorgang::Entfernen => {
                    // Was mitgeht: das Datenpaket und ein abhängiges Hilfspaket.
                    sim.entfernen.push(abhaengig(&p));
                    sim.entfernen.push(Paket {
                        id: paket_id(&format!("{}-plugins", p.name), "1.0-1", true),
                        name: format!("{}-plugins", p.name),
                        version: "1.0-1".into(),
                        arch: ARCH.into(),
                        zusammenfassung: format!("Erweiterungen für {}", p.name),
                        installiert: true,
                    });
                    sim.entfernen.push(p);
                }
                Vorgang::Aktualisieren => sim.aktualisieren.push(p),
                Vorgang::QuellenAuffrischen => {}
            }
        }
        Ok(sim)
    }

    async fn laufende_transaktionen(&self) -> Result<Vec<String>, Fehler> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------- AttrappeKatalog

#[derive(Default)]
pub struct AttrappeKatalog {
    anwendungen: Vec<Anwendung>,
}

impl AttrappeKatalog {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Katalog for AttrappeKatalog {
    fn laden(&mut self) -> Result<Statistik, String> {
        self.anwendungen = rohdaten()
            .into_iter()
            .map(|r| Anwendung {
                id: r.id.into(),
                name: r.name.into(),
                zusammenfassung: r.zusammenfassung.into(),
                beschreibung: r.beschreibung.into(),
                paketname: Some(r.paket.into()),
                kategorien: r.kategorien.to_vec(),
                symbol: r.symbol,
                bildschirmfotos: Vec::new(),
                lizenz: Some(r.lizenz.into()),
                homepage: Some(r.homepage.into()),
            })
            .collect();
        let deutsch = self
            .anwendungen
            .iter()
            .filter(|a| {
                !a.zusammenfassung
                    .starts_with(|c: char| c.is_ascii_uppercase())
                    || {
                        // grobe Heuristik nur für die Statistik der Attrappe
                        !matches!(
                            a.zusammenfassung.split(' ').next(),
                            Some("A") | Some("An") | Some("Spreadsheet")
                        )
                    }
            })
            .count();
        Ok(Statistik {
            komponenten: self.anwendungen.len(),
            mit_paket: self.anwendungen.len(),
            mit_symbol: self
                .anwendungen
                .iter()
                .filter(|a| a.symbol != Symbol::Keines)
                .count(),
            deutsch,
            quellen: vec!["Attrappe (src/attrappen.rs)".into()],
        })
    }

    fn kategorie(&self, k: Kategorie) -> Vec<&Anwendung> {
        let mut v: Vec<&Anwendung> = self
            .anwendungen
            .iter()
            .filter(|a| a.kategorien.contains(&k))
            .collect();
        v.sort_by_cached_key(|a| crate::logik::sortierschluessel(&a.name));
        v
    }

    fn suchen(&self, text: &str) -> Vec<&Anwendung> {
        let alle: Vec<&Anwendung> = self.anwendungen.iter().collect();
        crate::logik::suche(&alle, text)
    }

    fn nach_paket(&self, paketname: &str) -> Option<&Anwendung> {
        self.anwendungen
            .iter()
            .find(|a| a.paketname.as_deref() == Some(paketname))
    }

    fn nach_id(&self, id: &str) -> Option<&Anwendung> {
        self.anwendungen.iter().find(|a| a.id == id)
    }

    fn symbol_pfad(&self, a: &Anwendung, _groesse: u32) -> Option<PathBuf> {
        match &a.symbol {
            Symbol::Datei(p) if p.exists() => Some(p.clone()),
            _ => None,
        }
    }

    fn alle(&self) -> &[Anwendung] {
        &self.anwendungen
    }
}
