# Bauen

*Seit 0.1.1 unter `docs/` (Doku-Standard, ENTSCHEIDUNGEN Nr. 17); Pfade wie `werkzeug/…`, `data/…`, `src/…` gelten relativ zu `programm/`.*

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

**Symbole:** zwei SVG-Ressourcen unter `/org/querencia/Software/icons`
(`werkzeug/ressourcen.sh` erzeugt `data/resource/software.gresource`):
`org.querencia.Software-fenster` ist das Programmsymbol von SW-5 (byte-gleich
mit `software-paket/0.1.1`, Prüfsumme in `data/resource/HERKUNFT`), benutzt
als Fenstersymbol und im Info-Dialog; `org.querencia.Software-paket` ist das
graue Ersatzsymbol für Anwendungen ohne Symbol. Keine PNG-Stufen: liegt ein
Symbol in `48x48` und in Scalable, stürzt GTK 4.22 beim Schließen
fokussierter Dialoge ab. `xmllint` fehlt auf der VM, daher ohne
`xml-stripblanks`.
