// SPDX-License-Identifier: Apache-2.0
use crate::{state::*, ui};
use mod_api::{Identifier, ModBlockEntity, ModBlockEntityView, Tag, testing::FakeHost};

fn host() -> FakeHost {
    let mut host = FakeHost::new();
    host.registries
        .resolve_items(
            [
                "minecraft:iron_ore",
                "minecraft:coal",
                "minecraft:bucket",
                "ironfurnaces:augment_blasting",
                "ironfurnaces:augment_speed",
                "ironfurnaces:augment_factory",
                "ironfurnaces:item_heater",
                "ironfurnaces:rainbow_coal",
            ]
            .map(|s| Identifier::parse(s).unwrap()),
        )
        .unwrap();
    host
}
#[test]
fn legacy_augments_move_without_overwriting_new_slots() {
    let host = host();
    let mut saved = ModBlockEntity::new("ironfurnaces:iron_furnace".into(), Default::default());
    saved.nbt.insert(
        "Mode".into(),
        Tag::String("ironfurnaces:augment_blasting".into()),
    );
    saved.nbt.insert(
        "Efficiency".into(),
        Tag::String("ironfurnaces:augment_speed".into()),
    );
    saved.items = vec![
        stack(&host, "minecraft:iron_ore", 31),
        stack(&host, "minecraft:coal", 63),
        None,
    ];
    let mut e = ModBlockEntityView::new(&mut saved);
    initialize(&host, &mut e);
    assert_eq!(e.items.len(), 19);
    assert_eq!(e.items[0].as_ref().unwrap().count(), 31);
    assert_eq!(item(&host, &e, 3), "ironfurnaces:augment_blasting");
    assert_eq!(item(&host, &e, 4), "ironfurnaces:augment_speed");
    assert_eq!(number(&e.nbt, "Version"), 2);
    assert_eq!(number(&e.nbt, "Side0"), 2);
    assert_eq!(number(&e.nbt, "Side1"), 1);
    initialize(&host, &mut e);
    assert_eq!(e.items[1].as_ref().unwrap().count(), 63);
}
#[test]
fn consuming_last_typed_item_leaves_an_empty_slot() {
    let host = host();
    let mut s = stack(&host, "minecraft:coal", 2);
    consume(&mut s, 1);
    assert_eq!(s.as_ref().unwrap().count(), 1);
    consume(&mut s, 1);
    assert!(s.is_none());
    consume(&mut s, 1);
    assert!(s.is_none());
}
#[test]
fn wireless_receiver_retains_other_components_and_negative_coordinates() {
    let host = host();
    let mut saved = ModBlockEntity::new("ironfurnaces:heater".into(), Default::default());
    let mut e = ModBlockEntityView::new(&mut saved);
    initialize(&host, &mut e);
    let mut receiver = stack(&host, "ironfurnaces:item_heater", 1).unwrap();
    let mut custom = mod_api::Compound::new();
    custom.insert("foreign".into(), Tag::Int(42));
    crate::tools::set_custom(&mut receiver, custom);
    e.items[0] = Some(receiver);
    crate::power::heater(&host, [-40, 64, 100], &mut e);
    let receiver = e.items[0].as_ref().unwrap();
    assert_eq!(crate::power::link(&host, receiver), Some([-40, 64, 100]));
    assert_eq!(number(&crate::tools::custom(receiver), "foreign"), 42);
}
#[test]
fn recipe_fingerprint_detects_components_but_not_count_changes() {
    let host = host();
    let mut saved = ModBlockEntity::new("ironfurnaces:iron_furnace".into(), Default::default());
    let mut e = ModBlockEntityView::new(&mut saved);
    initialize(&host, &mut e);
    e.items[0] = stack(&host, "minecraft:iron_ore", 2);
    assert!(!fingerprint(&host, &mut e, 0, "", "minecraft:iron_ingot"));
    e.items[0].as_mut().unwrap().set_count(3).unwrap();
    assert!(fingerprint(&host, &mut e, 0, "", "minecraft:iron_ingot"));
    let mut custom = mod_api::Compound::new();
    custom.insert("quality".into(), Tag::Int(2));
    crate::tools::set_custom(e.items[0].as_mut().unwrap(), custom);
    assert!(!fingerprint(&host, &mut e, 0, "", "minecraft:iron_ingot"));
}
#[test]
fn every_factory_layout_hides_inactive_and_augment_slots() {
    for (tier, lanes) in [(0, 2..4), (1, 1..5), (2, 0..6)] {
        let mut data = vec![0; 64];
        data[4] = 1;
        data[5] = tier;
        let layout = ui::layout("ironfurnaces:iron_furnace", &data);
        for lane in 0..6 {
            assert_eq!(layout.slots[7 + lane].is_some(), lanes.contains(&lane));
            assert_eq!(layout.slots[13 + lane].is_some(), lanes.contains(&lane));
        }
        assert!((0..7).all(|i| layout.slots[i].is_none()));
        data[32] = 1;
        let layout = ui::layout("ironfurnaces:iron_furnace", &data);
        assert_eq!(layout.slots.iter().filter(|p| p.is_some()).count(), 3);
        assert!((3..6).all(|i| layout.slots[i].is_some()));
    }
}
#[test]
fn component_stack_limits_and_removed_limits_are_respected() {
    let host = host();
    let mut s = stack(&host, "minecraft:iron_ore", 1).unwrap();
    let mut patch = mod_api::Compound::new();
    patch.insert("minecraft:max_stack_size".into(), Tag::Int(16));
    s.set_components(patch).unwrap();
    assert_eq!(maximum(&host, &s), 16);
    let mut patch = mod_api::Compound::new();
    patch.insert(
        "!minecraft:max_stack_size".into(),
        Tag::Compound(Default::default()),
    );
    s.set_components(patch).unwrap();
    assert_eq!(maximum(&host, &s), 1);
}
// Saved vanilla inventory must travel through NBT: internal slots are mod-only in the new API.
#[test]
fn legacy_recipe_ids_preserve_paid_work_and_merge_earned_experience() {
    let host = host();
    let mut saved = ModBlockEntity::new("ironfurnaces:iron_furnace".into(), Default::default());
    let mut e = ModBlockEntityView::new(&mut saved);
    put(&mut e.nbt, "Version", 2);
    for key in ["Recipe", "Lane2Recipe"] {
        e.nbt.insert(key.into(), Tag::String("iron_ingot_from_smelting_raw_iron".into()));
    }
    e.nbt.insert("Lane3Recipe".into(), Tag::String("another:custom_recipe".into()));
    e.nbt.insert("Lane4Recipe".into(), Tag::String("".into()));
    for (key, value) in [("energy", 6575), ("LaneProgress2", 57), ("LaneReserved2", 2575)] {
        put(&mut e.nbt, key, value);
    }
    let mut used = mod_api::Compound::new();
    put(&mut used, "iron_ingot_from_smelting_raw_iron", 4);
    put(&mut used, "minecraft:iron_ingot_from_smelting_raw_iron", 2);
    put(&mut used, "another:custom_recipe", 3);
    e.nbt.insert("RecipesUsed".into(), Tag::Compound(used));
    initialize(&host, &mut e);
    for key in ["Recipe", "Lane2Recipe"] {
        assert_eq!(string(&e.nbt, key).as_deref(), Some("minecraft:iron_ingot_from_smelting_raw_iron"));
    }
    assert_eq!(string(&e.nbt, "Lane3Recipe").as_deref(), Some("another:custom_recipe"));
    assert_eq!(string(&e.nbt, "Lane4Recipe").as_deref(), Some(""));
    for (key, value) in [("energy", 6575), ("LaneProgress2", 57), ("LaneReserved2", 2575)] {
        assert_eq!(number(&e.nbt, key), value);
    }
    let Tag::Compound(used) = &e.nbt[&"RecipesUsed".into()] else { panic!("missing XP"); };
    assert_eq!(used.len(), 2);
    assert_eq!(number(used, "minecraft:iron_ingot_from_smelting_raw_iron"), 6);
    assert_eq!(number(used, "another:custom_recipe"), 3);
    let migrated = e.nbt.clone();
    initialize(&host, &mut e);
    assert_eq!(e.nbt, migrated);
}

#[test]
fn upgrade_inventory_roundtrip_preserves_slots_and_components() {
    let host = host();
    let mut items = vec![
        stack(&host, "minecraft:iron_ore", 31),
        stack(&host, "minecraft:coal", 63),
        None,
    ];
    let mut custom = mod_api::Compound::new();
    custom.insert("quality".into(), Tag::Int(7));
    crate::tools::set_custom(items[0].as_mut().unwrap(), custom);
    let mut nbt = mod_api::Compound::new();
    nbt.insert("Items".into(), encode_items(&host, &items).unwrap());
    let decoded = saved_items(&host, &nbt, 3).unwrap();
    assert_eq!(decoded, items);
    nbt.insert("Items".into(), Tag::List(vec![Tag::Int(2)]));
    assert!(saved_items(&host, &nbt, 3).is_none());
    items.push(stack(&host, "minecraft:bucket", 1));
    nbt.insert("Items".into(), encode_items(&host, &items).unwrap());
    assert!(saved_items(&host, &nbt, 3).is_none());
}
