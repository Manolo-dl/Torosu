# Third-party notices

Torosu itself is licensed under the MIT License (see `LICENSE`). It includes
the following third-party software, under its own license.

## osu-memory-reader

- Version: 0.2.1 (<https://crates.io/crates/osu-memory-reader>)
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

## dan-calc

- Version: commit `1edb4c3` of <https://github.com/aleju03/mania-hub> (`algorithms/dan-calc`)
- Copyright (c) 2026 aleju03
- License: MIT

Torosu uses it to estimate the dan and the MSD of mania maps, the one selected in
osu! and the converts of the trainer. It is statically linked into the Torosu
executable, and includes:

- Estimators ported from
  [LeoBlackMT/osumania_map_analyser](https://github.com/LeoBlackMT/osumania_map_analyser),
  Copyright (c) 2026 Leo_Black, MIT License.
- MinaCalc compiled to WebAssembly, from [Etterna](https://github.com/etternagame/etterna),
  Copyright (c) 2016-2023 Etterna, MIT License.

### Dan badges

The dan badges in `src/assets/images/dans/` (and `assets/images/dans/` in the
installed app) are Mania Tracker's, from `public/images/dans/` in
<https://github.com/aleju03/mania-hub> (commit `3628364`), and are used with
the permission of its author, aleju03. They are drawn after the marks of the
dan courses they stand for: the 4K reform and LN dans, JinJin's 7K dans, and
Arkman's and sunnyxxy's 6K dans. They are not covered by Torosu's MIT License.

The MIT License of each of them reads:

> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.
