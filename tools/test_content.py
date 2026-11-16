"""Content integrity tests against the actual upstream source and art."""
import json
from pathlib import Path
import unittest

import build_content as b


class ContentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = b.collect_content()
        cls.resources = b.source_resources(b.DEFAULT_UPSTREAM)

    def value(self, path):
        return json.loads(self.files[path])

    def test_all_original_furnace_modes_and_skins_are_present(self):
        self.assertEqual(len(b.FURNACES), 13)
        self.assertEqual(len(b.BLOCKS), 14)
        for block in b.FURNACES:
            state = self.value(f"assets/ironfurnaces/blockstates/{block}.json")
            self.assertEqual(len(state["variants"]), 144 if block == "million_furnace" else 72, block)
            original = self.resources / f"assets/ironfurnaces/blockstates/{block}.json"
            self.assertEqual(self.files[f"assets/ironfurnaces/blockstates/{block}.json"], original.read_bytes())
            definition = self.value(f"data/ironfurnaces/horizon/block/{block}.json")
            self.assertEqual(definition["template"], "minecraft:furnace")
            self.assertEqual(definition["state_light"]["values"]["true"], 14)
            for name in ("type", "jovial"):
                self.assertEqual(definition["state_properties"][name], {"type": "enum", "values": ["0", "1", "2"]})
                self.assertEqual(definition["default_state"][name], "0")
            if block == "million_furnace":
                self.assertEqual(definition["state_properties"]["rainbow"], {"type": "boolean"})
                self.assertEqual(definition["default_state"]["rainbow"], "false")
        heater = self.value("data/ironfurnaces/horizon/block/heater.json")
        self.assertEqual(heater["template"], "minecraft:iron_block")

    def test_registrations_match_all_upstream_entry_points(self):
        self.assertEqual(len(b.ITEMS), 29)
        self.assertEqual(len(b.REGISTERED), 43)
        self.assertEqual({p.stem for p in (self.resources / "assets/ironfurnaces/items").glob("*.json")}, b.REGISTERED)
        self.assertEqual({p.stem for p in (self.resources / "assets/ironfurnaces/blockstates").glob("*.json")}, b.BLOCKS)
        for block in b.BLOCKS:
            self.assertIn(f"data/ironfurnaces/horizon/block/{block}.json", self.files)
        for item in b.ITEMS:
            self.assertIn(f"data/ironfurnaces/horizon/item/{item}.json", self.files)

    def test_source_recipe_costs_and_output_counts_are_preserved(self):
        for name, payload in self.files.items():
            prefix = "data/ironfurnaces/recipe/"
            if not name.startswith(prefix):
                continue
            relative = name.removeprefix(prefix)
            if relative == "rainbow_plating.json":
                relative = "rainbow_plating_no_silver.json"
            original = b.read_json(self.resources / "data/ironfurnaces/recipe" / relative)
            converted = json.loads(payload)
            for key in ("type", "pattern", "key", "ingredients", "ingredient", "energy", "result"):
                self.assertEqual(converted.get(key), original.get(key), f"{name}: {key}")
            self.assertNotIn("fabric:load_conditions", converted)

    def test_external_materials_are_not_replaced_or_invented(self):
        for tag in b.EXTERNAL_TAGS:
            namespace, suffix = tag.split(":", 1)
            self.assertNotIn(f"data/{namespace}/tags/item/{suffix}.json", self.files)
        silver = self.value("data/ironfurnaces/recipe/furnaces/silver_furnace.json")
        self.assertIn("#c:ingots/silver", silver["key"].values())
        for material in ("allthemodium", "vibranium", "unobtainium"):
            recipe = self.value(f"data/ironfurnaces/recipe/furnaces/{material}_furnace.json")
            self.assertIn(f"#c:ingots/{material}", recipe["key"].values())
            self.assertEqual(recipe["neoforge:conditions"], [{"type": "neoforge:mod_loaded", "modid": "allthemodium"}])

    def test_rainbow_profile_has_exactly_one_plating_recipe(self):
        silver = b.collect_content(with_silver=True)
        for files, ingredient_count in ((self.files, 7), (silver, 8)):
            plating = [(name, json.loads(payload)) for name, payload in files.items()
                       if name.startswith("data/ironfurnaces/recipe/") and "rainbow_plating" in name]
            self.assertEqual(len(plating), 1)
            self.assertEqual(len(plating[0][1]["ingredients"]), ingredient_count)
            self.assertEqual(plating[0][1]["result"]["count"], 8)

    def test_new_features_have_original_recipes_and_item_limits(self):
        for relative in ("augments/augment_factory", "augments/augment_generator", "heater",
                         "item_heater", "item_copy", "item_spooky", "item_xmas"):
            self.assertIn(f"data/ironfurnaces/recipe/{relative}.json", self.files)
        for item in ("item_heater", "item_copy", "rainbow_coal"):
            self.assertEqual(self.value(f"data/ironfurnaces/horizon/item/{item}.json")["max_stack_size"], 1)
        coal = self.value("data/ironfurnaces/horizon/item/rainbow_coal.json")
        self.assertEqual(coal["components"]["minecraft:max_damage"], 5120)
        self.assertEqual(coal["components"]["minecraft:cooking_fuel"]["burn_time"], 200)
        # RGB are upstream augment class/slot categories, not three more items;
        # rainbow coal is obtained by exploding a rainbow furnace, not crafting.
        for nonexistent in ("augment_red", "augment_green", "augment_blue", "factory"):
            self.assertNotIn(nonexistent, b.REGISTERED)
        self.assertNotIn("data/ironfurnaces/recipe/rainbow_coal.json", self.files)

    def test_generator_energy_table_is_derived_from_all_source_recipes(self):
        source = self.resources / "data/ironfurnaces/recipe/generator_blasting"
        recipes = [b.read_json(path) for path in source.glob("*.json")]
        self.assertEqual(len(recipes), 12)
        self.assertEqual(self.value("data/ironfurnaces/generator_blasting.json"),
                         {recipe["ingredient"]: recipe["energy"] for recipe in recipes})
        for path in source.glob("*.json"):
            self.assertEqual(self.files[f"data/ironfurnaces/recipe/generator_blasting/{path.name}"], path.read_bytes())

    def test_every_block_drops_itself_and_has_mining_tags(self):
        pickaxe = self.value("data/minecraft/tags/block/mineable/pickaxe.json")["values"]
        self.assertIn("ironfurnaces:heater",self.value("data/minecraft/tags/block/needs_stone_tool.json")["values"])
        for block in b.BLOCKS:
            path = f"data/ironfurnaces/loot_table/blocks/{block}.json"
            loot = self.value(path)
            self.assertEqual(loot["pools"][0]["entries"][0]["name"], f"ironfurnaces:{block}")
            self.assertIn(f"ironfurnaces:{block}", pickaxe)
            if block != "heater":
                original=b.read_json(self.resources / path)
                self.assertEqual(loot.get("random_sequence"),original.get("random_sequence"))
                self.assertEqual(loot["pools"][0]["rolls"],original["pools"][0]["rolls"])

    def test_all_block_loot_uses_the_pinned_26_3_codec(self):
        # Horizon's condition evaluator only dispatches on object["type"].
        # Old {"condition":"..."} objects silently return false and lose drops.
        for block in b.BLOCKS:
            loot=self.value(f"data/ironfurnaces/loot_table/blocks/{block}.json")
            pool=loot["pools"][0]
            self.assertEqual(pool["condition"],{"type":"minecraft:survives_explosion"})
            entry=pool["entries"][0]
            self.assertEqual(entry["modifier"],{"type":"minecraft:copy_components",
                                                "include":["minecraft:custom_name"],"source":"block_entity"})
            self.assertNotIn("conditions",pool)
            self.assertNotIn("functions",entry)

    def test_all_original_textures_and_animation_metadata_are_unchanged(self):
        textures = self.resources / "assets/ironfurnaces/textures"
        for original in textures.rglob("*"):
            if original.is_file():
                name = "assets/ironfurnaces/textures/" + original.relative_to(textures).as_posix()
                self.assertEqual(self.files[name], original.read_bytes(), name)

    def test_missing_asset_and_bad_blockstate_are_detected(self):
        broken = dict(self.files)
        del broken["assets/ironfurnaces/textures/block/iron_furnace_front.png"]
        with self.assertRaisesRegex(ValueError, "Missing asset"):
            b.validate_content(broken)
        broken = dict(self.files)
        broken["assets/ironfurnaces/blockstates/iron_furnace.json"] = b.encoded({"variants": {"type=0": {}}})
        with self.assertRaisesRegex(ValueError, "Missing or unsupported block states"):
            b.validate_content(broken)

    def test_menu_output_is_result_only(self):
        menu = self.value("data/ironfurnaces/horizon/menu/furnace.json")
        self.assertEqual(menu, {"slots": 3, "data_slots": 4, "results": [2]})
        machine = self.value("data/ironfurnaces/horizon/menu/machine.json")
        self.assertEqual(machine, {"slots": 19, "data_slots": 64, "results": [2, 13, 14, 15, 16, 17, 18]})
        heater = self.value("data/ironfurnaces/horizon/menu/heater.json")
        self.assertEqual(heater, {"slots": 1, "data_slots": 64, "results": []})

    def test_unknown_load_condition_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "Unsupported upstream load condition"):
            b.convert_conditions({"fabric:load_conditions": [{"condition": "fabric:future_gate"}]})

    def test_repository_and_version_folder_produce_identical_files(self):
        self.assertEqual(b.collect_content(b.DEFAULT_UPSTREAM / "Fabric 26.3"), self.files)


if __name__ == "__main__":
    unittest.main()
