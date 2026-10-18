//! Dialoge: Info (Querencia-Q), Bestätigung vor dem Entfernen (mit
//! Simulationsliste), Meldung, Erklärdialog „KeinAgent“, Sperre belegt.
//! Dialoge sind gewöhnliche modale Fenster ohne eigene Titelleiste: labwc
//! dekoriert sie wie jedes andere Fenster.

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use qs_typen::Paket;

use crate::debug;
use crate::schnittstellen::Simulation;
use crate::APP_NAME;

/// Name des Fenstersymbols, das GTK labwc schickt (xdg-toplevel-icon).
///
/// Bewusst ein eigener, **nur als SVG** eingebetteter Name statt der App-ID
/// (Symbolregel: nie 48x48 zusammen mit Scalable; sonst stürzt GTK 4.22 beim
/// Schließen eines fokussierten Dialogs ab, siehe
/// ). Das endgültige
/// App-Symbol liefert SW-5.
pub const WINDOW_ICON: &str = "org.querencia.Software-fenster";

/// Bettet das Fenstersymbol als Ressource ein und macht es dem Symbolthema
/// bekannt; läuft nach dem Start von GTK.
pub fn register_window_icon() {
    let bytes = glib::Bytes::from_static(include_bytes!("../data/resource/software.gresource"));
    match gio::Resource::from_data(&bytes) {
        Ok(resource) => {
            gio::resources_register(&resource);
            if let Some(display) = gdk::Display::default() {
                let thema = gtk::IconTheme::for_display(&display);
                thema.add_resource_path("/org/querencia/Software/icons");
                debug(&format!(
                    "symbole\t{}\t{}\t{}\t{}",
                    WINDOW_ICON,
                    thema.has_icon(WINDOW_ICON),
                    crate::logik::ERSATZSYMBOL,
                    thema.has_icon(crate::logik::ERSATZSYMBOL)
                ));
            }
        }
        Err(err) => eprintln!("querencia-software: Ressourcen nicht lesbar: {err}"),
    }
}

pub fn set_window_icon(window: &impl IsA<gtk::Window>) {
    window.as_ref().set_icon_name(Some(WINDOW_ICON));
}

/// Schließt `window` erst im nächsten Leerlauf, nicht aus dem gerade
/// laufenden Tasten- oder Klick-Handler heraus.
pub fn close_later(window: &gtk::Window) {
    let window = window.clone();
    glib::idle_add_local_once(move || window.close());
}

fn modal_window(parent: &impl IsA<gtk::Window>, title: &str, kennung: &'static str) -> gtk::Window {
    let window = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .resizable(false)
        .destroy_with_parent(true)
        .build();
    window.add_css_class("qs-dialog");
    if let Some(app) = parent.as_ref().application() {
        window.set_application(Some(&app));
    }
    set_window_icon(&window);
    debug(&format!("dialog\t{kennung}"));
    window.connect_close_request(move |_| {
        debug(&format!("dialog\tzu\t{kennung}"));
        glib::Propagation::Proceed
    });
    window
}

fn close_on_escape(window: &gtk::Window, bei_escape: Option<Box<dyn Fn()>>) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let w = window.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(f) = &bei_escape {
                f();
            }
            if let Some(w) = w.upgrade() {
                close_later(&w);
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(keys);
}

/// Knopf, der das Fenster schließt und vorher `f` ruft.
fn knopf_schliesst(window: &gtk::Window, text: &str, f: Option<Box<dyn Fn()>>) -> gtk::Button {
    let b = gtk::Button::with_mnemonic(text);
    let w = window.downgrade();
    b.connect_clicked(move |_| {
        if let Some(f) = &f {
            f();
        }
        if let Some(w) = w.upgrade() {
            close_later(&w);
        }
    });
    b
}

fn rahmen(inhalt: &gtk::Box, knoepfe: &[&gtk::Button]) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.set_halign(gtk::Align::End);
    row.set_margin_top(12);
    for k in knoepfe {
        row.append(*k);
    }
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_margin_top(12);
    outer.set_margin_bottom(12);
    outer.set_margin_start(18);
    outer.set_margin_end(18);
    outer.append(inhalt);
    outer.append(&row);
    outer
}

const LOGO_64: &[u8] = include_bytes!("../data/logo/querencia-logo-64.png");

/// „Info“: offizielles Querencia-Logo C2 (das Q), Name, Version, Lizenz.
/// Der Schließen-Knopf hat den Tastaturfokus; Escape und Eingabetaste
/// schließen.
pub fn about(parent: &impl IsA<gtk::Window>) {
    let window = modal_window(parent, &format!("Info zu {APP_NAME}"), "info");
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_halign(gtk::Align::Center);
    // Programmsymbol (SW-5) neben dem Querencia-Q.
    let symbole = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    symbole.set_halign(gtk::Align::Center);
    let programm = gtk::Image::from_icon_name(WINDOW_ICON);
    programm.set_pixel_size(64);
    symbole.append(&programm);
    let mit_q = if let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from_static(LOGO_64)) {
        let image = gtk::Image::from_paintable(Some(&texture));
        image.set_pixel_size(64);
        symbole.append(&image);
        true
    } else {
        false
    };
    body.append(&symbole);
    debug(&format!(
        "info-symbole\t{WINDOW_ICON}\t{}",
        if mit_q { "querencia-q" } else { "-" }
    ));
    let name = gtk::Label::new(None);
    name.set_markup(&format!(
        "<b><big>{APP_NAME} {}</big></b>",
        env!("CARGO_PKG_VERSION")
    ));
    let text = gtk::Label::new(Some(
        "Programme für Querencia Desktop installieren,\n\
         aktualisieren und entfernen.\n\n\
         Grundlage: PackageKit und AppStream.",
    ));
    text.set_justify(gtk::Justification::Center);
    let tech = gtk::Label::new(Some(&format!(
        "GTK {}.{}.{}",
        gtk::major_version(),
        gtk::minor_version(),
        gtk::micro_version()
    )));
    tech.add_css_class("dim-label");
    let license = gtk::Label::new(Some("Lizenz: GNU GPL, Version 3 oder neuer"));
    license.add_css_class("dim-label");
    body.append(&name);
    body.append(&text);
    body.append(&tech);
    body.append(&license);
    let close = knopf_schliesst(&window, "_Schließen", None);
    window.set_child(Some(&rahmen(&body, &[&close])));
    window.set_default_widget(Some(&close));
    close_on_escape(&window, None);
    window.present();
    close.grab_focus();
}

/// Einfache Meldung mit Schließen-Knopf.
pub fn meldung(parent: &impl IsA<gtk::Window>, titel: &str, text: &str, kennung: &'static str) {
    let window = modal_window(parent, titel, kennung);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let ueberschrift = gtk::Label::new(None);
    ueberschrift.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(titel)));
    ueberschrift.set_xalign(0.0);
    let label = gtk::Label::new(Some(text));
    label.set_wrap(true);
    label.set_max_width_chars(60);
    label.set_xalign(0.0);
    body.append(&ueberschrift);
    body.append(&label);
    let close = knopf_schliesst(&window, "_Schließen", None);
    window.set_child(Some(&rahmen(&body, &[&close])));
    window.set_default_widget(Some(&close));
    close_on_escape(&window, None);
    window.present();
    close.grab_focus();
}

/// Erklärdialog für `Fehler::KeinAgent` (G2-I, Abschnitt 1.4).
pub fn kein_agent(parent: &impl IsA<gtk::Window>) {
    meldung(
        parent,
        "Berechtigungsdialog fehlt",
        "Es fehlt der Berechtigungsdialog der Sitzung (polkit-Agent). Ohne ihn kann \
         kein Programm Software installieren oder entfernen, weil die Sitzung die \
         Nachfrage nach dem Passwort nicht anzeigen kann.\n\n\
         Der Agent gehört zur Sitzungsverwaltung von Querencia Desktop (Teilprojekt \
         G2-K) und wird mit ihr nachgeliefert. Bis dahin ist dieser Vorgang nicht \
         möglich.",
        "kein-agent",
    );
}

fn paketliste(pakete: &[Paket]) -> gtk::Widget {
    let liste = gtk::ListBox::new();
    liste.set_selection_mode(gtk::SelectionMode::None);
    liste.add_css_class("qs-paketliste");
    for p in pakete {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_margin_start(6);
        row.set_margin_end(6);
        row.set_margin_top(2);
        row.set_margin_bottom(2);
        let name = gtk::Label::new(Some(&p.name));
        name.set_xalign(0.0);
        name.add_css_class("qs-fett");
        let version = gtk::Label::new(Some(&p.version));
        version.add_css_class("dim-label");
        let zusammenfassung = gtk::Label::new(Some(&p.zusammenfassung));
        zusammenfassung.set_xalign(0.0);
        zusammenfassung.set_ellipsize(gtk::pango::EllipsizeMode::End);
        zusammenfassung.set_hexpand(true);
        zusammenfassung.add_css_class("dim-label");
        row.append(&name);
        row.append(&version);
        row.append(&zusammenfassung);
        liste.append(&row);
    }
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_min_content_height(80);
    scroll.set_max_content_height(200);
    scroll.set_propagate_natural_height(true);
    scroll.set_child(Some(&liste));
    scroll.add_css_class("qs-rahmen");
    scroll.upcast()
}

/// Bestätigung vor dem Entfernen: „Folgende Pakete werden mit entfernt“.
/// Alt+E / Eingabetaste entfernt, Escape oder „Abbrechen“ bricht ab.
pub fn bestaetigen_entfernen(
    parent: &impl IsA<gtk::Window>,
    name: &str,
    simulation: &Simulation,
    entfernen: Box<dyn Fn()>,
    abbrechen: Box<dyn Fn()>,
) {
    let window = modal_window(parent, "Entfernen bestätigen", "entfernen");
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let frage = gtk::Label::new(None);
    frage.set_markup(&format!(
        "<b>Soll „{}“ wirklich entfernt werden?</b>",
        glib::markup_escape_text(name)
    ));
    frage.set_xalign(0.0);
    body.append(&frage);
    let mitgehend: Vec<Paket> = simulation.entfernen.clone();
    let hinweis = gtk::Label::new(Some(&format!(
        "Folgende Pakete werden mit entfernt ({}):",
        mitgehend.len()
    )));
    hinweis.set_xalign(0.0);
    body.append(&hinweis);
    body.append(&paketliste(&mitgehend));
    let abbruch_rc = std::rc::Rc::new(abbrechen);
    let a1 = abbruch_rc.clone();
    let knopf_abbrechen = knopf_schliesst(&window, "_Abbrechen", Some(Box::new(move || a1())));
    let knopf_entfernen = knopf_schliesst(&window, "_Entfernen", Some(entfernen));
    knopf_entfernen.add_css_class("destructive-action");
    window.set_child(Some(&rahmen(&body, &[&knopf_abbrechen, &knopf_entfernen])));
    window.set_default_widget(Some(&knopf_entfernen));
    let a2 = abbruch_rc.clone();
    close_on_escape(&window, Some(Box::new(move || a2())));
    window.set_default_width(460);
    window.present();
    knopf_entfernen.grab_focus();
}

/// Sperre belegt: Warten (erneut versuchen) oder Abbrechen. Alt+W / Eingabe
/// wartet, Escape bricht ab.
pub fn sperre_belegt(
    parent: &impl IsA<gtk::Window>,
    text: &str,
    warten: Box<dyn Fn()>,
    abbrechen: Box<dyn Fn()>,
) {
    let window = modal_window(parent, "Paketverwaltung belegt", "sperre");
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let ueberschrift = gtk::Label::new(None);
    ueberschrift.set_markup("<b>Die Paketverwaltung ist gerade belegt</b>");
    ueberschrift.set_xalign(0.0);
    let label = gtk::Label::new(Some(&format!(
        "{text}\n\nEin anderes Programm (zum Beispiel eine Aktualisierung im Hintergrund) \
         hält die Paketsperre. Sie können warten und es erneut versuchen oder den \
         Vorgang abbrechen."
    )));
    label.set_wrap(true);
    label.set_max_width_chars(60);
    label.set_xalign(0.0);
    body.append(&ueberschrift);
    body.append(&label);
    let abbruch_rc = std::rc::Rc::new(abbrechen);
    let a1 = abbruch_rc.clone();
    let knopf_abbrechen = knopf_schliesst(&window, "_Abbrechen", Some(Box::new(move || a1())));
    let knopf_warten = knopf_schliesst(&window, "_Warten", Some(warten));
    window.set_child(Some(&rahmen(&body, &[&knopf_abbrechen, &knopf_warten])));
    window.set_default_widget(Some(&knopf_warten));
    let a2 = abbruch_rc.clone();
    close_on_escape(&window, Some(Box::new(move || a2())));
    window.present();
    knopf_warten.grab_focus();
}
