//! Querencia Software – grafischer Software-Installer für Querencia Desktop
//! (Oberfläche, Teilprojekt SW-3).
//!
//! GTK 4 ohne libadwaita und ohne eigene Titelleiste: labwc zeichnet den
//! klassischen Server-Rahmen (Thema Querencia-Klassisch). Menüleiste,
//! Seitenleiste mit Kategorien, Liste, Detailseite, Fortschrittsleiste,
//! Statuszeile. Texte über GNU gettext (`i18n`, englische msgids, Deutsch aus
//! `po/de.po`); die Sprache folgt `LANGUAGE`/`LANG` der Sitzung.
//!
//! Die Oberfläche spricht über die Traits in `schnittstellen` mit den
//! Backends (`Rc<dyn Paketquelle>`, `Rc<dyn Katalog>`, Transaktionsmonitor).
//! Seit der Integration (SW-6) sind das PackageKit (`qs_paketkit`, SW-1),
//! AppStream (`qs_katalog`, SW-2) und `qs_transaktion::Monitor` (SW-4);
//! `QS_ATTRAPPE=1` schaltet auf die Attrappen aus `attrappen` um (GUI-Prüfung
//! ohne Daemon). Siehe `backends()` und `echt`.
//!
//! `QS_DEBUG=1` schreibt für die isolierte GUI-Prüfung Zustände und
//! Widget-Positionen nach stdout (Format: Tab-getrennt, siehe `fenster.rs`).

mod attrappen;
mod dialoge;
mod echt;
mod fenster;
mod fremdtexte;
mod hilfe;
mod i18n;
mod logik;
mod netz;
mod schnittstellen;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use gtk::prelude::*;
use gtk::{gio, glib};

use schnittstellen::{Katalog, Paketquelle};

pub const APP_ID: &str = "org.querencia.Software";
pub const APP_NAME: &str = "Querencia Software";

/// Ist `QS_DEBUG` gesetzt (isolierte GUI-Prüfung)?
pub fn debug_aktiv() -> bool {
    static AKTIV: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AKTIV.get_or_init(|| std::env::var_os("QS_DEBUG").is_some())
}

/// Debug-Zeile nach stdout (Tab-getrennt), sofort geschrieben.
pub fn debug(zeile: &str) {
    if debug_aktiv() {
        use std::io::Write;
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{zeile}");
        let _ = out.flush();
    }
}

/// Zeitpunkt des Programmstarts (für die Startzeit-Messung, `zeit`-Zeilen).
pub fn seit_start_ms() -> u128 {
    START.get_or_init(Instant::now).elapsed().as_millis()
}
static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Die Backends der Oberfläche.
pub struct Backends {
    pub quelle: Rc<dyn Paketquelle>,
    /// Im echten Betrieb zuerst `LeererKatalog`, nach dem Laden im
    /// Hintergrund-Thread der AppStream-Katalog.
    pub katalog: RefCell<Rc<dyn Katalog>>,
    pub monitor: Rc<qs_transaktion::Monitor>,
    /// Installiert-Zustand der Katalog-Pakete (integration.md Nr. 6).
    pub aufloeser: Rc<dyn echt::Aufloeser>,
    /// `Some`: Katalog muss noch geladen werden (echter Betrieb).
    pub katalog_laden: RefCell<Option<qs_katalog::Konfiguration>>,
    /// Echter Betrieb (PackageKit/AppStream) statt Attrappen.
    pub echt: bool,
    /// Stufen der Erreichbarkeitsprüfung (`netz`, QD-067).
    pub netz: Rc<dyn netz::Sonden>,
}

/// `QS_ATTRAPPE=1`: Attrappen von SW-3 (30 Beispielanwendungen, simulierte
/// Transaktionen), sonst PackageKit (`qs_paketkit`, Systembus oder
/// `QS_PK_BUS`) und AppStream (`qs_katalog`). Der Transaktionsmonitor ist in
/// beiden Fällen `qs_transaktion::Monitor`; sein Protokoll geht nach
/// `~/.local/state/querencia/software.log` (umlenkbar mit `QS_LOG`).
/// `QS_NUR_LESEN=1` sperrt schreibende Vorgänge (`echt::NurLesen`).
pub fn backends() -> Backends {
    let monitor = Rc::new(qs_transaktion::Monitor::neu(Rc::new(
        qs_transaktion::laufzeit::GlibLaufzeit,
    )));
    if std::env::var_os("QS_ATTRAPPE").is_some() {
        let quelle: Rc<dyn Paketquelle> = Rc::new(attrappen::AttrappeQuelle::new());
        let mut katalog = attrappen::AttrappeKatalog::new();
        match katalog.laden() {
            Ok(s) => debug_katalog(&s),
            Err(e) => eprintln!("querencia-software: Katalog nicht ladbar: {e}"),
        }
        debug("backends\tattrappe");
        return Backends {
            aufloeser: Rc::new(echt::LeersucheAufloeser(quelle.clone())),
            quelle,
            katalog: RefCell::new(Rc::new(katalog)),
            monitor,
            katalog_laden: RefCell::new(None),
            echt: false,
            netz: Rc::new(netz::AttrappeAusUmgebung),
        };
    }
    let mut verbindung = None;
    let (quelle, aufloeser): (Rc<dyn Paketquelle>, Rc<dyn echt::Aufloeser>) =
        match qs_paketkit::PaketKit::neu_blockierend() {
            Ok(pk) => {
                verbindung = Some(pk.verbindung().clone());
                (Rc::new(pk.clone()), Rc::new(echt::PaketKitAufloeser(pk)))
            }
            Err(e) => {
                eprintln!("querencia-software: keine Verbindung zu PackageKit: {e:?}");
                (
                    Rc::new(echt::OhneVerbindung(e.clone())),
                    Rc::new(echt::OhneVerbindung(e)),
                )
            }
        };
    let nur_lesen = std::env::var_os("QS_NUR_LESEN").is_some_and(|v| v == "1");
    let quelle: Rc<dyn Paketquelle> = if nur_lesen {
        Rc::new(echt::NurLesen(quelle))
    } else {
        quelle
    };
    debug(&format!(
        "backends\techt\t{}\t{}",
        std::env::var("QS_PK_BUS").unwrap_or_else(|_| "system".into()),
        if nur_lesen { "nur-lesen" } else { "schreibend" }
    ));
    Backends {
        quelle,
        katalog: RefCell::new(Rc::new(echt::LeererKatalog)),
        monitor,
        aufloeser,
        katalog_laden: RefCell::new(Some(echt::katalog_konfiguration())),
        echt: true,
        netz: Rc::new(netz::EchteSonden { verbindung }),
    }
}

/// `katalog`-Zeile der Debug-Ausgabe.
pub fn debug_katalog(s: &qs_typen::Statistik) {
    debug(&format!(
        "katalog\t{}\t{}\t{}\t{}\t{}",
        s.komponenten,
        s.mit_paket,
        s.mit_symbol,
        s.deutsch,
        s.quellen.join(",")
    ));
}

fn main() -> glib::ExitCode {
    seit_start_ms();
    i18n::init();
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("{APP_NAME} {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }
    if std::env::args().nth(1).as_deref() == Some("--netz-pruefen") {
        return netz_pruefen();
    }
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::default())
        .build();
    app.connect_startup(|app| {
        dialoge::register_window_icon();
        fenster::load_css();
        fenster::install_accels(app);
        // QD-081: GTK 4.22 lädt bei „prefer-dark“ das dunkle Thema-CSS nur bei
        // einem Wechsel im laufenden Programm; ein Neustart bliebe hell. Die
        // Meldung stößt das Nachladen einmal an (hell bleibt unverändert).
        if let Some(s) = gtk::Settings::default() {
            s.notify("gtk-interface-color-scheme");
        }
    });
    app.connect_activate(|app| fenster::Fenster::get(app).present());
    app.run()
}

/// `--netz-pruefen`: alle drei Stufen der Erreichbarkeitsprüfung auf der
/// Konsole (Diagnose, z. B. auf einem Server ohne NetworkManager). Exit 0
/// online, 1 offline. `QS_ATTRAPPE=1` nimmt die Attrappen-Stufen.
fn netz_pruefen() -> glib::ExitCode {
    // gtk_init setzt die Locale (setlocale) vor dem Öffnen der Anzeige; ohne
    // Anzeige (Konsole, SSH) scheitert nur das, die Übersetzung gilt trotzdem.
    let _ = gtk::init();
    let sonden: Rc<dyn netz::Sonden> = if std::env::var_os("QS_ATTRAPPE").is_some() {
        Rc::new(netz::AttrappeAusUmgebung)
    } else {
        let verbindung = qs_paketkit::PaketKit::neu_blockierend()
            .ok()
            .map(|pk| pk.verbindung().clone());
        Rc::new(netz::EchteSonden { verbindung })
    };
    let e = glib::MainContext::default().block_on(netz::pruefen(
        sonden.as_ref(),
        netz::system_quelle(),
        true,
    ));
    println!(
        "{}",
        i18n::gettext_f("Network: {state}", &[("state", &e.kurztext())])
    );
    println!("{}", e.hinweis());
    println!("{}", e.debug_zeile().replace('\t', " "));
    if e.ist_online() {
        glib::ExitCode::SUCCESS
    } else {
        glib::ExitCode::FAILURE
    }
}
