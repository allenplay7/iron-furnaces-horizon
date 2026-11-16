This audit describes the old 0.2 implementation before migration. See [README.md](README.md) for the new API port and its current compatibility requirements.

# Horizon compatibility audit — 16 November 2026

Iron Furnaces 0.2.0 remains compatible with its installed game and the latest published Nightly. It is **not a drop-in native package for the newly merged development API**. Keep the existing release pins until a separate migration is built and tested.

## Versions checked

- Latest published game: `nightly`, 13 November, `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`, matching our native manifest.
- Selected instance: `iron-furnace-port`, build `native-9f42064-6c4aaedde2ef`.
- Installed executable SHA-256: `1747048f05b1f56d765c6b390018eca4cc9fbebbc588fed185b1feaad7b0292d`, identical to the previously validated 0.2 release.
- Latest published launcher at inspection: 0.2.69. This audit did not upgrade or certify that launcher executable.
- Development source inspected: `85b5bcca7c79bb470b90bdb5914b413aa66fe7ae`, including the new mod API merge. This is separate from the published Nightly.

## Confirmed development-version incompatibilities

1. The new launcher's native discovery filters for `.hmod` files. Our native archive is `.hnmod`, so it will not be discovered by that code.
2. Native projects now use a root `horizon-mod.toml`, Cargo project and source under `src/`. The loader's tests explicitly reject old roots including `horizon-native.json` and `horizon-mixin.toml`. Renaming our archive is insufficient.
3. The new build links mod code against a prebuilt game instead of rebuilding the whole game with our injected source. Our 37 integrations and ten support modules need review against the new `[[native.mixin]]` / `[[native.module]]` machinery or typed hooks/Parts.
4. Our targets use the previous native hook module paths and internal game types. The new source splits `game/native_hooks` into modules; merely changing the pinned commit does not establish compatibility. Slot validation and the support module injected into the separate client library also require review under the new replacement restrictions.

No migrated package or successful development-branch compile is claimed. The existing package files, revision pins and launcher selection were left unchanged.

## Fresh local checks

- 24 Rust rules/menu tests and 14 content tests passed.
- Both distributed archives passed the package checker from the supported game toolchain. This does not claim acceptance by the development API's checker.
- The installed executable hash matches the validated release.
- All seven fresh rendered test groups passed: factory lanes, generator display, actual augment/settings controls, heater interaction, coal bars, four furnace-arrow checkpoints, and placement ownership. The heater and near-full arrow screenshots were also visually inspected and render correctly. [Raw results](validation/compat-2026-11-16/visual-results.json) and screenshots are recorded under `validation/compat-2026-11-16/`; the full isolated run remains under `.build/compat-2026-11-16/visual/`. The test game exited successfully.

The broad power, survival and persistence results from the original release remain in [VALIDATION.md](VALIDATION.md). Existing player worlds were not used for this audit.

## Primary sources

- [Published releases](https://github.com/horizon-rust/horizon/releases)
- [New API merge](https://github.com/horizon-rust/horizon/pull/17145)
- [Native package discovery and old-root rejection](https://github.com/horizon-rust/horizon/blob/85b5bcca7c79bb470b90bdb5914b413aa66fe7ae/launcher/src/native_mods.rs#L440)
- [New native build model](https://github.com/horizon-rust/horizon/blob/85b5bcca7c79bb470b90bdb5914b413aa66fe7ae/docs/modding/src/how-to/native-mods.md)
- [New mixin declarations and target restrictions](https://github.com/horizon-rust/horizon/blob/85b5bcca7c79bb470b90bdb5914b413aa66fe7ae/docs/modding/src/concepts/native-route/2-mixins.md)
- [New support modules](https://github.com/horizon-rust/horizon/blob/85b5bcca7c79bb470b90bdb5914b413aa66fe7ae/docs/modding/src/concepts/native-route/5-support-modules.md)

The public website's older make-a-mod guide still describes `.hnmod` and whole-game compilation. For the development API, this audit uses the source and modding book at the exact commit above.
