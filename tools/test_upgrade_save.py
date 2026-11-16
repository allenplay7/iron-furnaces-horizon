"""Open an actual 0.2 world with 0.3 and resume paid, partly completed work."""
from pathlib import Path
import argparse

from smoke import require
from test_power import PowerSession


def upgrade_save(s):
    generator, factory, furnace = (4, 200, 2), (10, 200, 2), (16, 200, 2)
    s.machine(generator, mode="generator")
    s.machine(factory, mode="factory")
    s.machine(furnace)
    s.replace(generator, 6, "minecraft:coal")
    s.merge(factory, energy=8000)
    s.replace(factory, 9, 'minecraft:raw_iron[minecraft:custom_data={migration_marker:"v02"}]', 2)
    s.replace(furnace, 0, "minecraft:raw_iron", 2)
    s.replace(furnace, 1, "minecraft:coal")
    s.sprint(57)
    fields = {
        generator: ("energy", "GeneratorTotal", "GeneratorRemaining"),
        factory: ("energy", "LaneProgress2", "LaneTotal2", "LaneCost2", "LaneReserved2"),
        furnace: ("BurnTime", "BurnTotal", "CookTime", "CookTotal"),
    }

    def snapshot():
        return {str(pos): {key: s.integer(pos, key) for key in keys} for pos, keys in fields.items()}

    before = snapshot()
    inventories = {pos: s.slots(pos) for pos in fields}
    s.call("save")
    s.stop()
    s.args.game = s.args.new_game
    s.args.resources = s.args.new_resources
    s.args.content = s.args.new_content
    s.start()
    require(snapshot() == before, ("0.2 save changed paid progress or energy", before, snapshot()))
    for pos, slots in inventories.items():
        actual=s.slots(pos)
        # A freshly reloaded holder omits trailing empty slots until its first tick.
        require(actual+[None]*(len(slots)-len(actual))==slots,
                ("0.2 inventory changed",pos,slots,actual))
    components = s.command('data get block 10 200 2 Items[{Slot:9b}].components')
    require("migration_marker" in str(components) and "v02" in str(components),
            ("Saved item components were lost", components))
    s.sprint(103)
    s.expect_slot(factory, 15, "minecraft:iron_ingot", 1)
    s.expect_slot(furnace, 2, "minecraft:iron_ingot", 1)
    require(s.integer(factory, "energy") == 4000, "Migration charged the factory twice")
    require(s.integer(generator, "energy") == 6400, "Migration changed the generator's yield")
    return {"saved_by": "0.2", "loaded_by": "0.3", "saved": before,
            "inventory_and_components": "retained", "continued": "furnace, factory and generator",
            "remaining_ticks_to_output": 103}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("game", "ctl", "resources", "content", "new-game", "new-resources", "new-content", "run-dir"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    for name in ("game", "ctl", "resources", "content", "new_game", "new_resources", "new_content"):
        setattr(args, name, getattr(args, name).resolve(strict=True))
    args.windowed, args.offscreen = True, True
    if args.run_dir.exists():
        parser.error("--run-dir must be fresh")
    s = PowerSession(args)
    try:
        s.start()
        s.run_case("upgrade_save", upgrade_save)
    finally:
        try:
            s.stop()
        finally:
            s.write()
    print(f"0.2 save migration passed: {s.run / 'results.json'}", flush=True)


if __name__ == "__main__":
    main()
