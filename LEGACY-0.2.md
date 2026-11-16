This documents **0.2.0 for Horizon's older modding API**, retained for a separate GitHub release alongside 0.3.1. See [README.md](README.md#choose-your-version) to choose the matching version. Package paths and build hashes below describe archived local tests; the 0.2 packages are not yet uploaded as their own GitHub release.

# Iron Furnaces for Horizon

A Rust port of [Astryxion's Iron Furnaces](https://github.com/astryxion/Iron-Furnaces), with the original textures, models and crafting progression. This repository targets **Minecraft 26.3 / Horizon Nightly 2026-11-13**, revision `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`.

The port has two required parts: a content mod (`.hmod`) that registers the blocks, items and recipes, and a native Rust mod (`.hnmod`) that implements the machines. Installing only the content does not install the faster-furnace behavior.

Compatibility checked 16 November: this release still passes on the installed/latest published Nightly. Horizon's newly merged development API changes native packaging and integration; this release is not compatible with that new build system. See the [compatibility audit](COMPATIBILITY-2026-11-16.md) before moving off the pinned game.

Version 0.2 adds electric factories, generators, wireless heating, per-side automation, redstone controls, settings copying, seasonal skins and Rainbow Coal to the furnace core. The Windows build is installed in the **IRON FURNACE PORT** instance. Rules, content, gameplay, survival drops and rendered interface checks passed; [VALIDATION.md](VALIDATION.md) records their results, executable provenance and remaining compatibility limits.

Use a matching upgrade item on a placed furnace to change its tier while keeping its inventory, energy and settings. Ordinary use opens its machine screen. The **+** button opens the augment page: red selects smoking or blasting, green selects speed or fuel efficiency, and blue selects an electric factory or a generator. Remove an augment from its slot to remove its effect. An empty blue slot selects the ordinary fuel furnace. Augments can also be installed by using their item on the block.

The speed augment halves cooking time and uses twice as much fuel per operation. The fuel-efficiency augment increases cooking time by 25% and doubles operations per fuel item. Timings round to whole ticks. Changing augments clears current burning and cooking progress. Changing the machine mode drops incompatible stored ingredients and results beside the furnace; changing a generator's fuel mode also clears its fuel inventory. Collect those items before leaving.

## Install

1. Use a separate Horizon launcher instance on the matching Nightly.
2. Put `dist/ironfurnaces-0.2.0.hmod` in that instance's `horizon-mods` folder and enable Iron Furnaces.
3. Put `dist/ironfurnaces-0.2.0.hnmod` in that instance's `horizon-native-mods` folder and enable/select **Iron Furnaces Native** for the native build.
4. In the launcher's Mods page, build the instance's native mods, then launch it. The native mod's title is **Iron Furnaces Native**.
5. Use a disposable world for the first launch. Craft a furnace, or use `/give @s ironfurnaces:iron_furnace` in creative mode.

The native package deliberately pins its Horizon revision: its server integration uses internal APIs as well as public hooks. Other pinned native mods in the same instance must target the same revision. The source can also be built with `horizon-launcher native-build native/mixins/ironfurnaces --trust-native-code`; the content pack still needs to be installed in the selected instance.

## Furnaces

| Tier | Ticks per ordinary smelt | Maximum items per operation |
| --- | ---: | ---: |
| Copper | 180 | 1 |
| Iron | 160 | 1 |
| Silver | 140 | 1 |
| Gold | 120 | 1 |
| Diamond | 80 | 1 |
| Emerald | 40 | 1 |
| Crystal | 40 | 1 |
| Obsidian | 20 | 1 |
| Netherite | 5 | 1 |
| Rainbow (`million_furnace`) | 20 | 64 |
| Allthemodium | 5 | 16 |
| Vibranium | 3 | 32 |
| Unobtainium | 1 | 64 |

At 20 ticks per second, an Iron Furnace takes eight seconds for a standard recipe. Recipe-specific cooking times scale the durations. Fuel burn times scale as in the Java mod, so an unaugmented, non-batch furnace smelts eight standard items per coal. Batch tiers intentionally process more items per fuel.

The table shows upstream defaults. Server configuration can change cooking speeds, storage tiers, energy capacities, generation rates and the ATM tiers' batch limits.

Silver recipes need another content mod supplying the silver material tags. The three ATM tiers need a compatible Allthemodium content mod and its materials; this port does not introduce replacement ores. The default Rainbow Plating recipe uses upstream's no-silver recipe. For a pack with silver, regenerate with `python tools/build_content.py --with-silver` before packaging.

## Factories and power

A factory uses energy instead of a fuel item and cooks independent input/output pairs in parallel. Copper and Iron have two active lanes and 80,000 FE storage; Silver and Gold have four lanes and 200,000 FE; higher tiers have six lanes and 1,000,000 FE. **Auto split** distributes matching ingredients over the available lanes. A standard unaugmented recipe uses 4,000 FE per operation; speed uses 8,000 FE and fuel efficiency uses 2,000 FE. Recipe duration scales these costs. Batch tiers retain their per-operation batch behavior.

A generator converts its fuel into energy. Without a red augment it burns ordinary furnace fuels; smoking selects food; blasting selects the supplied `ironfurnaces:generator_blasting` recipe data. Its green augment changes generation speed and fuel economy. Energy and its capacity appear in the screen, with the current generation rate in FE/t.

Place a generator next to a factory or a Wireless Heat Transmitter, and configure the generator's facing side to **Output** or **Input/Output**. Factories and transmitters accept energy from every face. Generators also expose their buffer through Horizon's shared energy API for other native mods using the same API.

Insert a **Wireless Heat Receiver** into a transmitter to bind it to that block's coordinates, then place the receiver in an ordinary furnace's fuel slot. It draws heat from that transmitter in the same dimension. There is no distance limit; a temporary chunk ticket loads the linked transmitter when needed. Its buffer holds 1,000,000 FE. Breaking the transmitter preserves its stored energy in the dropped block item, while its receiver drops separately. Its item tooltip and bar show retained energy.

The Rainbow generator adds its upstream bonus when its owner is online in the same dimension and owns a running, non-full generator of each required tier: Copper, Iron, Silver, Gold, Diamond, Emerald, Crystal, Obsidian and Netherite. Only one owned Rainbow generator may be active in that dimension. The ordinary generation and Rainbow bonus are separate; the default bonus is 50,000 FE/t. The network records placed furnace positions and maintains chunk tickets while it is eligible; tickets are released when its owner leaves or its Rainbow generator is removed.

Exploding a Rainbow Furnace (`ironfurnaces:million_furnace`) converts it into Rainbow Coal. This destroys the furnace's inventory, matching upstream. Rainbow Coal provides 5,120 fuel uses before it is consumed.

## Automation and appearance

The settings panel controls the six absolute world faces: **D/U/N/S/W/E**. Click a face to cycle **None → Input → Output → Input/Output → Fuel**; right click cycles backward. **Auto In** pulls compatible items from adjacent inventories; **Auto Out** pushes results and spent bucket remainders. Hopper insertion and extraction obey the same face settings. Unburned fuel is not an output.

New machines use the conventional furnace arrangement: Up is Input, Down is Output, and the horizontal faces are Fuel. The **Reset sides** button disables every face. Redstone buttons select ignored, signal required, signal absent, comparator output, or comparator output with subtraction; the **−/+** buttons set subtraction from 0 to 15.

Use the settings linker on a furnace to copy its settings, then sneak-use it on another to apply them. Its tooltip lists the copied values and facing. Apply a Spooky or Christmas item to select the corresponding skin; sneak-use an empty hand to clear the skin. Empty-hand sneaking no longer removes augments in 0.2: remove them through the augment page.

Existing 0.1 furnaces migrate their inventories and installed augments to the larger backing inventory and retain the conventional hopper directions. Back up worlds before upgrading. Peers must use matching content and the exact pinned native build; independent multiplayer and cross-mod compatibility require their own validation.

## Server configuration

Horizon creates `config/ironfurnaces.json` from the mod's defaults. A world's `serverconfig/ironfurnaces.json` overrides that file when present. Changes are reloaded by Horizon while the server is running. Invalid JSON keeps the last valid settings.

The structure follows upstream Fabric 26.3: ordinary tiers live under `furnaces`, ATM tiers under `modded_furnaces`, capacities under `furnaces.energy.tier_0` through `tier_2`, and Rainbow bonus generation under `furnaces.rainbow_furnace.rainbow_generation`. Each tier has `speed`, `generation` and `tier`; the ATM tiers also have `mult`. Use the generated complete file as the editing reference. There is no separate configuration GUI.

## Build and check

Python 3.11+ is used only for development and packaging. Players run Rust code in Horizon; no Python or Java runtime is part of the mod.

```text
python tools/build_content.py
python tools/build.py test --offline
python tools/build.py build --offline
python tools/build.py validate --offline
```

On Windows, the build helper reuses the launcher's installed Rust, MinGW linker and Cargo cache when ordinary Cargo is absent from PATH. It writes game sources and compiled output to `.build/` in this repository. It does not modify the launcher's game source, current instance or player saves. On other systems, install Rust and the normal Horizon build prerequisites, and pass `--source /path/to/pinned/horizon --mixins /path/to/horizon-mixins` to the build helper.

The initial upstream asset import requires a checkout at `.upstream/iron-furnaces`, or `--upstream /path/to/Iron-Furnaces` for the content generator. Generated content is included in this repository, so users can package the checked-in content without downloading Java sources. `dist/SHA256SUMS` records the two deterministic package hashes.

See [VALIDATION.md](VALIDATION.md) for checks actually performed, and [NOTICE](NOTICE) and [LICENSE](LICENSE) for attribution and license terms. Upstream reference: `e1fabfe21abf5b11de1efb2b2f773d5561539d4c`, Fabric 26.3.

Development references: [Horizon mod guide](https://playhorizon.dev/docs/make-a-mod), [content mods](https://playhorizon.dev/docs/content-mods), [native bundles](https://playhorizon.dev/docs/mod-manifest).
