# Third-party files of the page

Copied from their packages (Preact from the old Omsi-Hub launcher's `vendor/`, three.js bundled as
below); nothing is fetched at run time.

| File | Package | Licence |
|---|---|---|
| `preact-htm.js` | `htm@3.1.1` (`preact/standalone.module.js`: Preact 10 with its hooks, and htm) | MIT (Preact, `preact-LICENSE.txt`), Apache-2.0 (htm, `htm-LICENSE.txt`) |
| `three.js` | `three@0.186.1`: what the 3D bus viewer takes of it (scene, lights, `GLTFLoader`, `OrbitControls`), bundled by esbuild from `tools/three-entry.js` (the command is there) | MIT (`three-LICENSE.txt`) |

The font, Hanken Grotesk (`../fonts/`), is under the SIL Open Font Licence 1.1
(`../fonts/HankenGrotesk-OFL.txt`).
