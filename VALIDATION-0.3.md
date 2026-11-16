# Iron Furnaces 0.3 migration validation

Tested on Windows 11 x64 on 16 November 2026, using the actual Horizon game, rendered offscreen in disposable worlds. The existing 0.2 installation and player worlds were not modified.

## Exact build

- Horizon: `85b5bcca7c79bb470b90bdb5914b413aa66fe7ae`.
- Horizon mixins: `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.
- Rust: 1.99.1; target: `x86_64-pc-windows-gnu`.
- New prebuilt SDK format 6, release engine with `system-allocator`; optimized mod build.
- Launcher build route: `prebuilt`, with no omitted mods. All eight function-replacement hooks compiled and were included.
- Released package: `ironfurnaces-0.3.1.hmod`, SHA-256 `cb55cbedc7fc573f33016b86acc97c00440066a10e2d7ccd4ed33a679426487b`.

[Full-suite build receipt and runtime hashes](validation/v3/build.json) identify the 0.3.0 mod library, patch library and engine libraries used for the complete gameplay suite below. The corrected 0.3.1 package has identical Rust sources and assets; only version metadata and the modification-notice filename changed, as recorded in the [entry comparison](validation/v3/packaging.json). Its [build receipt](validation/v3/package-build.json) identifies the rebuilt 0.3.1 libraries. The tiny entry executable alone does not identify a prebuilt native game. The engine/SDK and preparer were built from the pinned source. A local Windows launcher icon/manifest workaround was needed to bootstrap its CLI; it did not change the game or mod implementation. Prepared Minecraft resources were regenerated with that revision's preparer and passed its completeness check.

The November 13 Nightly uses the older mod format and cannot load this release. This is a prerelease for the pinned new API and matching SDK, not a compatibility claim for arbitrary Nightlies.

## Results

| Check | Result | Evidence |
| --- | --- | --- |
| Rust rules and typed adapter | 28 passed | [Log](validation/v3/mod-tests-final.log) |
| Imported art, content and recipes | 14 passed | [Log](validation/v3/content-tests-final.log) |
| Factories, generators and automation | 16 scenarios passed | [Results](validation/v3/power/results.json) |
| Ordinary furnaces | 11 scenarios passed, none skipped | [Results](validation/v3/furnaces/results.json) |
| Drops, explosion and configuration | 4 scenarios passed | [Results](validation/v3/extras/results.json) |
| Real screen input and placement | 7 groups passed; 12 captures inspected | [Results](validation/v3/visual/results.json) |
| Actual 0.2 world opened by 0.3 | Passed | [Results](validation/v3/save-upgrade/results.json) |
| Package reproducibility | Identical SHA-256 from a clean Git-index checkout | Stored ZIP entries; fixed entry order, timestamps, permissions and platform metadata |
| 0.3.1 archive validation | Accepted by the pinned Horizon package reader | [Log](validation/v3/0.3.1/package-check.log) |
| Rebuilt 0.3.1 smoke checks | 3 power scenarios, 2 furnace scenarios and 1 screen group passed | [Power](validation/v3/0.3.1/power/results.json), [furnaces](validation/v3/0.3.1/smoke/results.json), [screen](validation/v3/0.3.1/visual/results.json) |

The power checks cover two/four/six factory lanes, component-aware auto split, blocked outputs, augment energy costs, generator fuel budgets and capacity pauses, ordinary/food/blasting/Rainbow Coal fuels, energy transfer, paid reservations across restart, per-face automation and hopper access, comparator and subtraction output, receiver binding, and a transmitter 1,014 blocks away in an initially unloaded chunk. They also exercise the nine-tier Rainbow generator network, seasonal skins, and settings copying/application/restart without duplicating the copier.

The furnace checks cover all thirteen tier timings, fuel economics, recipe and efficiency augments, tier upgrades, invalid upgrades, retained contents and fractional paid work, actual menu insertion/extraction and XP, stack limits, hoppers, blocked and component-incompatible outputs, batch capacity, lava-bucket and wet-sponge remainders, augment switching, and persistence. Sparse inventories are compared by their contents; the new engine restores trailing empty slots on first use/tick, which is also checked after an upgrade.

The extras check survival furnace drops and contents, retained transmitter energy after breaking and placing it, Rainbow Furnace TNT conversion, live configuration reload, invalid-JSON fallback, and world-specific configuration precedence. The screen checks exercise augment-slot limits and removal, settings buttons, factory layouts, the transmitter, Rainbow Coal durability bars and actual placement ownership. The Emerald Furnace arrow was inspected at idle, half progress, near completion and after completion; the old rectangular arrow artifact is absent. The [twelve screen captures](validation/v3/visual/) accompany the input/assertion evidence.

The save migration check creates and saves a world using the actual 0.2 executable, then opens that same world with 0.3. After 57 old-version ticks, a furnace and electric factory both resume their 160-tick operation and produce output after the remaining 103 ticks. Inventories, custom item components, generator fuel budget and factory energy/reservations are retained. A discovered difference in vanilla recipe IDs (bare names in 0.2, namespaced names in the new API) is migrated once before fingerprint comparison; a Rust regression also verifies merged earned-experience counters and idempotence.

The 0.3.1 smoke run additionally checks all factory lane layouts, generator economics, wireless heating, real furnace menu extraction/experience, save/reload, and the Emerald Furnace arrow at four stages. All four new captures were inspected. The version bump and packaging fix did not change any Rust source or machine resource from the complete suite.

## Reproduce

The release archive uses stored ZIP entries to avoid compression-library differences between Windows and Linux. Horizon's archive reader rejected the arbitrary root filename `MODIFICATIONS.md` in the initial 0.3.0 prerelease, which was withdrawn. Version 0.3.1 preserves those exact notice bytes as `NOTICE-MODIFICATIONS.md`, an accepted name. The 574 source, asset and license entries are byte-identical to the fully tested implementation; the remaining three manifests change only the version. `tools/check_package.py` now checks the release using the pinned engine's actual `Package::read`, both locally and in GitHub CI.

Build the package, use a Horizon game/SDK matching the pins, and prepare your own Minecraft 26.3 resources with that revision's preparer. The game tests accept explicit `--game`, `--ctl`, `--resources`, `--content` and a fresh `--run-dir`. The default content folder is the generated `.build/migration-v3/mod`.

```text
python tools/package_v3.py
python tools/check_v3.py --source /path/to/pinned-horizon --test
python tools/check_package.py --source /path/to/pinned-horizon
python tools/test_content.py
python tools/test_power.py <paths> --windowed --offscreen --keep-going
python tools/smoke.py <paths> --windowed --offscreen --keep-going
python tools/test_extras.py <paths>
python tools/check_power_visual.py <paths>
```

`tools/test_upgrade_save.py` additionally takes the old game/content/resources and the new equivalents. It creates its own 0.2 world; do not point the test harness at a personal save. Evidence files have local user paths removed. Original unsuccessful migration attempts remain private; the passing evidence above comes from the corrected build identified in the receipt.

## Limits

Dedicated-server, independent multiplayer, macOS and other native mods' interoperability have not been exercised here. GitHub's workflow runs the Rust/resource and reproducibility checks, not the full game. Missing optional Silver/Allthemodium materials disable their corresponding recipes as documented. The eight internal compatibility hooks require the pinned engine revision. Horizon/Minecraft binaries, SDKs, prepared resources and private saves are not distributed in this repository.
