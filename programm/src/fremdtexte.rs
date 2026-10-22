//! Übersetzung der Texte, die die Bausteine liefern (SW-1 `qs_typen` und
//! `qs_paketkit`, SW-4 `qs_transaktion`).
//!
//! Die Bausteine bleiben unverändert und liefern weiter deutsche Texte:
//! Kategorienamen (`Kategorie::name`), Statustexte im `Zustand::Laeuft`
//! (PackageKit-Status über `qs_paketkit::enums::statustext`, eigene Texte des
//! Monitors) und die Fehlerdialoge (`qs_transaktion::fehlertexte`). Die
//! Oberfläche übersetzt sie hier über englische msgids; die deutschen
//! Übersetzungen in `po/de.po` sind wortgleich mit den Bausteinen (Tests unten
//! gegen die Bausteine selbst). Unbekannte Statustexte zeigt sie unverändert.
//!
//! Langfristig sollten die Bausteine Kennungen statt fertiger Sätze liefern
//! (Vorschlag in der Übergabe Software 0.1.1).

use qs_transaktion::fehlertexte::Fehlertext;
use qs_typen::{Fehler, Kategorie};

use crate::i18n::{gettext, gettext_f, ngettext_f, pgettext};

/// Anzeigename einer Kategorie (GNOME-2-Menünamen).
pub fn kategorie(k: Kategorie) -> String {
    match k {
        Kategorie::Zubehoer => gettext("Accessories"),
        Kategorie::Buero => gettext("Office"),
        Kategorie::Grafik => gettext("Graphics"),
        Kategorie::Internet => gettext("Internet"),
        Kategorie::Multimedia => gettext("Sound & Video"),
        Kategorie::Spiele => gettext("Games"),
        Kategorie::Entwicklung => gettext("Programming"),
        Kategorie::Bildung => gettext("Education"),
        Kategorie::System => gettext("System Tools"),
        Kategorie::Sonstiges => gettext("Other"),
    }
}

/// Statustext eines laufenden Vorgangs. Der Monitor hängt Paketfortschritt
/// mit „ – “ an („Lade herunter … – gimp 42 %“); nur der Teil davor ist Text.
pub fn status(text: &str) -> String {
    match text.split_once(" – ") {
        Some((satz, rest)) => format!("{} – {rest}", status_satz(satz)),
        None => status_satz(text),
    }
}

fn status_satz(text: &str) -> String {
    if let Some(s) = pk_status(text) {
        return s;
    }
    if let Some(s) = nachfrage(text) {
        return s;
    }
    match text {
        "Gestartet" => gettext("Started"),
        "Wird vorbereitet …" => gettext("Preparing…"),
        "Wird abgebrochen …" => gettext("Cancelling…"),
        qs_transaktion::monitor::NEUSTART_HINWEIS => gettext("A restart will be required"),
        "Verbindung zur Paketverwaltung verloren, versuche Wiederanbindung …" => {
            gettext("Lost the connection to the package manager, trying to reconnect…")
        }
        "Warte auf eine laufende Transaktion der Paketverwaltung …" => {
            gettext("Waiting for a running package manager transaction…")
        }
        "Eine frühere Transaktion der Paketverwaltung läuft noch …" => frueher_laufend(1),
        _ => match text
            .strip_suffix(" frühere Transaktionen der Paketverwaltung laufen noch …")
            .and_then(|n| n.parse::<u32>().ok())
        {
            Some(n) => frueher_laufend(n),
            None => text.to_string(),
        },
    }
}

fn frueher_laufend(n: u32) -> String {
    ngettext_f(
        "An earlier package manager transaction is still running…",
        "{n} earlier package manager transactions are still running…",
        n,
        &[],
    )
}

/// Nachfrage des Monitors nach langer Zeit ohne Fortschritt
/// („Kein Fortschritt seit 10 Minuten. Weiter warten oder abbrechen?“).
fn nachfrage(text: &str) -> Option<String> {
    let rest = text.strip_prefix(qs_transaktion::monitor::NACHFRAGE_PRAEFIX)?;
    let rest = rest.strip_suffix(". Weiter warten oder abbrechen?")?;
    let (zahl, einheit) = rest.split_once(' ')?;
    let n: u32 = zahl.parse().ok()?;
    match einheit {
        "Minuten" => Some(ngettext_f(
            "No progress for {n} minute. Keep waiting or cancel?",
            "No progress for {n} minutes. Keep waiting or cancel?",
            n,
            &[],
        )),
        "Sekunden" => Some(ngettext_f(
            "No progress for {n} second. Keep waiting or cancel?",
            "No progress for {n} seconds. Keep waiting or cancel?",
            n,
            &[],
        )),
        _ => None,
    }
}

/// Texte von `qs_paketkit::enums::statustext` (PkStatusEnum) und der
/// Attrappe von SW-4 („Richte ein …“); Wortlaut nach PackageKit.
fn pk_status(text: &str) -> Option<String> {
    Some(match text {
        "Unbekannt" => pgettext("PackageKit status", "Unknown"),
        "Warte …" => pgettext("PackageKit status", "Waiting…"),
        "Bereite vor …" => pgettext("PackageKit status", "Preparing…"),
        "Läuft …" => pgettext("PackageKit status", "Running…"),
        "Frage ab …" => pgettext("PackageKit status", "Querying…"),
        "Hole Informationen …" => pgettext("PackageKit status", "Getting information…"),
        "Entferne …" => pgettext("PackageKit status", "Removing…"),
        "Aktualisiere Paketquellen …" => {
            pgettext("PackageKit status", "Refreshing software sources…")
        }
        "Lade herunter …" => pgettext("PackageKit status", "Downloading…"),
        "Installiere …" => pgettext("PackageKit status", "Installing…"),
        "Aktualisiere …" => pgettext("PackageKit status", "Updating…"),
        "Räume auf …" => pgettext("PackageKit status", "Cleaning up…"),
        "Ersetze veraltete Pakete …" => {
            pgettext("PackageKit status", "Replacing obsolete packages…")
        }
        "Löse Abhängigkeiten auf …" => pgettext("PackageKit status", "Resolving dependencies…"),
        "Prüfe Signaturen …" => pgettext("PackageKit status", "Checking signatures…"),
        "Prüfe Änderungen …" => pgettext("PackageKit status", "Testing changes…"),
        "Übernehme Änderungen …" => pgettext("PackageKit status", "Committing changes…"),
        "Frage an …" => pgettext("PackageKit status", "Requesting data…"),
        "Fertig" => pgettext("PackageKit status", "Finished"),
        "Breche ab …" => pgettext("PackageKit status", "Cancelling…"),
        "Lade Paketquelle herunter …" => {
            pgettext("PackageKit status", "Downloading repository information…")
        }
        "Lade Paketliste herunter …" => {
            pgettext("PackageKit status", "Downloading the list of packages…")
        }
        "Lade Dateiliste herunter …" => pgettext("PackageKit status", "Downloading file lists…"),
        "Lade Änderungsprotokoll herunter …" => {
            pgettext("PackageKit status", "Downloading lists of changes…")
        }
        "Lade Gruppen herunter …" => pgettext("PackageKit status", "Downloading groups…"),
        "Lade Aktualisierungsinformationen herunter …" => {
            pgettext("PackageKit status", "Downloading update information…")
        }
        "Packe neu …" => pgettext("PackageKit status", "Repackaging files…"),
        "Lade Zwischenspeicher …" => pgettext("PackageKit status", "Loading cache…"),
        "Suche Anwendungen …" => pgettext("PackageKit status", "Scanning applications…"),
        "Erzeuge Paketliste …" => pgettext("PackageKit status", "Generating package lists…"),
        "Warte auf die Paketsperre …" => {
            pgettext("PackageKit status", "Waiting for the package manager lock…")
        }
        "Warte auf Berechtigung …" => {
            pgettext("PackageKit status", "Waiting for authentication…")
        }
        "Prüfe laufende Prozesse …" => {
            pgettext("PackageKit status", "Checking running applications…")
        }
        "Prüfe ausführbare Dateien …" => {
            pgettext("PackageKit status", "Checking applications in use…")
        }
        "Prüfe Bibliotheken …" => pgettext("PackageKit status", "Checking libraries in use…"),
        "Kopiere Dateien …" => pgettext("PackageKit status", "Copying files…"),
        "Führe Skripte aus …" => pgettext("PackageKit status", "Running hooks…"),
        "Arbeite …" => pgettext("PackageKit status", "Working…"),
        "Richte ein …" => pgettext("PackageKit status", "Setting up…"),
        _ => return None,
    })
}

/// Überschrift, Erklärung und Handlung des Fehlerdialogs; Wortlaut wie
/// `qs_transaktion::fehlertexte::fehlertext` (SW-4). Einzige Abweichung: Bei
/// `KeinAgent` nennt der Text keine interne Kennung (QD-119; Test
/// `fehlerdialoge_deutsch_wie_sw4`).
pub fn fehlerdialog(fehler: &Fehler) -> Fehlertext {
    let (ueberschrift, erklaerung, handlung) = match fehler {
        Fehler::NichtBerechtigt => (
            gettext("Not Authorized"),
            gettext(
                "Permission to manage software was not granted. Either the authorization \
                 dialog was cancelled, the password was wrong, or the policy (polkit) does not \
                 allow this action for your account.",
            ),
            gettext(
                "Try again and confirm the authorization dialog, or contact your system \
                 administrator.",
            ),
        ),
        Fehler::KeinAgent => (
            gettext("Authorization Dialog Missing"),
            gettext(
                "The session’s authorization dialog (polkit agent) is missing. Without it, no \
                 program can install or remove packages. In Querencia Desktop, the session \
                 starts it as “querencia-polkit-agent”.",
            ),
            gettext(
                "Start the authorization dialog from a terminal in the session \
                 (“querencia-polkit-agent”) or log in again once it is set up, then repeat \
                 the operation.",
            ),
        ),
        Fehler::SperreBelegt(wer) => (
            gettext("Package Manager Is Busy"),
            if wer.trim().is_empty() {
                gettext(
                    "Another program is using the package manager right now, for example \
                     automatic updates (unattended-upgrades) or “apt” in a terminal.",
                )
            } else {
                gettext_f(
                    "Another program is using the package manager right now: {program}. As \
                     long as it is running, Querencia Software cannot change any packages.",
                    &[("program", wer.trim())],
                )
            },
            gettext(
                "Wait until the other program has finished; Querencia Software tries again \
                 every 10 seconds (for up to 5 minutes). Or cancel.",
            ),
        ),
        Fehler::NetzFehlt => (
            gettext("No Network Connection"),
            gettext(
                "The software sources cannot be reached. Either there is no connection to \
                 the internet, or the mirror server does not respond.",
            ),
            gettext("Check the network connection and then try again."),
        ),
        Fehler::PaketNichtGefunden(name) => (
            gettext("Package Not Found"),
            gettext_f(
                "The package “{package}” is not available in the configured software sources, \
                 or the list of sources is out of date.",
                &[("package", name)],
            ),
            gettext("Refresh the software sources (File → Check for Updates) and search again."),
        ),
        Fehler::AbhaengigkeitenKonflikt(text) => (
            gettext("Dependencies Cannot Be Resolved"),
            gettext_f(
                "The change conflicts with other installed packages or needs packages that \
                 are not available. Message from the package manager: {details}",
                &[("details", &kuerze(text, 300))],
            ),
            gettext(
                "Leave the system unchanged; check the updates or resolve the conflict with \
                 an expert tool (apt).",
            ),
        ),
        Fehler::DaemonWeg => (
            gettext("Lost Connection to the Package Manager"),
            gettext(
                "The PackageKit service was stopped or restarted during the operation. \
                 Querencia Software tried once to resume the operation; that did not succeed. \
                 The package database stays consistent, but the operation may not have \
                 finished.",
            ),
            gettext(
                "Check whether the package is installed (view “Installed”) and repeat the \
                 operation if necessary.",
            ),
        ),
        Fehler::Abgebrochen => (
            gettext("Operation Cancelled"),
            gettext(
                "The operation was cancelled at your request. Files already downloaded stay \
                 in the cache. If the cancellation only took effect during setup, some \
                 packages may already be installed or removed.",
            ),
            gettext(
                "If necessary, check the state in the view “Installed”, and start the \
                 operation again if you want to carry it out after all.",
            ),
        ),
        Fehler::Sonstiges { code, .. } if code.trim() == "zeitueberschreitung" => (
            gettext("Package Manager Does Not Respond"),
            gettext("PackageKit did not answer in time. The service may be overloaded or hanging."),
            gettext(
                "Try again in a few minutes. If that does not help, log in again or restart \
                 the computer.",
            ),
        ),
        Fehler::Sonstiges { code, text } => (
            gettext("Package Manager Error"),
            if text.trim().is_empty() {
                gettext_f(
                    "PackageKit reports the error “{code}”. Details: none",
                    &[("code", code)],
                )
            } else {
                gettext_f(
                    "PackageKit reports the error “{code}”. Details: {details}",
                    &[("code", code), ("details", &kuerze(text, 300))],
                )
            },
            gettext(
                "Try again later. If the error persists, note the error code and look in \
                 the log ~/.local/state/querencia/software.log.",
            ),
        ),
    };
    Fehlertext {
        ueberschrift,
        erklaerung,
        handlung,
    }
}

/// Wie `kuerze` in `qs_transaktion::fehlertexte` (dort privat).
fn kuerze(text: &str, max: usize) -> String {
    let t = text.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        let mut s: String = t.chars().take(max).collect();
        s.push('…');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::fill;
    use std::collections::HashMap;

    /// `po/de.po` als Tabelle (Kontext, msgid) → msgstr bzw. msgstr[0..].
    fn deutsch() -> HashMap<(Option<String>, String), Vec<String>> {
        fn wert(zeilen: &[&str]) -> String {
            zeilen
                .iter()
                .map(|z| {
                    let z = z.trim();
                    let innen = &z[z.find('"').unwrap() + 1..z.len() - 1];
                    innen.replace("\\n", "\n").replace("\\\"", "\"")
                })
                .collect()
        }
        let mut tabelle = HashMap::new();
        for block in include_str!("../po/de.po").split("\n\n") {
            let zeilen: Vec<&str> = block.lines().filter(|z| !z.starts_with('#')).collect();
            let mut felder: Vec<(String, Vec<&str>)> = Vec::new();
            for z in &zeilen {
                if z.starts_with('"') {
                    if let Some(f) = felder.last_mut() {
                        f.1.push(z);
                    }
                } else if let Some((name, _)) = z.split_once(' ') {
                    felder.push((name.to_string(), vec![z]));
                }
            }
            let feld = |n: &str| felder.iter().find(|(k, _)| k == n).map(|(_, v)| wert(v));
            let Some(id) = feld("msgid").filter(|i| !i.is_empty()) else {
                continue;
            };
            let mut str: Vec<String> = felder
                .iter()
                .filter(|(k, _)| k.starts_with("msgstr"))
                .map(|(_, v)| wert(v))
                .collect();
            if str.is_empty() {
                str.push(String::new());
            }
            tabelle.insert((feld("msgctxt"), id), str);
        }
        tabelle
    }

    fn de(ctxt: Option<&str>, id: &str) -> String {
        deutsch()
            .get(&(ctxt.map(str::to_string), id.to_string()))
            .and_then(|v| v.first().cloned())
            .unwrap_or_else(|| panic!("fehlt in de.po: {ctxt:?} {id}"))
    }

    #[test]
    fn kategorien_deutsch_wie_qs_typen() {
        for k in Kategorie::alle() {
            // Ohne Katalog liefert gettext die msgid; de.po muss den Namen von SW-1 haben.
            assert_eq!(de(None, &kategorie(*k)), k.name());
        }
    }

    #[test]
    fn jeder_packagekit_status_hat_eine_uebersetzung() {
        for code in 0..=40 {
            let text = qs_paketkit::enums::statustext(code);
            let en = status(text);
            assert_ne!(en, text, "PackageKit-Status {code} „{text}“ ohne msgid");
            assert_eq!(de(Some("PackageKit status"), &en), text);
        }
    }

    #[test]
    fn monitortexte_und_paketfortschritt() {
        assert_eq!(
            status("Lade herunter … – gimp 42 %"),
            "Downloading… – gimp 42 %"
        );
        assert_eq!(status("Gestartet"), "Started");
        assert_eq!(
            status("3 frühere Transaktionen der Paketverwaltung laufen noch …"),
            "3 earlier package manager transactions are still running…"
        );
        let n = status("Kein Fortschritt seit 10 Minuten. Weiter warten oder abbrechen?");
        assert_eq!(n, "No progress for 10 minutes. Keep waiting or cancel?");
        assert_eq!(status("Völlig neuer Text"), "Völlig neuer Text");
        for (deutsch, en) in [
            ("Wird vorbereitet …", "Preparing…"),
            ("Wird abgebrochen …", "Cancelling…"),
            (
                qs_transaktion::monitor::NEUSTART_HINWEIS,
                "A restart will be required",
            ),
        ] {
            assert_eq!(status(deutsch), en);
            assert_eq!(de(None, en), deutsch);
        }
    }

    /// Satz aus dem Text von SW-4 zu `KeinAgent`, den das Programm ohne die
    /// interne Kennung schreibt (QD-119), und sein Ersatz.
    const KEIN_AGENT_SW4: (&str, &str) = (
        "Querencia Sitzung (G2-K) liefert ihn als",
        "In Querencia Desktop startet die Sitzung ihn als",
    );

    /// Jede Fehlervariante: deutscher Katalogtext = Text von SW-4 (bis auf
    /// [`KEIN_AGENT_SW4`]).
    #[test]
    fn fehlerdialoge_deutsch_wie_sw4() {
        let tabelle = deutsch();
        let uebersetze = |en: &str| -> String {
            tabelle
                .get(&(None, en.to_string()))
                .and_then(|v| v.first().cloned())
                .unwrap_or_else(|| panic!("fehlt in de.po: {en}"))
        };
        let lang = "x".repeat(400);
        let faelle = [
            Fehler::NichtBerechtigt,
            Fehler::KeinAgent,
            Fehler::SperreBelegt(String::new()),
            Fehler::SperreBelegt("apt".into()),
            Fehler::NetzFehlt,
            Fehler::PaketNichtGefunden("gimp".into()),
            Fehler::AbhaengigkeitenKonflikt(lang.clone()),
            Fehler::DaemonWeg,
            Fehler::Abgebrochen,
            Fehler::Sonstiges {
                code: "zeitueberschreitung".into(),
                text: String::new(),
            },
            Fehler::Sonstiges {
                code: "internal-error".into(),
                text: String::new(),
            },
            Fehler::Sonstiges {
                code: "internal-error".into(),
                text: "kaputt".into(),
            },
        ];
        for f in faelle {
            let sw4 = qs_transaktion::fehlertexte::fehlertext(&f);
            let en = fehlerdialog(&f);
            // Platzhalter nach der Übersetzung einsetzen, wie gettext_f.
            let werte: Vec<(&str, String)> = match &f {
                Fehler::SperreBelegt(w) => vec![("program", w.trim().to_string())],
                Fehler::PaketNichtGefunden(p) => vec![("package", p.clone())],
                Fehler::AbhaengigkeitenKonflikt(t) => vec![("details", kuerze(t, 300))],
                Fehler::Sonstiges { code, text } => {
                    vec![("code", code.clone()), ("details", kuerze(text, 300))]
                }
                _ => vec![],
            };
            let args: Vec<(&str, &str)> = werte.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let zurueck = |en_text: &str| -> String {
                // msgid zurückgewinnen: Platzhalter wieder einsetzen ist nicht
                // umkehrbar, also über alle Einträge mit Platzhaltern suchen.
                if let Some(v) = tabelle.get(&(None, en_text.to_string())) {
                    return v[0].clone();
                }
                tabelle
                    .iter()
                    .find(|((c, id), _)| {
                        c.is_none() && id.contains('{') && fill(id, &args) == en_text
                    })
                    .map(|(_, v)| fill(&v[0], &args))
                    .unwrap_or_else(|| panic!("keine msgid für „{en_text}“"))
            };
            assert_eq!(uebersetze(&en.ueberschrift), sw4.ueberschrift, "{f:?}");
            let mut soll = sw4.erklaerung.clone();
            if f == Fehler::KeinAgent {
                let (alt, neu) = KEIN_AGENT_SW4;
                assert!(soll.contains(alt), "SW-4 hat den Satz geändert: {soll}");
                soll = soll.replace(alt, neu);
            }
            assert_eq!(zurueck(&en.erklaerung), soll, "{f:?}");
            assert_eq!(zurueck(&en.handlung), sw4.handlung, "{f:?}");
        }
    }

    /// Kein Nutzertext (msgid oder msgstr in Vorlage und de.po) enthält eine
    /// interne Kennung wie QD-119, G2-K, QREL-06, SW-4 oder GH-7 (QD-119).
    #[test]
    fn keine_internen_kennungen_in_nutzertexten() {
        fn kennung(text: &str) -> bool {
            let b = text.as_bytes();
            ["QD-", "QF-", "G1-", "G2-", "GH-", "SW-", "QREL"]
                .iter()
                .any(|k| {
                    text.match_indices(k).any(|(i, _)| {
                        let davor = i.checked_sub(1).map(|j| b[j]);
                        !davor.is_some_and(|c| c.is_ascii_alphanumeric())
                    })
                })
        }
        for (datei, inhalt) in [
            ("de.po", include_str!("../po/de.po")),
            (
                "querencia-software.pot",
                include_str!("../po/querencia-software.pot"),
            ),
        ] {
            for (nr, zeile) in inhalt.lines().enumerate() {
                if zeile.starts_with('#') {
                    continue;
                }
                assert!(!kennung(zeile), "{datei}:{}: {zeile}", nr + 1);
            }
        }
        assert!(kennung("Querencia Sitzung (G2-K) liefert"));
        assert!(!kennung("UTF-8, PGP-Schlüssel"));
    }
}
