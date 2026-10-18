# Bauen

Ubuntu 26.04: `cargo`, `rustc` (≥ 1.92, Ubuntu 1.93.1), `pkgconf`,
`libgtk-4-dev`. `Cargo.lock` stammt vom Querencia Rechner (gtk4 0.11.5,
glib 0.22.10) plus `async-trait 0.1.92`; alles aus dem lokalen Cache:

```bash
cargo build --release --locked --offline
```

`qs_typen` wird per `path` aus `../querencia-software-paketkit/qs_typen`
(SW-1) eingebunden. **Nicht `cargo fmt` aufrufen** (auch ohne `--all`): es
formatiert die Pfad-Abhängigkeit im fremden Repository mit. Stattdessen:

```bash
rustfmt --edition 2021 src/main.rs            # bzw. --check
```

Das Release-Binary liegt unter `target/release/querencia-software`; seine
Prüfsumme steht in `nachweis/release/SHA256SUMS`.

**Symbol:** nur ein Platzhalter-Fenstersymbol als SVG-Ressource
(`org.querencia.Software-fenster`, `werkzeug/ressourcen.sh` erzeugt
`data/resource/software.gresource`). Keine PNG-Stufen: liegt ein Symbol in
`48x48` und in Scalable, stürzt GTK 4.22 beim Schließen fokussierter Dialoge
ab. Das endgültige Symbol liefert SW-5.
