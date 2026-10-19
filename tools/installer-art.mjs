// The installers' pictures (tools/installer-art.html) rendered into app/installer/:
//   nsis-sidebar.bmp  164 x 314, the Windows installer's welcome and finish pages
//   nsis-header.bmp   150 x 57, beside the other pages' titles
//   dmg-background.png 660 x 400, the macOS disk image's window
// Each is drawn at twice its size and scaled down (smooth edges); NSIS takes 24-bit BMPs only.
//
//   node tools/installer-art.mjs
//
// Needs Playwright (its Chromium) and Python with Pillow, as the brand tools do. The pictures are
// committed: the release builds do not draw them.

import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const out = join(here, 'app', 'installer');
mkdirSync(out, { recursive: true });

const require = createRequire(import.meta.url);
let playwright;
try {
  playwright = require('playwright');
} catch {
  playwright = require(join(execFileSync('npm', ['root', '-g']).toString().trim(), 'playwright'));
}

const SCENES = [
  ['sidebar', 164, 314, 'nsis-sidebar.bmp'],
  ['header', 150, 57, 'nsis-header.bmp'],
  ['dmg', 660, 400, 'dmg-background.png'],
];

const browser = await playwright.chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: 2 });
const src = pathToFileURL(join(here, 'tools', 'installer-art.html')).href;
for (const [scene, w, h, file] of SCENES) {
  await page.setViewportSize({ width: w, height: h });
  await page.goto(`${src}?s=${scene}`);
  await page.evaluate(() => document.fonts.ready);
  const big = join(out, `.${scene}@2x.png`);
  await page.screenshot({ path: big, clip: { x: 0, y: 0, width: w, height: h } });
  // (down to its size; a BMP without alpha, as NSIS reads it)
  const py = `from PIL import Image\nimport os\ni=Image.open(${JSON.stringify(big)}).convert('RGB').resize((${w},${h}),Image.LANCZOS)\ni.save(${JSON.stringify(join(out, file))})\nos.remove(${JSON.stringify(big)})`;
  execFileSync('python3', ['-c', py]);
  console.log(`${file}  ${w} x ${h}`);
}
await browser.close();
