//! Listenlogik ohne GTK: Ansichten, Filter, Sortierung, Suchgewichtung.
//! Hier liegen die Unit-Tests gegen die Attrappendaten.

use std::collections::HashMap;

use qs_typen::{Aktualisierung, Anwendung, Fehler, Kategorie, Paket, Symbol};

use crate::i18n::{gettext, gettext_f, ngettext_f, pgettext};
use crate::schnittstellen::Katalog;

/// Was die Liste gerade zeigt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ansicht {
    Alle,
    Kategorie(Kategorie),
    Installiert,
    Aktualisierungen,
    Expertenliste,
}

impl Ansicht {
    pub fn titel(self) -> String {
        match self {
            Ansicht::Alle => gettext("All Software"),
            Ansicht::Kategorie(k) => crate::fremdtexte::kategorie(k),
            Ansicht::Installiert => gettext("Installed"),
            Ansicht::Aktualisierungen => gettext("Updates"),
            Ansicht::Expertenliste => gettext("Expert List"),
        }
    }

    /// Kurzname für Debug-Ausgabe und Tests (nicht übersetzt; Kategorien mit
    /// dem deutschen Namen aus `qs_typen`).
    pub fn kennung(self) -> String {
        match self {
            Ansicht::Alle => "alle".into(),
            Ansicht::Kategorie(k) => format!("kategorie:{}", k.name()),
            Ansicht::Installiert => "installiert".into(),
            Ansicht::Aktualisierungen => "aktualisierungen".into(),
            Ansicht::Expertenliste => "experten".into(),
        }
    }
}

/// Eine Zeile der Liste: Anwendung (Katalog) oder reines Paket (Expertenliste).
#[derive(Clone, Debug, PartialEq)]
pub struct Zeile {
    pub titel: String,
    pub zusammenfassung: String,
    pub symbol: Symbol,
    pub paketname: Option<String>,
    pub installiert: bool,
    pub aktualisierbar: bool,
    pub anwendung: Option<Anwendung>,
    pub paket: Option<Paket>,
}

impl Zeile {
    pub fn aus_anwendung(a: &Anwendung, paket: Option<&Paket>, aktualisierbar: bool) -> Self {
        Zeile {
            titel: a.name.clone(),
            zusammenfassung: a.zusammenfassung.clone(),
            symbol: a.symbol.clone(),
            paketname: a.paketname.clone(),
            installiert: paket.is_some_and(|p| p.installiert),
            aktualisierbar,
            anwendung: Some(a.clone()),
            paket: paket.cloned(),
        }
    }

    pub fn aus_paket(p: &Paket, aktualisierbar: bool) -> Self {
        Zeile {
            titel: p.name.clone(),
            zusammenfassung: p.zusammenfassung.clone(),
            symbol: Symbol::Thema("package-x-generic".into()),
            paketname: Some(p.name.clone()),
            installiert: p.installiert,
            aktualisierbar,
            anwendung: None,
            paket: Some(p.clone()),
        }
    }

    /// Zustandstext in der Liste.
    pub fn zustand(&self) -> String {
        if self.aktualisierbar {
            gettext("Update available")
        } else if self.installiert {
            gettext("Installed")
        } else if self.paket.is_some() {
            gettext("Available")
        } else {
            gettext("Not available")
        }
    }

    /// Beschriftung des Knopfs.
    pub fn knopf(&self) -> String {
        if self.aktualisierbar {
            gettext("Update")
        } else if self.installiert {
            gettext("Remove")
        } else {
            gettext("Install")
        }
    }
}

/// Snap-Hülle: Ubuntus Übergangspaket, das nur den gleichnamigen Snap
/// nachinstalliert (Version `2:1snap1-0ubuntu5` bei Thunderbird,
/// `1:1snap1-0ubuntu9.1` bei Firefox). Querencia verwendet keine Snaps und
/// holt diese Programme aus Ersatzquellen; die Hülle gilt nicht als
/// verfügbare Fassung.
pub fn ist_snap_huelle(version: &str) -> bool {
    let ohne_epoche = version.split_once(':').map_or(version, |(_, r)| r);
    let upstream = ohne_epoche.rsplit_once('-').map_or(ohne_epoche, |(u, _)| u);
    let ziffern = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    upstream
        .split_once("snap")
        .is_some_and(|(a, b)| ziffern(a) && ziffern(b))
}

/// Zustand der Paketseite, den die Oberfläche zwischenspeichert.
#[derive(Default)]
pub struct Paketstand {
    /// Paketname → Paket (installierte Version bevorzugt).
    pub pakete: HashMap<String, Paket>,
    pub aktualisierungen: Vec<Aktualisierung>,
    /// Ausgeblendete Snap-Hüllen (Paketname), siehe [`ist_snap_huelle`].
    pub snap_huellen: Vec<String>,
}

impl Paketstand {
    /// Paketstand aus der Antwort von PackageKit. Nicht installierte
    /// Snap-Hüllen und Aktualisierungen auf eine Snap-Hülle fallen weg: Das
    /// Paket gilt dann als nicht verfügbar, sein Knopf bleibt gesperrt. Eine
    /// schon installierte Hülle bleibt sichtbar (Entfernen bleibt möglich).
    pub fn aus(pakete: Vec<Paket>, mut aktualisierungen: Vec<Aktualisierung>) -> Self {
        let mut map: HashMap<String, Paket> = HashMap::new();
        let mut snap_huellen = Vec::new();
        for p in pakete {
            if !p.installiert && ist_snap_huelle(&p.version) {
                snap_huellen.push(p.name);
                continue;
            }
            match map.get(&p.name) {
                Some(alt) if alt.installiert && !p.installiert => {}
                _ => {
                    map.insert(p.name.clone(), p);
                }
            }
        }
        aktualisierungen.retain(|a| {
            let huelle = ist_snap_huelle(&a.paket.version);
            if huelle {
                snap_huellen.push(a.paket.name.clone());
            }
            !huelle
        });
        snap_huellen.sort();
        snap_huellen.dedup();
        Paketstand {
            pakete: map,
            aktualisierungen,
            snap_huellen,
        }
    }

    pub fn paket(&self, name: &str) -> Option<&Paket> {
        self.pakete.get(name)
    }

    pub fn aktualisierbar(&self, name: &str) -> bool {
        self.aktualisierungen.iter().any(|a| a.paket.name == name)
    }

    #[cfg(test)]
    pub fn installierte(&self) -> usize {
        self.pakete.values().filter(|p| p.installiert).count()
    }
}

/// Woher das Symbol einer Zeile kommt.
#[derive(Clone, Debug, PartialEq)]
pub enum SymbolWahl {
    Datei(std::path::PathBuf),
    Thema(String),
    /// Eigenes graues Paketsymbol (`ERSATZSYMBOL`).
    Ersatz,
}

/// Name des eingebetteten Ersatzsymbols für Anwendungen ohne Symbol.
pub const ERSATZSYMBOL: &str = "org.querencia.Software-paket";

/// Symbolwahl: Datei vom Katalog (nur wenn vorhanden), sonst das Symbol der
/// Anwendung – ein Themenname nur, wenn das Thema ihn kennt, eine Datei nur,
/// wenn sie existiert –, sonst das Ersatzsymbol. Nie ein kaputtes Bild.
pub fn symbol_wahl(
    katalog_pfad: Option<std::path::PathBuf>,
    symbol: &Symbol,
    thema_hat: impl Fn(&str) -> bool,
) -> SymbolWahl {
    if let Some(p) = katalog_pfad.filter(|p| p.is_file()) {
        return SymbolWahl::Datei(p);
    }
    match symbol {
        Symbol::Thema(n) if !n.is_empty() && thema_hat(n) => SymbolWahl::Thema(n.clone()),
        Symbol::Datei(p) if p.is_file() => SymbolWahl::Datei(p.clone()),
        _ => SymbolWahl::Ersatz,
    }
}

/// Sortierschlüssel: Groß/Klein und Umlaute gleichwertig (ä→a usw.).
pub fn sortierschluessel(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'ä' | 'Ä' => vec!['a'],
            'ö' | 'Ö' => vec!['o'],
            'ü' | 'Ü' => vec!['u'],
            'ß' => vec!['s', 's'],
            c => c.to_lowercase().collect::<Vec<_>>(),
        })
        .collect()
}

pub fn sortiert(mut zeilen: Vec<Zeile>) -> Vec<Zeile> {
    zeilen.sort_by_cached_key(|z| sortierschluessel(&z.titel));
    zeilen
}

/// Zeilen für die Ansicht ohne Suchtext.
pub fn zeilen_fuer(ansicht: Ansicht, katalog: &dyn Katalog, stand: &Paketstand) -> Vec<Zeile> {
    let zeile = |a: &Anwendung| {
        let paket = a.paketname.as_deref().and_then(|n| stand.paket(n));
        let akt = a
            .paketname
            .as_deref()
            .is_some_and(|n| stand.aktualisierbar(n));
        Zeile::aus_anwendung(a, paket, akt)
    };
    match ansicht {
        Ansicht::Alle => sortiert(katalog.alle().iter().map(zeile).collect()),
        Ansicht::Kategorie(k) => sortiert(katalog.kategorie(k).into_iter().map(zeile).collect()),
        Ansicht::Installiert => sortiert(
            katalog
                .alle()
                .iter()
                .map(zeile)
                .filter(|z| z.installiert)
                .collect(),
        ),
        Ansicht::Aktualisierungen => sortiert(
            stand
                .aktualisierungen
                .iter()
                .map(|a| match katalog.nach_paket(&a.paket.name) {
                    Some(anw) => Zeile::aus_anwendung(anw, Some(&a.paket), true),
                    None => Zeile::aus_paket(&a.paket, true),
                })
                .collect(),
        ),
        Ansicht::Expertenliste => {
            let mut v: Vec<Zeile> = stand
                .pakete
                .values()
                .map(|p| Zeile::aus_paket(p, stand.aktualisierbar(&p.name)))
                .collect();
            v.sort_by_cached_key(|z| sortierschluessel(&z.titel));
            v
        }
    }
}

/// Suchgewicht einer Anwendung für `text` (0 = kein Treffer).
/// Name-Anfang 100, Name enthält 60, Paketname 50, Zusammenfassung 30,
/// Beschreibung 10. Groß-/Kleinschreibung spielt keine Rolle.
pub fn gewicht(a: &Anwendung, text: &str) -> u32 {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return 0;
    }
    let name = a.name.to_lowercase();
    let mut g = 0;
    if name.starts_with(&t) {
        g += 100;
    } else if name.contains(&t) {
        g += 60;
    }
    if a.paketname
        .as_deref()
        .is_some_and(|p| p.to_lowercase().contains(&t))
    {
        g += 50;
    }
    if a.zusammenfassung.to_lowercase().contains(&t) {
        g += 30;
    }
    if a.beschreibung.to_lowercase().contains(&t) {
        g += 10;
    }
    g
}

/// Suche innerhalb der Anwendungen: nach Gewicht, bei Gleichstand nach Name.
pub fn suche<'a>(anwendungen: &[&'a Anwendung], text: &str) -> Vec<&'a Anwendung> {
    let mut treffer: Vec<(u32, &Anwendung)> = anwendungen
        .iter()
        .map(|a| (gewicht(a, text), *a))
        .filter(|(g, _)| *g > 0)
        .collect();
    treffer.sort_by(|(ga, a), (gb, b)| {
        gb.cmp(ga)
            .then_with(|| sortierschluessel(&a.name).cmp(&sortierschluessel(&b.name)))
    });
    treffer.into_iter().map(|(_, a)| a).collect()
}

/// Zeilen für die Ansicht mit Suchtext: in Katalogansichten die gewichtete
/// Suche (innerhalb der Ansicht), in der Expertenliste Teilstring auf
/// Paketname und Zusammenfassung.
pub fn zeilen_gesucht(
    ansicht: Ansicht,
    text: &str,
    katalog: &dyn Katalog,
    stand: &Paketstand,
) -> Vec<Zeile> {
    let text = text.trim();
    if text.is_empty() {
        return zeilen_fuer(ansicht, katalog, stand);
    }
    if ansicht == Ansicht::Expertenliste {
        let t = text.to_lowercase();
        return zeilen_fuer(ansicht, katalog, stand)
            .into_iter()
            .filter(|z| {
                z.titel.to_lowercase().contains(&t) || z.zusammenfassung.to_lowercase().contains(&t)
            })
            .collect();
    }
    let basis = zeilen_fuer(ansicht, katalog, stand);
    let anwendungen: Vec<&Anwendung> = basis.iter().filter_map(|z| z.anwendung.as_ref()).collect();
    let treffer = suche(&anwendungen, text);
    treffer
        .into_iter()
        .filter_map(|a| {
            basis
                .iter()
                .find(|z| z.anwendung.as_ref() == Some(a))
                .cloned()
        })
        .collect()
}

/// Größe in lesbarer Form (1.024er-Schritte; Dezimalzeichen aus dem
/// Katalog: deutsch „1,5 MiB“, englisch „1.5 MiB“).
pub fn groesse_text(bytes: u64) -> String {
    const EINHEITEN: [&str; 4] = ["Byte", "KiB", "MiB", "GiB"];
    let mut wert = bytes as f64;
    let mut i = 0;
    while wert >= 1024.0 && i < EINHEITEN.len() - 1 {
        wert /= 1024.0;
        i += 1;
    }
    if i == 0 {
        // TRANSLATORS: size of a package below 1 KiB
        ngettext_f("{n} byte", "{n} bytes", bytes as u32, &[])
    } else {
        // TRANSLATORS: decimal separator in sizes such as “1.5 MiB”
        let komma = pgettext("decimal separator", ".");
        format!("{:.1} {}", wert, EINHEITEN[i]).replace('.', &komma)
    }
}

/// Kurzer Fehlertext für Statuszeile und Detailseite.
pub fn fehlertext(f: &Fehler) -> String {
    match f {
        Fehler::NichtBerechtigt => {
            gettext("You are not allowed to install or remove software. The request was denied.")
        }
        Fehler::KeinAgent => {
            gettext("The session’s authorization dialog (polkit agent) is missing.")
        }
        Fehler::SperreBelegt(wer) => gettext_f(
            "The package manager is busy right now ({program}).",
            &[("program", wer)],
        ),
        Fehler::NetzFehlt => gettext("No network connection."),
        Fehler::PaketNichtGefunden(p) => {
            gettext_f("The package “{package}” was not found.", &[("package", p)])
        }
        Fehler::AbhaengigkeitenKonflikt(t) => {
            gettext_f("Conflicting dependencies: {details}", &[("details", t)])
        }
        Fehler::DaemonWeg => {
            gettext("The package manager (PackageKit) was stopped during the operation.")
        }
        Fehler::Abgebrochen => gettext("The operation was cancelled."),
        Fehler::Sonstiges { code, text } => format!("{text} ({code})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrappen::{AttrappeKatalog, AttrappeQuelle};
    use crate::schnittstellen::Paketquelle;

    fn aufbau() -> (AttrappeKatalog, Paketstand) {
        let mut katalog = AttrappeKatalog::new();
        katalog.laden().unwrap();
        let quelle = AttrappeQuelle::new();
        let pakete = quelle.alle_pakete();
        let akt = quelle.alle_aktualisierungen();
        (katalog, Paketstand::aus(pakete, akt))
    }

    #[test]
    fn attrappe_hat_30_anwendungen_in_allen_kategorien() {
        let (katalog, _) = aufbau();
        assert_eq!(katalog.alle().len(), 30);
        for k in Kategorie::alle() {
            assert!(
                !katalog.kategorie(*k).is_empty(),
                "Kategorie {} leer",
                k.name()
            );
        }
    }

    #[test]
    fn attrappe_quelle_30_pakete_8_installiert_3_aktualisierungen() {
        let quelle = AttrappeQuelle::new();
        let pakete = quelle.alle_pakete();
        assert_eq!(pakete.len(), 34, "30 Anwendungspakete + 4 reine Pakete");
        let stand = Paketstand::aus(pakete, quelle.alle_aktualisierungen());
        assert_eq!(stand.installierte(), 8);
        assert_eq!(stand.aktualisierungen.len(), 3);
        for a in &stand.aktualisierungen {
            assert!(stand.paket(&a.paket.name).unwrap().installiert);
        }
    }

    #[test]
    fn alle_ansicht_ist_nach_name_sortiert() {
        let (katalog, stand) = aufbau();
        let zeilen = zeilen_fuer(Ansicht::Alle, &katalog, &stand);
        assert_eq!(zeilen.len(), 30);
        let namen: Vec<String> = zeilen.iter().map(|z| sortierschluessel(&z.titel)).collect();
        let mut sortiert = namen.clone();
        sortiert.sort();
        assert_eq!(namen, sortiert);
    }

    #[test]
    fn umlaute_sortieren_wie_grundbuchstaben() {
        assert_eq!(sortierschluessel("Übersetzer"), "ubersetzer");
        assert_eq!(sortierschluessel("Straße"), "strasse");
        assert!(sortierschluessel("Ärger") < sortierschluessel("Buch"));
    }

    #[test]
    fn kategorie_filtert() {
        let (katalog, stand) = aufbau();
        for k in Kategorie::alle() {
            let zeilen = zeilen_fuer(Ansicht::Kategorie(*k), &katalog, &stand);
            assert!(!zeilen.is_empty());
            for z in &zeilen {
                assert!(z.anwendung.as_ref().unwrap().kategorien.contains(k));
            }
        }
    }

    #[test]
    fn installiert_ansicht_zeigt_nur_installierte() {
        let (katalog, stand) = aufbau();
        let zeilen = zeilen_fuer(Ansicht::Installiert, &katalog, &stand);
        assert_eq!(zeilen.len(), 8);
        assert!(zeilen.iter().all(|z| z.installiert));
        assert!(zeilen
            .iter()
            .all(|z| z.knopf() == "Remove" || z.knopf() == "Update"));
    }

    #[test]
    fn aktualisierungen_ansicht() {
        let (katalog, stand) = aufbau();
        let zeilen = zeilen_fuer(Ansicht::Aktualisierungen, &katalog, &stand);
        assert_eq!(zeilen.len(), 3);
        assert!(zeilen
            .iter()
            .all(|z| z.aktualisierbar && z.knopf() == "Update"));
        assert!(zeilen.iter().all(|z| z.zustand() == "Update available"));
    }

    #[test]
    fn expertenliste_zeigt_alle_pakete_sortiert() {
        let (katalog, stand) = aufbau();
        let zeilen = zeilen_fuer(Ansicht::Expertenliste, &katalog, &stand);
        assert_eq!(zeilen.len(), stand.pakete.len());
        assert!(zeilen.len() > 30, "auch reine Pakete ohne Anwendung");
        assert!(zeilen
            .iter()
            .all(|z| z.anwendung.is_none() && z.paket.is_some()));
        let namen: Vec<String> = zeilen.iter().map(|z| sortierschluessel(&z.titel)).collect();
        let mut s = namen.clone();
        s.sort();
        assert_eq!(namen, s);
    }

    #[test]
    fn suche_gewichtet_namensanfang_vor_beschreibung() {
        let (katalog, _) = aufbau();
        let alle: Vec<&Anwendung> = katalog.alle().iter().collect();
        let treffer = suche(&alle, "Text");
        assert!(treffer.len() >= 2, "Name und Beschreibung treffen");
        assert_eq!(treffer[0].name, "Texteditor");
        assert!(
            treffer[0].name.to_lowercase().starts_with("text"),
            "erster Treffer: {}",
            treffer[0].name
        );
    }

    #[test]
    fn suche_ignoriert_gross_klein_und_leerzeichen() {
        let (katalog, _) = aufbau();
        let alle: Vec<&Anwendung> = katalog.alle().iter().collect();
        let a = suche(&alle, "  GIMP ");
        let b = suche(&alle, "gimp");
        assert_eq!(a, b);
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn suche_findet_paketnamen_und_englische_texte() {
        let (katalog, _) = aufbau();
        let alle: Vec<&Anwendung> = katalog.alle().iter().collect();
        assert!(suche(&alle, "libreoffice-calc")
            .iter()
            .any(|a| a.paketname.as_deref() == Some("libreoffice-calc")));
        assert!(
            !suche(&alle, "spreadsheet").is_empty(),
            "englische Zusammenfassung"
        );
    }

    #[test]
    fn suche_ohne_treffer_und_leer() {
        let (katalog, stand) = aufbau();
        let alle: Vec<&Anwendung> = katalog.alle().iter().collect();
        assert!(suche(&alle, "xyzzy-nichts").is_empty());
        assert_eq!(
            zeilen_gesucht(Ansicht::Alle, "   ", &katalog, &stand).len(),
            30
        );
    }

    #[test]
    fn suche_bleibt_in_der_ansicht() {
        let (katalog, stand) = aufbau();
        let in_grafik =
            zeilen_gesucht(Ansicht::Kategorie(Kategorie::Grafik), "e", &katalog, &stand);
        assert!(in_grafik.iter().all(|z| z
            .anwendung
            .as_ref()
            .unwrap()
            .kategorien
            .contains(&Kategorie::Grafik)));
        let installiert = zeilen_gesucht(Ansicht::Installiert, "e", &katalog, &stand);
        assert!(installiert.iter().all(|z| z.installiert));
    }

    #[test]
    fn expertenliste_suche_teilstring() {
        let (katalog, stand) = aufbau();
        let z = zeilen_gesucht(Ansicht::Expertenliste, "lib", &katalog, &stand);
        assert!(!z.is_empty());
        assert!(z
            .iter()
            .all(|z| z.titel.contains("lib") || z.zusammenfassung.to_lowercase().contains("lib")));
    }

    #[test]
    fn katalog_nach_paket_und_id() {
        let (katalog, _) = aufbau();
        let a = katalog.nach_paket("gimp").expect("gimp");
        assert_eq!(katalog.nach_id(&a.id).map(|x| &x.name), Some(&a.name));
        assert!(katalog.nach_paket("gibt-es-nicht").is_none());
    }

    #[test]
    fn katalog_suche_liefert_nach_gewicht() {
        let (katalog, _) = aufbau();
        let t = katalog.suchen("Bild");
        assert!(t.len() >= 2);
        assert!(gewicht(t[0], "Bild") >= gewicht(t[1], "Bild"));
    }

    #[test]
    fn paketstand_bevorzugt_installierte_version() {
        let mk = |inst: bool, v: &str| Paket {
            id: format!("x;{v};amd64;a"),
            name: "x".into(),
            version: v.into(),
            arch: "amd64".into(),
            zusammenfassung: String::new(),
            installiert: inst,
        };
        let stand = Paketstand::aus(vec![mk(false, "2"), mk(true, "1"), mk(false, "3")], vec![]);
        let p = stand.paket("x").unwrap();
        assert!(p.installiert);
        assert_eq!(p.version, "1");
    }

    #[test]
    fn groesse_text_ohne_katalog_englisch() {
        assert_eq!(groesse_text(1), "1 byte");
        assert_eq!(groesse_text(512), "512 bytes");
        assert_eq!(groesse_text(1536), "1.5 KiB");
        assert_eq!(groesse_text(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(groesse_text(3 * 1024 * 1024 * 1024), "3.0 GiB");
    }

    #[test]
    fn zustand_und_knopf() {
        let (katalog, stand) = aufbau();
        let zeilen = zeilen_fuer(Ansicht::Alle, &katalog, &stand);
        let frei = zeilen.iter().find(|z| !z.installiert).unwrap();
        assert_eq!(frei.zustand(), "Available");
        assert_eq!(frei.knopf(), "Install");
        let inst = zeilen
            .iter()
            .find(|z| z.installiert && !z.aktualisierbar)
            .unwrap();
        assert_eq!(inst.zustand(), "Installed");
        assert_eq!(inst.knopf(), "Remove");
    }

    #[test]
    fn attrappe_details_und_suche() {
        let quelle = AttrappeQuelle::new();
        let ctx = gtk::glib::MainContext::default();
        let pakete = ctx.block_on(quelle.suchen("", false)).unwrap();
        assert_eq!(pakete.len(), quelle.alle_pakete().len());
        let inst = ctx.block_on(quelle.suchen("", true)).unwrap();
        assert_eq!(inst.len(), 8);
        let p = ctx
            .block_on(quelle.paket_nach_name("gimp"))
            .unwrap()
            .unwrap();
        let d = ctx.block_on(quelle.details(&p.id)).unwrap();
        assert!(d.groesse_bytes > 0);
        assert!(!d.beschreibung.is_empty());
        let sim = ctx
            .block_on(quelle.simulieren(Vorgang::Entfernen, vec![p.id.clone()]))
            .unwrap();
        // Nur was mitgeht (wie qs_paketkit), nicht das Paket selbst.
        assert!(sim.entfernen.iter().any(|x| x.name == "gimp-data"));
        assert!(!sim.entfernen.iter().any(|x| x.name == "gimp"));
    }

    #[test]
    fn symbolwahl_nie_kaputt() {
        use std::path::PathBuf;
        let ja = |_: &str| true;
        let nein = |_: &str| false;
        let htop = PathBuf::from("/usr/share/icons/hicolor/scalable/apps/htop.svg");
        let fehlt = PathBuf::from("/gibt/es/nicht.png");
        // Katalogdatei hat Vorrang, aber nur wenn sie existiert
        assert_eq!(
            symbol_wahl(Some(htop.clone()), &Symbol::Keines, ja),
            SymbolWahl::Datei(htop.clone())
        );
        assert_eq!(
            symbol_wahl(Some(fehlt.clone()), &Symbol::Keines, ja),
            SymbolWahl::Ersatz
        );
        // Themenname nur, wenn das Thema ihn kennt
        assert_eq!(
            symbol_wahl(None, &Symbol::Thema("gimp".into()), ja),
            SymbolWahl::Thema("gimp".into())
        );
        assert_eq!(
            symbol_wahl(None, &Symbol::Thema("gimp".into()), nein),
            SymbolWahl::Ersatz
        );
        assert_eq!(
            symbol_wahl(None, &Symbol::Thema(String::new()), ja),
            SymbolWahl::Ersatz
        );
        // Datei nur, wenn vorhanden
        assert_eq!(
            symbol_wahl(None, &Symbol::Datei(htop.clone()), nein),
            SymbolWahl::Datei(htop)
        );
        assert_eq!(
            symbol_wahl(None, &Symbol::Datei(fehlt), ja),
            SymbolWahl::Ersatz
        );
        assert_eq!(symbol_wahl(None, &Symbol::Keines, ja), SymbolWahl::Ersatz);
    }

    #[test]
    fn attrappe_deckt_symbolfaelle_ab() {
        let (katalog, _) = aufbau();
        let alle = katalog.alle();
        assert!(alle.iter().any(|a| a.symbol == Symbol::Keines));
        assert!(alle.iter().any(|a| matches!(&a.symbol, Symbol::Datei(_))));
        assert!(alle
            .iter()
            .any(|a| matches!(&a.symbol, Symbol::Thema(n) if n == "querencia-gibt-es-nicht")));
    }

    #[test]
    fn fehlertexte_mit_hinweis() {
        assert!(fehlertext(&Fehler::KeinAgent).contains("polkit agent"));
        assert!(fehlertext(&Fehler::SperreBelegt("apt".into())).contains("busy right now (apt)"));
        let t = fehlertext(&Fehler::Sonstiges {
            code: "internal-error".into(),
            text: "kaputt".into(),
        });
        assert_eq!(t, "kaputt (internal-error)");
    }

    fn pk(name: &str, version: &str, installiert: bool) -> Paket {
        Paket {
            id: format!(
                "{name};{version};amd64;{}",
                if installiert { "installed" } else { "quelle" }
            ),
            name: name.into(),
            version: version.into(),
            arch: "amd64".into(),
            zusammenfassung: String::new(),
            installiert,
        }
    }

    #[test]
    fn snap_huelle_erkannt() {
        for v in [
            "2:1snap1-0ubuntu5",
            "1:1snap1-0ubuntu9.1",
            "1snap1",
            "2:1snap1-0ubuntu2",
        ] {
            assert!(ist_snap_huelle(v), "{v}");
        }
        for v in [
            "1:140.17.0+build3-0ubuntu0.24.04.1~mt1",
            "158.0.1~build1",
            "1.0-1",
            "2:snap1-1",
            "1snap-1",
            "1:1.2snap3-1",
            "",
        ] {
            assert!(!ist_snap_huelle(v), "{v}");
        }
    }

    #[test]
    fn paketstand_blendet_snap_huellen_aus() {
        let pakete = vec![
            pk(
                "thunderbird",
                "1:140.17.0+build3-0ubuntu0.24.04.1~mt1",
                false,
            ),
            pk("firefox", "1:1snap1-0ubuntu9.1", false),
            pk("chromium-browser", "2:1snap1-0ubuntu2", true),
        ];
        let akt = vec![Aktualisierung {
            paket: pk("thunderbird", "2:1snap1-0ubuntu5", false),
            dringlichkeit: qs_typen::Dringlichkeit::Normal,
            neustart_noetig: false,
        }];
        let stand = Paketstand::aus(pakete, akt);
        assert!(stand.paket("thunderbird").is_some());
        assert!(stand.paket("firefox").is_none(), "Snap-Hülle verfügbar");
        assert!(
            stand.paket("chromium-browser").is_some(),
            "installierte Hülle bleibt"
        );
        assert!(
            !stand.aktualisierbar("thunderbird"),
            "Aktualisierung auf Hülle"
        );
        assert_eq!(stand.snap_huellen, ["firefox", "thunderbird"]);
    }

    use qs_typen::Vorgang;
}
