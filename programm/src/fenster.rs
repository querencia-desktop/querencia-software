//! Das Hauptfenster: Menüleiste Datei/Bearbeiten/Ansicht/Hilfe, links die
//! Seitenleiste (Alle, 10 Kategorien, Installiert, Aktualisierungen (n),
//! Expertenliste), rechts Suchfeld, Liste oder Detailseite, darunter die
//! Fortschrittsleiste (nur während eines Vorgangs) und die Statuszeile mit
//! dem Netzwerkzustand rechts (`netz`, QD-067: Ergebnis der Stufenprüfung,
//! Knopf „Erneut prüfen“, im Offline-Zustand alle 30 s neu).
//! Keine eigene Titelleiste: labwc zeichnet den klassischen Rahmen.
//!
//! Tastatur: Strg+F Suche · Escape zurück (Details → Liste, sonst Suche
//! leeren) · Eingabetaste öffnet die Details der gewählten Zeile (auch aus
//! dem Suchfeld) · Strg+Q Beenden · F5 Aktualisierungen prüfen.
//!
//! Debug-Ausgabe (`QS_DEBUG=1`, Tab-getrennt, für `werkzeug/szenario.sh`):
//! `ansicht <kennung> <zeilen> <suchtext>` · `details <titel> <zustand> <knopf>`
//! · `transaktion <zustand> …` · `status <text>` · `dialog <kennung>` ·
//! `netz pruefe` · `netz <online|offline> <stufe> …` (`netz::Ergebnis`) ·
//! `netzstatus <text>` · `zeiger x y` · Layout: `fenster w h`, `menue i x y w h`,
//! `seite <kennung> x y w h`, `suche x y w h`, `zeile i <titel> x y w h`,
//! `knopf i <text> x y w h`, `zurueck …`, `aktion <text> …`, `abbrechen …`,
//! `netzknopf …`, `netzfeld …`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use qs_typen::{Fehler, Paket, Vorgang};

use crate::debug;
use crate::dialoge;
use crate::fremdtexte;
use crate::hilfe;
use crate::i18n::{gettext, gettext_f, ngettext_f, pgettext};
use crate::logik::{self, fehlertext, Ansicht, Paketstand, SymbolWahl, Zeile};
use crate::netz;
use crate::schnittstellen::{
    Auftrag, Katalog, Simulation, TransaktionsGriff, Transaktionsmonitor, Zustand,
};
use crate::{Backends, APP_NAME};

const CSS: &str = include_str!("style.css");

/// Rückruf einer Fensteraktion.
type ActionFn = Box<dyn Fn(&Rc<Fenster>)>;

/// Wartet auf das Ergebnis der laufenden Netzwerkprüfung.
type NetzFn = Box<dyn FnOnce(&Rc<Fenster>, &netz::Ergebnis)>;

const FALLBACK_SYMBOL: &str = logik::ERSATZSYMBOL;

pub fn load_css() {
    let provider = gtk::CssProvider::new();
    // QD-081: Ein eigener Provider bleibt bei „prefers-color-scheme“ =
    // Standard; GTK bindet nur den Themenanbieter an die Einstellung.
    // Ohne Bindung greift der @media-dark-Block in style.css nie (isoliert
    // gemessen, Software 0.1.4).
    if let Some(settings) = gtk::Settings::default() {
        if settings
            .find_property("gtk-interface-color-scheme")
            .is_some()
            && provider.find_property("prefers-color-scheme").is_some()
        {
            settings
                .bind_property(
                    "gtk-interface-color-scheme",
                    &provider,
                    "prefers-color-scheme",
                )
                .sync_create()
                .build();
        }
    }
    provider.load_from_string(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn install_accels(app: &gtk::Application) {
    for (action, keys) in [
        ("win.suchen", &["<Control>f"][..]),
        ("win.quit", &["<Control>q"]),
        ("win.pruefen", &["F5"]),
    ] {
        app.set_accels_for_action(action, keys);
    }
}

fn menu_model() -> gio::Menu {
    let datei = gio::Menu::new();
    let s1 = gio::Menu::new();
    s1.append(Some(&gettext("_Check for Updates")), Some("win.pruefen"));
    datei.append_section(None, &s1);
    let s2 = gio::Menu::new();
    s2.append(Some(&gettext("_Quit")), Some("win.quit"));
    datei.append_section(None, &s2);

    let bearbeiten = gio::Menu::new();
    bearbeiten.append(Some(&gettext("_Find")), Some("win.suchen"));

    // TRANSLATORS: View menu; access keys must be unique within the menu
    let ansicht = gio::Menu::new();
    ansicht.append(Some(&gettext("_All Software")), Some("win.ansicht::alle"));
    ansicht.append(
        Some(&gettext("_Installed")),
        Some("win.ansicht::installiert"),
    );
    ansicht.append(
        Some(&gettext("_Updates")),
        Some("win.ansicht::aktualisierungen"),
    );
    ansicht.append(
        Some(&gettext("_Expert List")),
        Some("win.ansicht::experten"),
    );

    let hilfe = gio::Menu::new();
    let h1 = gio::Menu::new();
    // TRANSLATORS: Help menu; both entries open the web browser. Access keys
    // must be unique within the menu
    h1.append(Some(&gettext("Online _Help")), Some("win.online-hilfe"));
    h1.append(Some(&gettext("_Report a Bug")), Some("win.fehler-melden"));
    hilfe.append_section(None, &h1);
    let h2 = gio::Menu::new();
    h2.append(Some(&gettext("_About")), Some("win.about"));
    hilfe.append_section(None, &h2);

    let bar = gio::Menu::new();
    bar.append_submenu(Some(&gettext("_File")), &datei);
    bar.append_submenu(Some(&gettext("_Edit")), &bearbeiten);
    bar.append_submenu(Some(&gettext("_View")), &ansicht);
    bar.append_submenu(Some(&gettext("_Help")), &hilfe);
    bar
}

/// Einträge der Seitenleiste in Anzeigereihenfolge: Ansicht, Symbol.
fn seiten_eintraege() -> Vec<(Ansicht, &'static str)> {
    let mut v = vec![(Ansicht::Alle, "folder")];
    for k in qs_typen::Kategorie::alle() {
        let symbol = match k {
            qs_typen::Kategorie::Zubehoer => "applications-utilities-symbolic",
            qs_typen::Kategorie::Buero => "x-office-document",
            qs_typen::Kategorie::Grafik => "applications-graphics-symbolic",
            qs_typen::Kategorie::Internet => "network-workgroup",
            qs_typen::Kategorie::Multimedia => "applications-multimedia-symbolic",
            qs_typen::Kategorie::Spiele => "applications-games-symbolic",
            qs_typen::Kategorie::Entwicklung => "applications-engineering-symbolic",
            qs_typen::Kategorie::Bildung => "applications-science-symbolic",
            qs_typen::Kategorie::System => "applications-system-symbolic",
            qs_typen::Kategorie::Sonstiges => "preferences-other-symbolic",
        };
        v.push((Ansicht::Kategorie(*k), symbol));
    }
    v.push((Ansicht::Installiert, "drive-harddisk"));
    v.push((Ansicht::Aktualisierungen, "folder-download"));
    v.push((Ansicht::Expertenliste, "package-x-generic"));
    v
}

fn ansicht_aus_kennung(k: &str) -> Option<Ansicht> {
    match k {
        "alle" => Some(Ansicht::Alle),
        "installiert" => Some(Ansicht::Installiert),
        "aktualisierungen" => Some(Ansicht::Aktualisierungen),
        "experten" => Some(Ansicht::Expertenliste),
        _ => None,
    }
}

/// Widgets der Detailseite.
struct Detailseite {
    seite: gtk::Box,
    symbol: gtk::Image,
    name: gtk::Label,
    zusammenfassung: gtk::Label,
    zustand: gtk::Label,
    beschreibung: gtk::Label,
    version: gtk::Label,
    groesse: gtk::Label,
    lizenz: gtk::Label,
    homepage: gtk::LinkButton,
    homepage_fehlt: gtk::Label,
    knopf: gtk::Button,
    zurueck: gtk::Button,
}

/// Widgets der Fortschrittsleiste.
struct Fortschrittsleiste {
    leiste: gtk::Box,
    vorgang: gtk::Label,
    balken: gtk::ProgressBar,
    status: gtk::Label,
    abbrechen: gtk::Button,
}

/// Laufender Vorgang.
struct Transaktion {
    /// Zählt die Rückrufe zu: Meldungen eines früheren Griffs werden verworfen.
    generation: u64,
    griff: TransaktionsGriff,
    /// `None` bei einem nach dem Neustart wieder angebundenen Vorgang.
    vorgang: Option<Vorgang>,
    zeile: Zeile,
    simulation: Option<Simulation>,
    sperrdialog_offen: bool,
    /// Nachfrage „kein Fortschritt“ (SW-4, nach 10 min) wird angezeigt.
    nachfrage_offen: bool,
}

pub struct Fenster {
    window: gtk::ApplicationWindow,
    backends: Backends,
    stand: RefCell<Paketstand>,
    ansicht: Cell<Ansicht>,
    ansicht_action: gio::SimpleAction,
    seitenleiste: gtk::ListBox,
    seiten_zeilen: RefCell<Vec<(Ansicht, gtk::ListBoxRow, gtk::Label)>>,
    suchfeld: gtk::SearchEntry,
    ueberschrift: gtk::Label,
    stack: gtk::Stack,
    modell: gio::ListStore,
    auswahl: gtk::SingleSelection,
    listview: gtk::ListView,
    leer: gtk::Label,
    details: Detailseite,
    detail_zeile: RefCell<Option<Zeile>>,
    fortschritt: Fortschrittsleiste,
    transaktion: RefCell<Option<Transaktion>>,
    status_box: gtk::Box,
    status: gtk::Label,
    netz_label: gtk::Label,
    netz_knopf: gtk::Button,
    /// Letztes Ergebnis der Netzwerkprüfung (`None`: noch keines).
    netz: RefCell<Option<netz::Ergebnis>>,
    netz_laeuft: Cell<bool>,
    netz_warten: RefCell<Vec<NetzFn>>,
    /// Nächste Neuprüfung (30 s im Offline-Zustand, 2 s nach einer
    /// Netzänderung).
    netz_timer: RefCell<Option<glib::SourceId>>,
    /// Die letzte Meldung der Statuszeile ist ein `no-network`-Fehler; sie
    /// verschwindet, sobald die Prüfung wieder online meldet (NM-b S3).
    netz_meldung: Cell<bool>,
    /// „Trotzdem versuchen“: der nächste Vorgang startet ohne Netzprüfung.
    netz_trotzdem: Cell<bool>,
    letzte_pruefung: RefCell<Option<String>>,
    letzte_meldung: RefCell<Option<String>>,
    /// Nach dem nächsten Laden „Aktualisierungen geprüft“ melden.
    pruefung_melden: Cell<bool>,
    /// Echter Betrieb: der AppStream-Katalog lädt noch im Hintergrund.
    katalog_laedt: Cell<bool>,
    /// Echter Betrieb, Expertenliste: letzte PackageKit-Suche (Text, Ergebnis);
    /// `Ok(None)` = Suche läuft.
    experten: RefCell<Option<Expertensuche>>,
    /// Beobachter aus `wieder_anbinden` (SW-4 liefert immer genau einen):
    /// Generation, Griff, ob die Fortschrittsleiste für ihn sichtbar ist.
    beobachter: RefCell<Option<(u64, TransaktionsGriff, bool)>>,
    generation: Cell<u64>,
    debug: bool,
}

/// Suchtext und Ergebnis der PackageKit-Suche in der Expertenliste.
type Expertensuche = (String, Result<Option<Vec<Paket>>, Fehler>);

/// Mindestlänge des Suchtexts für die PackageKit-Suche in der Expertenliste.
const EXPERTEN_MINDESTLAENGE: usize = 2;

thread_local! {
    static FENSTER: RefCell<Option<Rc<Fenster>>> = const { RefCell::new(None) };
}

impl Fenster {
    /// Das (einzige) Fenster der Anwendung, bei Bedarf neu angelegt.
    pub fn get(app: &gtk::Application) -> Rc<Self> {
        if let Some(f) = FENSTER.with(|f| f.borrow().clone()) {
            return f;
        }
        let f = Self::build(app);
        FENSTER.with(|slot| *slot.borrow_mut() = Some(f.clone()));
        f
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn build(app: &gtk::Application) -> Rc<Self> {
        app.set_menubar(Some(&menu_model()));
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title(APP_NAME)
            .default_width(960)
            .default_height(640)
            .show_menubar(true)
            .build();
        window.add_css_class("qs-window");
        dialoge::set_window_icon(&window);

        let backends = crate::backends();

        // ---------------------------------------------------- Seitenleiste
        let seitenleiste = gtk::ListBox::new();
        seitenleiste.add_css_class("qs-seite");
        seitenleiste.set_selection_mode(gtk::SelectionMode::Browse);
        let mut seiten_zeilen = Vec::new();
        for (ansicht, symbol) in seiten_eintraege() {
            let row = gtk::ListBoxRow::new();
            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let image = gtk::Image::from_icon_name(symbol);
            image.set_pixel_size(16);
            let label = gtk::Label::new(Some(&ansicht.titel()));
            label.set_xalign(0.0);
            hbox.append(&image);
            hbox.append(&label);
            row.set_child(Some(&hbox));
            seitenleiste.append(&row);
            seiten_zeilen.push((ansicht, row, label));
        }
        // Dezente Trennlinien vor „Installiert“ und „Expertenliste“.
        seitenleiste.set_header_func(|row, _| {
            let i = row.index();
            let trenner = i == 1 || i == 11 || i == 13;
            if trenner {
                let linie = gtk::Separator::new(gtk::Orientation::Horizontal);
                row.set_header(Some(&linie));
            } else {
                row.set_header(None::<&gtk::Separator>);
            }
        });
        let seiten_scroll = gtk::ScrolledWindow::new();
        seiten_scroll.add_css_class("qs-seite");
        seiten_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        seiten_scroll.set_child(Some(&seitenleiste));
        seiten_scroll.set_width_request(190);

        // ---------------------------------------------------- Kopf mit Suchfeld
        let kopf = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        kopf.add_css_class("qs-kopf");
        let ueberschrift = gtk::Label::new(Some(&Ansicht::Alle.titel()));
        ueberschrift.add_css_class("qs-ueberschrift");
        ueberschrift.set_xalign(0.0);
        ueberschrift.set_hexpand(true);
        ueberschrift.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let suchfeld = gtk::SearchEntry::new();
        suchfeld.set_placeholder_text(Some(&gettext("Search software…")));
        suchfeld.set_search_delay(300);
        suchfeld.set_width_chars(28);
        kopf.append(&ueberschrift);
        kopf.append(&suchfeld);

        // ---------------------------------------------------- Liste
        let modell = gio::ListStore::new::<glib::BoxedAnyObject>();
        let auswahl = gtk::SingleSelection::new(Some(modell.clone()));
        let fabrik = gtk::SignalListItemFactory::new();
        let listview = gtk::ListView::new(Some(auswahl.clone()), Some(fabrik.clone()));
        listview.add_css_class("qs-liste");
        listview.set_vexpand(true);
        let liste_scroll = gtk::ScrolledWindow::new();
        liste_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        liste_scroll.set_child(Some(&listview));
        liste_scroll.set_vexpand(true);
        let leer = gtk::Label::new(Some(&gettext("No applications found.")));
        leer.add_css_class("qs-leer");
        leer.set_margin_top(24);
        leer.set_visible(false);
        let liste_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        liste_box.append(&leer);
        liste_box.append(&liste_scroll);

        // ---------------------------------------------------- Detailseite
        let details = Self::baue_details();

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.add_named(&liste_box, Some("liste"));
        stack.add_named(&details.seite, Some("details"));
        stack.set_visible_child_name("liste");

        // ---------------------------------------------------- Fortschritt
        let fortschritt = Self::baue_fortschritt();

        // ---------------------------------------------------- Statuszeile
        let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        status_box.add_css_class("qs-status");
        let status = gtk::Label::new(None);
        status.add_css_class("qs-status-text");
        status.set_xalign(0.0);
        status.set_hexpand(true);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        status_box.append(&status);
        let netz_label = gtk::Label::new(None);
        netz_label.add_css_class("qs-status-text");
        netz_label.add_css_class("qs-netz");
        netz_label.set_margin_start(12);
        let netz_knopf = gtk::Button::with_label(&gettext("Check Again"));
        netz_knopf.add_css_class("qs-netz-knopf");
        netz_knopf.set_tooltip_text(Some(&gettext("Check the network connection again")));
        netz_knopf.set_margin_start(6);
        netz_knopf.set_valign(gtk::Align::Center);
        status_box.append(&netz_label);
        status_box.append(&netz_knopf);

        let rechts = gtk::Box::new(gtk::Orientation::Vertical, 0);
        rechts.set_hexpand(true);
        rechts.append(&kopf);
        rechts.append(&stack);
        rechts.append(&fortschritt.leiste);

        let mitte = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        mitte.set_vexpand(true);
        mitte.append(&seiten_scroll);
        mitte.append(&rechts);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.append(&mitte);
        vbox.append(&status_box);
        window.set_child(Some(&vbox));

        let ansicht_action = gio::SimpleAction::new_stateful(
            "ansicht",
            Some(glib::VariantTy::STRING),
            &"alle".to_variant(),
        );
        window.add_action(&ansicht_action);

        let debug = crate::debug_aktiv();
        let fenster = Rc::new(Self {
            window,
            backends,
            stand: RefCell::new(Paketstand::default()),
            ansicht: Cell::new(Ansicht::Alle),
            ansicht_action,
            seitenleiste,
            seiten_zeilen: RefCell::new(seiten_zeilen),
            suchfeld,
            ueberschrift,
            stack,
            modell,
            auswahl,
            listview,
            leer,
            details,
            detail_zeile: RefCell::new(None),
            fortschritt,
            transaktion: RefCell::new(None),
            status_box,
            status,
            netz_label,
            netz_knopf,
            netz: RefCell::new(None),
            netz_laeuft: Cell::new(false),
            netz_warten: RefCell::new(Vec::new()),
            netz_timer: RefCell::new(None),
            netz_meldung: Cell::new(false),
            netz_trotzdem: Cell::new(false),
            letzte_pruefung: RefCell::new(None),
            letzte_meldung: RefCell::new(None),
            pruefung_melden: Cell::new(false),
            katalog_laedt: Cell::new(false),
            experten: RefCell::new(None),
            beobachter: RefCell::new(None),
            generation: Cell::new(0),
            debug,
        });

        fenster.install_fabrik(&fabrik);
        fenster.install_actions();
        fenster.install_signale();
        fenster.install_keys();
        if debug {
            fenster.install_debug();
        }
        fenster
            .seitenleiste
            .select_row(fenster.seiten_zeilen.borrow().first().map(|(_, r, _)| r));
        fenster.katalog_laden();
        fenster.netz_pruefen(None);
        fenster.paketstand_laden(true);
        fenster.wieder_anbinden();
        fenster
    }

    /// Der aktuelle Katalog (im echten Betrieb bis zum Ende des Ladens leer).
    fn katalog(&self) -> Rc<dyn Katalog> {
        self.backends.katalog.borrow().clone()
    }

    /// Echter Betrieb: AppStream-Katalog im Hintergrund-Thread laden
    /// (schnittstellen.md: `laden` darf 1–2 s dauern), danach Liste und
    /// Paketstand neu aufbauen.
    fn katalog_laden(self: &Rc<Self>) {
        let Some(konfiguration) = self.backends.katalog_laden.borrow_mut().take() else {
            return;
        };
        self.katalog_laedt.set(true);
        self.melde(&gettext("Loading catalog…"));
        self.aktualisiere_liste();
        let weak = Rc::downgrade(self);
        glib::MainContext::default().spawn_local(async move {
            let beginn = std::time::Instant::now();
            let ergebnis =
                gio::spawn_blocking(move || crate::echt::AppKatalog::laden(konfiguration)).await;
            let Some(s) = weak.upgrade() else {
                return;
            };
            s.katalog_laedt.set(false);
            match ergebnis {
                Ok(Ok((katalog, statistik))) => {
                    for (datei, fehler) in katalog.ladefehler() {
                        eprintln!("querencia-software: Katalogdatei {datei}: {fehler}");
                    }
                    crate::debug_katalog(&statistik);
                    debug(&format!(
                        "zeit\tkatalog\t{}\t{}",
                        beginn.elapsed().as_millis(),
                        crate::seit_start_ms()
                    ));
                    *s.backends.katalog.borrow_mut() = Rc::new(katalog);
                    *s.letzte_meldung.borrow_mut() = None;
                }
                Ok(Err(e)) => s.melde(&gettext_f(
                    "The catalog could not be loaded: {error}",
                    &[("error", &e)],
                )),
                Err(_) => s.melde(&gettext("The catalog could not be loaded: read error")),
            }
            s.aktualisiere_liste();
            s.paketstand_laden(true);
        });
    }

    fn baue_details() -> Detailseite {
        let seite = gtk::Box::new(gtk::Orientation::Vertical, 12);
        seite.add_css_class("qs-details");
        let zurueck = gtk::Button::with_mnemonic(&gettext("← _Back"));
        zurueck.set_halign(gtk::Align::Start);
        seite.append(&zurueck);

        let kopf = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        let symbol = gtk::Image::from_icon_name(FALLBACK_SYMBOL);
        symbol.set_pixel_size(64);
        symbol.set_valign(gtk::Align::Start);
        let texte = gtk::Box::new(gtk::Orientation::Vertical, 4);
        texte.set_hexpand(true);
        let name = gtk::Label::new(None);
        name.add_css_class("qs-detail-name");
        name.set_xalign(0.0);
        name.set_wrap(true);
        let zusammenfassung = gtk::Label::new(None);
        zusammenfassung.set_xalign(0.0);
        zusammenfassung.set_wrap(true);
        let zustand = gtk::Label::new(None);
        zustand.add_css_class("dim-label");
        zustand.set_xalign(0.0);
        texte.append(&name);
        texte.append(&zusammenfassung);
        texte.append(&zustand);
        let knopf = gtk::Button::with_label(&gettext("Install"));
        knopf.set_valign(gtk::Align::Start);
        knopf.set_size_request(120, -1);
        kopf.append(&symbol);
        kopf.append(&texte);
        kopf.append(&knopf);
        seite.append(&kopf);

        seite.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let beschreibung = gtk::Label::new(None);
        beschreibung.set_xalign(0.0);
        beschreibung.set_yalign(0.0);
        beschreibung.set_wrap(true);
        beschreibung.set_selectable(true);
        beschreibung.set_hexpand(true);
        let beschr_scroll = gtk::ScrolledWindow::new();
        beschr_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        beschr_scroll.set_vexpand(true);
        beschr_scroll.set_child(Some(&beschreibung));
        seite.append(&beschr_scroll);

        seite.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let fakten = gtk::Grid::new();
        fakten.add_css_class("qs-fakten");
        fakten.set_column_spacing(18);
        fakten.set_row_spacing(4);
        let fakt = |zeile: i32, titel: &str| -> gtk::Label {
            let t = gtk::Label::new(Some(titel));
            t.add_css_class("qs-fakt-name");
            t.set_xalign(0.0);
            fakten.attach(&t, 0, zeile, 1, 1);
            let w = gtk::Label::new(None);
            w.set_xalign(0.0);
            w.set_selectable(true);
            w.set_ellipsize(gtk::pango::EllipsizeMode::End);
            w.set_hexpand(true);
            fakten.attach(&w, 1, zeile, 1, 1);
            w
        };
        let version = fakt(0, &gettext("Version"));
        let groesse = fakt(1, &gettext("Size"));
        let lizenz = fakt(2, &gettext("License"));
        let hp_titel = gtk::Label::new(Some(&gettext("Website")));
        hp_titel.add_css_class("qs-fakt-name");
        hp_titel.set_xalign(0.0);
        fakten.attach(&hp_titel, 0, 3, 1, 1);
        let hp_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let homepage = gtk::LinkButton::with_label("https://", "");
        homepage.set_halign(gtk::Align::Start);
        homepage.add_css_class("qs-homepage");
        let homepage_fehlt = gtk::Label::new(Some("–"));
        homepage_fehlt.set_xalign(0.0);
        hp_box.append(&homepage);
        hp_box.append(&homepage_fehlt);
        fakten.attach(&hp_box, 1, 3, 1, 1);
        seite.append(&fakten);

        Detailseite {
            seite,
            symbol,
            name,
            zusammenfassung,
            zustand,
            beschreibung,
            version,
            groesse,
            lizenz,
            homepage,
            homepage_fehlt,
            knopf,
            zurueck,
        }
    }

    fn baue_fortschritt() -> Fortschrittsleiste {
        let leiste = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        leiste.add_css_class("qs-fortschritt");
        leiste.set_visible(false);
        let texte = gtk::Box::new(gtk::Orientation::Vertical, 2);
        texte.set_hexpand(true);
        let vorgang = gtk::Label::new(None);
        vorgang.add_css_class("qs-fett");
        vorgang.set_xalign(0.0);
        vorgang.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let balken = gtk::ProgressBar::new();
        balken.set_show_text(true);
        balken.set_text(Some("0 %"));
        balken.set_hexpand(true);
        let status = gtk::Label::new(None);
        status.add_css_class("dim-label");
        status.set_xalign(0.0);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        texte.append(&vorgang);
        texte.append(&balken);
        texte.append(&status);
        // TRANSLATORS: progress bar button; German keeps its old access key
        let abbrechen = gtk::Button::with_mnemonic(&pgettext("progress bar", "_Cancel"));
        abbrechen.set_valign(gtk::Align::Center);
        leiste.append(&texte);
        leiste.append(&abbrechen);
        Fortschrittsleiste {
            leiste,
            vorgang,
            balken,
            status,
            abbrechen,
        }
    }

    // ------------------------------------------------------------ Fabrik

    fn install_fabrik(self: &Rc<Self>, fabrik: &gtk::SignalListItemFactory) {
        let weak = Rc::downgrade(self);
        fabrik.connect_setup(move |_, obj| {
            let Some(item) = obj.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            hbox.add_css_class("qs-zeile");
            let symbol = gtk::Image::from_icon_name(FALLBACK_SYMBOL);
            symbol.set_pixel_size(48);
            symbol.add_css_class("qs-symbol");
            let texte = gtk::Box::new(gtk::Orientation::Vertical, 1);
            texte.set_hexpand(true);
            texte.set_valign(gtk::Align::Center);
            let name = gtk::Label::new(None);
            name.add_css_class("qs-name");
            name.set_xalign(0.0);
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            let zusammenfassung = gtk::Label::new(None);
            zusammenfassung.add_css_class("qs-zusammenfassung");
            zusammenfassung.set_xalign(0.0);
            zusammenfassung.set_ellipsize(gtk::pango::EllipsizeMode::End);
            let zustand = gtk::Label::new(None);
            zustand.add_css_class("qs-zustand");
            zustand.add_css_class("dim-label");
            zustand.set_xalign(0.0);
            texte.append(&name);
            texte.append(&zusammenfassung);
            texte.append(&zustand);
            let knopf = gtk::Button::with_label(&gettext("Install"));
            knopf.add_css_class("qs-knopf");
            knopf.set_valign(gtk::Align::Center);
            knopf.set_size_request(110, -1);
            knopf.set_focusable(false);
            hbox.append(&symbol);
            hbox.append(&texte);
            hbox.append(&knopf);
            item.set_child(Some(&hbox));
            // Der Knopf liest beim Klick die aktuell gebundene Zeile.
            let item_weak = item.downgrade();
            let weak = weak.clone();
            knopf.connect_clicked(move |_| {
                let Some(f) = weak.upgrade() else {
                    return;
                };
                let zeile = item_weak
                    .upgrade()
                    .and_then(|it| it.item())
                    .and_then(|o| o.downcast::<glib::BoxedAnyObject>().ok())
                    .map(|b| b.borrow::<Zeile>().clone());
                if let Some(z) = zeile {
                    f.aktion(z);
                }
            });
        });
        let weak = Rc::downgrade(self);
        fabrik.connect_bind(move |_, obj| {
            let Some(item) = obj.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let (Some(f), Some(hbox), Some(o)) = (weak.upgrade(), item.child(), item.item()) else {
                return;
            };
            let Ok(boxed) = o.downcast::<glib::BoxedAnyObject>() else {
                return;
            };
            let zeile = boxed.borrow::<Zeile>();
            let symbol = finde_klasse(&hbox, "qs-symbol").and_downcast::<gtk::Image>();
            let name = finde_klasse(&hbox, "qs-name").and_downcast::<gtk::Label>();
            let zus = finde_klasse(&hbox, "qs-zusammenfassung").and_downcast::<gtk::Label>();
            let zustand = finde_klasse(&hbox, "qs-zustand").and_downcast::<gtk::Label>();
            let knopf = finde_klasse(&hbox, "qs-knopf").and_downcast::<gtk::Button>();
            if let Some(s) = symbol {
                f.setze_symbol(&s, &zeile, 48);
            }
            if let Some(n) = name {
                n.set_text(&zeile.titel);
            }
            if let Some(z) = zus {
                z.set_text(&zeile.zusammenfassung);
            }
            if let Some(z) = zustand {
                z.set_text(&zeile.zustand());
            }
            if let Some(k) = knopf {
                k.set_label(&zeile.knopf());
                k.set_sensitive(zeile.paket.is_some());
            }
        });
    }

    /// Symbol einer Zeile: Datei vom Katalog, sonst Themenname, sonst Fallback.
    /// Symbol einer Zeile (siehe `logik::symbol_wahl`): nie ein kaputtes Bild.
    fn setze_symbol(&self, image: &gtk::Image, zeile: &Zeile, groesse: u32) {
        image.set_pixel_size(groesse as i32);
        let pfad = zeile
            .anwendung
            .as_ref()
            .and_then(|a| self.katalog().symbol_pfad(a, groesse));
        match logik::symbol_wahl(pfad, &zeile.symbol, symbol_vorhanden) {
            SymbolWahl::Datei(p) => image.set_from_file(Some(&p)),
            SymbolWahl::Thema(n) => image.set_icon_name(Some(&n)),
            SymbolWahl::Ersatz => image.set_icon_name(Some(FALLBACK_SYMBOL)),
        }
    }

    // ------------------------------------------------------------ Aktionen

    fn install_actions(self: &Rc<Self>) {
        let add = |name: &str, f: ActionFn| {
            let action = gio::SimpleAction::new(name, None);
            let weak = Rc::downgrade(self);
            action.connect_activate(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    f(&s);
                }
            });
            self.window.add_action(&action);
        };
        add("quit", Box::new(|s| s.window.close()));
        add("about", Box::new(|s| dialoge::about(&s.window)));
        add(
            "online-hilfe",
            Box::new(|s| hilfe::oeffnen(&s.window, &hilfe::online_hilfe())),
        );
        add(
            "fehler-melden",
            Box::new(|s| hilfe::oeffnen(&s.window, &hilfe::fehler_melden())),
        );
        add(
            "suchen",
            Box::new(|s| {
                s.suchfeld.grab_focus();
            }),
        );
        add("pruefen", Box::new(|s| s.aktualisierungen_pruefen()));
        let weak = Rc::downgrade(self);
        self.ansicht_action.connect_activate(move |a, param| {
            let Some(s) = weak.upgrade() else {
                return;
            };
            let Some(k) = param.and_then(|p| p.get::<String>()) else {
                return;
            };
            if let Some(ansicht) = ansicht_aus_kennung(&k) {
                a.set_state(&k.to_variant());
                s.wechsle_ansicht(ansicht);
            }
        });
    }

    fn install_signale(self: &Rc<Self>) {
        // Netzwerk: Knopf „Erneut prüfen“; im echten Betrieb zusätzlich nach
        // jeder Änderung, die GLib meldet (Routen, NetworkManager).
        let weak = Rc::downgrade(self);
        self.netz_knopf.connect_clicked(move |_| {
            if let Some(s) = weak.upgrade() {
                s.netz_pruefen(None);
            }
        });
        if self.backends.echt {
            let weak = Rc::downgrade(self);
            gio::NetworkMonitor::default().connect_network_changed(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    s.netz_spaeter_pruefen(std::time::Duration::from_secs(2));
                }
            });
        }
        // Seitenleiste
        let weak = Rc::downgrade(self);
        self.seitenleiste.connect_row_selected(move |_, row| {
            let (Some(s), Some(row)) = (weak.upgrade(), row) else {
                return;
            };
            let ansicht = s
                .seiten_zeilen
                .borrow()
                .iter()
                .find(|(_, r, _)| r == row)
                .map(|(a, _, _)| *a);
            if let Some(a) = ansicht {
                if a != s.ansicht.get() {
                    s.wechsle_ansicht(a);
                }
            }
        });
        // Suche (search-changed kommt nach der eingestellten Verzögerung)
        let weak = Rc::downgrade(self);
        self.suchfeld.connect_search_changed(move |_| {
            if let Some(s) = weak.upgrade() {
                s.zeige_liste();
                s.aktualisiere_liste();
            }
        });
        let weak = Rc::downgrade(self);
        self.suchfeld.connect_activate(move |_| {
            if let Some(s) = weak.upgrade() {
                s.oeffne_auswahl();
            }
        });
        // Der Fokus liegt im inneren Textfeld des Suchfelds; ein
        // Fokus-Controller meldet auch das.
        if self.debug {
            let fokus = gtk::EventControllerFocus::new();
            fokus.connect_enter(|_| debug("fokus\tsuche"));
            self.suchfeld.add_controller(fokus);
        }
        // Liste: Eingabetaste / Doppelklick
        let weak = Rc::downgrade(self);
        self.listview.connect_activate(move |_, pos| {
            if let Some(s) = weak.upgrade() {
                s.oeffne_details_an(pos);
            }
        });
        // Detailseite
        let weak = Rc::downgrade(self);
        self.details.zurueck.connect_clicked(move |_| {
            if let Some(s) = weak.upgrade() {
                s.zeige_liste();
            }
        });
        let weak = Rc::downgrade(self);
        self.details.knopf.connect_clicked(move |_| {
            if let Some(s) = weak.upgrade() {
                let zeile = s.detail_zeile.borrow().clone();
                if let Some(z) = zeile {
                    s.aktion(z);
                }
            }
        });
        // Fortschritt abbrechen
        let weak = Rc::downgrade(self);
        self.fortschritt.abbrechen.connect_clicked(move |_| {
            if let Some(s) = weak.upgrade() {
                s.abbrechen();
            }
        });
    }

    /// Escape: Details → Liste, sonst Suchfeld leeren. Menüs behalten ihre
    /// Tasten.
    fn install_keys(self: &Rc<Self>) {
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |ctrl, key, _, _| {
            let Some(f) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if key != gdk::Key::Escape {
                return glib::Propagation::Proceed;
            }
            let im_popup =
                ctrl.current_event().and_then(|e| e.surface()) != NativeExt::surface(&f.window);
            if im_popup || menue_offen(f.window.upcast_ref()) {
                return glib::Propagation::Proceed;
            }
            if f.stack.visible_child_name().as_deref() == Some("details") {
                f.zeige_liste();
                f.listview.grab_focus();
                return glib::Propagation::Stop;
            }
            if !f.suchfeld.text().is_empty() {
                f.suchfeld.set_text("");
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(keys);
    }

    // ------------------------------------------------------------ Zustand

    /// Paketstand (alle Pakete, Aktualisierungen) von der Quelle holen und
    /// die Liste neu aufbauen. `pruefung`: als Aktualisierungsprüfung zählen.
    fn paketstand_laden(self: &Rc<Self>, pruefung: bool) {
        if self.katalog_laedt.get() {
            return; // kommt nach dem Laden des Katalogs
        }
        let weak = Rc::downgrade(self);
        let quelle = self.backends.quelle.clone();
        let aufloeser = self.backends.aufloeser.clone();
        let mut namen: Vec<String> = self
            .katalog()
            .alle()
            .iter()
            .filter_map(|a| a.paketname.clone())
            .collect();
        namen.sort();
        namen.dedup();
        glib::MainContext::default().spawn_local(async move {
            let beginn = std::time::Instant::now();
            let n = namen.len();
            let pakete = aufloeser.aufloesen(namen).await;
            debug(&format!(
                "zeit\tpaketstand\t{}\t{n}\t{}",
                beginn.elapsed().as_millis(),
                pakete.as_ref().map(|p| p.len()).unwrap_or(0)
            ));
            let akt = quelle.aktualisierungen().await;
            let Some(s) = weak.upgrade() else {
                return;
            };
            match (pakete, akt) {
                (Ok(p), Ok(a)) => {
                    let stand = Paketstand::aus(p, a);
                    debug(&format!("snap-huellen\t{}", stand.snap_huellen.join(",")));
                    *s.stand.borrow_mut() = stand;
                    if pruefung {
                        *s.letzte_pruefung.borrow_mut() = Some(uhrzeit());
                    }
                    if s.pruefung_melden.replace(false) {
                        let n = s.stand.borrow().aktualisierungen.len();
                        *s.letzte_meldung.borrow_mut() = Some(ngettext_f(
                            "Checked for updates: {n} available",
                            "Checked for updates: {n} available",
                            n as u32,
                            &[],
                        ));
                    }
                }
                (Err(e), _) | (_, Err(e)) => {
                    s.melde(&gettext_f(
                        "Package source not reachable: {error}",
                        &[("error", &fehlertext(&e))],
                    ));
                }
            }
            s.aktualisiere_seitenleiste();
            s.aktualisiere_liste();
            s.aktualisiere_details();
            s.aktualisiere_status();
        });
    }

    fn aktualisierungen_pruefen(self: &Rc<Self>) {
        self.pruefung_melden.set(true);
        self.melde(&gettext("Checking for updates…"));
        self.paketstand_laden(true);
    }

    fn wechsle_ansicht(self: &Rc<Self>, ansicht: Ansicht) {
        self.ansicht.set(ansicht);
        self.ansicht_action
            .set_state(&ansicht.kennung().to_variant());
        // Seitenleiste nachführen (ohne Rückkopplung: gleiche Ansicht → kein Wechsel)
        let row = self
            .seiten_zeilen
            .borrow()
            .iter()
            .find(|(a, _, _)| *a == ansicht)
            .map(|(_, r, _)| r.clone());
        if let Some(r) = row {
            if self.seitenleiste.selected_row().as_ref() != Some(&r) {
                self.seitenleiste.select_row(Some(&r));
            }
        }
        self.zeige_liste();
        self.aktualisiere_liste();
    }

    fn aktuelle_zeilen(self: &Rc<Self>) -> Vec<Zeile> {
        let text = self.suchfeld.text().trim().to_string();
        if self.backends.echt && self.ansicht.get() == Ansicht::Expertenliste && !text.is_empty() {
            return self.experten_zeilen(&text);
        }
        let stand = self.stand.borrow();
        logik::zeilen_gesucht(self.ansicht.get(), &text, &*self.katalog(), &stand)
    }

    /// Echter Betrieb, Expertenliste mit Suchtext: PackageKit-Suche über alle
    /// Pakete (ab `EXPERTEN_MINDESTLAENGE` Zeichen), Ergebnis zwischengespeichert.
    fn experten_zeilen(self: &Rc<Self>, text: &str) -> Vec<Zeile> {
        if text.chars().count() < EXPERTEN_MINDESTLAENGE {
            self.leer.set_text(&gettext(
                "Type at least two characters to search all packages.",
            ));
            return Vec::new();
        }
        let gespeichert = self.experten.borrow().clone();
        match gespeichert {
            Some((t, Ok(Some(pakete)))) if t == text => {
                let stand = self.stand.borrow();
                let mut zeilen: Vec<Zeile> = Paketstand::aus(pakete, Vec::new())
                    .pakete
                    .values()
                    .map(|p| Zeile::aus_paket(p, stand.aktualisierbar(&p.name)))
                    .collect();
                zeilen.sort_by_cached_key(|z| logik::sortierschluessel(&z.titel));
                zeilen
            }
            Some((t, Ok(None))) if t == text => {
                self.leer.set_text(&gettext("Searching…"));
                Vec::new()
            }
            Some((t, Err(e))) if t == text => {
                self.leer.set_text(&gettext_f(
                    "Search failed: {error}",
                    &[("error", &fehlertext(&e))],
                ));
                Vec::new()
            }
            _ => {
                *self.experten.borrow_mut() = Some((text.to_string(), Ok(None)));
                self.leer.set_text(&gettext("Searching…"));
                let weak = Rc::downgrade(self);
                let quelle = self.backends.quelle.clone();
                let text = text.to_string();
                glib::MainContext::default().spawn_local(async move {
                    let ergebnis = quelle.suchen(&text, false).await;
                    let Some(s) = weak.upgrade() else {
                        return;
                    };
                    let aktuell = s
                        .experten
                        .borrow()
                        .as_ref()
                        .is_some_and(|(t, _)| *t == text);
                    if aktuell {
                        *s.experten.borrow_mut() = Some((text, ergebnis.map(Some)));
                        s.aktualisiere_liste();
                    }
                });
                Vec::new()
            }
        }
    }

    fn aktualisiere_liste(self: &Rc<Self>) {
        self.leer.set_text(&if self.katalog_laedt.get() {
            gettext("Loading catalog…")
        } else {
            gettext("No applications found.")
        });
        let zeilen = self.aktuelle_zeilen();
        let n = zeilen.len();
        if self.debug {
            // `listenzeile i titel paketname zustand` (erste 40) und `leer text`:
            // für die Szenarien der Integration (SW-6), die Zeilen über den
            // Paketnamen finden.
            for (i, z) in zeilen.iter().take(40).enumerate() {
                debug(&format!(
                    "listenzeile\t{i}\t{}\t{}\t{}",
                    z.titel,
                    z.paketname.as_deref().unwrap_or("-"),
                    z.zustand()
                ));
            }
            if n == 0 {
                debug(&format!("leer\t{}", self.leer.text()));
            }
        }
        let suchtext = self.suchfeld.text().to_string();
        let objekte: Vec<glib::BoxedAnyObject> =
            zeilen.into_iter().map(glib::BoxedAnyObject::new).collect();
        self.modell.splice(0, self.modell.n_items(), &objekte);
        if n > 0 {
            self.auswahl.set_selected(0);
        }
        self.leer.set_visible(n == 0);
        let ansicht = self.ansicht.get();
        let titel = ansicht.titel();
        let anzahl = n as u32;
        self.ueberschrift.set_text(&if !suchtext.trim().is_empty() {
            ngettext_f(
                "{view} – Search “{text}”: {n} result",
                "{view} – Search “{text}”: {n} results",
                anzahl,
                &[("view", &titel), ("text", suchtext.trim())],
            )
        } else if ansicht == Ansicht::Expertenliste {
            ngettext_f(
                "{view} – {n} package",
                "{view} – {n} packages",
                anzahl,
                &[("view", &titel)],
            )
        } else {
            ngettext_f(
                "{view} – {n} application",
                "{view} – {n} applications",
                anzahl,
                &[("view", &titel)],
            )
        });
        if self.debug {
            debug(&format!(
                "ansicht\t{}\t{n}\t{}",
                ansicht.kennung(),
                suchtext.trim()
            ));
            self.debug_layout_spaeter();
        }
    }

    fn aktualisiere_seitenleiste(&self) {
        let n = self.stand.borrow().aktualisierungen.len();
        for (a, _, label) in self.seiten_zeilen.borrow().iter() {
            if *a == Ansicht::Aktualisierungen {
                // TRANSLATORS: sidebar entry with the number of available updates
                label.set_text(&gettext_f("Updates ({n})", &[("n", &n.to_string())]));
                debug(&format!("seitentext\taktualisierungen\t{n}"));
            }
        }
    }

    fn aktualisiere_status(&self) {
        let stand = self.stand.borrow();
        let katalog = self.katalog();
        let programme = katalog.alle().len();
        let installiert = katalog
            .alle()
            .iter()
            .filter(|a| {
                a.paketname
                    .as_deref()
                    .and_then(|n| stand.paket(n))
                    .is_some_and(|p| p.installiert)
            })
            .count();
        let installiert = installiert.to_string();
        let mut text = match self.letzte_pruefung.borrow().as_deref() {
            Some(zeit) => ngettext_f(
                "Catalog: {n} application, {installed} installed · last check for updates {time}",
                "Catalog: {n} applications, {installed} installed · last check for updates {time}",
                programme as u32,
                &[("installed", &installiert), ("time", zeit)],
            ),
            None => ngettext_f(
                "Catalog: {n} application, {installed} installed · no check for updates yet",
                "Catalog: {n} applications, {installed} installed · no check for updates yet",
                programme as u32,
                &[("installed", &installiert)],
            ),
        };
        if let Some(m) = self.letzte_meldung.borrow().as_ref() {
            text.push_str(" · ");
            text.push_str(m);
        }
        if self.status.text() != text {
            self.status.set_text(&text);
            ganz_neu_zeichnen(&self.status_box);
        }
        debug(&format!("status\t{text}"));
    }

    /// Kurze Meldung in der Statuszeile.
    fn melde(&self, text: &str) {
        self.netz_meldung.set(false);
        *self.letzte_meldung.borrow_mut() = Some(text.to_string());
        self.aktualisiere_status();
    }

    // ------------------------------------------------------------ Netzwerk

    /// Stufenprüfung starten (`netz::pruefen`); `danach` läuft mit dem
    /// Ergebnis. Läuft schon eine, wartet `danach` auf deren Ergebnis.
    fn netz_pruefen(self: &Rc<Self>, danach: Option<NetzFn>) {
        if let Some(f) = danach {
            self.netz_warten.borrow_mut().push(f);
        }
        if self.netz_laeuft.replace(true) {
            return;
        }
        if let Some(id) = self.netz_timer.borrow_mut().take() {
            id.remove();
        }
        self.netz_knopf.set_sensitive(false);
        self.setze_netztext(&gettext("checking…"), None, None);
        debug("netz\tpruefe");
        let sonden = self.backends.netz.clone();
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let e = netz::pruefen(sonden.as_ref(), netz::system_quelle(), false).await;
            if let Some(s) = weak.upgrade() {
                s.netz_gemeldet(e);
            }
        });
    }

    fn netz_gemeldet(self: &Rc<Self>, e: netz::Ergebnis) {
        self.netz_laeuft.set(false);
        self.netz_knopf.set_sensitive(true);
        debug(&e.debug_zeile());
        let klasse = if !e.ist_online() {
            Some("qs-netz-offline")
        } else if e.pk_widerspricht() {
            Some("qs-netz-warnung")
        } else {
            None
        };
        self.setze_netztext(&e.kurztext(), Some(&e.hinweis()), klasse);
        if !e.ist_online() {
            self.netz_spaeter_pruefen(netz::NEUPRUEFUNG);
        }
        let war_offline = self.netz.borrow().as_ref().is_some_and(|v| !v.ist_online());
        if war_offline && e.ist_online() && self.netz_meldung.replace(false) {
            debug("netz\tmeldung-entfernt");
            *self.letzte_meldung.borrow_mut() = None;
            self.aktualisiere_status();
        }
        *self.netz.borrow_mut() = Some(e.clone());
        let warten: Vec<NetzFn> = self.netz_warten.borrow_mut().drain(..).collect();
        for f in warten {
            f(self, &e);
        }
    }

    /// Neuprüfung nach `frist`; ersetzt eine schon geplante.
    fn netz_spaeter_pruefen(self: &Rc<Self>, frist: std::time::Duration) {
        if let Some(id) = self.netz_timer.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(frist, move || {
            if let Some(s) = weak.upgrade() {
                // Die Quelle ist mit diesem Aufruf verbraucht, nicht entfernen.
                s.netz_timer.borrow_mut().take();
                s.netz_pruefen(None);
            }
        });
        *self.netz_timer.borrow_mut() = Some(id);
    }

    /// „Netzwerk: …“ rechts in der Statuszeile, Kurzinfo mit allen Stufen;
    /// `klasse`: `qs-netz-offline` (rot) oder `qs-netz-warnung` (PackageKit
    /// widerspricht der erreichbaren Quelle).
    fn setze_netztext(&self, zustand: &str, hinweis: Option<&str>, klasse: Option<&str>) {
        let text = gettext_f("Network: {state}", &[("state", zustand)]);
        for k in ["qs-netz-offline", "qs-netz-warnung"] {
            if Some(k) == klasse {
                self.netz_label.add_css_class(k);
            } else {
                self.netz_label.remove_css_class(k);
            }
        }
        if hinweis.is_some() {
            self.netz_label.set_tooltip_text(hinweis);
        }
        if self.netz_label.text() != text {
            self.netz_label.set_text(&text);
            ganz_neu_zeichnen(&self.status_box);
        }
        debug(&format!("netzstatus\t{text}"));
    }

    /// Nach `no-network` von PackageKit: neu prüfen, dann erklären – entweder
    /// wirklich offline (geprüfte Quelle nennen) oder PackageKit widerspricht
    /// der erreichbaren Quelle (NetworkManager verwaltet die Verbindung nicht).
    fn netz_fehler_erklaeren(self: &Rc<Self>) {
        self.netz_pruefen(Some(Box::new(|s, e| {
            let (titel, text, kennung) = if e.ist_online() {
                let (t, x) = netz::pk_widerspruch_dialog(e);
                (t, x, "netz-pk")
            } else {
                let (t, x) = netz::offline_dialog(e, false);
                (t, x, "netz-offline")
            };
            dialoge::meldung(&s.window, &titel, &text, kennung);
        })));
    }

    // ------------------------------------------------------------ Details

    fn zeige_liste(&self) {
        if self.stack.visible_child_name().as_deref() != Some("liste") {
            self.stack.set_visible_child_name("liste");
            *self.detail_zeile.borrow_mut() = None;
            debug("details\tzu");
        }
    }

    /// Eingabetaste im Suchfeld: Details der gewählten (ersten) Zeile.
    fn oeffne_auswahl(self: &Rc<Self>) {
        let pos = self.auswahl.selected();
        if pos != gtk::INVALID_LIST_POSITION {
            self.oeffne_details_an(pos);
        }
    }

    fn oeffne_details_an(self: &Rc<Self>, pos: u32) {
        let zeile = self
            .modell
            .item(pos)
            .and_then(|o| o.downcast::<glib::BoxedAnyObject>().ok())
            .map(|b| b.borrow::<Zeile>().clone());
        if let Some(z) = zeile {
            self.zeige_details(z);
        }
    }

    fn zeige_details(self: &Rc<Self>, zeile: Zeile) {
        let d = &self.details;
        self.setze_symbol(&d.symbol, &zeile, 64);
        d.name.set_text(&zeile.titel);
        d.zusammenfassung.set_text(&zeile.zusammenfassung);
        d.zustand.set_text(&zeile.zustand());
        d.knopf.set_label(&zeile.knopf());
        d.knopf.set_sensitive(zeile.paket.is_some());
        let (beschreibung, lizenz, homepage) = match &zeile.anwendung {
            Some(a) => (
                a.beschreibung.clone(),
                a.lizenz.clone().unwrap_or_else(|| "–".into()),
                a.homepage.clone(),
            ),
            None => (zeile.zusammenfassung.clone(), "–".into(), None),
        };
        d.beschreibung.set_text(&beschreibung);
        d.lizenz.set_text(&lizenz);
        d.version.set_text(
            zeile
                .paket
                .as_ref()
                .map(|p| p.version.as_str())
                .unwrap_or("–"),
        );
        d.groesse.set_text("…");
        self.setze_homepage(homepage.as_deref());
        *self.detail_zeile.borrow_mut() = Some(zeile.clone());
        self.stack.set_visible_child_name("details");
        d.zurueck.grab_focus();
        debug(&format!(
            "detailsymbol\t{}\t{}",
            zeile.titel,
            bildquelle(&d.symbol)
        ));
        debug(&format!(
            "details\t{}\t{}\t{}",
            zeile.titel,
            zeile.zustand(),
            zeile.knopf()
        ));
        // Paketdetails (Größe, Beschreibung bei reinen Paketen, Lizenz) nachladen.
        if let Some(p) = zeile.paket.clone() {
            let weak = Rc::downgrade(self);
            let quelle = self.backends.quelle.clone();
            let ohne_anwendung = zeile.anwendung.is_none();
            glib::MainContext::default().spawn_local(async move {
                let ergebnis = quelle.details(&p.id).await;
                let Some(s) = weak.upgrade() else {
                    return;
                };
                let noch_dieselbe = s
                    .detail_zeile
                    .borrow()
                    .as_ref()
                    .and_then(|z| z.paket.as_ref())
                    .is_some_and(|q| q.name == p.name);
                if !noch_dieselbe {
                    return;
                }
                match ergebnis {
                    Ok(det) => {
                        s.details
                            .groesse
                            .set_text(&logik::groesse_text(det.groesse_bytes));
                        if ohne_anwendung {
                            s.details.beschreibung.set_text(&det.beschreibung);
                            s.details.lizenz.set_text(&det.lizenz);
                            s.setze_homepage(det.url.as_deref());
                        }
                        debug(&format!(
                            "paketdetails\t{}\t{}",
                            det.paket.name, det.groesse_bytes
                        ));
                    }
                    Err(e) => s.details.groesse.set_text(&fehlertext(&e)),
                }
            });
        } else {
            d.groesse.set_text("–");
        }
        if self.debug {
            self.debug_layout_spaeter();
        }
    }

    fn setze_homepage(&self, url: Option<&str>) {
        match url {
            Some(u) => {
                self.details.homepage.set_uri(u);
                self.details.homepage.set_label(u);
                self.details.homepage.set_visible(true);
                self.details.homepage_fehlt.set_visible(false);
            }
            None => {
                self.details.homepage.set_visible(false);
                self.details.homepage_fehlt.set_visible(true);
            }
        }
    }

    /// Nach einer Transaktion: Zeile der Detailseite nachführen.
    fn aktualisiere_details(self: &Rc<Self>) {
        let alt = self.detail_zeile.borrow().clone();
        let Some(alt) = alt else {
            return;
        };
        let stand = self.stand.borrow();
        let neu = match (&alt.anwendung, &alt.paketname) {
            (Some(a), Some(n)) => Zeile::aus_anwendung(a, stand.paket(n), stand.aktualisierbar(n)),
            (None, Some(n)) => match stand.paket(n) {
                Some(p) => Zeile::aus_paket(p, stand.aktualisierbar(n)),
                None => alt.clone(),
            },
            _ => alt.clone(),
        };
        drop(stand);
        if neu != alt {
            self.zeige_details(neu);
        }
    }

    // ------------------------------------------------------------ Transaktion

    /// Knopf in Liste oder Detailseite: Installieren / Entfernen / Aktualisieren.
    fn aktion(self: &Rc<Self>, zeile: Zeile) {
        let trotzdem = self.netz_trotzdem.replace(false);
        if self.transaktion.borrow().is_some() {
            dialoge::meldung(
                &self.window,
                &gettext("Operation in Progress"),
                &gettext(
                    "Another operation is already running. Please wait until it has finished, \
                     or cancel it.",
                ),
                "laeuft",
            );
            return;
        }
        let Some(paket) = zeile.paket.clone() else {
            return;
        };
        let (vorgang, paket) = if zeile.aktualisierbar {
            let neu = self
                .stand
                .borrow()
                .aktualisierungen
                .iter()
                .find(|a| a.paket.name == paket.name)
                .map(|a| a.paket.clone())
                .unwrap_or(paket);
            (Vorgang::Aktualisieren, neu)
        } else if zeile.installiert {
            (Vorgang::Entfernen, paket)
        } else {
            (Vorgang::Installieren, paket)
        };
        // QD-067: Sagte die letzte Prüfung in allen Stufen offline, vor dem
        // Installieren/Aktualisieren frisch prüfen und, wenn es dabei bleibt,
        // fragen statt sperren: Pakete aus dem apt-Zwischenspeicher gehen auch
        // ohne Netz (NM-b S1). Entfernen braucht kein Netz. Meldet nur
        // PackageKit offline, läuft der Versuch ohne Rückfrage.
        let offline = self.netz.borrow().as_ref().is_some_and(|e| !e.ist_online());
        if vorgang != Vorgang::Entfernen && offline && !trotzdem {
            debug("netz\tvor-vorgang");
            self.netz_pruefen(Some(Box::new(move |s, e| {
                if e.ist_online() {
                    s.aktion(zeile);
                    return;
                }
                debug("netz\tfrage-offline");
                let (titel, text) = netz::offline_dialog(e, true);
                let w = Rc::downgrade(s);
                let zeile = zeile.clone();
                dialoge::offline_frage(
                    &s.window,
                    &titel,
                    &text,
                    Box::new(move || {
                        if let Some(s) = w.upgrade() {
                            debug("netz\ttrotzdem");
                            s.netz_trotzdem.set(true);
                            s.aktion(zeile.clone());
                        }
                    }),
                );
            })));
            return;
        }
        let auftrag = Auftrag {
            vorgang: vorgang.clone(),
            pakete: vec![paket.clone()],
            anwendung: zeile.anwendung.clone(),
        };
        self.fortschritt
            .vorgang
            .set_text(&vorgang_titel(&vorgang, &zeile.titel, &paket.name));
        self.fortschritt.balken.set_fraction(0.0);
        self.fortschritt.balken.set_text(Some("0 %"));
        self.fortschritt.status.set_text(&gettext("Preparing…"));
        self.fortschritt.leiste.set_visible(true);
        self.fortschritt.abbrechen.set_sensitive(true);
        let generation = self.naechste_generation();
        let melden = self.melde_rueckruf(generation, false);
        let griff = self
            .backends
            .monitor
            .starten(auftrag, self.backends.quelle.clone(), melden);
        *self.transaktion.borrow_mut() = Some(Transaktion {
            generation,
            griff,
            vorgang: Some(vorgang.clone()),
            zeile,
            simulation: None,
            sperrdialog_offen: false,
            nachfrage_offen: false,
        });
        debug(&format!(
            "transaktion\tgestartet\t{}\t{}",
            vorgang_text(&vorgang),
            paket.name
        ));
        if self.debug {
            self.debug_layout_spaeter();
        }
    }

    fn naechste_generation(&self) -> u64 {
        self.generation.set(self.generation.get() + 1);
        self.generation.get()
    }

    /// Rückruf für den Monitor. Meldungen laufen immer über den
    /// Hauptschleifen-Leerlauf, damit der Griff vor der ersten Meldung
    /// gespeichert ist und Dialoge nie aus einem Klick-Handler heraus
    /// geöffnet werden. `beobachter`: Rückruf des Wiederanbindungs-Griffs.
    fn melde_rueckruf(self: &Rc<Self>, generation: u64, beobachter: bool) -> Box<dyn Fn(Zustand)> {
        let weak = Rc::downgrade(self);
        Box::new(move |z| {
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(s) = weak.upgrade() {
                    if beobachter {
                        s.beobachter_gemeldet(generation, z);
                    } else {
                        s.zustand_gemeldet(generation, z);
                    }
                }
            });
        })
    }

    /// Nach einem Neustart: laufende Transaktionen wieder anbinden. Der
    /// Monitor (SW-4) liefert immer einen Beobachter-Griff; er meldet `Laeuft`,
    /// solange beim Daemon eine frühere Transaktion läuft, sonst gleich
    /// `Fertig`. Erst bei `Laeuft` erscheint die Fortschrittsleiste; der
    /// Beobachter blockiert eigene Vorgänge nicht.
    fn wieder_anbinden(self: &Rc<Self>) {
        let generation = self.naechste_generation();
        let melden = self.melde_rueckruf(generation, true);
        let mut griffe = self
            .backends
            .monitor
            .wieder_anbinden(self.backends.quelle.clone(), melden);
        if griffe.is_empty() {
            debug("transaktion\twieder-angebunden\t0");
            return;
        }
        let griff = griffe.remove(0);
        *self.beobachter.borrow_mut() = Some((generation, griff, false));
    }

    fn beobachter_gemeldet(self: &Rc<Self>, generation: u64, zustand: Zustand) {
        let sichtbar = match self.beobachter.borrow().as_ref() {
            Some((g, _, sichtbar)) if *g == generation => *sichtbar,
            _ => return,
        };
        let eigener_vorgang = self.transaktion.borrow().is_some();
        match zustand {
            Zustand::Laeuft { status, .. } => {
                if !sichtbar {
                    debug("transaktion\twieder-angebunden\t1");
                }
                if let Some(b) = self.beobachter.borrow_mut().as_mut() {
                    b.2 = true;
                }
                if !eigener_vorgang {
                    let f = &self.fortschritt;
                    f.vorgang.set_text(&gettext(
                        "Reattached: package manager operation in progress",
                    ));
                    f.balken.set_fraction(0.0);
                    f.balken.set_text(Some("…"));
                    f.status.set_text(&fremdtexte::status(&status));
                    f.leiste.set_visible(true);
                    f.abbrechen.set_sensitive(true);
                }
            }
            ende => {
                *self.beobachter.borrow_mut() = None;
                if !sichtbar {
                    debug("transaktion\twieder-angebunden\t0");
                    if let Zustand::Fehlgeschlagen(e) = &ende {
                        eprintln!(
                            "querencia-software: Wiederanbindung: {}",
                            qs_transaktion::fehlertexte::kurz(e)
                        );
                    }
                    return;
                }
                debug("transaktion\twieder-angebunden-ende");
                if !eigener_vorgang {
                    self.fortschritt.leiste.set_visible(false);
                }
                self.melde(&gettext(
                    "An earlier package manager operation has finished.",
                ));
                self.paketstand_laden(false);
            }
        }
    }

    fn abbrechen(&self) {
        if let Some(t) = self.transaktion.borrow().as_ref() {
            self.fortschritt.abbrechen.set_sensitive(false);
            self.fortschritt.status.set_text(&gettext("Cancelling…"));
            t.griff.abbrechen();
        } else if let Some((_, griff, true)) = self.beobachter.borrow().as_ref() {
            self.fortschritt.abbrechen.set_sensitive(false);
            self.fortschritt.status.set_text(&gettext("Cancelling…"));
            griff.abbrechen();
        }
    }

    fn zustand_gemeldet(self: &Rc<Self>, generation: u64, zustand: Zustand) {
        let f = &self.fortschritt;
        let (titel, paketname) = match self.transaktion.borrow().as_ref() {
            Some(t) if t.generation == generation => (
                t.zeile.titel.clone(),
                t.zeile.paketname.clone().unwrap_or_default(),
            ),
            _ => return,
        };
        match zustand {
            Zustand::Simuliert(sim) => {
                debug(&format!(
                    "transaktion\tsimuliert\t{}",
                    sim.entfernen
                        .iter()
                        .map(|p| p.name.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                ));
                debug(&format!(
                    "simulation\t{}\t{}\t{}",
                    sim.installieren
                        .iter()
                        .map(|p| p.name.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                    sim.aktualisieren.len(),
                    sim.download_bytes
                ));
                if let Some(t) = self.transaktion.borrow_mut().as_mut() {
                    t.simulation = Some(sim);
                }
                f.status
                    .set_text(&gettext("Simulation finished, waiting for confirmation…"));
            }
            Zustand::WartetAufBestaetigung => {
                debug("transaktion\twartet-auf-bestaetigung");
                let (sim, vorgang) = self
                    .transaktion
                    .borrow()
                    .as_ref()
                    .map(|t| (t.simulation.clone().unwrap_or_default(), t.vorgang.clone()))
                    .unwrap_or_default();
                let entfernen = vorgang == Some(Vorgang::Entfernen);
                let w1 = Rc::downgrade(self);
                let w2 = Rc::downgrade(self);
                let bestaetigt: Box<dyn Fn()> = Box::new(move || {
                    if let Some(s) = w1.upgrade() {
                        if let Some(t) = s.transaktion.borrow().as_ref() {
                            s.fortschritt.status.set_text(&if entfernen {
                                gettext("Confirmed, removing…")
                            } else {
                                gettext("Confirmed, installing…")
                            });
                            t.griff.bestaetigen();
                        }
                    }
                });
                let abgelehnt: Box<dyn Fn()> = Box::new(move || {
                    if let Some(s) = w2.upgrade() {
                        s.abbrechen();
                    }
                });
                if entfernen {
                    dialoge::bestaetigen_entfernen(
                        &self.window,
                        &titel,
                        &sim,
                        bestaetigt,
                        abgelehnt,
                    );
                } else {
                    // SW-4: auch Installieren mit Zusatzpaketen oder > 50 MiB.
                    dialoge::bestaetigen_installieren(
                        &self.window,
                        &titel,
                        &sim,
                        bestaetigt,
                        abgelehnt,
                    );
                }
            }
            Zustand::Laeuft { prozent, status } => {
                match prozent {
                    Some(p) => {
                        f.balken.set_fraction(f64::from(p) / 100.0);
                        f.balken.set_text(Some(&format!("{p} %")));
                    }
                    None => f.balken.set_text(Some("…")),
                }
                f.status.set_text(&fremdtexte::status(&status));
                debug(&format!(
                    "transaktion\tlaeuft\t{}\t{status}",
                    prozent.map(|p| p.to_string()).unwrap_or_else(|| "-".into())
                ));
                let nachfrage = qs_transaktion::ist_nachfrage(&Zustand::Laeuft {
                    prozent,
                    status: status.clone(),
                });
                let schon_offen = !nachfrage
                    || self
                        .transaktion
                        .borrow_mut()
                        .as_mut()
                        .map(|t| std::mem::replace(&mut t.nachfrage_offen, true))
                        .unwrap_or(true);
                if !schon_offen {
                    let w1 = Rc::downgrade(self);
                    let w2 = Rc::downgrade(self);
                    dialoge::nachfrage(
                        &self.window,
                        &fremdtexte::status(&status),
                        Box::new(move || {
                            if let Some(s) = w1.upgrade() {
                                if let Some(t) = s.transaktion.borrow_mut().as_mut() {
                                    t.nachfrage_offen = false;
                                }
                            }
                        }),
                        Box::new(move || {
                            if let Some(s) = w2.upgrade() {
                                s.abbrechen();
                            }
                        }),
                    );
                }
            }
            Zustand::WartetAufSperre { .. } => {
                debug("transaktion\twartet-auf-sperre");
                f.status.set_text(&gettext("Waiting for the package lock…"));
                let schon_offen = self
                    .transaktion
                    .borrow_mut()
                    .as_mut()
                    .map(|t| std::mem::replace(&mut t.sperrdialog_offen, true))
                    .unwrap_or(true);
                if !schon_offen {
                    let w1 = Rc::downgrade(self);
                    let w2 = Rc::downgrade(self);
                    dialoge::sperre_belegt(
                        &self.window,
                        &gettext("Another program is using the package manager right now."),
                        Box::new(move || {
                            if let Some(s) = w1.upgrade() {
                                if let Some(t) = s.transaktion.borrow_mut().as_mut() {
                                    t.sperrdialog_offen = false;
                                    s.fortschritt.status.set_text(&gettext("Trying again…"));
                                    t.griff.sperre_erneut_versuchen();
                                }
                            }
                        }),
                        Box::new(move || {
                            if let Some(s) = w2.upgrade() {
                                s.abbrechen();
                            }
                        }),
                    );
                }
            }
            Zustand::Fertig => {
                debug(&format!("transaktion\tfertig\t{paketname}"));
                let (vorgang, neustart) = self
                    .transaktion
                    .borrow()
                    .as_ref()
                    .map(|t| {
                        (
                            t.vorgang.clone(),
                            self.backends.monitor.braucht_neustart(t.griff.id),
                        )
                    })
                    .unwrap_or_default();
                self.beende_transaktion();
                if neustart {
                    debug("transaktion\tneustart");
                }
                let text = fertig_text(vorgang.as_ref(), neustart, &titel);
                self.benachrichtigen(&text);
                self.melde(&text);
                self.paketstand_laden(false);
            }
            Zustand::Fehlgeschlagen(e) => {
                debug(&format!("transaktion\tfehler\t{e:?}"));
                self.beende_transaktion();
                self.benachrichtigen(&gettext_f("{name}: operation failed", &[("name", &titel)]));
                self.melde(&format!("{titel}: {}", fehlertext(&e)));
                if e == Fehler::NetzFehlt {
                    self.netz_meldung.set(true);
                    self.netz_fehler_erklaeren();
                    return;
                }
                // Dialogtexte von SW-4 (Überschrift, Erklärung, Handlung), übersetzt.
                let text = fremdtexte::fehlerdialog(&e);
                let kennung = match e {
                    Fehler::KeinAgent => "kein-agent",
                    Fehler::NichtBerechtigt => "nicht-berechtigt",
                    _ => "fehler",
                };
                dialoge::fehler(&self.window, &text, kennung);
            }
            Zustand::Abgebrochen => {
                debug("transaktion\tabgebrochen");
                self.beende_transaktion();
                self.melde(&gettext_f(
                    "{name}: operation cancelled.",
                    &[("name", &titel)],
                ));
                self.paketstand_laden(false);
            }
        }
    }

    /// Meldung an den Benachrichtigungsdienst der Sitzung, wenn kein Fenster
    /// des Programms aktiv ist (Vorgang im Hintergrund abgeschlossen). „Nicht
    /// stören“ entscheidet der Dienst (Querencia Benachrichtigungen).
    fn benachrichtigen(&self, text: &str) {
        let Some(app) = self.window.application() else {
            return;
        };
        // Ein eigener Dialog (Info, Sperre …) zählt als aktiv.
        if app.windows().iter().any(|w| w.is_active()) {
            return;
        }
        let n = gio::Notification::new(APP_NAME);
        n.set_body(Some(text));
        n.set_icon(&gio::ThemedIcon::new(crate::APP_ID));
        app.send_notification(Some("vorgang"), &n);
        debug(&format!("benachrichtigung\t{text}"));
    }

    fn beende_transaktion(&self) {
        *self.transaktion.borrow_mut() = None;
        self.fortschritt.leiste.set_visible(false);
    }

    // ------------------------------------------------------------ Debug

    /// Nur für die isolierte GUI-Prüfung (`QS_DEBUG`): Zeigerbewegungen und
    /// Layout – daraus bestimmt das Prüfskript die Fensterlage.
    fn install_debug(self: &Rc<Self>) {
        let motion = gtk::EventControllerMotion::new();
        motion.connect_motion(|_, x, y| debug(&format!("zeiger\t{x:.0}\t{y:.0}")));
        self.window.add_controller(motion);
    }

    fn debug_layout_spaeter(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
            if let Some(s) = weak.upgrade() {
                s.debug_layout();
            }
        });
    }

    fn debug_layout(&self) {
        let w = &self.window;
        let rechteck = |name: &str, widget: &gtk::Widget| {
            if !widget.is_visible() {
                return;
            }
            if let Some(r) = widget.compute_bounds(w) {
                debug(&format!(
                    "{name}\t{:.0}\t{:.0}\t{:.0}\t{:.0}",
                    r.x(),
                    r.y(),
                    r.width(),
                    r.height()
                ));
            }
        };
        debug("layout\tanfang");
        if let Some(bar) = finde_typ(w.upcast_ref(), gtk::PopoverMenuBar::static_type()) {
            let mut item = bar.first_child();
            let mut i = 0;
            while let Some(it) = item {
                rechteck(&format!("menue\t{i}"), &it);
                i += 1;
                item = it.next_sibling();
            }
        }
        for (a, row, _) in self.seiten_zeilen.borrow().iter() {
            rechteck(&format!("seite\t{}", a.kennung()), row.upcast_ref());
        }
        rechteck("suche", self.suchfeld.upcast_ref());
        rechteck("liste", self.listview.upcast_ref());
        let mut kind = self.listview.first_child();
        let mut i = 0;
        while let Some(k) = kind {
            let sichtbar = k.compute_bounds(w).is_some_and(|r| r.width() > 100.0);
            if let (true, Some(hbox)) = (sichtbar, finde_klasse(&k, "qs-zeile")) {
                let titel = finde_klasse(&hbox, "qs-name")
                    .and_downcast::<gtk::Label>()
                    .map(|l| l.text().to_string())
                    .unwrap_or_default();
                rechteck(&format!("zeile\t{i}\t{titel}"), &hbox);
                if let Some(img) = finde_klasse(&hbox, "qs-symbol").and_downcast::<gtk::Image>() {
                    debug(&format!("zeilensymbol\t{i}\t{titel}\t{}", bildquelle(&img)));
                }
                if let Some(b) = finde_klasse(&hbox, "qs-knopf").and_downcast::<gtk::Button>() {
                    let text = b.label().map(|t| t.to_string()).unwrap_or_default();
                    rechteck(&format!("knopf\t{i}\t{text}"), b.upcast_ref());
                }
                i += 1;
            }
            kind = k.next_sibling();
        }
        if self.stack.visible_child_name().as_deref() == Some("details") {
            rechteck("zurueck", self.details.zurueck.upcast_ref());
            let text = self
                .details
                .knopf
                .label()
                .map(|t| t.to_string())
                .unwrap_or_default();
            rechteck(&format!("aktion\t{text}"), self.details.knopf.upcast_ref());
            rechteck("homepage", self.details.homepage.upcast_ref());
        }
        rechteck("abbrechen", self.fortschritt.abbrechen.upcast_ref());
        rechteck("statuszeile", self.status_box.upcast_ref());
        rechteck("netzfeld", self.netz_label.upcast_ref());
        rechteck("netzknopf", self.netz_knopf.upcast_ref());
        debug(&format!("fenster\t{}\t{}", w.width(), w.height()));
        for t in gtk::Window::list_toplevels() {
            if let Ok(t) = t.downcast::<gtk::Window>() {
                debug(&format!(
                    "toplevel\t{}\t{}\t{}\t{}\t{}",
                    t.type_().name(),
                    t.title().unwrap_or_default(),
                    t.is_visible(),
                    t.width(),
                    t.height()
                ));
            }
        }
        debug("layout\tende");
    }
}

fn vorgang_text(v: &Vorgang) -> String {
    match v {
        Vorgang::Installieren => gettext("Install"),
        Vorgang::Entfernen => gettext("Remove"),
        Vorgang::Aktualisieren => gettext("Update"),
        Vorgang::QuellenAuffrischen => gettext("Refresh software sources"),
    }
}

/// Erste Zeile der Fortschrittsleiste: „Installieren: GIMP (gimp)“.
fn vorgang_titel(v: &Vorgang, name: &str, paket: &str) -> String {
    let args = [("name", name), ("package", paket)];
    match v {
        Vorgang::Installieren => gettext_f("Install: {name} ({package})", &args),
        Vorgang::Entfernen => gettext_f("Remove: {name} ({package})", &args),
        Vorgang::Aktualisieren => gettext_f("Update: {name} ({package})", &args),
        Vorgang::QuellenAuffrischen => {
            gettext_f("Refresh software sources: {name} ({package})", &args)
        }
    }
}

/// Meldung nach einem erfolgreichen Vorgang (Statuszeile, Benachrichtigung).
fn fertig_text(v: Option<&Vorgang>, neustart: bool, name: &str) -> String {
    let args = [("name", name)];
    match (v, neustart) {
        (Some(Vorgang::Installieren), false) => gettext_f("{name} was installed.", &args),
        (Some(Vorgang::Entfernen), false) => gettext_f("{name} was removed.", &args),
        (Some(Vorgang::Aktualisieren), false) => gettext_f("{name} was updated.", &args),
        (_, false) => gettext_f("{name} was completed.", &args),
        (Some(Vorgang::Installieren), true) => {
            gettext_f("{name} was installed. A restart is required.", &args)
        }
        (Some(Vorgang::Entfernen), true) => {
            gettext_f("{name} was removed. A restart is required.", &args)
        }
        (Some(Vorgang::Aktualisieren), true) => {
            gettext_f("{name} was updated. A restart is required.", &args)
        }
        (_, true) => gettext_f("{name} was completed. A restart is required.", &args),
    }
}

/// Lässt die Statusleiste nach einem neuen Text vollständig neu zeichnen und
/// nicht nur den Bereich des Texts (Werkstatt QREL-05 W.5).
///
/// Mit den GPU-Renderern von GTK 4.22 (Vulkan, NGL; live über die GPU der VM)
/// blieb nach einem Textwechsel manchmal die oberste Bildpunktzeile des alten
/// Texts stehen: i-Punkte, Umlaute, Oberlängen (verwandt: GTK-Issue 6531).
/// Mit `GSK_DEBUG=full-redraw` oder dem Cairo-Renderer tritt das nicht auf.
/// Abhilfe: zwei Frames lang trägt die Leiste die Klasse `qs-neu`, deren
/// Hintergrund sich um einen Farbwert unterscheidet (style.css). Der
/// geänderte Farbknoten umfasst die ganze Leiste, also zeichnet GTK sie in
/// beiden Frames vollständig neu; danach gilt wieder die normale Farbe.
/// Eine Deckkraft knapp unter 1 statt der Farbe hat im Versuch nicht gereicht.
fn ganz_neu_zeichnen(leiste: &gtk::Box) {
    leiste.add_css_class("qs-neu");
    // Der erste Tick fällt in den Frame, der schon mit `qs-neu` zeichnet;
    // erst der zweite nimmt die Klasse wieder weg.
    let ticks = Cell::new(0u8);
    leiste.add_tick_callback(move |w, _| {
        ticks.set(ticks.get() + 1);
        if ticks.get() < 2 {
            return glib::ControlFlow::Continue;
        }
        w.remove_css_class("qs-neu");
        glib::ControlFlow::Break
    });
}

fn uhrzeit() -> String {
    glib::DateTime::now_local()
        .ok()
        .and_then(|d| d.format("%H:%M").ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "–".into())
}

fn symbol_vorhanden(name: &str) -> bool {
    gdk::Display::default()
        .map(|d| gtk::IconTheme::for_display(&d).has_icon(name))
        .unwrap_or(false)
}

/// Woher ein Bild kommt (nur für die Debug-Ausgabe): `thema:<name>`,
/// `datei:<pfad>` oder `leer`.
fn bildquelle(img: &gtk::Image) -> String {
    if let Some(n) = img.icon_name() {
        format!("thema:{n}")
    } else if let Some(f) = img.file() {
        format!("datei:{f}")
    } else {
        "leer".into()
    }
}

/// Ist unter `wurzel` ein Popover (Menü) sichtbar?
fn menue_offen(wurzel: &gtk::Widget) -> bool {
    let mut kind = wurzel.first_child();
    while let Some(k) = kind {
        if (k.is::<gtk::Popover>() && k.is_visible()) || menue_offen(&k) {
            return true;
        }
        kind = k.next_sibling();
    }
    false
}

/// Erstes Widget des Typs `typ` unter `wurzel` (Tiefensuche).
fn finde_typ(wurzel: &gtk::Widget, typ: glib::Type) -> Option<gtk::Widget> {
    let mut kind = wurzel.first_child();
    while let Some(k) = kind {
        if k.type_().is_a(typ) {
            return Some(k);
        }
        if let Some(t) = finde_typ(&k, typ) {
            return Some(t);
        }
        kind = k.next_sibling();
    }
    None
}

/// Erstes Widget mit der CSS-Klasse `klasse` unter `wurzel` (einschließlich).
fn finde_klasse(wurzel: &gtk::Widget, klasse: &str) -> Option<gtk::Widget> {
    if wurzel.has_css_class(klasse) {
        return Some(wurzel.clone());
    }
    let mut kind = wurzel.first_child();
    while let Some(k) = kind {
        if let Some(t) = finde_klasse(&k, klasse) {
            return Some(t);
        }
        kind = k.next_sibling();
    }
    None
}
