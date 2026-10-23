# Paketquellen für Programme ohne Ubuntu-Paket

Ubuntu 26.04 liefert einige Programme nur noch als Snap aus. Die Pakete
`firefox` und `thunderbird` im Ubuntu-Archiv sind dort Übergangspakete
(„Snap-Hüllen“, Versionen `1:1snap1-…` und `2:1snap1-…`), die bei der
Installation `snapd` und den Snap nachziehen. Querencia Desktop verwendet keine
Snaps und holt diese Programme deshalb als normale Debian-Pakete aus den
Quellen unten. Diese Quellen liefern keine AppStream-Daten mit; die
Beschreibungen im Software-Center kommen aus den Katalog-Ergänzungen
(`katalog-ergaenzungen.md`).

Querencia Software blendet Snap-Hüllen aus: Eine nicht installierte Hülle gilt
nicht als verfügbare Fassung, eine Aktualisierung auf eine Hülle wird nicht
angeboten. Fehlt die Ersatzquelle, zeigt der Eintrag „Nicht verfügbar“ und der
Knopf bleibt gesperrt, statt den Snap über die Hülle zu installieren.

## Mozilla Team (Launchpad-PPA) – Thunderbird

| Was | Wert |
|---|---|
| Zweck | Thunderbird (Fassung mit verlängerter Unterstützung, ESR) als Debian-Paket |
| Adresse | `https://ppa.launchpadcontent.net/mozillateam/ppa/ubuntu` |
| Suite, Komponente | `resolute`, `main` |
| Schlüssel | RSA 4096, „Launchpad PPA for Mozilla Team“, Fingerabdruck `738B EB93 21D1 AAEC 13EA 9391 AEBD F481 9BE2 1867` (Schlüssel-ID `AEBDF4819BE21867`); der ältere RSA-1024-Schlüssel der PPA wird von apt abgelehnt |
| Schlüsselablage | `/etc/apt/keyrings/mozillateam-ppa.asc`, eingebunden über `Signed-By` |
| Pin | `Package: thunderbird*`, `Pin: release o=LP-PPA-mozillateam`, `Pin-Priority: 1001` |
| AppStream-Daten | keine (das Verzeichnis `dep11/` der PPA fehlt) |

Der Pin ist nötig: Die Snap-Hülle im Ubuntu-Archiv trägt die Epoche `2:` und
gewinnt sonst gegen das Paket der PPA (Epoche `1:`). Die Priorität über 1000
erlaubt auch den Wechsel von einer schon installierten Hülle zurück auf das
Paket der PPA. Der Pin gilt nur für `thunderbird*`; die PPA bietet auch
Firefox an, Firefox bleibt aber bei der Mozilla-Quelle unten.

Einrichtung als Quelle im deb822-Format
(`/etc/apt/sources.list.d/mozillateam-ppa.sources`):

```text
Types: deb
URIs: https://ppa.launchpadcontent.net/mozillateam/ppa/ubuntu
Suites: resolute
Components: main
Signed-By: /etc/apt/keyrings/mozillateam-ppa.asc
```

und als Pin (`/etc/apt/preferences.d/mozillateam-thunderbird`):

```text
Package: thunderbird*
Pin: release o=LP-PPA-mozillateam
Pin-Priority: 1001
```

**Stand:** Die Quelle gehört ins Metapaket `querencia-desktop`
(Packaging-Folgepunkt, zusammen mit der Katalog-Ergänzung im Paket
`querencia-software`). Sie ist dort **noch nicht enthalten**.

## Mozilla (packages.mozilla.org) – Firefox

| Was | Wert |
|---|---|
| Zweck | Firefox als Debian-Paket direkt von Mozilla |
| Adresse | `https://packages.mozilla.org/apt` |
| Suite, Komponente | `mozilla`, `main` |
| Schlüsselablage | `/etc/apt/keyrings/packages.mozilla.org.asc`, eingebunden über `Signed-By` |
| Pin | `Package: *`, `Pin: origin packages.mozilla.org`, `Pin-Priority: 1000` |
| AppStream-Daten | keine |

**Stand:** wie oben noch nicht im Metapaket enthalten.
