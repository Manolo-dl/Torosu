# Third-party notices

Torosu itself is licensed under the MIT License (see `LICENSE`). It includes
the following third-party software, under its own license.

## osu-memory-reader

- Version: 0.1.0 (<https://crates.io/crates/osu-memory-reader>)
- Source: <https://github.com/Manolo-dl/Osu-memory-reader>
- Copyright (C) 2026 Manolo-dl
- License: GNU Lesser General Public License version 3 (LGPL-3.0-only)

Torosu uses it to read the beatmap selected in osu! stable. It is a modified
version of parts of [tosu](https://github.com/tosuapp/tosu) 4.26.3, Copyright
(C) 2023-2026 Mikhail Babynichev and contributors, also licensed under the
LGPL-3.0.

The library is statically linked into the Torosu executable. Its license
texts and notice are distributed with Torosu, in
`licenses/osu-memory-reader/` inside the installation folder (and in
`src-tauri/licenses/osu-memory-reader/` in the source code):

- `COPYING.LESSER`: the GNU Lesser General Public License version 3.
- `COPYING`: the GNU General Public License version 3, which the LGPL builds on.
- `NOTICE`: where each file of the library comes from and what was changed.

### Using a modified version of the library

The LGPL lets you replace the library with your own modified version. The
complete source code of Torosu is available at
<https://github.com/Manolo-dl/Torosu>. To build Torosu with a modified
copy of osu-memory-reader:

1. Clone Torosu and the library, for example next to each other.
2. Make your changes to the library.
3. Add this to `src-tauri/Cargo.toml` in Torosu:

   ```toml
   [patch.crates-io]
   osu-memory-reader = { path = "../../Osu-memory-reader" }
   ```

4. Build Torosu with `pnpm install` and `pnpm tauri build`.
