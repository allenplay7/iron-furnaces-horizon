"""Capture a real furnace menu at frozen recipe checkpoints.

Uses a fresh disposable world and renders offscreen by default. The assertions
check cooking and inventory; review the PNGs to judge the menu's appearance.
"""
from __future__ import annotations

import argparse
from pathlib import Path

from smoke import ROOT, Session, require


def visual_progress(s):
    pos = (4, 200, 2)
    duration = {"iron": 160, "emerald": 40}[s.args.tier]
    # This is the documented numeric chatVisibility option. It hides command
    # feedback in the screenshots without suppressing control replies.
    s.call("options-set", "chatVisibility", "2")
    s.seed(pos, s.args.tier, 1)
    s.interact(pos, close=False)
    captures = {}

    def capture(label, elapsed, expected_progress, expected_output):
        # Wait for client menu packets/rendering while the world stays frozen.
        s.call("wait", 4)
        timers = {field: s.integer(pos, field)
                  for field in ("CookTime", "CookTotal", "BurnTime", "BurnTotal")}
        require(timers["CookTime"] == expected_progress,
                (label, "unexpected cooking progress", timers))
        if elapsed:
            require(timers["CookTotal"] == duration,
                    (label, "unexpected furnace recipe duration", s.args.tier, timers))
            require(timers["BurnTime"] > 0,
                    (label, "paid fuel did not remain lit", timers))
        else:
            require(timers["BurnTime"] == 0,
                    (label, "frozen furnace consumed fuel", timers))
        s.expect_slot(pos, 0, "minecraft:raw_iron", 1 - expected_output)
        s.expect_slot(pos, 1, "minecraft:coal", 0 if elapsed else 1)
        s.expect_slot(pos, 2, "minecraft:iron_ingot", expected_output)
        screenshot = s.run / f"furnace-{label}.png"
        menu = s.call("menu")
        s.call("screenshot", str(screenshot))
        require(screenshot.is_file(), (label, "screenshot was not saved", screenshot))
        with screenshot.open("rb") as image:
            require(image.read(8) == b"\x89PNG\r\n\x1a\n",
                    (label, "screenshot is not a PNG", screenshot))
        captures[label] = {"recipe_ticks_elapsed": elapsed, "timers": timers,
                           "slots": s.slots(pos), "menu": menu,
                           "screenshot": str(screenshot)}
        # Preserve completed checkpoints even if a later checkpoint fails.
        s.results["captures"] = {"status": "recorded", "detail": captures}
        s.write()

    capture("idle", 0, 0, 0)
    s.sprint(duration // 2)
    capture("half", duration // 2, duration // 2, 0)
    s.sprint(duration - duration // 2 - 1)
    capture("near-full", duration - 1, duration - 1, 0)
    s.sprint(1)
    capture("complete", duration, 0, 1)
    return {"tier": s.args.tier, "checkpoints": list(captures), "output": "minecraft:iron_ingot",
            "visual_review": "Inspect the captured PNGs; appearance is not asserted."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game", type=Path, required=True)
    parser.add_argument("--ctl", type=Path, required=True)
    parser.add_argument("--resources", type=Path, required=True)
    parser.add_argument("--content", type=Path, default=ROOT / "content/ironfurnaces")
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--tier", choices=("iron", "emerald"), default="emerald",
                        help="Furnace to capture (default: emerald, 40 ticks per recipe)")
    parser.add_argument("--visible", action="store_true",
                        help="Show the game window instead of rendering offscreen")
    args = parser.parse_args()
    args.windowed = True
    args.offscreen = not args.visible
    for name in ("game", "ctl", "resources", "content"):
        setattr(args, name, getattr(args, name).resolve(strict=True))
    if args.run_dir.exists():
        parser.error("--run-dir must not already exist")
    if args.run_dir.resolve().is_relative_to(args.resources):
        parser.error("run directory must be outside resources")
    session = Session(args)
    try:
        session.start()
        session.run_case("visual_progress", visual_progress)
    finally:
        try:
            session.stop()
        finally:
            session.write()
    print(f"Captured four furnace checkpoints: {session.run / 'results.json'}")


if __name__ == "__main__":
    main()
