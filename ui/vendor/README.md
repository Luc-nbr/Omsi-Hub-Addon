# Third-party files of the page

Copied from their packages (Preact from the old Omsi-Hub launcher's `vendor/`); nothing is fetched
at run time. The 3D view draws with the page's own WebGL2 renderer (`../js/hub/bus3d/`).

| File | Package | Licence |
|---|---|---|
| `browser_wasi_shim/` | `@bjorn3/browser_wasi_shim@0.4.2` (`dist/`): WASI's calls in the browser, its folders in memory, for the website's bus company (`../js/web/backend.js`) | MIT (`browser_wasi_shim/LICENSE-MIT`; also Apache-2.0) |
| `preact-htm.js` | `htm@3.1.1` (`preact/standalone.module.js`: Preact 10 with its hooks, and htm) | MIT (Preact, `preact-LICENSE.txt`), Apache-2.0 (htm, `htm-LICENSE.txt`) |

The font, Hanken Grotesk (`../fonts/`), is under the SIL Open Font Licence 1.1
(`../fonts/HankenGrotesk-OFL.txt`).
