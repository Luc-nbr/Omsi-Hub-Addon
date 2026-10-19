// The bus company for the website (omsihub.com): hub-api built for the browser
// (`crates/hub-web`, wasm32-wasip1) and the app's page files (ui/), into dist/web/. The website
// takes that folder as it is (its `company-app/`); see docs/WEB.md.
//
//     node tools/build-web.mjs           (dist/web/)
//     node tools/build-web.mjs <folder>  (another folder, e.g. the website's company-app/)

import { execFileSync } from 'node:child_process';
import { cpSync, mkdirSync, rmSync, writeFileSync, readFileSync, statSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = resolve(process.argv[2] || join(root, 'dist', 'web'));

execFileSync('cargo', ['build', '-p', 'hub-web', '--target', 'wasm32-wasip1', '--profile', 'web'], { cwd: root, stdio: 'inherit' });
const wasm = join(root, 'target', 'wasm32-wasip1', 'web', 'hub_web.wasm');

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
for (const d of ['css', 'fonts', 'img', 'js', 'models', 'vendor']) cpSync(join(root, 'ui', d), join(out, d), { recursive: true });
// (the mock backend and the app's own start stay behind: the website starts web/company.js)
rmSync(join(out, 'js', 'mock'), { recursive: true, force: true });
rmSync(join(out, 'js', 'mock.js'), { force: true });
cpSync(wasm, join(out, 'hub_web.wasm'));
const bytes = readFileSync(wasm);
const build = {
  sha256: createHash('sha256').update(bytes).digest('hex'),
  bytes: statSync(wasm).size,
  commit: (() => { try { return execFileSync('git', ['rev-parse', '--short', 'HEAD'], { cwd: root }).toString().trim(); } catch { return ''; } })(),
};
writeFileSync(join(out, 'build.json'), JSON.stringify(build, null, 1) + '\n');
console.log(`dist: ${out} (hub_web.wasm ${(build.bytes / 1e6).toFixed(1)} MB, ${build.commit})`);
