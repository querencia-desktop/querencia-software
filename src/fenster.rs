//! Das Hauptfenster: Menüleiste Datei/Bearbeiten/Ansicht/Hilfe, links die
//! Seitenleiste (Alle, 10 Kategorien, Installiert, Aktualisierungen (n),
//! Expertenliste), rechts Suchfeld, Liste oder Detailseite, darunter die
//! Fortschrittsleiste (nur während eines Vorgangs) und die Statuszeile.
//! Keine eigene Titelleiste: labwc zeichnet den klassischen Rahmen.
//!
//! Tastatur: Strg+F Suche · Escape zurück (Details → Liste, sonst Suche
//! leeren) · Eingabetaste öffnet die Details der gewählten Zeile (auch aus
//! dem Suchfeld) · Strg+Q Beenden · F5 Aktualisierungen prüfen.
//!
//! Debug-Ausgabe (`QS_DEBUG=1`, Tab-getrennt, für `werkzeug/szenario.sh`):
//! `ansicht <kennung> <zeilen> <suchtext>` · `details <titel> <zustand> <knopf>`
//! · `transaktion <zustand> …` · `status <text>` · `dialog <kennung>` ·
//! `zeiger x y` · Layout: `fenster w h`, `menue i x y w h`,
//! `seite <kennung> x y w h`, `suche x y w h`, `zeile i <titel> x y w h`,
//! `knopf i <text> x y w h`, `zurueck …`, `aktion <text> …`, `abbrechen …`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use qs_typen::{Fehler, Symbol, Vorgang};

use crate::debug;
use crate::dialoge;
use crate::logik::{self, Ansicht, Paketstand, Zeile};
use crate::monitor::fehlertext;
use crate::schnittstellen::{Auftrag, Simulation, TransaktionsGriff, Zustand};
use crate::{Backends, APP_NAME};

const CSS: &str = include_str!("style.css");

/// Rückruf einer Fensteraktion.
type ActionFn = Box<dyn Fn(&Rc<Fenster>)>;

const FALLBACK_SYMBOL: &str = "package-x-generic";

pub fn load_css() {
    let provider = gtk::CssProvider::new();
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
    s1.append(Some("Aktualisierungen _prüfen"), Some("win.pruefen"));
    datei.append_section(None, &s1);
    let s2 = gio::Menu::new();
    s2.append(Some("_Beenden"), Some("win.quit"));
    datei.append_section(None, &s2);

    let bearbeiten = gio::Menu::new();
    bearbeiten.append(Some("_Suchen"), Some("win.suchen"));

    let ansicht = gio::Menu::new();
    ansicht.append(Some("_Alle Programme"), Some("win.ansicht::alle"));
    ansicht.append(Some("_Installiert"), Some("win.ansicht::installiert"));
    ansicht.append(
        Some("A_ktualisierungen"),
        Some("win.ansicht::aktualisierungen"),
    );
    ansicht.append(Some("_Expertenliste"), Some("win.ansicht::experten"));

    let hilfe = gio::Menu::new();
    hilfe.append(Some("_Info"), Some("win.about"));

    let bar = gio::Menu::new();
    bar.append_submenu(Some("_Datei"), &datei);
    bar.append_submenu(Some("_Bearbeiten"), &bearbeiten);
    bar.append_submenu(Some("_Ansicht"), &ansicht);
    bar.append_submenu(Some("_Hilfe"), &hilfe);
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
    griff: TransaktionsGriff,
    vorgang: Vorgang,
    zeile: Zeile,
    simulation: Option<Simulation>,
    sperrdialog_offen: bool,
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
    status: gtk::Label,
    letzte_pruefung: RefCell<Option<String>>,
    letzte_meldung: RefCell<Option<String>>,
    /// Nach dem nächsten Laden „Aktualisierungen geprüft“ melden.
    pruefung_melden: Cell<bool>,
    debug: bool,
}

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
        let ueberschrift = gtk::Label::new(Some("Alle Programme"));
        ueberschrift.add_css_class("qs-ueberschrift");
        ueberschrift.set_xalign(0.0);
        ueberschrift.set_hexpand(true);
        ueberschrift.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let suchfeld = gtk::SearchEntry::new();
        suchfeld.set_placeholder_text(Some("Programme suchen …"));
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
        let leer = gtk::Label::new(Some("Keine Programme gefunden."));
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
            status,
            letzte_pruefung: RefCell::new(None),
            letzte_meldung: RefCell::new(None),
            pruefung_melden: Cell::new(false),
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
        fenster.paketstand_laden(true);
        fenster
    }

    fn baue_details() -> Detailseite {
        let seite = gtk::Box::new(gtk::Orientation::Vertical, 12);
        seite.add_css_class("qs-details");
        let zurueck = gtk::Button::with_mnemonic("← _Zurück");
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
        let knopf = gtk::Button::with_label("Installieren");
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
        let version = fakt(0, "Version");
        let groesse = fakt(1, "Größe");
        let lizenz = fakt(2, "Lizenz");
        let hp_titel = gtk::Label::new(Some("Homepage"));
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
        let abbrechen = gtk::Button::with_mnemonic("Ab_brechen");
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
            let knopf = gtk::Button::with_label("Installieren");
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
                z.set_text(zeile.zustand());
            }
            if let Some(k) = knopf {
                k.set_label(zeile.knopf());
                k.set_sensitive(zeile.paket.is_some());
            }
        });
    }

    /// Symbol einer Zeile: Datei vom Katalog, sonst Themenname, sonst Fallback.
    fn setze_symbol(&self, image: &gtk::Image, zeile: &Zeile, groesse: u32) {
        image.set_pixel_size(groesse as i32);
        if let Some(a) = &zeile.anwendung {
            if let Some(pfad) = self.backends.katalog.symbol_pfad(a, groesse) {
                image.set_from_file(Some(&pfad));
                return;
            }
        }
        match &zeile.symbol {
            Symbol::Thema(name) if symbol_vorhanden(name) => image.set_icon_name(Some(name)),
            Symbol::Datei(p) if p.exists() => image.set_from_file(Some(p)),
            _ => image.set_icon_name(Some(FALLBACK_SYMBOL)),
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
        let weak = Rc::downgrade(self);
        let quelle = self.backends.quelle.clone();
        glib::MainContext::default().spawn_local(async move {
            let pakete = quelle.suchen("", false).await;
            let akt = quelle.aktualisierungen().await;
            let Some(s) = weak.upgrade() else {
                return;
            };
            match (pakete, akt) {
                (Ok(p), Ok(a)) => {
                    *s.stand.borrow_mut() = Paketstand::aus(p, a);
                    if pruefung {
                        *s.letzte_pruefung.borrow_mut() = Some(uhrzeit());
                    }
                    if s.pruefung_melden.replace(false) {
                        let n = s.stand.borrow().aktualisierungen.len();
                        *s.letzte_meldung.borrow_mut() =
                            Some(format!("Aktualisierungen geprüft: {n} verfügbar"));
                    }
                }
                (Err(e), _) | (_, Err(e)) => {
                    s.melde(&format!("Paketquelle nicht erreichbar: {}", fehlertext(&e)));
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
        self.melde("Prüfe auf Aktualisierungen …");
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

    fn aktuelle_zeilen(&self) -> Vec<Zeile> {
        let stand = self.stand.borrow();
        logik::zeilen_gesucht(
            self.ansicht.get(),
            &self.suchfeld.text(),
            &*self.backends.katalog,
            &stand,
        )
    }

    fn aktualisiere_liste(self: &Rc<Self>) {
        let zeilen = self.aktuelle_zeilen();
        let n = zeilen.len();
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
        let einheit = if ansicht == Ansicht::Expertenliste {
            "Pakete"
        } else {
            "Programme"
        };
        self.ueberschrift.set_text(&if suchtext.trim().is_empty() {
            format!("{titel} – {n} {einheit}")
        } else {
            format!("{titel} – Suche „{}“: {n} Treffer", suchtext.trim())
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
                label.set_text(&format!("Aktualisierungen ({n})"));
                debug(&format!("seitentext\taktualisierungen\t{n}"));
            }
        }
    }

    fn aktualisiere_status(&self) {
        let stand = self.stand.borrow();
        let katalog = &self.backends.katalog;
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
        let pruefung = self
            .letzte_pruefung
            .borrow()
            .clone()
            .map(|t| format!("letzte Aktualisierungsprüfung {t}"))
            .unwrap_or_else(|| "noch keine Aktualisierungsprüfung".into());
        let mut text =
            format!("Katalog: {programme} Programme, {installiert} installiert · {pruefung}");
        if let Some(m) = self.letzte_meldung.borrow().as_ref() {
            text.push_str(" · ");
            text.push_str(m);
        }
        self.status.set_text(&text);
        debug(&format!("status\t{text}"));
    }

    /// Kurze Meldung in der Statuszeile.
    fn melde(&self, text: &str) {
        *self.letzte_meldung.borrow_mut() = Some(text.to_string());
        self.aktualisiere_status();
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
        d.zustand.set_text(zeile.zustand());
        d.knopf.set_label(zeile.knopf());
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
        if self.transaktion.borrow().is_some() {
            dialoge::meldung(
                &self.window,
                "Vorgang läuft",
                "Es läuft bereits ein Vorgang. Bitte warten Sie, bis er abgeschlossen ist, \
                 oder brechen Sie ihn ab.",
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
        let auftrag = Auftrag {
            vorgang: vorgang.clone(),
            pakete: vec![paket.clone()],
            anwendung: zeile.anwendung.clone(),
        };
        self.fortschritt.vorgang.set_text(&format!(
            "{}: {} ({})",
            vorgang_text(&vorgang),
            zeile.titel,
            paket.name
        ));
        self.fortschritt.balken.set_fraction(0.0);
        self.fortschritt.balken.set_text(Some("0 %"));
        self.fortschritt.status.set_text("Wird vorbereitet …");
        self.fortschritt.leiste.set_visible(true);
        self.fortschritt.abbrechen.set_sensitive(true);
        // Meldungen immer über den Hauptschleifen-Leerlauf, damit der Griff
        // vor der ersten Meldung gespeichert ist und Dialoge nie aus dem
        // Klick-Handler heraus geöffnet werden.
        let weak = Rc::downgrade(self);
        let melden: Box<dyn Fn(Zustand)> = Box::new(move |z| {
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(s) = weak.upgrade() {
                    s.zustand_gemeldet(z);
                }
            });
        });
        let griff = self
            .backends
            .monitor
            .starten(auftrag, self.backends.quelle.clone(), melden);
        *self.transaktion.borrow_mut() = Some(Transaktion {
            griff,
            vorgang: vorgang.clone(),
            zeile,
            simulation: None,
            sperrdialog_offen: false,
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

    fn abbrechen(&self) {
        if let Some(t) = self.transaktion.borrow().as_ref() {
            self.fortschritt.abbrechen.set_sensitive(false);
            self.fortschritt.status.set_text("Wird abgebrochen …");
            t.griff.abbrechen();
        }
    }

    fn zustand_gemeldet(self: &Rc<Self>, zustand: Zustand) {
        let f = &self.fortschritt;
        let (titel, paketname) = match self.transaktion.borrow().as_ref() {
            Some(t) => (
                t.zeile.titel.clone(),
                t.zeile.paketname.clone().unwrap_or_default(),
            ),
            None => return,
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
                if let Some(t) = self.transaktion.borrow_mut().as_mut() {
                    t.simulation = Some(sim);
                }
                f.status
                    .set_text("Simulation abgeschlossen, warte auf Bestätigung …");
            }
            Zustand::WartetAufBestaetigung => {
                debug("transaktion\twartet-auf-bestaetigung");
                let sim = self
                    .transaktion
                    .borrow()
                    .as_ref()
                    .and_then(|t| t.simulation.clone())
                    .unwrap_or_default();
                let w1 = Rc::downgrade(self);
                let w2 = Rc::downgrade(self);
                dialoge::bestaetigen_entfernen(
                    &self.window,
                    &titel,
                    &sim,
                    Box::new(move || {
                        if let Some(s) = w1.upgrade() {
                            if let Some(t) = s.transaktion.borrow().as_ref() {
                                s.fortschritt.status.set_text("Bestätigt, entferne …");
                                t.griff.bestaetigen();
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
            Zustand::Laeuft { prozent, status } => {
                match prozent {
                    Some(p) => {
                        f.balken.set_fraction(f64::from(p) / 100.0);
                        f.balken.set_text(Some(&format!("{p} %")));
                    }
                    None => f.balken.set_text(Some("…")),
                }
                f.status.set_text(&status);
                debug(&format!(
                    "transaktion\tlaeuft\t{}\t{status}",
                    prozent.map(|p| p.to_string()).unwrap_or_else(|| "-".into())
                ));
            }
            Zustand::WartetAufSperre { .. } => {
                debug("transaktion\twartet-auf-sperre");
                f.status.set_text("Wartet auf die Paketsperre …");
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
                        &fehlertext(&Fehler::SperreBelegt("apt-get, PID 4242".into())),
                        Box::new(move || {
                            if let Some(s) = w1.upgrade() {
                                if let Some(t) = s.transaktion.borrow_mut().as_mut() {
                                    t.sperrdialog_offen = false;
                                    s.fortschritt.status.set_text("Erneuter Versuch …");
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
                let vorgang = self
                    .transaktion
                    .borrow()
                    .as_ref()
                    .map(|t| t.vorgang.clone());
                self.beende_transaktion();
                let was = match vorgang {
                    Some(Vorgang::Installieren) => "installiert",
                    Some(Vorgang::Entfernen) => "entfernt",
                    Some(Vorgang::Aktualisieren) => "aktualisiert",
                    _ => "abgeschlossen",
                };
                self.melde(&format!("{titel} wurde {was}."));
                self.paketstand_laden(false);
            }
            Zustand::Fehlgeschlagen(e) => {
                debug(&format!("transaktion\tfehler\t{e:?}"));
                self.beende_transaktion();
                self.melde(&format!("{titel}: {}", fehlertext(&e)));
                match e {
                    Fehler::KeinAgent => dialoge::kein_agent(&self.window),
                    Fehler::NichtBerechtigt => dialoge::meldung(
                        &self.window,
                        "Keine Berechtigung",
                        &fehlertext(&e),
                        "nicht-berechtigt",
                    ),
                    other => dialoge::meldung(
                        &self.window,
                        "Vorgang fehlgeschlagen",
                        &fehlertext(&other),
                        "fehler",
                    ),
                }
            }
            Zustand::Abgebrochen => {
                debug("transaktion\tabgebrochen");
                self.beende_transaktion();
                self.melde(&format!("{titel}: Vorgang abgebrochen."));
                self.paketstand_laden(false);
            }
        }
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

fn vorgang_text(v: &Vorgang) -> &'static str {
    match v {
        Vorgang::Installieren => "Installieren",
        Vorgang::Entfernen => "Entfernen",
        Vorgang::Aktualisieren => "Aktualisieren",
        Vorgang::QuellenAuffrischen => "Quellen auffrischen",
    }
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
