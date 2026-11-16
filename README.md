# Iron Furnaces for Horizon

An unofficial Rust port of [Astryxion's Iron Furnaces](https://github.com/astryxion/Iron-Furnaces), with the original textures, models and crafting progression. Original credits: **Astryxion, pizzatime and XenoMustache**. Distributed under **Apache-2.0**; see [LICENSE](LICENSE), [NOTICE](NOTICE) and [modification notices](MODIFICATIONS.md).

Version **0.3.1** targets Horizon's new native mod API at commit `85b5bcca7c79bb470b90bdb5914b413aa66fe7ae` (Minecraft 26.3). It uses typed block/item/menu parts, transactional storage capabilities and chunk tickets, packaged together with its content in **one `.hmod`**. The game is supplied separately by Horizon.

**Compatibility:** the older November 13 Nightly and its launcher cannot build this package. Use a Horizon build and SDK matching the pinned new API revision. Download the [0.3.1 prerelease](https://github.com/allenplay7/iron-furnaces-horizon/releases/tag/v0.3.1). The previous paired-package release remains documented in [LEGACY-0.2.md](LEGACY-0.2.md). See [0.3 validation](VALIDATION-0.3.md) for actual-game checks and a saved-world migration from 0.2.

The port includes thirteen furnace tiers, electric factories, three generator fuel modes, wireless heating, per-side automation, redstone controls, settings copying, seasonal skins and Rainbow Coal.

Use a matching upgrade item on a placed furnace to change its tier while keeping its inventory, energy and settings. Ordinary use opens its machine screen. The **+** button opens the augment page: red selects smoking or blasting, green selects speed or fuel efficiency, and blue selects an electric factory or a generator. Remove an augment from its slot to remove its effect. An empty blue slot selects the ordinary fuel furnace. Augments can also be installed by using their item on the block.

The speed augment halves cooking time and uses twice as much fuel per operation. The fuel-efficiency augment increases cooking time by 25% and doubles operations per fuel item. Timings round to whole ticks. Changing augments clears current burning and cooking progress. Changing the machine mode drops incompatible stored ingredients and results beside the furnace; changing a generator's fuel mode also clears its fuel inventory. Collect those items before leaving.

## Install

1. Use a separate Horizon instance running the matching new API build. Back up any world you intend to migrate.
2. Install `ironfurnaces-0.3.1.hmod` through that launcher's Mods page, or place it in the instance's `horizon-mods` folder. Remove/disable the older Iron Furnaces content/native pair in this test instance to avoid duplicate IDs.
3. Enable Iron Furnaces, allow its native code and build the instance's mods, then launch it.
4. Test a disposable world first. Craft a furnace or use `/give @s ironfurnaces:iron_furnace` in creative mode.

There is no separate `.hnmod` for 0.3. Native mods run with the player's rights, as Horizon's native-code prompt explains. The package pins the game and mixin tool revisions because a few menu, redstone and removal features still require function-replacement hooks. All peers need compatible matching mods. The GitHub package contains this mod's sources and resources, not a Horizon executable or Minecraft assets.

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

Python 3.11+ is used for development and packaging; the installed mod runs Rust code. Generated content is checked in. The new adapter is in `native-v3/`; `native/` and `tools/build.py` retain the previous release for reference.

```text
python tools/package_v3.py
python tools/check_v3.py --source /path/to/horizon-at-the-pinned-commit --test
horizon-launcher native-build .build/migration-v3/mod --trust-native-code
```

The first command creates the unified source package and a complete mod folder for the launcher. It includes the upstream license, attribution and change notices and produces a deterministic SHA-256 sidecar. The check command verifies the typed API adapter and machine rules; it does not build the engine compatibility hooks or replace a full game test. The launcher command requires the matching installed game/SDK and operates on its selected instance, so select a disposable test instance first.

To regenerate imported resources, check out upstream revision `e1fabfe21abf5b11de1efb2b2f773d5561539d4c` at `.upstream/iron-furnaces`, then run:

```text
python tools/build_content.py
python tools/test_content.py
python tools/package_v3.py
```

Use `--with-silver` on the content generator only for a pack that supplies silver. Tests in `tools/smoke.py`, `tools/test_power.py`, `tools/test_extras.py` and `tools/check_power_visual.py` use fresh disposable worlds. Each takes `--game`, `--ctl`, `--resources`, `--content` and a fresh `--run-dir`; rendered checks use `--windowed --offscreen`. Prepared Minecraft resources remain private and are not included in this repository.

The source/content GitHub workflow does not claim to run the full game. See [0.3 validation](VALIDATION-0.3.md) for the exact tested build, gameplay results, screenshots and known limits; [VALIDATION.md](VALIDATION.md) retains historical 0.2 results.

Development references: [mod guide at the pinned revision](https://github.com/horizon-rust/horizon/blob/85b5bcca7c79bb470b90bdb5914b413aa66fe7ae/docs/modding/src/start/first-mod.md), [pinned Horizon source](https://github.com/horizon-rust/horizon/tree/85b5bcca7c79bb470b90bdb5914b413aa66fe7ae). The public website guide may describe an earlier package format.
