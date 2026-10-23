//! Echte Backends (Integration SW-6): PackageKit über `qs_paketkit` (SW-1),
//! AppStream über `qs_katalog` (SW-2); der Transaktionsmonitor aus
//! `qs_transaktion` (SW-4) wird in `main.rs` für beide Betriebsarten gebaut.
//!
//! Dazu zwei Ergänzungen, die die Traits aus `qs_typen` nicht abdecken:
//!
//! * [`Aufloeser`]: Installiert-Zustand vieler Paketnamen in **einem**
//!   PackageKit-`Resolve` (statt der Leersuche über alle Pakete, die SW-3
//!   gegen die Attrappe nutzt; integration.md Nr. 6). Der Vorschlag, das als
//!   `Paketquelle::aufloesen` in `qs_typen` aufzunehmen, ist noch
//!   offen; bis dahin baut dieses Modul den Aufruf aus den
//!   öffentlichen Proxys von `qs_paketkit`.
//! * [`AppKatalog`]: hüllt `AppStreamKatalog` ein. `symbol_pfad` fällt auf den
//!   swcatalog-Symbolcache zurück, wenn eine Anwendung einen Themennamen
//!   nennt, den kein installiertes Thema hat (DEP-11 `stock` + `cached`);
//!   MetaInfo-Dateien werden aus `$XDG_DATA_HOME/metainfo` gelesen, wenn die
//!   Variable gesetzt ist (isolierte Prüfung mit eigenem Profil).
//! * Katalog-Ergänzungen (ab 0.1.6, docs/katalog-ergaenzungen.md): Querencias
//!   eigene AppStream-Beschreibungen für Pakete aus Ersatzquellen ohne
//!   AppStream-Daten (Thunderbird, Firefox) im swcatalog-Aufbau. Systemweit
//!   liegen sie unter `/usr/share/swcatalog` und kommen über die Systempfade;
//!   [`ergaenzungen_dir`] findet sie zusätzlich neben der Binary in der Ablage.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use qs_katalog::{AppStreamKatalog, Konfiguration};
use qs_paketkit::enums::{self, filter, info};
use qs_paketkit::proxies::{PackageKitProxy, TransactionProxy};
use qs_paketkit::transaktion::{fehler_aus_exit, fehler_aus_zbus, Ereignis, Lauscher};
use qs_paketkit::{PaketKit, HINWEISE};
use qs_typen::{
    Abbruch, Aktualisierung, Anwendung, Fehler, Fortschritt, Kategorie, Paket, PaketId,
    Paketdetails, Simulation, Statistik, Vorgang,
};

use crate::i18n::{gettext, gettext_f};
use crate::schnittstellen::{Katalog, Paketquelle};

/// Frist für das Auflösen aller Katalog-Paketnamen.
const FRIST_AUFLOESEN: Duration = Duration::from_secs(60);

/// Installiert-/Verfügbar-Zustand vieler Paketnamen auf einmal.
#[async_trait::async_trait(?Send)]
pub trait Aufloeser {
    /// Pakete zu `namen` (installierte und verfügbare Fassungen; unbekannte
    /// Namen fehlen einfach im Ergebnis).
    async fn aufloesen(&self, namen: Vec<String>) -> Result<Vec<Paket>, Fehler>;
}

/// Attrappen-Betrieb: wie SW-3 – die Attrappe liefert bei der Leersuche
/// alle Pakete.
pub struct LeersucheAufloeser(pub Rc<dyn Paketquelle>);

#[async_trait::async_trait(?Send)]
impl Aufloeser for LeersucheAufloeser {
    async fn aufloesen(&self, _namen: Vec<String>) -> Result<Vec<Paket>, Fehler> {
        self.0.suchen("", false).await
    }
}

/// Echter Betrieb: ein `Resolve(arch;newest, namen)` auf der Verbindung von
/// `PaketKit` (Systembus oder `QS_PK_BUS`).
pub struct PaketKitAufloeser(pub PaketKit);

#[async_trait::async_trait(?Send)]
impl Aufloeser for PaketKitAufloeser {
    async fn aufloesen(&self, namen: Vec<String>) -> Result<Vec<Paket>, Fehler> {
        if namen.is_empty() {
            return Ok(Vec::new());
        }
        match gtk::glib::future_with_timeout(FRIST_AUFLOESEN, resolve(&self.0, namen)).await {
            Ok(r) => r,
            Err(_) => Err(Fehler::Sonstiges {
                code: qs_paketkit::CODE_ZEIT.into(),
                text: gettext_f(
                    "PackageKit did not answer within {seconds} s",
                    &[("seconds", &FRIST_AUFLOESEN.as_secs().to_string())],
                ),
            }),
        }
    }
}

async fn resolve(pk: &PaketKit, namen: Vec<String>) -> Result<Vec<Paket>, Fehler> {
    let conn = pk.verbindung();
    let dienst = PackageKitProxy::new(conn).await.map_err(fehler_aus_zbus)?;
    let pfad = dienst.create_transaction().await.map_err(fehler_aus_zbus)?;
    let mut lauscher = Lauscher::neu(conn, &pfad).await?;
    let tx = TransactionProxy::builder(conn)
        .path(pfad)
        .map_err(fehler_aus_zbus)?
        .build()
        .await
        .map_err(fehler_aus_zbus)?;
    tx.set_hints(&HINWEISE).await.map_err(fehler_aus_zbus)?;
    let r: Vec<&str> = namen.iter().map(String::as_str).collect();
    tx.resolve(filter::ARCH | filter::NEUESTE, &r)
        .await
        .map_err(fehler_aus_zbus)?;
    let mut pakete = Vec::new();
    let mut letzter_fehler = None;
    loop {
        match lauscher.naechstes().await {
            Ereignis::Paket {
                info,
                id,
                zusammenfassung,
            } if info == info::INSTALLIERT || info == info::VERFUEGBAR => {
                pakete.push(enums::paket_aus_id(&id, &zusammenfassung, info));
            }
            Ereignis::Fehlercode { code, details } => {
                letzter_fehler = Some(enums::fehler_aus_code(code, &details, true));
            }
            Ereignis::Fertig { exit, .. } => {
                return match fehler_aus_exit(exit, letzter_fehler) {
                    None => Ok(pakete),
                    Some(f) => Err(f),
                };
            }
            Ereignis::DaemonWeg | Ereignis::Zerstoert => return Err(Fehler::DaemonWeg),
            _ => {}
        }
    }
}

/// Ersatz, wenn keine Verbindung zu PackageKit zustande kommt: jede Anfrage
/// liefert den Verbindungsfehler; die Oberfläche zeigt ihn in der Statuszeile.
pub struct OhneVerbindung(pub Fehler);

struct KeinAbbruch;
impl Abbruch for KeinAbbruch {
    fn abbrechen(&self) {}
}

#[async_trait::async_trait(?Send)]
impl Paketquelle for OhneVerbindung {
    async fn suchen(&self, _: &str, _: bool) -> Result<Vec<Paket>, Fehler> {
        Err(self.0.clone())
    }
    async fn paket_nach_name(&self, _: &str) -> Result<Option<Paket>, Fehler> {
        Err(self.0.clone())
    }
    async fn details(&self, _: &PaketId) -> Result<Paketdetails, Fehler> {
        Err(self.0.clone())
    }
    async fn aktualisierungen(&self) -> Result<Vec<Aktualisierung>, Fehler> {
        Err(self.0.clone())
    }
    fn ausfuehren(
        &self,
        _: Vorgang,
        _: Vec<PaketId>,
        melden: Box<dyn Fn(Fortschritt)>,
    ) -> Box<dyn Abbruch> {
        let fehler = self.0.clone();
        gtk::glib::idle_add_local_once(move || melden(Fortschritt::Fehler(fehler)));
        Box::new(KeinAbbruch)
    }
    async fn simulieren(&self, _: Vorgang, _: Vec<PaketId>) -> Result<Simulation, Fehler> {
        Err(self.0.clone())
    }
    async fn laufende_transaktionen(&self) -> Result<Vec<String>, Fehler> {
        Ok(Vec::new())
    }
}

#[async_trait::async_trait(?Send)]
impl Aufloeser for OhneVerbindung {
    async fn aufloesen(&self, _: Vec<String>) -> Result<Vec<Paket>, Fehler> {
        Err(self.0.clone())
    }
}

/// Fehlercode des Nur-Lese-Betriebs.
pub const CODE_NUR_LESEN: &str = "nur-lesen";

/// `QS_NUR_LESEN=1`: lesende Aufrufe gehen an die echte Quelle, `ausfuehren`
/// und `simulieren` werden verweigert, bevor irgendetwas den Daemon erreicht.
/// Für Prüfläufe gegen den System-PackageKit (keine Transaktion, kein
/// polkit-Dialog auf dem Bildschirm des Besitzers).
pub struct NurLesen(pub Rc<dyn Paketquelle>);

impl NurLesen {
    fn verweigert() -> Fehler {
        Fehler::Sonstiges {
            code: CODE_NUR_LESEN.into(),
            text: gettext(
                "Read-only mode (QS_NUR_LESEN): operations that change the system are disabled",
            ),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl Paketquelle for NurLesen {
    async fn suchen(&self, text: &str, nur_installiert: bool) -> Result<Vec<Paket>, Fehler> {
        self.0.suchen(text, nur_installiert).await
    }
    async fn paket_nach_name(&self, name: &str) -> Result<Option<Paket>, Fehler> {
        self.0.paket_nach_name(name).await
    }
    async fn details(&self, id: &PaketId) -> Result<Paketdetails, Fehler> {
        self.0.details(id).await
    }
    async fn aktualisierungen(&self) -> Result<Vec<Aktualisierung>, Fehler> {
        self.0.aktualisierungen().await
    }
    fn ausfuehren(
        &self,
        _: Vorgang,
        _: Vec<PaketId>,
        melden: Box<dyn Fn(Fortschritt)>,
    ) -> Box<dyn Abbruch> {
        crate::debug("nur-lesen\tausfuehren-verweigert");
        gtk::glib::idle_add_local_once(move || melden(Fortschritt::Fehler(Self::verweigert())));
        Box::new(KeinAbbruch)
    }
    async fn simulieren(&self, _: Vorgang, _: Vec<PaketId>) -> Result<Simulation, Fehler> {
        crate::debug("nur-lesen\tsimulieren-verweigert");
        Err(Self::verweigert())
    }
    async fn laufende_transaktionen(&self) -> Result<Vec<String>, Fehler> {
        self.0.laufende_transaktionen().await
    }
}

/// Katalogkonfiguration: Systempfade nach schnittstellen.md; mit gesetztem
/// `XDG_DATA_HOME` ersetzt `$XDG_DATA_HOME/metainfo` den Ordner
/// `~/.local/share/metainfo`. Dazu die Katalog-Ergänzungen ([`ergaenzungen_dir`]).
pub fn katalog_konfiguration() -> Konfiguration {
    let mut k = Konfiguration::system();
    if let Some(daten) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        let eigen = PathBuf::from(daten).join("metainfo");
        let standard =
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/metainfo"));
        k.metainfo_verzeichnisse
            .retain(|p| Some(p) != standard.as_ref());
        k.metainfo_verzeichnisse.push(eigen);
    }
    if let Some(d) = ergaenzungen_dir() {
        ergaenzungen_einhaengen(&mut k, &d);
    }
    k
}

/// Ordner der Katalog-Ergänzungen im swcatalog-Aufbau (`xml/`,
/// `icons/<origin>/<B>x<H>/`), gesucht wie die Übersetzungen (`i18n`):
/// `QUERENCIA_SOFTWARE_SWCATALOG`, sonst neben der Binary
/// (`<ablage>/share/swcatalog`), sonst im Präfix der Binary
/// (`/usr/bin/../share/swcatalog`, dann ohnehin ein Systempfad).
pub fn ergaenzungen_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("QUERENCIA_SOFTWARE_SWCATALOG").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    // current_exe() löst den Link ~/opt/querencia/bin/querencia-software auf.
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.canonicalize().ok())
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    ergaenzungen_dir_for(exe_dir.as_deref())
}

fn ergaenzungen_dir_for(exe_dir: Option<&Path>) -> Option<PathBuf> {
    let exe_dir = exe_dir?;
    let beside = exe_dir.join("share/swcatalog");
    if beside.join("xml").is_dir() {
        return Some(beside);
    }
    let prefixed = exe_dir.parent()?.join("share/swcatalog");
    prefixed.join("xml").is_dir().then_some(prefixed)
}

/// Hängt `<swcatalog>/xml` als letzte Katalogquelle an (spätere Quellen
/// gewinnen bei gleicher ID, fehlende Felder kommen aus früheren) und
/// `<swcatalog>/icons` an die Symbolcaches; schon vorhandene Pfade (auch über
/// Links) werden nicht doppelt gelesen.
pub fn ergaenzungen_einhaengen(k: &mut Konfiguration, swcatalog: &Path) {
    fn anhaengen(liste: &mut Vec<PathBuf>, pfad: PathBuf) {
        let echt = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        let neu = echt(&pfad);
        if !liste.iter().any(|p| echt(p) == neu) {
            liste.push(pfad);
        }
    }
    anhaengen(&mut k.katalog_verzeichnisse, swcatalog.join("xml"));
    anhaengen(&mut k.symbol_cache_verzeichnisse, swcatalog.join("icons"));
}

/// `AppStreamKatalog` mit Rückfall auf den Symbolcache (siehe Moduldoku).
pub struct AppKatalog {
    innen: AppStreamKatalog,
}

impl AppKatalog {
    /// Lädt den Katalog (blockierend; im Hintergrund-Thread rufen).
    pub fn laden(konfiguration: Konfiguration) -> Result<(Self, Statistik), String> {
        let mut innen = AppStreamKatalog::mit_konfiguration(konfiguration);
        let s = innen.laden()?;
        Ok((AppKatalog { innen }, s))
    }

    /// Ladefehler einzelner Dateien (Datei, Meldung).
    pub fn ladefehler(&self) -> &[(String, String)] {
        &self.innen.ladebericht().fehler
    }
}

impl Katalog for AppKatalog {
    fn laden(&mut self) -> Result<Statistik, String> {
        self.innen.laden()
    }
    fn kategorie(&self, k: Kategorie) -> Vec<&Anwendung> {
        self.innen.kategorie(k)
    }
    fn suchen(&self, text: &str) -> Vec<&Anwendung> {
        self.innen.suchen(text)
    }
    fn nach_paket(&self, paketname: &str) -> Option<&Anwendung> {
        self.innen.nach_paket(paketname)
    }
    fn nach_id(&self, id: &str) -> Option<&Anwendung> {
        self.innen.nach_id(id)
    }
    fn symbol_pfad(&self, a: &Anwendung, groesse: u32) -> Option<PathBuf> {
        self.innen.symbol_pfad(a, groesse).or_else(|| {
            let z = self.innen.zusatz(a)?;
            qs_katalog::symbole::cache_pfad(
                &self.innen.konfiguration().symbol_cache_verzeichnisse,
                &z.origin,
                &z.symbole_cache,
                groesse,
            )
        })
    }
    fn alle(&self) -> &[Anwendung] {
        self.innen.alle()
    }
}

/// Katalog, solange der echte im Hintergrund lädt.
pub struct LeererKatalog;

impl Katalog for LeererKatalog {
    fn laden(&mut self) -> Result<Statistik, String> {
        Ok(Statistik::default())
    }
    fn kategorie(&self, _: Kategorie) -> Vec<&Anwendung> {
        Vec::new()
    }
    fn suchen(&self, _: &str) -> Vec<&Anwendung> {
        Vec::new()
    }
    fn nach_paket(&self, _: &str) -> Option<&Anwendung> {
        None
    }
    fn nach_id(&self, _: &str) -> Option<&Anwendung> {
        None
    }
    fn symbol_pfad(&self, _: &Anwendung, _: u32) -> Option<PathBuf> {
        None
    }
    fn alle(&self) -> &[Anwendung] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_data_home_ersetzt_benutzer_metainfo() {
        // Nur die Abbildung prüfen, ohne die Umgebung des Testprozesses zu ändern.
        let k = Konfiguration::system();
        assert!(k
            .metainfo_verzeichnisse
            .iter()
            .any(|p| p.ends_with(".local/share/metainfo")));
        assert_eq!(
            k.metainfo_verzeichnisse.first(),
            Some(&PathBuf::from("/usr/share/metainfo"))
        );
    }

    use crate::logik::{self, Ansicht, Paketstand};
    use qs_typen::Paket;

    /// Die Ergänzungen aus dem Repo (`programm/data/swcatalog`).
    fn repo_ergaenzungen() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/swcatalog"))
    }

    /// Katalog nur aus den Ergänzungen, ohne Themen und Systemquellen.
    fn nur_ergaenzungen() -> AppKatalog {
        let mut k = Konfiguration {
            katalog_verzeichnisse: Vec::new(),
            metainfo_verzeichnisse: Vec::new(),
            symbol_cache_verzeichnisse: Vec::new(),
            themen_verzeichnisse: Vec::new(),
            mit_konsolenanwendungen: true,
        };
        ergaenzungen_einhaengen(&mut k, &repo_ergaenzungen());
        AppKatalog::laden(k).expect("Ergänzungen laden").0
    }

    fn pk(name: &str, version: &str, installiert: bool) -> Paket {
        Paket {
            id: format!(
                "{name};{version};amd64;{}",
                if installiert {
                    "installed"
                } else {
                    "mozillateam"
                }
            ),
            name: name.into(),
            version: version.into(),
            arch: "amd64".into(),
            zusammenfassung: String::new(),
            installiert,
        }
    }

    const TB: &str = "1:140.17.0+build3-0ubuntu0.24.04.1~mt1";

    #[test]
    fn ergaenzungen_neben_binary_oder_im_praefix() {
        let basis = std::env::temp_dir().join(format!("qs-ergaenzungen-{}", std::process::id()));
        let ablage = basis.join("ablage");
        let praefix = basis.join("usr");
        std::fs::create_dir_all(ablage.join("share/swcatalog/xml")).unwrap();
        std::fs::create_dir_all(praefix.join("bin")).unwrap();
        std::fs::create_dir_all(praefix.join("share/swcatalog/xml")).unwrap();
        std::fs::create_dir_all(basis.join("leer/bin")).unwrap();
        assert_eq!(
            ergaenzungen_dir_for(Some(&ablage)),
            Some(ablage.join("share/swcatalog"))
        );
        assert_eq!(
            ergaenzungen_dir_for(Some(&praefix.join("bin"))),
            Some(praefix.join("share/swcatalog"))
        );
        assert_eq!(ergaenzungen_dir_for(Some(&basis.join("leer/bin"))), None);
        assert_eq!(ergaenzungen_dir_for(None), None);
        std::fs::remove_dir_all(&basis).unwrap();
    }

    #[test]
    fn ergaenzungen_einhaengen_ohne_doppel() {
        let mut k = Konfiguration::system();
        let n = k.katalog_verzeichnisse.len();
        let m = k.symbol_cache_verzeichnisse.len();
        // Systempfad: schon da, wird nicht doppelt gelesen.
        ergaenzungen_einhaengen(&mut k, Path::new("/usr/share/swcatalog"));
        assert_eq!(k.katalog_verzeichnisse.len(), n);
        assert_eq!(k.symbol_cache_verzeichnisse.len(), m);
        // Ablage: als letzte Quelle (gewinnt bei gleicher ID).
        ergaenzungen_einhaengen(&mut k, &repo_ergaenzungen());
        ergaenzungen_einhaengen(&mut k, &repo_ergaenzungen());
        assert_eq!(k.katalog_verzeichnisse.len(), n + 1);
        assert_eq!(
            k.katalog_verzeichnisse.last(),
            Some(&repo_ergaenzungen().join("xml"))
        );
        assert_eq!(
            k.symbol_cache_verzeichnisse.last(),
            Some(&repo_ergaenzungen().join("icons"))
        );
    }

    #[test]
    fn ergaenzung_in_suche_und_kategorie() {
        let k = nur_ergaenzungen();
        assert_eq!(k.alle().len(), 2);
        let treffer = k.suchen("thunderbird");
        let tb = treffer.first().expect("Treffer für thunderbird");
        assert_eq!(tb.id, "org.mozilla.Thunderbird");
        assert_eq!(tb.name, "Thunderbird");
        assert_eq!(tb.paketname.as_deref(), Some("thunderbird"));
        assert_eq!(
            tb.zusammenfassung,
            "E-Mails lesen und schreiben, Kalender und Kontakte verwalten"
        );
        assert!(tb.beschreibung.contains("nicht als Snap"));
        assert_eq!(tb.lizenz.as_deref(), Some("MPL-2.0"));
        assert_eq!(tb.homepage.as_deref(), Some("https://www.thunderbird.net/"));
        assert!(k
            .kategorie(Kategorie::Internet)
            .iter()
            .any(|a| a.id == "org.mozilla.Thunderbird"));
        assert_eq!(
            k.nach_paket("thunderbird").map(|a| a.id.as_str()),
            Some(tb.id.as_str())
        );
        // Deutsche Schlüsselwörter und Umlaut-Toleranz der Suche.
        assert_eq!(
            k.suchen("E-Mail").first().map(|a| a.id.as_str()),
            Some(tb.id.as_str())
        );
        assert_eq!(
            k.suchen("Webbrowser").first().map(|a| a.id.as_str()),
            Some("org.mozilla.firefox")
        );
        // Symbol: ohne Thema aus dem Cache der Ergänzungen.
        let pfad = k.symbol_pfad(tb, 64).expect("Symbol");
        assert!(
            pfad.ends_with("querencia-ergaenzungen/64x64/thunderbird.png"),
            "{}",
            pfad.display()
        );
        let gross = k.symbol_pfad(tb, 128).expect("Symbol 128");
        assert!(gross.ends_with("querencia-ergaenzungen/128x128/thunderbird.png"));
        assert_eq!(
            k.innen.zusatz(tb).map(|z| z.origin.as_str()),
            Some("querencia-ergaenzungen")
        );
    }

    #[test]
    fn ergaenzung_mit_paketstand_aus_packagekit() {
        let k = nur_ergaenzungen();
        // Antwort von Resolve(thunderbird, firefox), wie PackageKit sie liefert:
        // Thunderbird verfügbar aus der Ersatzquelle, Firefox nur als Snap-Hülle.
        let stand = Paketstand::aus(
            vec![
                pk("thunderbird", TB, false),
                pk("firefox", "1:1snap1-0ubuntu9.1", false),
            ],
            Vec::new(),
        );
        let zeilen = logik::zeilen_gesucht(Ansicht::Alle, "thunderbird", &k, &stand);
        let z = zeilen.first().expect("Zeile");
        assert_eq!(z.titel, "Thunderbird");
        assert_eq!(z.zustand(), "Available");
        assert_eq!(z.knopf(), "Install");
        assert_eq!(z.paket.as_ref().map(|p| p.version.as_str()), Some(TB));
        let internet = logik::zeilen_fuer(Ansicht::Kategorie(Kategorie::Internet), &k, &stand);
        let titel: Vec<(&str, String)> = internet
            .iter()
            .map(|z| (z.titel.as_str(), z.zustand()))
            .collect();
        assert_eq!(
            titel,
            [
                ("Firefox", "Not available".to_string()),
                ("Thunderbird", "Available".to_string())
            ]
        );
        // Installiert und Aktualisierung.
        let stand = Paketstand::aus(
            vec![pk("thunderbird", TB, true)],
            vec![qs_typen::Aktualisierung {
                paket: pk(
                    "thunderbird",
                    "1:140.18.0+build1-0ubuntu0.24.04.1~mt1",
                    false,
                ),
                dringlichkeit: qs_typen::Dringlichkeit::Sicherheit,
                neustart_noetig: false,
            }],
        );
        let inst = logik::zeilen_fuer(Ansicht::Installiert, &k, &stand);
        assert_eq!(inst.len(), 1);
        assert_eq!(inst[0].zustand(), "Update available");
        let akt = logik::zeilen_fuer(Ansicht::Aktualisierungen, &k, &stand);
        assert_eq!(
            akt[0].anwendung.as_ref().map(|a| a.id.as_str()),
            Some("org.mozilla.Thunderbird")
        );
    }

    /// Mit dem Katalog des Systems (sofern vorhanden): Die Suche „thunderbird“
    /// bringt den Eintrag der Ergänzung vor Birdtray und Co.
    #[test]
    fn ergaenzung_mit_systemkatalog() {
        let mut k = Konfiguration::system();
        if !k.katalog_verzeichnisse.iter().any(|d| d.is_dir()) {
            eprintln!("kein Systemkatalog – übersprungen");
            return;
        }
        k.metainfo_verzeichnisse.truncate(1);
        ergaenzungen_einhaengen(&mut k, &repo_ergaenzungen());
        let (k, _) = AppKatalog::laden(k).expect("Systemkatalog");
        let treffer = k.suchen("thunderbird");
        assert_eq!(
            treffer.first().map(|a| a.id.as_str()),
            Some("org.mozilla.Thunderbird")
        );
        assert_eq!(
            treffer
                .iter()
                .filter(|a| a.paketname.as_deref() == Some("thunderbird"))
                .count(),
            1
        );
        assert_eq!(
            k.suchen("firefox").first().map(|a| a.id.as_str()),
            Some("org.mozilla.firefox")
        );
    }

    /// Nur ausdrücklich (`--ignored`): Resolve am System-PackageKit (lesend,
    /// keine Transaktion, die etwas ändert). Prüft, dass die Pakete der
    /// Ergänzungen dort auflösbar sind und keine Snap-Hülle übrig bleibt.
    #[test]
    #[ignore]
    fn ergaenzung_paketstatus_systemweit_lesend() {
        let k = nur_ergaenzungen();
        let namen: Vec<String> = k
            .alle()
            .iter()
            .filter_map(|a| a.paketname.clone())
            .collect();
        let pk = qs_paketkit::PaketKit::neu_blockierend().expect("PackageKit");
        let pakete = gtk::glib::MainContext::new()
            .block_on(PaketKitAufloeser(pk).aufloesen(namen))
            .expect("Resolve");
        let stand = Paketstand::aus(pakete, Vec::new());
        for z in logik::zeilen_fuer(Ansicht::Kategorie(Kategorie::Internet), &k, &stand) {
            let p = z.paket.as_ref().expect("Paket aufgelöst");
            eprintln!("{}\t{}\t{}", z.titel, p.version, z.zustand());
            assert!(!logik::ist_snap_huelle(&p.version) || p.installiert);
        }
    }

    #[test]
    fn leerer_katalog_ist_leer() {
        let k = LeererKatalog;
        assert!(k.alle().is_empty());
        assert!(k.suchen("x").is_empty());
        assert!(k.kategorie(Kategorie::Grafik).is_empty());
    }
}
