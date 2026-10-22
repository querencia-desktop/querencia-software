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

use std::path::PathBuf;
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
/// `~/.local/share/metainfo`.
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
    k
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

    #[test]
    fn leerer_katalog_ist_leer() {
        let k = LeererKatalog;
        assert!(k.alle().is_empty());
        assert!(k.suchen("x").is_empty());
        assert!(k.kategorie(Kategorie::Grafik).is_empty());
    }
}
