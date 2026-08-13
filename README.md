## Overview

gdse is a simple, fully-automatic alternative to the popular [Rainbow
Filter](https://forums.crateentertainment.com/t/tool-rainbow-filter-item-highlighting/42765/917) mod
for the action RPG Grim Dawn, created by [Crate Entertainment](https://www.crateentertainment.com/).
It recolors game text for items in two ways:

1. Separate coloring for affix rarity and base item rarity, for Common, Magic, and Rare items that
   roll affixes.
2. Colored text in item tooltips for different damage types.

<img width="375" height="238" alt="image" src="https://github.com/user-attachments/assets/2cad86a0-da01-4817-90d8-53452dd20def" />

These are the two features of rainbow filter that the author of gdse finds essential and can't play
Grim Dawn without. All other features of Rainbow Filter are omitted. One other small difference from
Rainbow Filter is that gdse colors Pierce damage pink, not red, to distinguish it from Bleed (you can
opt-in to Rainbow Filter's exact damage color scheme if you prefer it with the
`--rainbow-filter-damage-colors` flag).

gdse works by writing modified copies of some game resources to the `/settings/` directory of your
Grim Dawn installation, just like Rainbow Filter. It's not compatible with Rainbow Filter or other
mods that change game text. If new text is added to the game by a patch, you may see missing tag
warnings until you re-run gdse.

## Why not just use Rainbow Filter?

Great question! I used Rainbow Filter for years. It's a great mod. You should continue to use it if
it works for you.

The main motivation for gdse was that Rainbow Filter depends on hand-crafted configuration that
needs to be updated after every game patch. That gives Rainbow Filter a lot of power and
flexibility, but it also causes delays — it takes time to produce a new Rainbow Filter mod after
each patch.

gdse depends on zero manually configured colors. Instead, it infers correct coloring based on game
database files and a few simple heuristics. Therefore, when a new patch lands, all you need to do to
get updated, recolored text files is rerun gdse. No waiting for me to release a new version in the
typical case (major changes to the game may require updates to handle new game features, but it
should mostly just work).

Additionally, I find many of Rainbow Filter's choices to be a little too much. I don't need an (S)
to get inserted on every set item, or MIs to have a special color, or "Empowered" and "Mythical" to
be a different color than the base item name, or for "Physique" to be colored pink. Your preferences
may not match mine, and that's OK! One of Rainbow Filter's strengths is that it's highly
customizable (it just takes a lot of effort to customize). gdse is simpler and more opinionated.
This can be a virtue if you don't strongly disagree with its opinions.

## Installation & Usage

gdse has been tested with game version 1.3.0 on linux only, and only for
English text files, but in theory it should work on windows and with localizations.

There are not currently any precompiled binaries available. You must build from source.

### Quick Start (Windows, PowerShell)

1. Install Rust (choose one option):

   Option A (recommended, PowerShell or cmd with winget):

   ```powershell
   winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
   ```

   Option B (manual installer from rust-lang.org):
   - Go to https://www.rust-lang.org/tools/install
   - Download and run `rustup-init.exe`
   - Keep the default installation settings unless you have a specific reason to change them

2. Open a new terminal, then verify tools are available wtih these commands:

   ```powershell
   rustc --version
   cargo --version
   ```

3. Find your Grim Dawn install path in Steam:
   - Open Steam library.
   - Right-click Grim Dawn -> Manage -> Browse local files.
   - Copy the folder path from the file explorer address bar.

4. Set the required environment variable (replace the example path with your own):

   ```powershell
   setx GRIM_DAWN_INSTALL_PATH "C:\Program Files (x86)\Steam\steamapps\common\Grim Dawn"
   ```

5. Clone and run from the repo root:

   When you clone any repo (including gdse), you choose a local folder on your PC where it will be saved.
   You must `cd` into that exact local folder before running cargo commands.
   `C:\repos` below is only an example location:

   ```powershell
   cd C:\repos
   git clone git@github.com:gregates/gdse.git
   cd C:\repos\gdse
   cargo run --release
   ```

   gdse will show whether it detected changes since your last run, then prompt:
   `Proceed with recoloring run? [Y/N]`.

   If you cloned somewhere else, replace `C:\repos\gdse` with your own full path.
   Example: if you cloned into Downloads, use `cd C:\Users\<your-username>\Downloads\gdse`.

Important: the Grim Dawn install path above is only an example. Use the actual path from your own installation.

### Quick Start (Linux)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version
cargo --version
export GRIM_DAWN_INSTALL_PATH="/path/to/Grim Dawn"
git clone git@github.com:gregates/gdse.git
cd /path/where/you/cloned/gdse
cargo run --release
```

gdse will show whether it detected changes since your last run, then prompt:
`Proceed with recoloring run? [Y/N]`.

### Environment Variable Requirement

`GRIM_DAWN_INSTALL_PATH` is required. gdse writes modified text resources to:

`$GRIM_DAWN_INSTALL_PATH/settings/`

Re-run gdse after game patches if you see missing tags or incorrect colors.

To make this easier to track, gdse also writes `gdse-db-hash.txt` in the output folder.
Each run of `cargo run --release` appends one line in this format (local computer time, minute precision):

`<YYYY-MM-DD HH:MM> hash=<hash> steam_build_id=<build_id_or_unknown> patch_versions=<version_list_or_unknown>`

`patch_versions` is inferred from release-marker comments found in game text (for example, lines starting with `#Patch`, `#Hotfix`, or `#Update`).

### Common Windows Issues

- `rustc` or `cargo` not found:
  - Close and reopen your terminal after installing Rust.
  - Confirm `%USERPROFILE%\\.cargo\\bin` is in your PATH.
- linker error about `link.exe` not found:
  - Install Visual Studio Build Tools with the "Desktop development with C++" workload.
- `GRIM_DAWN_INSTALL_PATH` missing or wrong:
  - Print it with `echo $env:GRIM_DAWN_INSTALL_PATH` in PowerShell.
  - Update it with `setx GRIM_DAWN_INSTALL_PATH "<your path>"`.

### Notes for AI Coding Agents

If you are using Copilot, Claude Code, Codex, or another coding agent:

1. Run commands from the repository root so Cargo can find `Cargo.toml`.
2. Check `GRIM_DAWN_INSTALL_PATH` before running:

   ```powershell
   echo $env:GRIM_DAWN_INSTALL_PATH
   ```

3. Prefer this command order when validating changes:

   ```powershell
   cargo build --release
   cargo run --release
   ```

4. Do not run destructive git commands on a user's repo (for example, `git reset --hard`).
