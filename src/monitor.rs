//! Einfacher lokaler Transaktionsmonitor (Übergang, bis `qs_transaktion` von
//! SW-4 eingehängt wird). Er bildet die Zustände aus SCHNITTSTELLEN.md ab:
//!
//! * Entfernen: erst `simulieren`, Zustand `Simuliert`, dann warten auf
//!   `bestaetigen()`; sonst sofort `ausfuehren`.
//! * `Fortschritt::WartetAufSperre` und `Fehler::SperreBelegt` →
//!   `Zustand::WartetAufSperre`; die
//!   Oberfläche fragt, `sperre_erneut_versuchen()` startet die Transaktion
//!   neu.
//! * `Fortschritt::Fehler(..)` → `Fehlgeschlagen`, `Fertig`, `Abgebrochen`.
//!
//! Kein Logging, keine Zeitbegrenzung, keine Wiederanbindung – das liefert
//! SW-4. INTEGRATION: Modul löschen, `LokalerMonitor` durch den Monitor aus
//! `qs_transaktion` ersetzen (gleiche Trait-Signatur).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::glib;
use qs_typen::{Fehler, Fortschritt, Vorgang};

use crate::schnittstellen::{
    Abbruch, Auftrag, Paketquelle, TransaktionsGriff, Transaktionsmonitor, Zustand,
};

/// Steuerung einer laufenden Transaktion, erreichbar über die Griff-ID.
struct Steuerung {
    abbruch: Option<Box<dyn Abbruch>>,
    /// Wird von `bestaetigen()` gerufen (nur nach `Simuliert`).
    bestaetigen: Option<Box<dyn FnOnce()>>,
    /// Startet die Transaktion neu (nach `WartetAufSperre`).
    erneut: Option<Rc<dyn Fn()>>,
    melden: Rc<dyn Fn(Zustand)>,
    beendet: bool,
}

thread_local! {
    static LAUFEND: RefCell<HashMap<u64, Steuerung>> = RefCell::new(HashMap::new());
    static NAECHSTE_ID: RefCell<u64> = const { RefCell::new(1) };
}

fn griff_abbrechen(id: u64) {
    let (abbruch, melden) = LAUFEND.with(|l| {
        let mut l = l.borrow_mut();
        match l.get_mut(&id) {
            Some(s) if !s.beendet => {
                s.beendet = true;
                s.bestaetigen = None;
                s.erneut = None;
                (s.abbruch.take(), Some(s.melden.clone()))
            }
            _ => (None, None),
        }
    });
    if let Some(a) = abbruch {
        a.abbrechen();
    }
    if let Some(m) = melden {
        m(Zustand::Abgebrochen);
    }
    LAUFEND.with(|l| l.borrow_mut().remove(&id));
}

fn griff_bestaetigen(id: u64) {
    let f = LAUFEND.with(|l| {
        l.borrow_mut()
            .get_mut(&id)
            .and_then(|s| s.bestaetigen.take())
    });
    if let Some(f) = f {
        f();
    }
}

fn griff_sperre_erneut(id: u64) {
    let f = LAUFEND.with(|l| l.borrow().get(&id).and_then(|s| s.erneut.clone()));
    if let Some(f) = f {
        f();
    }
}

fn neue_id() -> u64 {
    NAECHSTE_ID.with(|n| {
        let mut n = n.borrow_mut();
        let id = *n;
        *n += 1;
        id
    })
}

pub struct LokalerMonitor;

impl LokalerMonitor {
    pub fn new() -> Self {
        LokalerMonitor
    }
}

impl Default for LokalerMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Startet `quelle.ausfuehren` und übersetzt die Fortschrittsmeldungen.
fn ausfuehren(id: u64, vorgang: Vorgang, ids: Vec<String>, quelle: &Rc<dyn Paketquelle>) {
    let melden = LAUFEND.with(|l| l.borrow().get(&id).map(|s| s.melden.clone()));
    let Some(melden) = melden else {
        return;
    };
    let letzter_status = RefCell::new(String::from("Wird vorbereitet …"));
    let letztes_prozent: std::cell::Cell<Option<u8>> = std::cell::Cell::new(None);
    let adapter: Box<dyn Fn(Fortschritt)> = Box::new(move |f| {
        let zustand = match f {
            Fortschritt::Gestartet { .. } => Zustand::Laeuft {
                prozent: None,
                status: letzter_status.borrow().clone(),
            },
            Fortschritt::Prozent(p) => {
                letztes_prozent.set(if p <= 100 { Some(p) } else { None });
                Zustand::Laeuft {
                    prozent: letztes_prozent.get(),
                    status: letzter_status.borrow().clone(),
                }
            }
            Fortschritt::Status(s) => {
                *letzter_status.borrow_mut() = s;
                Zustand::Laeuft {
                    prozent: letztes_prozent.get(),
                    status: letzter_status.borrow().clone(),
                }
            }
            Fortschritt::PaketFortschritt { .. } => return,
            Fortschritt::WartetAufSperre => Zustand::WartetAufSperre {
                seit: std::time::Instant::now(),
            },
            Fortschritt::BrauchtNeustart => Zustand::Laeuft {
                prozent: letztes_prozent.get(),
                status: "Ein Neustart wird nötig sein".into(),
            },
            Fortschritt::Fertig => Zustand::Fertig,
            Fortschritt::Abgebrochen => Zustand::Abgebrochen,
            // Belegte Sperre ist kein Endzustand: die Oberfläche fragt, ob
            // sie warten (`sperre_erneut_versuchen`) oder abbrechen soll.
            Fortschritt::Fehler(Fehler::SperreBelegt(_)) => Zustand::WartetAufSperre {
                seit: std::time::Instant::now(),
            },
            Fortschritt::Fehler(e) => Zustand::Fehlgeschlagen(e),
        };
        let endgueltig = matches!(
            zustand,
            Zustand::Fertig | Zustand::Abgebrochen | Zustand::Fehlgeschlagen(_)
        );
        if endgueltig {
            // Abbruch nach Abschluss ist wirkungslos.
            let schon_beendet = LAUFEND.with(|l| {
                let mut l = l.borrow_mut();
                match l.get_mut(&id) {
                    Some(s) if !s.beendet => {
                        s.beendet = true;
                        s.abbruch = None;
                        false
                    }
                    _ => true,
                }
            });
            if schon_beendet {
                return;
            }
        }
        melden(zustand);
        if endgueltig {
            LAUFEND.with(|l| l.borrow_mut().remove(&id));
        }
    });
    let abbruch = quelle.ausfuehren(vorgang, ids, adapter);
    LAUFEND.with(|l| {
        if let Some(s) = l.borrow_mut().get_mut(&id) {
            if s.beendet {
                abbruch.abbrechen();
            } else {
                s.abbruch = Some(abbruch);
            }
        }
    });
}

impl Transaktionsmonitor for LokalerMonitor {
    fn starten(
        &self,
        auftrag: Auftrag,
        quelle: Rc<dyn Paketquelle>,
        melden: Box<dyn Fn(Zustand)>,
    ) -> TransaktionsGriff {
        let id = neue_id();
        let melden: Rc<dyn Fn(Zustand)> = Rc::from(melden);
        let ids: Vec<String> = auftrag.pakete.iter().map(|p| p.id.clone()).collect();
        let vorgang = auftrag.vorgang.clone();
        // Neustart nach einer belegten Sperre: dieselbe Transaktion noch einmal.
        let erneut: Rc<dyn Fn()> = {
            let quelle = quelle.clone();
            let ids = ids.clone();
            let vorgang = vorgang.clone();
            Rc::new(move || ausfuehren(id, vorgang.clone(), ids.clone(), &quelle))
        };
        LAUFEND.with(|l| {
            l.borrow_mut().insert(
                id,
                Steuerung {
                    abbruch: None,
                    bestaetigen: None,
                    erneut: Some(erneut),
                    melden: melden.clone(),
                    beendet: false,
                },
            )
        });
        if vorgang == Vorgang::Entfernen {
            let ids2 = ids.clone();
            let quelle2 = quelle.clone();
            glib::MainContext::default().spawn_local(async move {
                match quelle2.simulieren(Vorgang::Entfernen, ids2.clone()).await {
                    Ok(sim) => {
                        let noch_da =
                            LAUFEND.with(|l| l.borrow().get(&id).is_some_and(|s| !s.beendet));
                        if !noch_da {
                            return;
                        }
                        let q = quelle2.clone();
                        LAUFEND.with(|l| {
                            if let Some(s) = l.borrow_mut().get_mut(&id) {
                                s.bestaetigen = Some(Box::new(move || {
                                    ausfuehren(id, Vorgang::Entfernen, ids2, &q)
                                }));
                            }
                        });
                        melden(Zustand::Simuliert(sim));
                        melden(Zustand::WartetAufBestaetigung);
                    }
                    Err(e) => {
                        LAUFEND.with(|l| l.borrow_mut().remove(&id));
                        melden(Zustand::Fehlgeschlagen(e));
                    }
                }
            });
        } else {
            ausfuehren(id, vorgang, ids, &quelle);
        }
        TransaktionsGriff::mit_rueckrufen(
            id,
            Rc::new(griff_abbrechen),
            Rc::new(griff_bestaetigen),
            Rc::new(griff_sperre_erneut),
        )
    }

    fn wieder_anbinden(
        &self,
        _quelle: Rc<dyn Paketquelle>,
        _melden: Box<dyn Fn(Zustand)>,
    ) -> Vec<TransaktionsGriff> {
        Vec::new()
    }
}
