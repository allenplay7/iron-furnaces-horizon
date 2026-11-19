# Iron Furnaces for Horizon

An unofficial Rust port of [Astryxion's Iron Furnaces](https://github.com/astryxion/Iron-Furnaces) for Horizon. It keeps the original textures, progression and credits. See [LICENSE](LICENSE), [NOTICE](NOTICE) and [MODIFICATIONS.md](MODIFICATIONS.md).

## Choose your version

Use the version that matches your Horizon installation. Do not install both versions in the same instance.

| Version | Horizon requirement | Files | Download |
| --- | --- | --- | --- |
| **0.2.0 — legacy API** | Nightly 2026-11-13, commit `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0` | `.hmod` + `.hnmod` | [GitHub release](https://github.com/allenplay7/iron-furnaces-horizon/releases/tag/v0.2.0) |
| **0.3.1 — registry-compatible** | Nightly 2026-11-13, commit `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0` | `.hmod` + `.hnmod` | [GitHub release](https://github.com/allenplay7/iron-furnaces-horizon/releases/tag/v0.3.1) |

## Next Nightly release candidate (unpublished)

The `native-v3` candidate, **0.3.2**, targets the November 18 Nightly
(`0b14f8acfd97fa2c750e42795249dcc981178ca4`) and mod API 0.2. It is one
native source `.hmod`, not the older `.hmod` + `.hnmod` pair. The
downloads above are still for their original Nightly. The existing 0.3.1
gameplay evidence does not validate the 0.3.2 candidate on this Nightly.

Validation and publication gates:

1. Run the source and content checks in `.github/workflows/check.yml`.
2. Stage the package with `python3 tools/package_v3.py`. In
   `.build/migration-v3/mod`, run `cargo horizon-mod build` with the
   official CLI installed from the exact Nightly revision. Run
   `cargo horizon-mod check target/horizon-mod/ironfurnaces.hmod` and
   `cargo test` there. Use the official SDK output, not the intermediate
   ZIP from the staging script, for any eventual release.
3. Install the resulting native package in an isolated launcher instance
   pinned to the same Nightly. Use the launcher's native build to verify
   compatibility hooks, then run the [in-game regression suite](VALIDATION-0.3.md#reproduce)
   in disposable worlds. Record the exact package SHA-256 and game pin.
4. Publish a new GitHub release without editing the old release. For the
   registry, follow the [manual publishing procedure](https://github.com/horizon-rust/horizon-mods#publishing-without-the-form)
   and supply the exact-SDK build evidence.

## Install

### 0.2.0 legacy API

1. Create a separate instance using the required November 13 Nightly.
2. Put `ironfurnaces-0.2.0.hmod` in `horizon-mods`.
3. Put `ironfurnaces-0.2.0.hnmod` in `horizon-native-mods`.
4. Enable both parts, build the native mods, and launch a disposable test world.

See [legacy instructions](LEGACY-0.2.md) for details.

### 0.3.1 registry-compatible

1. Create a separate instance using the required November 13 Nightly.
2. Put `ironfurnaces-0.3.1.hmod` in `horizon-mods`.
3. Put `ironfurnaces-0.3.1.hnmod` in `horizon-native-mods`.
4. Enable both parts, build the native mods, and test a disposable world first.

Back up worlds before migrating. Remove or disable the 0.2.0 pair in a 0.3.1 instance.

## Features

Thirteen furnace tiers, electric factories, generators, wireless heating, upgrades, augments, per-side automation, redstone controls, settings copying, seasonal skins and Rainbow Coal.

Factories and wireless transmitters accept energy through Horizon's shared `ENERGY` capability. Generators provide energy through the same API. Interoperability with other Horizon native mods using that API is possible but has not been tested with specific third-party mods; Forge Energy compatibility is not guaranteed.

Optional Silver and Allthemodium recipes require mods that provide those materials and tags.

## Validation and limits

- [0.3.1 validation](VALIDATION-0.3.md)
- [0.2.0 validation](VALIDATION.md)

The validated scope is Windows integrated-server play on the pinned Horizon builds. Dedicated servers, independent multiplayer, other operating systems and interoperability with other native mods remain unvalidated.

## License

Distributed under Apache-2.0. Original Iron Furnaces credits and notices are retained. This is an unofficial port with no endorsement claimed by Astryxion, Horizon, Mojang or Microsoft.
