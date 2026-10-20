//! Erreichbarkeit der Paketquellen (QD-067, Befund B2 der Pflichtprobe).
//!
//! PackageKit entscheidet über `NetworkState`, ob es Pakete herunterlädt.
//! packagekitd fragt dafür den GNetworkMonitor von GLib; ist NetworkManager
//! installiert, verwaltet die Verbindung aber nicht (Server mit netplan und
//! systemd-networkd), meldet er „offline“, und das apt-Backend bricht jeden
//! Download mit `no-network` ab, obwohl das Archiv erreichbar ist.
//!
//! Die Oberfläche prüft deshalb selbst, in drei Stufen:
//!
//! 1. PackageKit `NetworkState` meldet online → fertig.
//! 2. sonst GNetworkMonitor im eigenen Prozess (NetworkManager, wenn er läuft,
//!    sonst die Routing-Tabelle) → bei „Netz verfügbar“ fertig.
//! 3. sonst ein HTTP-`HEAD` auf die APT-Quelle des Systems (Host aus
//!    `/etc/apt/sources.list` bzw. `sources.list.d/*.{list,sources}`),
//!    Zeitlimit 3 s; jede HTTP-Antwort gilt als erreichbar.
//!
//! Offline ist das System nur, wenn keine Stufe online sagt. Die Stufenlogik
//! ([`pruefen`]) hängt nur am Trait [`Sonden`]; [`EchteSonden`] fragt
//! PackageKit, GLib und das Netz, [`AttrappeSonden`] liefert feste Antworten
//! (`QS_NETZ_ATTRAPPE`, GUI-Prüfung und Unit-Tests).

use std::path::Path;
use std::time::Duration;

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib;

use crate::i18n::{gettext, gettext_f};

/// Zeitlimit des `HEAD` auf die APT-Quelle (und der PackageKit-Abfrage).
pub const FRIST: Duration = Duration::from_secs(3);

/// Abstand der automatischen Neuprüfung im Offline-Zustand.
pub const NEUPRUEFUNG: Duration = Duration::from_secs(30);

/// Antwort von PackageKit (`NetworkState`: 0 unbekannt, 1 offline, 2 online,
/// 3 kabel, 4 wlan, 5 mobil).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PkNetz {
    Online,
    Offline,
    /// Kein PackageKit, Wert 0 oder keine Antwort.
    Unbekannt,
}

impl PkNetz {
    pub fn aus_wert(wert: u32) -> Self {
        match wert {
            1 => PkNetz::Offline,
            2..=5 => PkNetz::Online,
            _ => PkNetz::Unbekannt,
        }
    }
}

/// Die APT-Quelle, die Stufe 3 prüft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quelle {
    /// Wie in der Quellenliste, z. B. `http://archive.ubuntu.com/ubuntu`.
    pub uri: String,
    pub host: String,
    pub port: u16,
    pub tls: bool,
    /// Pfad für den `HEAD`, mit Schrägstrich am Ende.
    pub pfad: String,
}

impl Quelle {
    /// `http://` oder `https://` mit Host; andere Schemata (`file:`, `cdrom:`,
    /// `mirror+file:` …) prüft Stufe 3 nicht.
    pub fn aus_uri(uri: &str) -> Option<Self> {
        let uri = uri.trim();
        let ohne = uri.strip_prefix("mirror+").unwrap_or(uri);
        let (tls, rest) = if let Some(r) = ohne.strip_prefix("http://") {
            (false, r)
        } else if let Some(r) = ohne.strip_prefix("https://") {
            (true, r)
        } else {
            return None;
        };
        let (autoritaet, pfad) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        // Zugangsdaten (user:pass@host) gehören nicht in Anzeige und Anfrage.
        let autoritaet = autoritaet.rsplit('@').next().unwrap_or(autoritaet);
        // IPv6-Adressen stehen in Klammern und behalten sie (gio und Host-Kopf).
        let (host, port_text) = if autoritaet.starts_with('[') {
            let ende = autoritaet.find(']')? + 1;
            let rest = &autoritaet[ende..];
            (&autoritaet[..ende], rest.strip_prefix(':'))
        } else {
            match autoritaet.split_once(':') {
                Some((h, p)) => (h, Some(p)),
                None => (autoritaet, None),
            }
        };
        let port = match port_text {
            Some(p) => p.parse().ok()?,
            None if tls => 443,
            None => 80,
        };
        if host.is_empty() {
            return None;
        }
        let mut pfad = pfad.to_string();
        if !pfad.ends_with('/') {
            pfad.push('/');
        }
        Some(Quelle {
            uri: uri.trim_end_matches('/').to_string(),
            host: host.to_string(),
            port,
            tls,
            pfad,
        })
    }
}

/// Eine `deb`-Zeile bzw. ein `Types: deb`-Absatz.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Eintrag {
    uri: String,
    suites: Vec<String>,
}

/// Einzeilige Quellen (`sources.list`, `*.list`): `deb [opt …] uri suite …`.
fn eintraege_list(text: &str) -> Vec<Eintrag> {
    let mut aus = Vec::new();
    for zeile in text.lines() {
        let zeile = zeile.split('#').next().unwrap_or("").trim();
        let Some(rest) = zeile.strip_prefix("deb") else {
            continue;
        };
        // „deb-src“ und Wörter wie „debian“ ausschließen.
        if !rest.starts_with([' ', '\t']) {
            continue;
        }
        let mut rest = rest.trim_start();
        if rest.starts_with('[') {
            match rest.find(']') {
                Some(i) => rest = rest[i + 1..].trim_start(),
                None => continue,
            }
        }
        let mut teile = rest.split_whitespace();
        let (Some(uri), Some(suite)) = (teile.next(), teile.next()) else {
            continue;
        };
        aus.push(Eintrag {
            uri: uri.to_string(),
            suites: vec![suite.to_string()],
        });
    }
    aus
}

/// deb822-Quellen (`*.sources`): Absätze mit `Types`, `URIs`, `Suites`,
/// `Enabled`; Fortsetzungszeilen beginnen mit Leerraum.
fn eintraege_sources(text: &str) -> Vec<Eintrag> {
    let mut aus = Vec::new();
    for absatz in text.split("\n\n") {
        let mut felder: Vec<(String, String)> = Vec::new();
        for zeile in absatz.lines() {
            if zeile.trim_start().starts_with('#') {
                continue;
            }
            if zeile.starts_with([' ', '\t']) {
                if let Some((_, wert)) = felder.last_mut() {
                    wert.push(' ');
                    wert.push_str(zeile.trim());
                }
            } else if let Some((name, wert)) = zeile.split_once(':') {
                felder.push((name.trim().to_ascii_lowercase(), wert.trim().to_string()));
            }
        }
        let feld = |n: &str| {
            felder
                .iter()
                .find(|(name, _)| name == n)
                .map(|(_, w)| w.as_str())
        };
        let typen = feld("types").unwrap_or("");
        if !typen.split_whitespace().any(|t| t == "deb") {
            continue;
        }
        if feld("enabled").is_some_and(|e| e.eq_ignore_ascii_case("no")) {
            continue;
        }
        let suites: Vec<String> = feld("suites")
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_string)
            .collect();
        for uri in feld("uris").unwrap_or("").split_whitespace() {
            aus.push(Eintrag {
                uri: uri.to_string(),
                suites: suites.clone(),
            });
        }
    }
    aus
}

/// Wählt die Quelle für Stufe 3 aus den Quellendateien (Name, Inhalt) in
/// apt-Reihenfolge: die erste `deb`-Quelle über http(s), deren Suite zur
/// Distribution gehört (`codename` oder `codename-…`, z. B. `resolute-updates`),
/// sonst die erste überhaupt. So prüft Stufe 3 das Archiv der Distribution
/// und nicht eine Fremdquelle, die alphabetisch davor steht.
pub fn quelle_waehlen(dateien: &[(String, String)], codename: Option<&str>) -> Option<Quelle> {
    let mut alle: Vec<(Quelle, Vec<String>)> = Vec::new();
    for (name, inhalt) in dateien {
        let eintraege = if name.ends_with(".sources") {
            eintraege_sources(inhalt)
        } else {
            eintraege_list(inhalt)
        };
        for e in eintraege {
            if let Some(q) = Quelle::aus_uri(&e.uri) {
                alle.push((q, e.suites));
            }
        }
    }
    let zur_distribution = |suites: &[String]| {
        codename.is_some_and(|c| {
            !c.is_empty()
                && suites
                    .iter()
                    .any(|s| s == c || s.strip_prefix(c).is_some_and(|r| r.starts_with('-')))
        })
    };
    alle.iter()
        .find(|(_, s)| zur_distribution(s))
        .or(alle.first())
        .map(|(q, _)| q.clone())
}

/// `VERSION_CODENAME` (bzw. `UBUNTU_CODENAME`) aus `/etc/os-release`.
pub fn codename_aus_os_release(text: &str) -> Option<String> {
    let wert = |schluessel: &str| {
        text.lines()
            .find_map(|z| z.strip_prefix(schluessel)?.strip_prefix('='))
            .map(|w| w.trim().trim_matches('"').to_string())
            .filter(|w| !w.is_empty())
    };
    wert("VERSION_CODENAME").or_else(|| wert("UBUNTU_CODENAME"))
}

/// Liest die Quellendateien unter `wurzel` (`/etc/apt` im Betrieb):
/// `sources.list`, dann `sources.list.d/*.list` und `*.sources` nach Namen.
pub fn quellendateien(wurzel: &Path) -> Vec<(String, String)> {
    let mut aus = Vec::new();
    if let Ok(t) = std::fs::read_to_string(wurzel.join("sources.list")) {
        aus.push(("sources.list".to_string(), t));
    }
    let mut namen: Vec<String> = std::fs::read_dir(wurzel.join("sources.list.d"))
        .map(|d| {
            d.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.ends_with(".list") || n.ends_with(".sources"))
                .collect()
        })
        .unwrap_or_default();
    namen.sort();
    for n in namen {
        if let Ok(t) = std::fs::read_to_string(wurzel.join("sources.list.d").join(&n)) {
            aus.push((n, t));
        }
    }
    aus
}

/// Die Quelle des Systems. `QS_APT_QUELLE=<uri>` setzt sie ausdrücklich
/// (Prüfung der Zeitüberschreitung gegen eine unerreichbare Adresse).
pub fn system_quelle() -> Option<Quelle> {
    if let Some(uri) = std::env::var("QS_APT_QUELLE")
        .ok()
        .filter(|u| !u.is_empty())
    {
        return Quelle::aus_uri(&uri);
    }
    let codename = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| codename_aus_os_release(&t));
    quelle_waehlen(&quellendateien(Path::new("/etc/apt")), codename.as_deref())
}

/// Welche Stufe „online“ ergab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stufe {
    PackageKit,
    Netzmonitor,
    Archiv,
}

impl Stufe {
    pub fn kennung(self) -> &'static str {
        match self {
            Stufe::PackageKit => "packagekit",
            Stufe::Netzmonitor => "netzmonitor",
            Stufe::Archiv => "archiv",
        }
    }
}

/// Ergebnis einer Prüfung mit allen gefragten Stufen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ergebnis {
    /// `Some(stufe)`: online laut dieser Stufe; `None`: alle Stufen offline.
    pub online: Option<Stufe>,
    pub pk: PkNetz,
    /// `None`: nicht gefragt (eine frühere Stufe sagte online).
    pub netzmonitor: Option<bool>,
    /// `None`: nicht gefragt; `Ok(antwort)` z. B. „HTTP 200“; `Err(grund)`.
    pub archiv: Option<Result<String, String>>,
    pub quelle: Option<Quelle>,
}

impl Ergebnis {
    pub fn ist_online(&self) -> bool {
        self.online.is_some()
    }

    /// Erreichbar, aber PackageKit hält das System für offline – dann lehnt
    /// PackageKit Downloads ab, die Oberfläche versucht es trotzdem und
    /// erklärt einen `no-network`-Fehler mit [`pk_widerspruch_dialog`].
    pub fn pk_widerspricht(&self) -> bool {
        self.ist_online() && self.pk == PkNetz::Offline
    }

    /// Text der Statuszeile nach „Netzwerk:“.
    pub fn kurztext(&self) -> String {
        match self.online {
            Some(Stufe::PackageKit) => gettext("online (PackageKit)"),
            Some(Stufe::Netzmonitor) => gettext("online (network available)"),
            Some(Stufe::Archiv) => gettext("online (archive reachable)"),
            None => match (&self.quelle, &self.archiv) {
                (Some(q), Some(Err(grund))) => gettext_f(
                    "offline: {host} not reachable ({reason})",
                    &[("host", &q.host), ("reason", grund)],
                ),
                _ => gettext("offline: no network and no APT source to check"),
            },
        }
    }

    /// Kurzinfo der Statuszeile: [`Self::stufen`], dazu die Warnung, wenn
    /// PackageKit der erreichbaren Quelle widerspricht.
    pub fn hinweis(&self) -> String {
        let mut text = self.stufen();
        if self.pk_widerspricht() {
            text.push('\n');
            text.push_str(&gettext(
                "PackageKit considers the system offline and may refuse to download packages.",
            ));
        }
        text
    }

    /// Alle gefragten Stufen, eine je Zeile. Nennt die geprüfte Quelle.
    pub fn stufen(&self) -> String {
        let mut zeilen = vec![gettext_f(
            "PackageKit: {state}",
            &[(
                "state",
                &match self.pk {
                    PkNetz::Online => gettext("online"),
                    PkNetz::Offline => gettext("offline"),
                    PkNetz::Unbekannt => gettext("no answer"),
                },
            )],
        )];
        if let Some(m) = self.netzmonitor {
            zeilen.push(gettext_f(
                "Network monitor (GLib): {state}",
                &[(
                    "state",
                    &if m {
                        gettext("network available")
                    } else {
                        gettext("no network")
                    },
                )],
            ));
        }
        match (&self.quelle, &self.archiv) {
            (Some(q), Some(Ok(antwort))) => zeilen.push(gettext_f(
                "Software source {uri}: reachable ({answer})",
                &[("uri", &q.uri), ("answer", antwort)],
            )),
            (Some(q), Some(Err(grund))) => zeilen.push(gettext_f(
                "Software source {uri}: not reachable ({reason})",
                &[("uri", &q.uri), ("reason", grund)],
            )),
            (None, Some(_)) => zeilen.push(gettext("Software source: none found in /etc/apt")),
            _ => {}
        }
        zeilen.join("\n")
    }

    /// Kennzeile der Debug-Ausgabe: `netz <online|offline> <stufe|-> <pk>
    /// <monitor> <archiv> <host>`.
    pub fn debug_zeile(&self) -> String {
        format!(
            "netz\t{}\t{}\t{:?}\t{}\t{}\t{}",
            if self.ist_online() {
                "online"
            } else {
                "offline"
            },
            self.online.map(Stufe::kennung).unwrap_or("-"),
            self.pk,
            match self.netzmonitor {
                Some(true) => "an",
                Some(false) => "aus",
                None => "-",
            },
            match &self.archiv {
                Some(Ok(_)) => "erreichbar",
                Some(Err(_)) => "nicht-erreichbar",
                None => "-",
            },
            self.quelle.as_ref().map(|q| q.host.as_str()).unwrap_or("-"),
        )
    }
}

/// Dialog, wenn alle Stufen offline sagen (Installieren, Aktualisieren):
/// Überschrift und Text.
pub fn offline_dialog(e: &Ergebnis) -> (String, String) {
    let quelle = match &e.quelle {
        Some(q) => gettext_f(
            "Querencia Software could not reach the software source {uri}.",
            &[("uri", &q.uri)],
        ),
        None => gettext("Querencia Software found no software source in /etc/apt to check."),
    };
    (
        gettext("No Network Connection"),
        format!(
            "{quelle}\n\n{}\n\n{}",
            e.stufen(),
            gettext(
                "Check the network connection. While offline, Querencia Software checks \
                 again every 30 seconds; “Check Again” in the status bar checks at once."
            )
        ),
    )
}

/// Dialog nach `no-network` von PackageKit, obwohl die Quelle erreichbar ist.
pub fn pk_widerspruch_dialog(e: &Ergebnis) -> (String, String) {
    let uri = e
        .quelle
        .as_ref()
        .map(|q| q.uri.clone())
        .unwrap_or_else(|| "–".into());
    (
        gettext("PackageKit Reports No Network"),
        format!(
            "{}\n\n{}\n\n{}",
            gettext_f(
                "The software source {uri} is reachable, but PackageKit considers the system \
                 offline and refuses to download packages. This happens when NetworkManager is \
                 installed but does not manage the network connection, for example on a server \
                 set up with netplan and systemd-networkd.",
                &[("uri", &uri)],
            ),
            e.stufen(),
            gettext(
                "Let NetworkManager manage the connection (netplan: “renderer: \
                 NetworkManager”), or ask your system administrator. PackageKit notices the \
                 change by itself; then try again."
            )
        ),
    )
}

/// Die drei Stufen als Abfragen.
#[async_trait::async_trait(?Send)]
pub trait Sonden {
    async fn packagekit(&self) -> PkNetz;
    fn netzmonitor(&self) -> bool;
    /// `Ok(antwort)` bei jeder HTTP-Antwort, sonst `Err(grund)`.
    async fn archiv(&self, quelle: &Quelle) -> Result<String, String>;
}

/// Die Stufenlogik. `alle`: jede Stufe fragen, auch nach einem „online“
/// (Diagnose `--netz-pruefen`); die Entscheidung bleibt dieselbe.
pub async fn pruefen(sonden: &dyn Sonden, quelle: Option<Quelle>, alle: bool) -> Ergebnis {
    let mut e = Ergebnis {
        online: None,
        pk: sonden.packagekit().await,
        netzmonitor: None,
        archiv: None,
        quelle,
    };
    if e.pk == PkNetz::Online {
        e.online = Some(Stufe::PackageKit);
        if !alle {
            return e;
        }
    }
    let monitor = sonden.netzmonitor();
    e.netzmonitor = Some(monitor);
    if monitor && e.online.is_none() {
        e.online = Some(Stufe::Netzmonitor);
        if !alle {
            return e;
        }
    }
    e.archiv = Some(match &e.quelle {
        Some(q) => sonden.archiv(q).await,
        None => Err(gettext("no APT source found")),
    });
    if matches!(e.archiv, Some(Ok(_))) && e.online.is_none() {
        e.online = Some(Stufe::Archiv);
    }
    e
}

// ------------------------------------------------------------------ echt

/// PackageKit über die Verbindung von `qs_paketkit` (Systembus oder
/// `QS_PK_BUS`), GNetworkMonitor, `HEAD` über `gio::SocketClient`.
pub struct EchteSonden {
    pub verbindung: Option<qs_paketkit::zbus::Connection>,
}

#[async_trait::async_trait(?Send)]
impl Sonden for EchteSonden {
    async fn packagekit(&self) -> PkNetz {
        let Some(conn) = &self.verbindung else {
            return PkNetz::Unbekannt;
        };
        // Properties.Get direkt statt eines Proxys: kein Zwischenspeicher,
        // jede Neuprüfung fragt den Daemon.
        let abfrage = async {
            let antwort = conn
                .call_method(
                    Some("org.freedesktop.PackageKit"),
                    "/org/freedesktop/PackageKit",
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.PackageKit", "NetworkState"),
                )
                .await
                .ok()?;
            let wert: qs_paketkit::zbus::zvariant::OwnedValue =
                antwort.body().deserialize().ok()?;
            u32::try_from(wert).ok()
        };
        match glib::future_with_timeout(FRIST, abfrage).await {
            Ok(Some(w)) => PkNetz::aus_wert(w),
            _ => PkNetz::Unbekannt,
        }
    }

    fn netzmonitor(&self) -> bool {
        gio::NetworkMonitor::default().is_network_available()
    }

    async fn archiv(&self, quelle: &Quelle) -> Result<String, String> {
        match glib::future_with_timeout(FRIST, kopf_anfrage(quelle.clone())).await {
            Ok(r) => r,
            Err(_) => Err(gettext_f(
                "no answer within {seconds} s",
                &[("seconds", &FRIST.as_secs().to_string())],
            )),
        }
    }
}

/// `HEAD <pfad>` an die Quelle; liefert die Statuszeile („HTTP 200“). Für
/// https ohne TLS-Modul (glib-networking) zählt der TCP-Aufbau zu Port 443.
async fn kopf_anfrage(q: Quelle) -> Result<String, String> {
    let client = gio::SocketClient::new();
    client.set_timeout(FRIST.as_secs() as u32);
    let mit_tls = q.tls && gio::TlsBackend::default().supports_tls();
    client.set_tls(mit_tls);
    let ziel = format!("{}:{}", q.host, q.port);
    let verbindung = client
        .connect_to_host_future(&ziel, q.port)
        .await
        .map_err(|e| e.message().to_string())?;
    if q.tls && !mit_tls {
        return Ok(gettext("TCP connection, TLS not checked"));
    }
    let anfrage = format!(
        "HEAD {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: querencia-software/{}\r\n\
         Connection: close\r\n\r\n",
        q.pfad,
        q.host,
        env!("CARGO_PKG_VERSION")
    );
    verbindung
        .output_stream()
        .write_all_future(anfrage.into_bytes(), glib::Priority::DEFAULT)
        .await
        .map_err(|(_, e)| e.message().to_string())?;
    let (puffer, n) = verbindung
        .input_stream()
        .read_future(vec![0u8; 512], glib::Priority::DEFAULT)
        .await
        .map_err(|(_, e)| e.message().to_string())?;
    let _ = verbindung.close_future(glib::Priority::DEFAULT).await;
    statuszeile(&puffer[..n]).ok_or_else(|| gettext("no HTTP answer"))
}

/// „HTTP/1.1 200 OK“ → „HTTP 200“.
fn statuszeile(antwort: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(&antwort[..antwort.len().min(64)]);
    let zeile = text.lines().next()?;
    let mut teile = zeile.split_whitespace();
    if !teile.next()?.starts_with("HTTP/") {
        return None;
    }
    let code = teile.next()?;
    (code.len() == 3 && code.bytes().all(|b| b.is_ascii_digit())).then(|| format!("HTTP {code}"))
}

// ------------------------------------------------------------------ Attrappe

/// Feste Antworten der drei Stufen. `QS_NETZ_ATTRAPPE=<pk>,<monitor>,<archiv>`
/// mit je `online` oder `offline` (pk zusätzlich `unbekannt`), Standard
/// `online,online,online`. Stufe 3 antwortet nach 400 ms (die Statuszeile
/// zeigt kurz „wird geprüft“).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrappeSonden {
    pub pk: PkNetz,
    pub monitor: bool,
    pub archiv: bool,
    pub verzoegerung: Duration,
}

impl AttrappeSonden {
    pub fn aus_text(text: &str) -> Option<Self> {
        let teile: Vec<&str> = text.split(',').map(str::trim).collect();
        let [pk, monitor, archiv] = teile.as_slice() else {
            return None;
        };
        let an = |w: &str| match w {
            "online" => Some(true),
            "offline" => Some(false),
            _ => None,
        };
        Some(AttrappeSonden {
            pk: match *pk {
                "online" => PkNetz::Online,
                "offline" => PkNetz::Offline,
                "unbekannt" => PkNetz::Unbekannt,
                _ => return None,
            },
            monitor: an(monitor)?,
            archiv: an(archiv)?,
            verzoegerung: Duration::from_millis(400),
        })
    }

    pub fn aus_umgebung() -> Self {
        std::env::var("QS_NETZ_ATTRAPPE")
            .ok()
            .and_then(|t| Self::aus_text(&t))
            .unwrap_or(AttrappeSonden {
                pk: PkNetz::Online,
                monitor: true,
                archiv: true,
                verzoegerung: Duration::from_millis(400),
            })
    }
}

#[async_trait::async_trait(?Send)]
impl Sonden for AttrappeSonden {
    async fn packagekit(&self) -> PkNetz {
        self.pk
    }

    fn netzmonitor(&self) -> bool {
        self.monitor
    }

    async fn archiv(&self, _quelle: &Quelle) -> Result<String, String> {
        if !self.verzoegerung.is_zero() {
            glib::timeout_future(self.verzoegerung).await;
        }
        if self.archiv {
            Ok("HTTP 200".into())
        } else {
            Err(gettext_f(
                "no answer within {seconds} s",
                &[("seconds", &FRIST.as_secs().to_string())],
            ))
        }
    }
}

/// Attrappen-Betrieb: PackageKit-Attrappe gilt als offline
/// (`QS_NETZ_ATTRAPPE=offline,…`) – dann scheitern Installieren und
/// Aktualisieren der `AttrappeQuelle` mit `no-network` wie beim echten
/// apt-Backend („Cannot download packages whilst offline“).
pub fn attrappe_pk_offline() -> bool {
    std::env::var("QS_NETZ_ATTRAPPE")
        .ok()
        .and_then(|t| AttrappeSonden::aus_text(&t))
        .is_some_and(|s| s.pk == PkNetz::Offline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Zählt, welche Stufen gefragt wurden.
    struct Zaehler {
        innen: AttrappeSonden,
        pk: Cell<u32>,
        monitor: Cell<u32>,
        archiv: Cell<u32>,
    }

    impl Zaehler {
        fn neu(text: &str) -> Self {
            let mut innen = AttrappeSonden::aus_text(text).unwrap();
            innen.verzoegerung = Duration::ZERO;
            Zaehler {
                innen,
                pk: Cell::new(0),
                monitor: Cell::new(0),
                archiv: Cell::new(0),
            }
        }
    }

    #[async_trait::async_trait(?Send)]
    impl Sonden for Zaehler {
        async fn packagekit(&self) -> PkNetz {
            self.pk.set(self.pk.get() + 1);
            self.innen.packagekit().await
        }
        fn netzmonitor(&self) -> bool {
            self.monitor.set(self.monitor.get() + 1);
            self.innen.netzmonitor()
        }
        async fn archiv(&self, q: &Quelle) -> Result<String, String> {
            self.archiv.set(self.archiv.get() + 1);
            self.innen.archiv(q).await
        }
    }

    fn archiv() -> Option<Quelle> {
        Quelle::aus_uri("http://archive.ubuntu.com/ubuntu")
    }

    fn lauf(z: &Zaehler, quelle: Option<Quelle>, alle: bool) -> Ergebnis {
        glib::MainContext::new().block_on(pruefen(z, quelle, alle))
    }

    #[test]
    fn pk_online_beendet_die_pruefung() {
        let z = Zaehler::neu("online,offline,offline");
        let e = lauf(&z, archiv(), false);
        assert_eq!(e.online, Some(Stufe::PackageKit));
        assert_eq!((z.pk.get(), z.monitor.get(), z.archiv.get()), (1, 0, 0));
        assert_eq!(e.netzmonitor, None);
        assert_eq!(e.archiv, None);
        assert!(!e.pk_widerspricht());
    }

    #[test]
    fn netzmonitor_rettet_pk_offline() {
        let z = Zaehler::neu("offline,online,offline");
        let e = lauf(&z, archiv(), false);
        assert_eq!(e.online, Some(Stufe::Netzmonitor));
        assert_eq!((z.monitor.get(), z.archiv.get()), (1, 0));
        assert!(e.pk_widerspricht());
    }

    /// Pflichtprobe B2: NetworkManager installiert, verwaltet eth0 nicht.
    #[test]
    fn nm_offline_archiv_erreichbar() {
        let z = Zaehler::neu("offline,offline,online");
        let e = lauf(&z, archiv(), false);
        assert_eq!(e.online, Some(Stufe::Archiv));
        assert_eq!((z.pk.get(), z.monitor.get(), z.archiv.get()), (1, 1, 1));
        assert_eq!(e.archiv, Some(Ok("HTTP 200".into())));
        assert!(e.pk_widerspricht());
        assert_eq!(e.kurztext(), "online (archive reachable)");
        assert!(e
            .hinweis()
            .contains("http://archive.ubuntu.com/ubuntu: reachable (HTTP 200)"));
        assert!(e
            .hinweis()
            .contains("PackageKit considers the system offline"));
        let (titel, text) = pk_widerspruch_dialog(&e);
        assert_eq!(titel, "PackageKit Reports No Network");
        assert!(text.contains("The software source http://archive.ubuntu.com/ubuntu is reachable"));
        assert!(
            !text.contains("may refuse"),
            "Warnung nicht doppelt im Dialog"
        );
    }

    #[test]
    fn alles_offline() {
        let z = Zaehler::neu("offline,offline,offline");
        let e = lauf(&z, archiv(), false);
        assert_eq!(e.online, None);
        assert!(!e.ist_online());
        assert!(!e.pk_widerspricht());
        assert_eq!(
            e.kurztext(),
            "offline: archive.ubuntu.com not reachable (no answer within 3 s)"
        );
        let h = e.hinweis();
        assert!(h.contains("PackageKit: offline"));
        assert!(h.contains("Network monitor (GLib): no network"));
        assert!(h.contains("http://archive.ubuntu.com/ubuntu: not reachable"));
        let (titel, text) = offline_dialog(&e);
        assert_eq!(titel, "No Network Connection");
        assert!(
            text.contains("could not reach the software source http://archive.ubuntu.com/ubuntu.")
        );
    }

    #[test]
    fn ohne_quelle_offline_mit_grund() {
        let z = Zaehler::neu("unbekannt,offline,online");
        let e = lauf(&z, None, false);
        assert_eq!(e.online, None);
        assert_eq!(z.archiv.get(), 0, "ohne Quelle kein HEAD");
        assert_eq!(e.archiv, Some(Err("no APT source found".into())));
        assert_eq!(
            e.kurztext(),
            "offline: no network and no APT source to check"
        );
        assert!(e.hinweis().contains("none found in /etc/apt"));
    }

    #[test]
    fn pk_unbekannt_zaehlt_nicht_als_online() {
        let z = Zaehler::neu("unbekannt,offline,online");
        let e = lauf(&z, archiv(), false);
        assert_eq!(e.online, Some(Stufe::Archiv));
        assert!(
            !e.pk_widerspricht(),
            "nur ein ausdrückliches offline widerspricht"
        );
    }

    #[test]
    fn alle_stufen_fuer_die_diagnose() {
        let z = Zaehler::neu("online,offline,offline");
        let e = lauf(&z, archiv(), true);
        assert_eq!(e.online, Some(Stufe::PackageKit), "Entscheidung bleibt");
        assert_eq!((z.pk.get(), z.monitor.get(), z.archiv.get()), (1, 1, 1));
        assert_eq!(e.netzmonitor, Some(false));
        assert!(matches!(e.archiv, Some(Err(_))));
    }

    #[test]
    fn debug_zeile() {
        let z = Zaehler::neu("offline,offline,online");
        let e = lauf(&z, archiv(), false);
        assert_eq!(
            e.debug_zeile(),
            "netz\tonline\tarchiv\tOffline\taus\terreichbar\tarchive.ubuntu.com"
        );
    }

    #[test]
    fn pk_werte() {
        assert_eq!(PkNetz::aus_wert(0), PkNetz::Unbekannt);
        assert_eq!(PkNetz::aus_wert(1), PkNetz::Offline);
        for w in 2..=5 {
            assert_eq!(PkNetz::aus_wert(w), PkNetz::Online);
        }
        assert_eq!(PkNetz::aus_wert(9), PkNetz::Unbekannt);
    }

    #[test]
    fn attrappe_aus_text() {
        let s = AttrappeSonden::aus_text("offline, offline ,online").unwrap();
        assert_eq!((s.pk, s.monitor, s.archiv), (PkNetz::Offline, false, true));
        assert!(AttrappeSonden::aus_text("offline,offline").is_none());
        assert!(AttrappeSonden::aus_text("aus,an,an").is_none());
    }

    #[test]
    fn uri_zerlegen() {
        let q = Quelle::aus_uri("http://archive.ubuntu.com/ubuntu").unwrap();
        assert_eq!(
            (q.host.as_str(), q.port, q.tls, q.pfad.as_str()),
            ("archive.ubuntu.com", 80, false, "/ubuntu/")
        );
        let q = Quelle::aus_uri("https://user:geheim@spiegel.example:8443/debian/").unwrap();
        assert_eq!(
            (q.host.as_str(), q.port, q.tls, q.pfad.as_str()),
            ("spiegel.example", 8443, true, "/debian/")
        );
        let q = Quelle::aus_uri("http://[2001:db8::1]:3142/ubuntu").unwrap();
        assert_eq!((q.host.as_str(), q.port), ("[2001:db8::1]", 3142));
        let q = Quelle::aus_uri("http://[2001:db8::1]/ubuntu").unwrap();
        assert_eq!((q.host.as_str(), q.port), ("[2001:db8::1]", 80));
        let q = Quelle::aus_uri("mirror+http://mirrors.example/liste").unwrap();
        assert_eq!(q.host, "mirrors.example");
        let q = Quelle::aus_uri("http://deb.example").unwrap();
        assert_eq!(q.pfad, "/");
        assert!(Quelle::aus_uri("file:/srv/repo").is_none());
        assert!(Quelle::aus_uri("cdrom:[Ubuntu]/").is_none());
        assert!(Quelle::aus_uri("mirror+file:/etc/apt/spiegel.txt").is_none());
        assert!(Quelle::aus_uri("http://host:abc/x").is_none());
    }

    const UBUNTU_SOURCES: &str = "\
## Ubuntu 26.04
Types: deb
URIs: http://archive.ubuntu.com/ubuntu
Suites: resolute resolute-updates resolute-backports
Components: main universe restricted multiverse
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg

Types: deb
URIs: http://security.ubuntu.com/ubuntu
Suites: resolute-security
Components: main universe restricted multiverse
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
";

    fn dateien(liste: &[(&str, &str)]) -> Vec<(String, String)> {
        liste
            .iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect()
    }

    /// entwicklungs-vm: eine Fremdquelle sortiert vor ubuntu.sources.
    #[test]
    fn distribution_vor_fremdquelle() {
        let d = dateien(&[
            (
                "beispiel.list",
                "deb [arch=amd64 signed-by=/usr/share/keyrings/x.gpg] https://downloads.example.com/apt stable main\n",
            ),
            ("ubuntu.sources", UBUNTU_SOURCES),
        ]);
        let q = quelle_waehlen(&d, Some("resolute")).unwrap();
        assert_eq!(q.uri, "http://archive.ubuntu.com/ubuntu");
        // Ohne Codename: die erste Quelle in apt-Reihenfolge.
        let q = quelle_waehlen(&d, None).unwrap();
        assert_eq!(q.host, "downloads.example.com");
        assert!(q.tls);
    }

    #[test]
    fn nur_sicherheitsquelle_zaehlt_auch() {
        let d = dateien(&[(
            "ubuntu.sources",
            "Types: deb\nURIs: http://security.ubuntu.com/ubuntu\nSuites: resolute-security\n",
        )]);
        assert_eq!(
            quelle_waehlen(&d, Some("resolute")).unwrap().host,
            "security.ubuntu.com"
        );
    }

    #[test]
    fn sources_abgeschaltet_quelltext_mehrere_uris() {
        let d = dateien(&[(
            "a.sources",
            "Types: deb-src\nURIs: http://src.example/ubuntu\nSuites: resolute\n\n\
             Types: deb\nEnabled: no\nURIs: http://aus.example/ubuntu\nSuites: resolute\n\n\
             # Kommentar\nTypes: deb deb-src\nURIs: file:/srv/lokal\n http://zwei.example/ubuntu\n\
             Suites: resolute\n",
        )]);
        let q = quelle_waehlen(&d, Some("resolute")).unwrap();
        assert_eq!(q.host, "zwei.example");
    }

    #[test]
    fn einzeiliges_format() {
        let d = dateien(&[(
            "sources.list",
            "# deb http://kommentar.example/ubuntu resolute main\n\
             deb-src http://src.example/ubuntu resolute main\n\
             deb [ arch=amd64 ] http://de.archive.ubuntu.com/ubuntu/ resolute main # Rest\n\
             deb http://alt.example/ubuntu noble main\n",
        )]);
        let q = quelle_waehlen(&d, Some("resolute")).unwrap();
        assert_eq!(q.uri, "http://de.archive.ubuntu.com/ubuntu");
        assert_eq!(
            quelle_waehlen(&d, Some("noble")).unwrap().host,
            "alt.example"
        );
        assert!(quelle_waehlen(&dateien(&[("leer.list", "# nichts\n")]), Some("x")).is_none());
    }

    #[test]
    fn os_release() {
        let t = "PRETTY_NAME=\"Ubuntu 26.04.1 LTS\"\nVERSION_CODENAME=resolute\nUBUNTU_CODENAME=resolute\n";
        assert_eq!(codename_aus_os_release(t).as_deref(), Some("resolute"));
        assert_eq!(
            codename_aus_os_release("VERSION_CODENAME=\nUBUNTU_CODENAME=\"noble\"\n").as_deref(),
            Some("noble")
        );
        assert_eq!(codename_aus_os_release("ID=debian\n"), None);
    }

    #[test]
    fn quellendateien_in_apt_reihenfolge() {
        let d = std::env::temp_dir().join(format!("qs-netz-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sources.list.d")).unwrap();
        std::fs::write(
            d.join("sources.list"),
            "deb http://a.example/u resolute main\n",
        )
        .unwrap();
        std::fs::write(d.join("sources.list.d/b.sources"), UBUNTU_SOURCES).unwrap();
        std::fs::write(d.join("sources.list.d/a.list"), "").unwrap();
        std::fs::write(
            d.join("sources.list.d/c.save"),
            "deb http://nein.example/ resolute x\n",
        )
        .unwrap();
        let namen: Vec<String> = quellendateien(&d).into_iter().map(|(n, _)| n).collect();
        assert_eq!(namen, ["sources.list", "a.list", "b.sources"]);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn http_statuszeile() {
        assert_eq!(
            statuszeile(b"HTTP/1.1 200 OK\r\nDate: x\r\n").as_deref(),
            Some("HTTP 200")
        );
        assert_eq!(
            statuszeile(b"HTTP/1.0 301 Moved\r\n").as_deref(),
            Some("HTTP 301")
        );
        assert_eq!(statuszeile(b"SSH-2.0-OpenSSH\r\n"), None);
        assert_eq!(statuszeile(b"HTTP/1.1 2x0\r\n"), None);
        assert_eq!(statuszeile(b""), None);
    }
}
