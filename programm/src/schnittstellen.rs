//! Die drei Traits, über die die Oberfläche mit den Backends spricht, und
//! die zugehörigen Typen. Alle stammen aus `qs_typen` (SW-1, nach
//! schnittstellen.md); die übrigen Module importieren nur von hier, damit
//! die Oberfläche an genau einer Stelle an die Schnittstelle gebunden ist.

pub use qs_typen::{
    Abbruch, Auftrag, Katalog, Paketquelle, Simulation, Statistik, TransaktionsGriff,
    Transaktionsmonitor, Zustand,
};
