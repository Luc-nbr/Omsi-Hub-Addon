# Patches proposed to openOMSI

Changes to openOMSI (`openOMSI-org/openomsi`) that Omsi-Hub uses and that are not in an openOMSI
release yet, as `git format-patch` files made on openOMSI's `main`. Apply one with
`git am <file>` in a clone of openOMSI and open a pull request from it.

| Patch | What it adds | Omsi-Hub uses it for |
| --- | --- | --- |
| `0001-Skins-...patch` | **Skins**: a folder plugin's `skin.json` gives the loading screen's logo, colours and tips, the pause menu's colours, name, mark and lines of the plugin's own (the `menu_entry` event), and the navigator's map colours; chosen with `skin=<name>` in `settings.cfg` (docs/PLUGINS.md "Skins") | `plugin/skin.json`: Omsi-Hub's loading screen and pause menu, "My duty" and "Messages" in the pause menu |

Without it the game keeps its own look: `skin=` is a key an older openOMSI does not know and
keeps as it is, and the plugin's `menu_entry` handler is never called.
