## Overview

gdse is a simple, fully-automatic alternative to the popular [Rainbow
Filter](https://forums.crateentertainment.com/t/tool-rainbow-filter-item-highlighting/42765/917) mod
for the action RPG Grim Dawn, created by [Crate Entertainment](https://www.crateentertainment.com/).
It recolors game text for items in two ways:

1. Separate coloring for affix rarity and base item rarity, for Common, Magic, and Rare items that
   roll affixes.
2. Colored text in item tooltips for different damage types.

<img width="311" height="212" alt="image" src="https://github.com/user-attachments/assets/86f26fd2-0a83-4963-bc63-bb6cca7d3681" />

These are the two features of rainbow filter that the author of gdse finds essential and can't play
Grim Dawn without. All other features of Rainbow Filter are omitted.

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

There are not currently any precompiled binaries available. You must build from source. To use gdse,
you can:

1. [Install rust](https://www.rust-lang.org/tools/install)
2. `git clone git@github.com:gregates/gdse.git`
3. `cd gdse`
4. `cargo run --release`

You must set GRIM_DAWN_INSTALL_PATH environment to the path to your game installation for the
program to work. By default, it will write the modified text resources to
`$GRIM_DAWN_INSTALL_PATH/settings/`. Re-run any time Grim Dawn gets patched, if you see any missing
tag errors or incorrect colorings.
