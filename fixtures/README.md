# Test fixtures

Fake folders for the first-run setup's tests (`crates/hub-api/tests/setup.rs`, and the app's
selftest with `OMSIHUB_SELFTEST_FIXTURES`). No OMSI 2 or openOMSI content is in this repository:
the "programs" here are one line of text each and are never run.

- `openOMSI-0.2.30-windows-x64/` - an openOMSI release unpacked: its version comes from the
  folder's name (the stand-in has no version resource). Supported.
- `openOMSI-0.2.10-windows-x64/` - one older than 0.2.22: refused, with the message to update.
- `OMSI 2/` - an OMSI 2 folder: `Omsi.exe`, `maps`, `Vehicles`, `Sceneryobjects`, `Splines`.

Checking a folder may make openOMSI's `Plugins` folder in it (as it would beside a real
openOMSI); `.gitignore` keeps that out.
