# Third-party provenance

`data/scalars.json`, `data/monitors.json`, `data/devices.json` and `data/features.json` record source-derived setting names, types, literal bounds, default expressions, interfaces and factual audit results. Their source URLs point to exact Hyprland commits. No upstream implementation code is compiled into this project. Documentation contains original audit/engineering analysis and links to upstream materials.

Hyprland is an independent project by its contributors, distributed under BSD-3-Clause; its wiki has its own licensing terms. This project is not affiliated with or endorsed by Hyprland. Source-derived default-expression fragments and interface names are retained only as metadata for interoperability. Upstream material remains subject to its original license; the MIT project license does not relicense upstream code or documentation.

Rust dependencies and GTK/libadwaita are independently licensed. The exact Rust dependency graph is recorded in Cargo.lock. Distributors must review and include the applicable dependency and platform notices for their package. No bundled system libraries or binary redistributions are shipped in this source repository.

The pinned Tree-sitter runtime (0.27.1) and Lua grammar (0.5.0) are MIT licensed dependencies; SHA-2 (0.10.9) is dual licensed MIT or Apache-2.0. Their sources are fetched by Cargo and not vendored into this repository. Cargo.lock records their integrity checksums and transitive dependencies.

The pinned quick-xml parser (0.38.4) is an MIT licensed dependency. Its checksum is recorded in Cargo.lock. Installed XKB registry choices are read locally on explicit request and are not redistributed; the repository includes only a small original synthetic XML fixture for tests. The keyboard registry parser does not resolve external DTDs/entities or compile a keymap.
