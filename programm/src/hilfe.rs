//! Adressen für den Info-Dialog und das Hilfe-Menü (Website, Online-Hilfe,
//! Fehler melden) und das Öffnen im Standardprogramm der Sitzung.
//!
//! „Fehler melden“ nimmt `BUG_REPORT_URL` aus der Versionsdatei des
//! Metapakets (`/usr/share/querencia/release`, Syntax wie os-release(5)).
//! Fehlt die Datei oder der Schlüssel, gilt [`FEHLER_MELDEN`]; das ist kein
//! Fehler (einzelne Pakete, Bau aus dem Quellbaum).

use gtk::prelude::*;

use crate::debug;
use crate::i18n::gettext;

/// Website von Querencia Desktop (Info-Dialog).
pub const WEBSITE: &str = "https://querencialinux.org";

/// Rückfall für „Fehler melden“, wenn die Versionsdatei fehlt.
pub const FEHLER_MELDEN: &str = "https://github.com/querencia-desktop/querencia/issues/new/choose";

/// Versionsdatei des Metapakets `querencia-desktop`; der Pfad ist fest.
const VERSIONSDATEI: &str = "/usr/share/querencia/release";

/// Adresse der Online-Hilfe in der Sprache der Oberfläche: Die Adresse steht
/// selbst im Katalog, gettext wählt sie wie jeden Text nach der Locale
/// (ohne Katalog Englisch).
pub fn online_hilfe() -> String {
    // TRANSLATORS: address of the online help; only change the language code
    // (German …/de/docs/…)
    gettext("https://querencialinux.org/en/docs/installing-software")
}

/// Adresse für „Fehler melden“: `BUG_REPORT_URL` der Versionsdatei, sonst
/// [`FEHLER_MELDEN`].
pub fn fehler_melden() -> String {
    std::fs::read_to_string(VERSIONSDATEI)
        .ok()
        .and_then(|inhalt| wert(&inhalt, "BUG_REPORT_URL"))
        .filter(|url| url.starts_with("https://"))
        .unwrap_or_else(|| FEHLER_MELDEN.to_string())
}

/// Wert von `schluessel` in einer Datei mit os-release-Syntax: eine Zuweisung
/// je Zeile, leere Zeilen und `#`-Zeilen zählen nicht, Wert in doppelten oder
/// einfachen Anführungszeichen oder ohne; `\"`, `\\`, `` \` `` und `\$` sind
/// die einzigen Escapes. Ein leerer Wert gilt als nicht gesetzt.
fn wert(inhalt: &str, schluessel: &str) -> Option<String> {
    inhalt.lines().find_map(|zeile| {
        let zeile = zeile.trim();
        if zeile.starts_with('#') {
            return None;
        }
        let (k, v) = zeile.split_once('=')?;
        if k.trim() != schluessel {
            return None;
        }
        let v = v.trim();
        let v = if v.len() >= 2 && (v.starts_with('"') && v.ends_with('"')) {
            ohne_escapes(&v[1..v.len() - 1])
        } else if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
            v[1..v.len() - 1].to_string()
        } else {
            v.to_string()
        };
        Some(v).filter(|v| !v.is_empty())
    })
}

fn ohne_escapes(s: &str) -> String {
    let mut aus = String::with_capacity(s.len());
    let mut zeichen = s.chars().peekable();
    while let Some(c) = zeichen.next() {
        if c == '\\' {
            if let Some(&n) = zeichen.peek() {
                if matches!(n, '"' | '\\' | '`' | '$') {
                    aus.push(n);
                    zeichen.next();
                    continue;
                }
            }
        }
        aus.push(c);
    }
    aus
}

/// Öffnet `uri` im Standardprogramm der Sitzung (`gtk::UriLauncher`; ohne
/// Sandbox wie xdg-open über GIO).
pub fn oeffnen(parent: &impl IsA<gtk::Window>, uri: &str) {
    debug(&format!("uri\t{uri}"));
    let uri = uri.to_string();
    gtk::UriLauncher::new(&uri).launch(
        Some(parent.as_ref()),
        None::<&gtk::gio::Cancellable>,
        move |ergebnis| {
            if let Err(err) = ergebnis {
                eprintln!("querencia-software: {uri} nicht geöffnet: {err}");
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inhalt Alpha 2.1 aus der Doku des Metapakets.
    const ALPHA21: &str = "NAME=\"Querencia Desktop\"\n\
        VERSION=\"Alpha 2.1\"\n\
        VERSION_ID=\"0.1.0~alpha2.1\"\n\
        HOME_URL=\"https://querencialinux.org\"\n\
        BUG_REPORT_URL=\"https://github.com/querencia-desktop/querencia/issues\"\n\
        DOCUMENTATION_URL=\"https://querencialinux.org/de/docs/\"\n";

    #[test]
    fn versionsdatei_alpha21() {
        assert_eq!(
            wert(ALPHA21, "BUG_REPORT_URL").as_deref(),
            Some("https://github.com/querencia-desktop/querencia/issues")
        );
        assert_eq!(wert(ALPHA21, "VERSION").as_deref(), Some("Alpha 2.1"));
        assert_eq!(wert(ALPHA21, "HOME_URL").as_deref(), Some(WEBSITE));
    }

    #[test]
    fn versionsdatei_syntax() {
        let inhalt = "# BUG_REPORT_URL=\"https://falsch\"\n\n\
            VERSION_CODENAME=neu\n\
            BUG_REPORT_URL=https://ohne.anfuehrung/x\n";
        assert_eq!(
            wert(inhalt, "BUG_REPORT_URL").as_deref(),
            Some("https://ohne.anfuehrung/x")
        );
        assert_eq!(wert("A='ein fach'", "A").as_deref(), Some("ein fach"));
        assert_eq!(
            wert(r#"A="a\"b\\c\$d\`e\n""#, "A").as_deref(),
            Some(r#"a"b\c$d`e\n"#)
        );
        assert_eq!(wert("A=\"\"", "A"), None);
        assert_eq!(wert("AB=\"x\"", "A"), None);
        assert_eq!(wert("", "BUG_REPORT_URL"), None);
    }

    #[test]
    fn adressen() {
        assert!(FEHLER_MELDEN.starts_with("https://github.com/querencia-desktop/"));
        // Ohne Katalog (Tests) Englisch.
        assert_eq!(
            online_hilfe(),
            "https://querencialinux.org/en/docs/installing-software"
        );
    }
}
