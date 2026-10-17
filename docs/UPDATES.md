# The app's updates

Omsi-Hub updates itself from this repository's GitHub releases
(`github.com/Luc-nbr/Omsi-Hub-Addon`). Nothing of openOMSI's updater is used.

## How it works

There are two halves.

**The look** (`crates/hub-api/src/update.rs`, ours):

- It asks GitHub's API for the releases a second after the start, every half hour, and when a
  game started from Omsi-Hub has ended.
- It skips drafts, pre-releases, and tags with a `-` in them (`v0.3.0-beta1`).
- It compares the versions by their numbers (`0.2.10` > `0.2.9`) and offers the newest one
  above the running version.
- A test build (a version with a `-`) is never offered anything.
- It finds the platform's asset (the table below): the NSIS installer on Windows, the
  `.app.tar.gz` on macOS, the `.AppImage` on Linux.
- The page shows the offer in a dialog (`ui/js/update.js`): Update now, Not now, What's new, and
  "Install updates without asking".
- Settings has "Check now", "Look for updates" (`update_check`) and "Install updates without
  asking" (`update_auto`), kept in `settings.json`.
- `OMSIHUB_NO_UPDATE=1` switches the looks off.

**The installation** (`app/src/main.rs`, Tauri's `tauri-plugin-updater`):

- "Update now" makes the plugin read the newest release's `latest.json`
  (`releases/latest/download/latest.json`).
- It downloads the asset that file names and checks its signature against the public key in
  `app/tauri.conf.json` (`plugins.updater.pubkey`).
- Then it installs the new version and starts it:
  - On Windows, the NSIS installer runs in passive mode. It ends Omsi-Hub and starts the new
    version itself.
  - On macOS, the `.app` bundle is replaced and the app restarts.
  - On Linux, the AppImage file the player started (`$APPIMAGE`) is replaced and the app
    restarts. Only an AppImage updates itself: a copy installed from a `.deb` or `.rpm` (or built by hand)
    is offered the release's page instead.
- Nothing is installed while a game runs.

Until the public key is in `tauri.conf.json`, the app does not use the plugin. "Update now"
then becomes "Download from GitHub", which opens the release's page. The same happens for a
release without a `latest.json`.

## The release files

| Platform | `latest.json` key | Updater file (signed: `<file>.sig` beside it) | For a first download |
|---|---|---|---|
| Windows x64 | `windows-x86_64` | `Omsi-Hub_<version>_x64-setup.exe` | the same |
| macOS Apple silicon | `darwin-aarch64` | `Omsi-Hub_aarch64.app.tar.gz` | `Omsi-Hub_<version>_aarch64.dmg` |
| macOS Intel | `darwin-x86_64` | `Omsi-Hub_x64.app.tar.gz` | `Omsi-Hub_<version>_x64.dmg` |
| Linux x64 | `linux-x86_64` | `Omsi-Hub_<version>_amd64.AppImage` | the same, or the `.deb` / `.rpm` (not self-updating) |
| Linux ARM64 | `linux-aarch64` | `Omsi-Hub_<version>_aarch64.AppImage` | the same |

The names are Tauri's bundler's, from `productName` (`Omsi-Hub`) and the version. The look
matches the processor in the name: `x64`, `amd64` or `x86_64`; `aarch64` or `arm64`; a macOS
`universal` bundle when there is no own one.

## What Luc sets up (once)

0. **The releases must be public.** The app reads them without signing in. The repository is
   private now (2026-10-17), so GitHub answers 404 and every look ends in "The last check
   failed". Make the repository public before the first release, or publish the releases
   somewhere public and change `API` and `LATEST_JSON` in `update.rs` and the endpoint in
   `tauri.conf.json`.

1. **Make the signing key**, on your own computer:

   ```
   npx @tauri-apps/cli@2 signer generate -w ~/.tauri/omsi-hub-updater.key
   ```

   Choose a password. This makes `omsi-hub-updater.key` (private: never commit it) and
   `omsi-hub-updater.key.pub` (public). Keep the private key and its password somewhere safe. If
   they are lost, installed copies can no longer update themselves: players would have to
   download the next version by hand.

2. **Put the public key in the app.** Paste the contents of `omsi-hub-updater.key.pub` into
   `app/tauri.conf.json`, `plugins.updater.pubkey`, and commit it. It is public by design.

3. **Add two secrets to the repository** (GitHub → Settings → Secrets and variables → Actions):
   - `TAURI_SIGNING_PRIVATE_KEY`: the contents of `omsi-hub-updater.key`;
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password.

4. **Make the release workflow sign the updater files and publish `latest.json`.** CI's own
   `tauri build --no-bundle` stays as it is: it needs no key. The release build adds
   `createUpdaterArtifacts`, with the two secrets in its environment. With `tauri-action` that
   is:

   ```yaml
   - uses: tauri-apps/tauri-action@v0
     env:
       GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
       TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
       TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
     with:
       projectPath: app
       tagName: v__VERSION__
       releaseName: Omsi-Hub __VERSION__
       releaseDraft: true
       prerelease: false
       includeUpdaterJson: true
       args: --config '{"bundle":{"createUpdaterArtifacts":true}}'
   ```

   Run it on `windows-latest`, `macos-latest` and `ubuntu-22.04`:
   - On macOS, use `--target aarch64-apple-darwin` and `--target x86_64-apple-darwin`, or
     `universal-apple-darwin`.
   - On Linux, the bundle targets are `appimage`, `deb` and `rpm` (`app/tauri.linux.conf.json`). The runner needs
     Tauri's packages first: `sudo apt-get install -y libwebkit2gtk-4.1-dev
     libayatana-appindicator3-dev librsvg2-dev patchelf`. Build on the oldest Ubuntu you
     support: an AppImage runs on the same or newer glibc only.

   Each run uploads its installer, its `.sig` and its part of `latest.json` to the same draft
   release.

5. **Publish each release** as a normal release: not a draft, not a pre-release. Its tag is
   `v<version>`, and the version is the workspace's (`Cargo.toml`, `[workspace.package]
   version`) and `app/tauri.conf.json`'s. A draft or a pre-release is never offered. That makes
   a pre-release the place for a test build.

`latest.json`, if you ever write it by hand:

```json
{
  "version": "0.2.0",
  "notes": "What changed",
  "pub_date": "2026-11-01T12:00:00Z",
  "platforms": {
    "windows-x86_64": { "signature": "<contents of Omsi-Hub_0.2.0_x64-setup.exe.sig>", "url": "https://github.com/Luc-nbr/Omsi-Hub-Addon/releases/download/v0.2.0/Omsi-Hub_0.2.0_x64-setup.exe" },
    "darwin-aarch64": { "signature": "<contents of Omsi-Hub_aarch64.app.tar.gz.sig>", "url": "https://github.com/Luc-nbr/Omsi-Hub-Addon/releases/download/v0.2.0/Omsi-Hub_aarch64.app.tar.gz" },
    "darwin-x86_64": { "signature": "<contents of Omsi-Hub_x64.app.tar.gz.sig>", "url": "https://github.com/Luc-nbr/Omsi-Hub-Addon/releases/download/v0.2.0/Omsi-Hub_x64.app.tar.gz" },
    "linux-x86_64": { "signature": "<contents of Omsi-Hub_0.2.0_amd64.AppImage.sig>", "url": "https://github.com/Luc-nbr/Omsi-Hub-Addon/releases/download/v0.2.0/Omsi-Hub_0.2.0_amd64.AppImage" }
  }
}
```

## Still to check on the first real release

- That an AppImage replaces itself and starts again on a Linux desktop. The look and the asset
  choice are tested; the replacement is the plugin's and has not been tried yet.
- That the NSIS installer starts the new version after a passive update on Windows.
- That macOS Gatekeeper lets the replaced bundle start. Without an Apple Developer ID
  signature and notarization, the first start after a manual download already needs
  right-click → Open. An update keeps the bundle's quarantine state, but this has not been tried
  yet.
