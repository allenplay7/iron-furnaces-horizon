# Validation

Historical 0.2 results follow. For the new API release, see [0.3 validation](VALIDATION-0.3.md).

Target: Horizon Nightly 2026-11-13, game 26.3, commit `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`.

Upstream reference: Iron Furnaces `e1fabfe21abf5b11de1efb2b2f773d5561539d4c`, Fabric 26.3. Native transformation tool: `5a64af938ec86b00d53d16a408aabc0038fb3e57`.

## Version 0.2.0 — validated and installed

The expanded port passes 24 Rust rules/menu-context tests and 14 content integrity tests. Horizon transformed 37 native hooks and ten modules, and the final complete launcher build succeeded. The [official package validator](validation/v2/package-validation.log) accepts both 0.2.0 archives.

Actual-game results obtained on 15 November 2026:

- [16 power/gameplay cases](validation/v2/power-results.json): legacy migration; an active vanilla furnace upgrade retaining paid progress, names and locks; 2/4/6 factory lanes and exact energy costs; batching/splitting/component safety; generator fuel economics, capacity pauses and food/blasting fuels; shared energy transfer; hopper and automatic I/O rules; redstone; wireless heat, including an initially unloaded transmitter 1,014 blocks away; the owned Rainbow network; settings copy, skins, and full game restarts.
- [All 11 existing furnace regression groups](validation/v2/furnace-regression-results.json) passed without skips, including the actual result-slot clicks and recipe XP.
- [Three survival/explosion cases](validation/v2/extras-results.json): one furnace plus its exact stored contents drop when mined; one transmitter retains 543,210 FE through mining, pickup and placement while its receiver drops separately; a TNT explosion creates exactly one surviving Rainbow Coal and destroys the Rainbow Furnace's contents as upstream does.

The power and furnace suites used the first 0.2 executable (`5c0450ca58ee2f9a899fb37f9679e89b1e5d166524045b49b896cdef2349e06a`). Separate provenance records preserve initial fixture failures and successful focused reruns. Survival checks exposed and then verified the correction of upstream loot's old condition/modifier syntax, which Horizon otherwise silently rejected.

The [final rendered interface suite](validation/v2/visual/results.json) passed seven groups: 2/4/6-lane factory layouts, active generator display, actual augment/settings controls and the auto-split shortcut, bound transmitter interaction and sneak-use guards, Rainbow Coal durability bars, the four Emerald Furnace arrow checkpoints, and owner assignment through actual player placement. All twelve screenshots were visually inspected; [visual provenance](validation/v2/visual-provenance.json) records individual findings and hashes. The progress arrows have clean silhouettes at [idle](validation/v2/visual/furnace-idle.png), [half](validation/v2/visual/furnace-half.png), [near-full](validation/v2/visual/furnace-near-full.png) and [completed](validation/v2/visual/furnace-complete.png) states. Some captures retain Horizon's tutorial toast above the menu; it does not cover the controls. The first interface run exposed a missing transmitter menu-open action; the final run verifies its correction.

The launcher built and selected **`native-9f42064-6c4aaedde2ef`** for **IRON FURNACE PORT** in 496 seconds. The [installation receipt](validation/v2/installation.json) verifies that the installed executable is byte-identical to the final interface-tested executable, SHA-256 `1747048f05b1f56d765c6b390018eca4cc9fbebbc588fed185b1feaad7b0292d`. Both installed packages match [dist/SHA256SUMS](dist/SHA256SUMS), and the native build includes Iron Furnaces Native 0.2.0 with no omitted enabled native mods. Build logs and the native build record are preserved under `validation/v2/`.

Tests used isolated worlds and game directories. Existing player worlds were not opened. The final build's localized interface, interaction and bar changes were exercised by the final rendered suite; the full power and furnace suites were not repeated on that executable. Their earlier executable provenance is retained above.

Current compatibility limits: integrated-server Windows testing only; dedicated servers, remote multiplayer, other operating systems and other native-mod combinations have not been validated. Silver and ATM crafting still require external material providers. The port targets the exact Horizon revision above, and its server configuration is edited as JSON rather than through a configuration screen.

## Version 0.1.0 history

### Build and content checks

- Eight Rust rules tests passed, covering tier defaults, recipe timing and rounding, fuel efficiency, short fuels, batch capacity, upgrade routes, augment channels and overflow limits.
- Ten content integrity tests passed. All 13 furnace loot tables match the upstream files; original item models, selected crafting recipes and mining tags are retained.
- Horizon applied all 14 native integrations and three injected modules successfully.
- The initial complete Windows GNU game build passed using the launcher's Rust 1.99.1 toolchain and the `fast` profile. That rebuild completed in 6m 05s, after source transformation and staging.
- Horizon's official package checker accepted both files in `dist/`. Their hashes are recorded in `dist/SHA256SUMS`.

The compiler reports ten non-fatal warnings about internal interface visibility and unused rule helper methods, plus warnings from Horizon's bundled MinGW allocator sources. No compiler errors remain. Rustfmt and Clippy were not available in the installed toolchain and were not run.

### Gameplay checks

The final rendered integration run passed **all 11 groups, with zero failures and zero skips**, on 14 November 2026. Machine-readable results are in [validation/gameplay-results.json](validation/gameplay-results.json).

| Group | Verified behavior |
| --- | --- |
| Tier timing | Exact first-operation ticks and batch sizes for all 13 tiers |
| Fuel economics | Eight standard smelts per coal for each of the nine ordinary tiers |
| Augments | Speed gives four outputs per coal; efficiency gives 16; smoking and blasting select the correct recipes and use their actual cooking times |
| Upgrades | Vanilla and custom upgrades; rejected invalid routes; inventory and work preservation; empty-burning Iron-to-Gold upgrade scales remaining burn from 1,120 to 840 ticks |
| Menu | Result insertion and invalid fuel rejected; component stack limit of one respected; shift-click moves eight cooked ingots; five recipe XP collected |
| Hoppers | Top input, side fuel and bottom output transfer eight finished ingots into a chest |
| Blocked output | Full slots, different items and different components preserve input and unspent fuel |
| Batch capacity | Rainbow Furnace fills three remaining output spaces and leaves 61 inputs intact |
| Remainders | Lava retains an empty bucket; wet sponge produces a water bucket |
| Augment changes | Switching modes/efficiency and removing augments clears paid burn and cooking progress without free outputs |
| Persistence | Inventory, 3,181 remaining burn ticks and 20 progress ticks survive a full game restart; cooking resumes correctly |

The [furnace menu](validation/furnace-menu.png) and [placed furnace textures](validation/furnace-world.png) were rendered with Vulkan on an NVIDIA RTX 4060 Laptop GPU and visually inspected. Test-command chat and Horizon's first-run tutorial remain visible in these unedited screenshots.

The integration harness uses the actual compiled game and `horizon-ctl`, not a simulation of the adapter. It creates a new world, freezes and advances server ticks explicitly, and isolates its game directory, launcher directories and save data. It does not alter an existing player world or the launcher's original source tree.

Reproduce the rendered suite with the matching game resources and control executable:

```text
python tools/smoke.py --game .build/target/fast/horizon-client.exe --ctl /path/to/horizon-ctl.exe --resources /path/to/26.3 --run-dir .build/another-run --windowed --offscreen --keep-going
```

Use a new `--run-dir` for each run. Omitting `--windowed` disables rendering and skips the menu-click group. `--offscreen` keeps the rendered window hidden. The harness saves command responses, results and game logs; rendered menu checks also save screenshots. The final run's complete local evidence is under `.build/smoke-rendered/` and the build log is `.build/build-final.log`.

Earlier test runs exposed and corrected setup assumptions: fresh mod inventories need their initialization tick; time-query replies include units; the current smoking/blasting recipes declare 200 ticks; and saved inventories omit trailing empty slots. Horizon also clears simulated keyboard Shift, so the removal check uses its supported virtual-controller input. Those earlier failures are retained in `.build/`; they are not counted as passing runs.

### Arrow rendering correction — 14 November 2026

The original menu used a solid gray rectangle behind the cooking arrow and cropped the active sprite to 17 pixels even though the texture is 16 pixels high. The corrected renderer draws the inactive arrow from the existing vanilla furnace panel and uses the active sprite's proper height. The earlier menu screenshot above predates this correction.

The updated native package passed Horizon's official validator. The launcher rebuilt and selected `native-9f42064-e89a1131021c` for the `iron-furnace-port` instance; its build record contains Iron Furnaces Native with no omitted mods. Compilation completed in 4m 47s; transformation, compilation and installation together took 688 seconds.

The new executable was tested in a separate rendered world with an Emerald Furnace. Screenshots at [idle](validation/arrow-fix/furnace-idle.png), [half progress](validation/arrow-fix/furnace-half.png), [full arrow before completion](validation/arrow-fix/furnace-near-full.png), and [reset after completion](validation/arrow-fix/furnace-complete.png) were visually inspected. All four show a clean arrow silhouette with no rectangular overlay or sprite overscan. Cooking progress was 0/40, 20/40, 39/40 and then 0/40 with exactly one iron ingot produced. The test and game exited successfully.

[Arrow-check results](validation/arrow-fix/results.json) include the installed version and executable hash. The tested executable and the installed game are byte-identical. Full command evidence is retained in `.build/arrow-visual/`; launcher build logs are in `.build/arrow-fix/`. Reproduce this focused check with `tools/check_visual.py` and the same game, control-tool and resource arguments as the integration harness; it defaults to Emerald and offscreen rendering. The earlier gameplay suite was not repeated for this drawing-only change.

### Historical 0.1 limits

- This is the furnace-focused feature set documented in README.md. Electric factories/generators, wireless heating, configuration/linking systems, seasonal skins and Rainbow Coal are outside this release.
- Silver and ATM-tier crafting depends on external content supplying the original material tags. Those external mods were not installed in the test world.
- Tests use Horizon's integrated server on Windows. Dedicated-server deployment, remote multiplayer, other operating systems and combinations with other native mods have not been exercised.
- The native mod is pinned to the exact Horizon revision above. A later game build requires a new compatibility review and build.
- Recipe XP is released on the first machine tick after a player takes output. A rejected result-slot click followed by hopper extraction before that tick can release the already-earned XP bank early; it does not create extra XP or duplicate items.
