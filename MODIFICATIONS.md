# Attribution and changes

This is an unofficial port of [Iron Furnaces by Astryxion](https://github.com/astryxion/Iron-Furnaces), based on its **Fabric 26.3** source at `e1fabfe21abf5b11de1efb2b2f773d5561539d4c`. The original project credits **Astryxion, pizzatime and XenoMustache**. The original Apache-2.0 license and copyright notice are retained in `LICENSE`; `NOTICE` accompanies both source and mod packages.

The Horizon port contributors made the following changes in 2026:

- Reimplemented the Java machines in Rust, including fuel furnaces, electric factories, generators, wireless heating, upgrades, augments, redstone, automation and saved state.
- Added Horizon block, item and menu registrations, material tags and settings labels. Adapted recipe loader conditions and loot definitions. Changed upstream JSON files carry a `_comment` change notice. Original textures, models and unmodified resources retain their original bytes.
- Added automated rule, resource, gameplay and interface checks, packaging helpers and compatibility documentation.
- Migrated the native implementation from the old paired content/native packages to Horizon's typed mod API, transaction-aware item and energy capabilities, chunk tickets and unified source `.hmod` packaging. Narrow function-replacement hooks provide menu layout, redstone and removal behavior not exposed by the typed events used here.

This repository does not contain the Java mod's compiled JAR, Horizon's game/SDK binaries, or Minecraft game files. Horizon is an independent LGPL-3.0-or-later project at the new API revision. Its license applies to its separately obtained sources and binaries. No endorsement by Astryxion, the other original contributors, Horizon, Mojang or Microsoft is claimed.

The original mod's optional Silver and Allthemodium recipes still need their corresponding material mods/tags; this port does not replace those projects or their assets.
