//! Querencia Software – grafischer Software-Installer für Querencia Desktop
//! (Oberfläche, Teilprojekt SW-3).
//!
//! GTK 4 ohne libadwaita und ohne eigene Titelleiste: labwc zeichnet den
//! klassischen Server-Rahmen (Thema Querencia-Klassisch). Menüleiste,
//! Seitenleiste mit Kategorien, Liste, Detailseite, Fortschrittsleiste,
//! Statuszeile – alles deutsch.
//!
//! Die Oberfläche spricht nur über die Traits in `schnittstellen` mit den
//! Backends (`Rc<dyn Paketquelle>`, `Rc<dyn Katalog>`,
//! `Rc<dyn Transaktionsmonitor>`). Ohne echte Backends laufen die Attrappen
//! aus `attrappen` (`QS_ATTRAPPE=1`); siehe `backends()`.
//!
//! `QS_DEBUG=1` schreibt für die isolierte GUI-Prüfung Zustände und
//! Widget-Positionen nach stdout (Format: Tab-getrennt, siehe `fenster.rs`).

mod attrappen;
mod dialoge;
mod fenster;
mod logik;
mod monitor;
mod schnittstellen;

use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gio, glib};

use schnittstellen::{Katalog, Paketquelle, Transaktionsmonitor};

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

/// Die drei Backends der Oberfläche.
pub struct Backends {
    pub quelle: Rc<dyn Paketquelle>,
    pub katalog: Rc<dyn Katalog>,
    pub monitor: Rc<dyn Transaktionsmonitor>,
}

/// INTEGRATION: Hier echte Backends einhängen. Vorgesehen:
///
/// ```ignore
/// if std::env::var_os("QS_ATTRAPPE").is_some() { attrappen() } else {
///     let quelle = Rc::new(qs_paketkit::PackageKitQuelle::verbinden().await?);
///     let mut katalog = qs_katalog::AppStreamKatalog::new();   // laden() im Hintergrund-Thread
///     let monitor = Rc::new(qs_transaktion::Monitor::new());
///     …
/// }
/// ```
/// Solange die echten Crates fehlen, laufen immer die Attrappen; ohne
/// `QS_ATTRAPPE` wird eine Warnung auf stderr ausgegeben.
pub fn backends() -> Backends {
    if std::env::var_os("QS_ATTRAPPE").is_none() {
        eprintln!("querencia-software: keine echten Backends eingebunden, verwende Attrappen (QS_ATTRAPPE=1 unterdrückt diese Meldung)");
    }
    let quelle = Rc::new(attrappen::AttrappeQuelle::new());
    let mut katalog = attrappen::AttrappeKatalog::new();
    match katalog.laden() {
        Ok(s) => {
            debug(&format!(
                "katalog\t{}\t{}\t{}\t{}\t{}",
                s.komponenten,
                s.mit_paket,
                s.mit_symbol,
                s.deutsch,
                s.quellen.join(",")
            ));
        }
        Err(e) => eprintln!("querencia-software: Katalog nicht ladbar: {e}"),
    }
    Backends {
        quelle,
        katalog: Rc::new(katalog),
        monitor: Rc::new(monitor::LokalerMonitor::new()),
    }
}

fn main() -> glib::ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("{APP_NAME} {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::default())
        .build();
    app.connect_startup(|app| {
        dialoge::register_window_icon();
        fenster::load_css();
        fenster::install_accels(app);
    });
    app.connect_activate(|app| fenster::Fenster::get(app).present());
    app.run()
}
