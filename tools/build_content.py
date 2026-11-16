#!/usr/bin/env python3
"""Build Iron Furnaces' Horizon content from the Apache-2.0 upstream assets."""
from __future__ import annotations

import argparse
from itertools import product
import json
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_UPSTREAM = ROOT / ".upstream" / "iron-furnaces"
DEFAULT_OUTPUT = ROOT / "content" / "ironfurnaces"
TIERS = ("copper", "iron", "silver", "gold", "diamond", "emerald", "crystal", "obsidian", "netherite", "million", "allthemodium", "vibranium", "unobtainium")
FURNACES = frozenset(f"{tier}_furnace" for tier in TIERS)
BLOCKS = FURNACES | {"heater"}
UPGRADES = frozenset("upgrade_" + tier for tier in ("copper", "iron", "iron2", "silver", "silver2", "gold", "gold2", "diamond", "emerald", "crystal", "obsidian", "obsidian2", "netherite", "allthemodium", "vibranium", "unobtainium"))
AUGMENTS = frozenset(f"augment_{name}" for name in ("smoking", "blasting", "speed", "fuel", "factory", "generator"))
TOOLS = frozenset({"item_heater", "item_copy", "item_spooky", "item_xmas"})
ITEMS = UPGRADES | AUGMENTS | TOOLS | {"rainbow_core", "rainbow_plating", "rainbow_coal"}
REGISTERED = BLOCKS | ITEMS
UI_LABELS = {
    "settings": "Settings", "auto_in": "Auto In", "auto_out": "Auto Out",
    "redstone": "Redstone", "reset_sides": "Reset sides", "auto_split": "Auto split",
    "smelting": "Smelting", "smoking": "Smoking", "blasting": "Blasting",
    "rainbow_active": "Rainbow active", "rainbow_waiting": "Waiting for generators",
    "heater_link": "Insert a Wireless Heat Receiver to link it.",
    "heater_use": "Then put the receiver in a furnace fuel slot.",
    "side0": "Down", "side1": "Up", "side2": "North", "side3": "South", "side4": "West", "side5": "East",
    "side_cycle": "Left click: next; right click: previous",
    "redstone_value": "Comparator subtraction: 0–15",
    "auto_split_hint": "Distribute matching input evenly across factory lanes",
    "copy_settings": "Right click a furnace to copy its settings.",
    "apply_settings": "Sneak and right click to apply copied settings.",
    "facing": "Facing",
}
EXTERNAL_TAGS = frozenset({"c:ingots/silver", "c:ingots/allthemodium", "c:ingots/vibranium", "c:ingots/unobtainium", "c:storage_blocks/allthemodium", "c:storage_blocks/vibranium", "c:storage_blocks/unobtainium"})
VANILLA_TAGS = {
    "ingots/copper": ["minecraft:copper_ingot"],
    "ingots/iron": ["minecraft:iron_ingot"],
    "ingots/gold": ["minecraft:gold_ingot"],
    "gems/diamond": ["minecraft:diamond"],
    "gems/emerald": ["minecraft:emerald"],
    "glass_blocks/colorless": ["minecraft:glass"],
    "storage_blocks/gold": ["minecraft:gold_block"],
    "storage_blocks/redstone": ["minecraft:redstone_block"],
    "obsidians": ["minecraft:obsidian", "minecraft:crying_obsidian"],
    "rods/blaze": ["minecraft:blaze_rod"],
    "dusts/redstone": ["minecraft:redstone"],
    "stones": ["minecraft:stone", "minecraft:granite", "minecraft:diorite", "minecraft:andesite", "minecraft:deepslate", "minecraft:tuff"],
}
MANIFEST = '''id = "ironfurnaces"
name = "Iron Furnaces"
version = "0.2.0"
description = "Upgraded furnaces, electric factories, generators, wireless heating, upgrades and augments. Requires the Iron Furnaces native companion."
authors = ["Astryxion", "pizzatime", "XenoMustache", "Horizon port contributors"]
api = "0.2"
side = "both"
capabilities = ["registries"]
'''


def encoded(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def modified(value: dict, change: str) -> dict:
    """Keep Apache-2.0 section 4(b)'s change notice in modified upstream JSON."""
    return {"_comment": f"Modified for the unofficial Iron Furnaces Horizon port (2026): {change}. Original copyright 2026 Astryxion; Apache-2.0; see LICENSE and NOTICE.", **value}


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def strings(value: object):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for child in value.values():
            yield from strings(child)
    elif isinstance(value, list):
        for child in value:
            yield from strings(child)


def source_resources(upstream: Path) -> Path:
    version = upstream / "Fabric 26.3" if (upstream / "Fabric 26.3").is_dir() else upstream
    resources = version / "src" / "main" / "resources"
    if not (resources / "assets" / "ironfurnaces" / "textures" / "block" / "iron_furnace_front.png").is_file():
        raise ValueError(f"Not an Iron Furnaces source folder: {upstream}")
    return resources


def convert_conditions(recipe: dict) -> dict:
    """Convert mod gates; ingredient tags enforce optional material availability.

    Horizon lacks Fabric tag-populated predicates. The caller selects exactly
    one rainbow plating recipe at build time. No silver/ATM materials are added.
    """
    recipe = dict(recipe)
    gates = []
    for condition in recipe.pop("fabric:load_conditions", []):
        kind = condition.get("condition")
        if kind == "fabric:all_mods_loaded":
            gates.extend({"type": "neoforge:mod_loaded", "modid": mod} for mod in condition["values"])
        elif kind == "fabric:tags_populated":
            continue
        elif kind == "fabric:not" and condition.get("value", {}).get("condition") == "fabric:tags_populated":
            continue
        else:
            raise ValueError(f"Unsupported upstream load condition: {condition!r}")
    if gates:
        recipe["neoforge:conditions"] = gates
    return recipe


def block_definition(tier: str) -> dict:
    # Upstream copies the corresponding material's physical properties.
    hardness, resistance, sound = {
        "copper": (3.0, 6.0, "copper_block"), "silver": (3.0, 6.0, "copper_block"),
        "gold": (3.0, 6.0, "gold_block"), "crystal": (0.3, 0.3, "glass"),
        "obsidian": (50.0, 1200.0, "obsidian"), "netherite": (50.0, 1200.0, "netherite_block"),
    }.get(tier, (5.0, 6.0, "iron_block"))
    properties = {"hardness": hardness, "explosion_resistance": resistance,
                  "requires_tool": tier != "crystal", "sounds": f"minecraft:{sound}"}
    if tier == "crystal":
        properties["occludes"] = False
    state_properties = {name: {"type": "enum", "values": ["0", "1", "2"]}
                        for name in ("type", "jovial")}
    default_state = {"type": "0", "jovial": "0"}
    if tier == "million":
        state_properties["rainbow"] = {"type": "boolean"}
        default_state["rainbow"] = "false"
    return {"template": "minecraft:furnace", "properties": properties,
            "state_properties": state_properties, "default_state": default_state,
            "state_light": {"property": "lit", "values": {"true": 14, "false": 0}, "default": 0}}


def expected_states(block: str) -> set[tuple[tuple[str, str], ...]]:
    """The complete state space, independent of JSON property ordering."""
    if block == "heater":
        return {()}
    properties = {"facing": ("north", "east", "south", "west"),
                  "jovial": ("0", "1", "2"), "lit": ("false", "true"),
                  "type": ("0", "1", "2")}
    if block == "million_furnace":
        properties["rainbow"] = ("false", "true")
    names = sorted(properties)
    return {tuple(zip(names, values)) for values in product(*(properties[name] for name in names))}


def item_definition(item: str) -> dict:
    definition = {"template": "minecraft:paper", "max_stack_size": 64, "rarity": "common"}
    if item in {"item_heater", "item_copy", "rainbow_coal"}:
        definition["max_stack_size"] = 1
    if item == "rainbow_coal":
        definition["components"] = {
            "minecraft:max_damage": 5120, "minecraft:damage": 0,
            "minecraft:cooking_fuel": {"burn_time": 200, "speed_multiplier": 1.0},
        }
    return definition


def heater_loot() -> dict:
    # Java manually drops the heater and consequently has an empty loot table.
    # Horizon uses its normal block loot path; the native companion handles
    # energy/component preservation without spawning a second block item.
    return {"type": "minecraft:block", "pools": [{
        "rolls": 1, "conditions": [{"condition": "minecraft:survives_explosion"}],
        "entries": [{"type": "minecraft:item", "name": "ironfurnaces:heater",
                     "functions": [{"function": "minecraft:copy_components",
                                    "source": "block_entity", "include": ["minecraft:custom_name"]}]}],
    }]}


def convert_loot(value):
    """Update the upstream legacy loot syntax to Horizon's 26.3 codec.

    The upstream Fabric folder still uses `condition`/`function` as object
    discriminators. In 26.3 those discriminators are `type`; lists move to
    `condition` and `modifier`. Leaving the old discriminator silently makes
    Horizon reject the pool, so the mined block would never drop itself.
    """
    if isinstance(value, list):
        return [convert_loot(item) for item in value]
    if not isinstance(value, dict):
        return value
    converted = {}
    for key, item in value.items():
        if key == "function" or (key == "condition" and isinstance(item, str) and "type" not in value):
            key = "type"
        elif key == "functions":
            key = "modifier"
        elif key == "conditions":
            key = "condition"
        converted[key] = convert_loot(item)
        if key in ("condition", "modifier") and isinstance(converted[key], list) and len(converted[key]) == 1:
            converted[key] = converted[key][0]
    return converted


def collect_content(upstream: Path = DEFAULT_UPSTREAM, *, with_silver: bool = False) -> dict[str, bytes]:
    """Produce the deterministic package file map without writing."""
    resources = source_resources(upstream)
    assets = resources / "assets" / "ironfurnaces"
    files = {"horizon-mod.toml": MANIFEST.encode("utf-8")}
    # A declared result slot prevents players from inserting arbitrary outputs
    # into this custom block entity through the generic menu transaction path.
    files["data/ironfurnaces/horizon/menu/furnace.json"] = encoded({
        "slots": 3, "data_slots": 4, "results": [2],
    })
    # Keep the v1 menu for already-open/save migration paths. The v2 companion
    # selects the common machine inventory so mode switches preserve all slots.
    files["data/ironfurnaces/horizon/menu/machine.json"] = encoded({
        "slots": 19, "data_slots": 64, "results": [2, 13, 14, 15, 16, 17, 18],
    })
    files["data/ironfurnaces/horizon/menu/heater.json"] = encoded({
        "slots": 1, "data_slots": 64, "results": [],
    })
    for path in sorted(assets.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(assets)
        # Every upstream block/item entry point is supported by the v2 companion;
        # copy the original models, skins, animations and GUI atlas unchanged.
        files[(Path("assets/ironfurnaces") / relative).as_posix()] = path.read_bytes()
    english_path = "assets/ironfurnaces/lang/en_us.json"
    english = json.loads(files[english_path])
    english.update({f"gui.ironfurnaces.{key}": value for key, value in UI_LABELS.items()})
    files[english_path] = encoded(modified(english, "added Horizon machine settings labels"))
    for tier in TIERS:
        block = f"{tier}_furnace"
        files[f"data/ironfurnaces/horizon/block/{block}.json"] = encoded(block_definition(tier))
        loot = resources / "data/ironfurnaces/loot_table/blocks" / f"{block}.json"
        files[f"data/ironfurnaces/loot_table/blocks/{block}.json"] = encoded(modified(convert_loot(read_json(loot)), "adapted loot component identifiers for Horizon"))
    files["data/ironfurnaces/horizon/block/heater.json"] = encoded({
        "template": "minecraft:iron_block",
        "properties": {"hardness": 5.0, "explosion_resistance": 6.0,
                       "requires_tool": True, "sounds": "minecraft:iron_block"},
    })
    files["data/ironfurnaces/loot_table/blocks/heater.json"] = encoded(modified(convert_loot(heater_loot()), "adapted heater drops for Horizon"))
    for item in sorted(ITEMS):
        files[f"data/ironfurnaces/horizon/item/{item}.json"] = encoded(item_definition(item))
    recipes = resources / "data/ironfurnaces/recipe"
    generator_energy = {}
    for path in sorted(recipes.rglob("*.json")):
        recipe = read_json(path)
        result = recipe.get("result", {}).get("id", "")
        if recipe.get("type") == "ironfurnaces:generator_blasting":
            files[f"data/ironfurnaces/recipe/{path.relative_to(recipes).as_posix()}"] = path.read_bytes()
            ingredient, energy = recipe["ingredient"], recipe["energy"]
            if not isinstance(ingredient, str) or not isinstance(energy, int) or energy <= 0:
                raise ValueError(f"Unsupported generator recipe: {path}")
            if ingredient in generator_energy:
                raise ValueError(f"Duplicate generator ingredient: {ingredient}")
            generator_energy[ingredient] = energy
            continue
        if result.removeprefix("ironfurnaces:") not in REGISTERED:
            continue
        if path.name in {"rainbow_plating.json", "rainbow_plating_no_silver.json"}:
            chosen = "rainbow_plating.json" if with_silver else "rainbow_plating_no_silver.json"
            if path.name != chosen:
                continue
            target = "rainbow_plating.json"
        else:
            target = path.relative_to(recipes).as_posix()
        files[f"data/ironfurnaces/recipe/{target}"] = encoded(modified(convert_conditions(recipe), "adapted loader conditions and selected the pack's plating recipe"))
    # The native companion reads the same source costs in one resource. Keep
    # the original recipes too, for recipe integrations and future loaders.
    files["data/ironfurnaces/generator_blasting.json"] = encoded(generator_energy)
    for namespace in ("c", "ironfurnaces"):
        tag_root = resources / "data" / namespace / "tags/item"
        for path in sorted(tag_root.rglob("*.json")):
            files[f"data/{namespace}/tags/item/{path.relative_to(tag_root).as_posix()}"] = path.read_bytes()
    for tag, values in VANILLA_TAGS.items():
        files[f"data/c/tags/item/{tag}.json"] = encoded({"replace": False, "values": values})
    files["data/minecraft/tags/block/mineable/pickaxe.json"] = encoded({
        "replace": False, "values": [f"ironfurnaces:{tier}_furnace" for tier in TIERS] + ["ironfurnaces:heater"]})
    for tag, tiers in {
        "needs_stone_tool": ["copper", "iron", "silver", "million", "allthemodium", "vibranium", "unobtainium"],
        "needs_iron_tool": ["gold", "diamond", "emerald"],
        "needs_diamond_tool": ["obsidian", "netherite"],
    }.items():
        files[f"data/minecraft/tags/block/{tag}.json"] = encoded({
            "replace": False, "values": [f"ironfurnaces:{tier}_furnace" for tier in tiers] +
            (["ironfurnaces:heater"] if tag == "needs_stone_tool" else [])})
    validate_content(files)
    return files


def validate_content(files: dict[str, bytes]) -> None:
    """Check registry consistency, crafting inputs, state coverage and assets."""
    manifest = tomllib.loads(files["horizon-mod.toml"].decode("utf-8"))
    if manifest["id"] != "ironfurnaces" or manifest["api"] != "0.2":
        raise ValueError("Unexpected content manifest")
    parsed = {}
    for name, payload in files.items():
        if name != "horizon-mod.toml" and not re.fullmatch(r"(?:assets|data)/[a-z0-9_./-]+", name):
            raise ValueError(f"Invalid content path: {name}")
        if name.endswith((".json", ".mcmeta")):
            parsed[name] = json.loads(payload.decode("utf-8-sig"))
    for block in BLOCKS:
        state = parsed[f"assets/ironfurnaces/blockstates/{block}.json"]
        actual = {tuple(sorted(tuple(pair.split("=", 1)) for pair in key.split(",") if pair))
                  for key in state["variants"]}
        if actual != expected_states(block):
            raise ValueError(f"Missing or unsupported block states: {block}")
    for item in REGISTERED:
        if f"assets/ironfurnaces/items/{item}.json" not in parsed:
            raise ValueError(f"Missing item model: {item}")
    for name, value in parsed.items():
        if name.startswith("assets/"):
            for ref in strings(value):
                if not ref.startswith("ironfurnaces:"):
                    continue
                suffix = ref.split(":", 1)[1]
                if not suffix.startswith(("block/", "item/")):
                    continue
                if (f"assets/ironfurnaces/models/{suffix}.json" not in files and
                        f"assets/ironfurnaces/textures/{suffix}.png" not in files):
                    raise ValueError(f"Missing asset {ref} referenced by {name}")
        if name.startswith("data/ironfurnaces/recipe/"):
            for ref in strings(value):
                if ref.startswith("ironfurnaces:") and ref.split(":", 1)[1] not in REGISTERED and ref != "ironfurnaces:generator_blasting":
                    raise ValueError(f"Recipe references unregistered item {ref}: {name}")
                if ref.startswith("#") and not ref.startswith("#minecraft:") and ":" in ref:
                    tag = ref[1:]
                    namespace, path = tag.split(":", 1)
                    if tag not in EXTERNAL_TAGS and f"data/{namespace}/tags/item/{path}.json" not in files:
                        raise ValueError(f"Missing ingredient tag {tag}: {name}")


def build_content(upstream: Path = DEFAULT_UPSTREAM, output: Path = DEFAULT_OUTPUT,
                  *, with_silver: bool = False) -> dict[str, bytes]:
    files = collect_content(upstream, with_silver=with_silver)
    output = output.resolve()
    # Refuse unknown stale content instead of deleting arbitrary output files.
    for root in (output / "assets", output / "data"):
        if root.exists():
            stale = [p for p in root.rglob("*") if p.is_file() and p.relative_to(output).as_posix() not in files]
            if stale:
                raise ValueError(f"Output contains unknown files; choose a fresh --output: {stale[0]}")
    for name, payload in files.items():
        path = output / name
        if path.is_symlink() or any(parent.is_symlink() for parent in path.parents):
            raise ValueError(f"Refusing a symlink output: {path}")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)
    return files


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", type=Path, default=DEFAULT_UPSTREAM,
                        help="Iron Furnaces repository root or Fabric 26.3 version folder")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--with-silver", action="store_true", help="Use the silver-dependent rainbow plating recipe")
    args = parser.parse_args()
    files = build_content(args.upstream, args.output, with_silver=args.with_silver)
    recipes = sum(name.startswith("data/ironfurnaces/recipe/") for name in files)
    print(f"Built {args.output.resolve()}: {len(FURNACES)} furnaces + heater, {len(ITEMS)} items, {recipes} recipes, {len(files)} files")


if __name__ == "__main__":
    main()
