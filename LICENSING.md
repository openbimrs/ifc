# Licensing

## Current license

Unless a file or package states otherwise, repository-authored software and
documentation are dual-licensed: `AGPL-3.0-or-later OR LicenseRef-Commercial`.
Recipients choose either:

- the **GNU Affero General Public License, version 3 or any later version**
  (`AGPL-3.0-or-later`); see [`LICENSE`](LICENSE); or
- a **commercial licence** from the copyright holder named in
  [`NOTICE`](NOTICE); see
  [`LICENSES/LicenseRef-Commercial.txt`](LICENSES/LicenseRef-Commercial.txt).
  That file grants no rights by itself; a commercial licence exists only by
  separate written agreement.

Contributions are accepted under the terms in [`CONTRIBUTING.md`](CONTRIBUTING.md),
which keep the commercial option possible.

Section 13 of the AGPL requires an operator of a modified version that supports
remote network interaction to offer the Corresponding Source prominently to
users who interact with it remotely. The license does not require private
modifications to be published or changes to be submitted upstream.

## Historical MIT terms

Repository-authored work in history before the commit that first replaced the
top-level MIT license with the GNU AGPL was published under the MIT License. Those grants
remain valid for the versions in which they were made. Verbatim copies of the
former notices are preserved in `LICENSES/MIT.txt` and package-local
`LICENSES/MIT.txt` files for attribution and compliance. Their preservation does
not license post-transition changes under MIT.

## Third-party material

Dependencies, standards, schemas, catalogs, fixtures, generated material, and
other third-party content retain their own copyright and license terms. Their
notices control where they differ from this repository's license. The OpenBIM.rs
license grant does not cover material its contributors do not have the right to
license.

### IFC schema-derived data

`crates/ifc-schema/data/` and `crates/ifc-geometry/data/` hold tables derived
from the buildingSMART IFC EXPRESS schemas (CC BY-ND 4.0) -- IFC2X3 TC1,
IFC4 ADD2 TC1, IFC4.1 FINAL, IFC4.2 FINAL and IFC4.3 ADD2: names, attribute
order and types, and rule labels, plus this repository's own analysis. They
contain no schema text. The schemas themselves are fetched, never committed.
See the `NOTICE.md` in each directory.

### IFC template catalog

The `ifc-template-catalog` package contains an official catalog artifact under
`CC-BY-ND-4.0`. Its Rust code and OpenBIM.rs correction overlays are covered by
`AGPL-3.0-or-later OR LicenseRef-Commercial`; they remain separate from the unmodified official artifact.
See `crates/ifc-template-catalog/data/NOTICE.md` and the package-local license files.

## Contributions

Contributions are accepted under the contributor terms in
[`CONTRIBUTING.md`](CONTRIBUTING.md), which license them under
`AGPL-3.0-or-later` and grant the copyright holder the rights the commercial
option needs.
