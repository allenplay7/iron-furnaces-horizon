// Copyright 2026 Astryxion; Copyright 2026 Iron Furnaces Horizon port contributors.
// SPDX-License-Identifier: Apache-2.0
// Adapted from Fabric 26.3 FurnaceSettings, DirectionUtil, ItemFurnaceCopy and
// BlockIronFurnaceTileBase. See NOTICE. Inventory transfers use Horizon's own
// sided-container transactions; no neighbor inventory is edited directly.
use super::{Game, ironfurnaces as runtime, ironfurnaces_config as config, iron_furnaces_rules::Tier};
use horizon_client::{containers::{BlockEntity, ModBlockEntity}, items::{ItemStack, Items}, text::Text};
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

const SETTINGS: [&str; 12] = ["Side0", "Side1", "Side2", "Side3", "Side4", "Side5",
    "AutoInput", "AutoOutput", "RedstoneMode", "RedstoneValue", "AugmentTab", "AutoSplit"];
const DIRECTIONS: [Direction; 6] = [Direction::Down, Direction::Up, Direction::North,
    Direction::South, Direction::West, Direction::East];
const COPY_KEY: &str = "ironfurnaces:settings";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Machine { Furnace, Factory, Generator }

fn machine(entity: &ModBlockEntity) -> Machine {
    match entity.items.get(5).and_then(Option::as_ref).filter(|s| s.count > 0).map(|s| &*s.item) {
        Some("ironfurnaces:augment_factory") => Machine::Factory,
        Some("ironfurnaces:augment_generator") => Machine::Generator,
        _ => Machine::Furnace,
    }
}
fn number(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&JavaString::from(key)) { Some(Tag::Int(n)) => *n, _ => 0 }
}
fn put(nbt: &mut Compound, key: &str, value: i32) { nbt.insert(key.into(), Tag::Int(value)); }

pub fn face_index(face: Direction) -> usize {
    match face { Direction::Down => 0, Direction::Up => 1, Direction::North => 2,
        Direction::South => 3, Direction::West => 4, Direction::East => 5 }
}
fn opposite(face: Direction) -> Direction { DIRECTIONS[face_index(face) ^ 1] }
fn neighbor(pos: [i32; 3], face: Direction) -> [i32; 3] {
    let step = match face { Direction::Down => [0,-1,0], Direction::Up => [0,1,0],
        Direction::North => [0,0,-1], Direction::South => [0,0,1],
        Direction::West => [-1,0,0], Direction::East => [1,0,0] };
    [pos[0] + step[0], pos[1] + step[1], pos[2] + step[2]]
}
pub fn side(entity: &ModBlockEntity, index: usize) -> i32 {
    SETTINGS.get(index).filter(|_| index < 6).map_or(0, |key| number(&entity.nbt, key).clamp(0, 4))
}

/// Combine this with runtime::accepts. A None face is an unsided/internal
/// insertion; menu routing still checks the active mode's slots separately.
pub fn can_insert_through(entity: &ModBlockEntity, slot: usize, face: Option<Direction>) -> bool {
    let Some(face) = face else { return true; };
    match (machine(entity), side(entity, face_index(face))) {
        (Machine::Furnace, 1 | 3) => slot == 0 || slot == 1,
        (Machine::Furnace, 4) => slot == 1,
        (Machine::Factory, 1 | 3) => (7..=12).contains(&slot),
        (Machine::Generator, 4) => slot == 6,
        _ => false,
    }
}

/// A dynamic restriction after Containers::can_take. Empty buckets leave a
/// fuel face; actual fuel never does. Auto-output also removes spent water
/// buckets, matching upstream's separate remainder-output path.
pub fn can_extract_through(entity: &ModBlockEntity, slot: usize, stack: &ItemStack, face: Direction) -> bool {
    match (machine(entity), side(entity, face_index(face))) {
        (Machine::Furnace, 2 | 3) => slot == 2,
        (Machine::Furnace, 4) => slot == 1 && &*stack.item == "minecraft:bucket",
        (Machine::Factory, 2 | 3) => (13..=18).contains(&slot) && factory_inputs(entity).contains(&(slot - 6)),
        (Machine::Generator, 4) => slot == 6 && &*stack.item == "minecraft:bucket",
        _ => false,
    }
}

fn factory_inputs(entity: &ModBlockEntity) -> Vec<usize> {
    Tier::from_block(&entity.id).map_or_else(Vec::new, |tier|
        config::active_factory_lanes(tier).map(|lane| lane + 7).collect())
}

/// This only runs on the authority while the ticking block entity is detached.
/// Reserve local capacity before extraction, so a refused destination never
/// destroys items. The native helpers enforce the neighbor's side rules,
/// component equality, double-chest layout and container change notifications.
pub fn tick_io(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if Tier::from_block(&entity.id).is_none() { return; }
    let auto_in = number(&entity.nbt, "AutoInput") == 1;
    let auto_out = number(&entity.nbt, "AutoOutput") == 1;
    if !auto_in && !auto_out { return; }
    if entity.items.len() < 19 { entity.items.resize(19, None); }
    for face in DIRECTIONS {
        let setting = side(entity, face_index(face));
        let other = neighbor(pos, face);
        let other_face = opposite(face);
        let mode = machine(entity);
        if auto_in {
            let targets = match (mode, setting) {
                (Machine::Furnace, 1 | 3) => vec![0],
                (Machine::Furnace, 4) => vec![1],
                (Machine::Generator, 4) => vec![6],
                (Machine::Factory, 1 | 3) => factory_inputs(entity),
                _ => vec![],
            };
            for target in targets { pull(game, entity, target, other, other_face); }
        }
        if auto_out {
            match (mode, setting) {
                (Machine::Furnace, 2 | 3) => push(game, entity, 2, other, other_face),
                (Machine::Factory, 2 | 3) => {
                    for input in factory_inputs(entity) { push(game, entity, input + 6, other, other_face); }
                }
                (Machine::Furnace, 4) | (Machine::Generator, 4) => {
                    let slot = if mode == Machine::Generator { 6 } else { 1 };
                    if entity.items.get(slot).and_then(Option::as_ref).is_some_and(|stack| {
                        if matches!(&*stack.item, "minecraft:bucket" | "minecraft:water_bucket") { return true; }
                        !runtime::accepts(game, entity, slot, stack)
                    }) { push(game, entity, slot, other, other_face); }
                }
                _ => {}
            }
        }
    }
}

fn pull(game: &mut Game<'_>, entity: &mut ModBlockEntity, target: usize, other: [i32; 3], face: Direction) {
    let dimension = game.native_dimension();
    let Some(slots) = game.native_container_slots(dimension, other, face) else { return; };
    for source in slots {
        let Some(stack) = source.stack.filter(|s| s.count > 0) else { continue; };
        if !source.can_take || !runtime::accepts(game, entity, target, &stack) { continue; }
        // Empty buckets are manually insertable for wet sponges, but upstream
        // automatic fueling only pulls actual fuel/receivers.
        if target == 1 && &*stack.item == "minecraft:bucket" { continue; }
        let max = runtime::effective_stack_max(game, &stack);
        let room = match &entity.items[target] {
            None => max,
            Some(existing) if existing.count == 0 => max,
            Some(existing) if existing.same_item_same_components(&stack) => max.saturating_sub(existing.count),
            _ => 0,
        };
        if room == 0 { continue; }
        // No await or mutating call occurs between the capacity decision and
        // extraction. The engine returns this very stack, with its components.
        let Some(taken) = game.native_extract_item(dimension, other, face, source.slot, room.min(stack.count)) else { continue; };
        match &mut entity.items[target] {
            Some(existing) if existing.count > 0 => existing.count += taken.count,
            target => *target = Some(taken),
        }
    }
}
fn push(game: &mut Game<'_>, entity: &mut ModBlockEntity, source: usize, other: [i32; 3], face: Direction) {
    let Some(stack) = entity.items.get_mut(source).and_then(Option::take) else { return; };
    entity.items[source] = game.native_insert_item(other, face, stack);
}

pub fn permits_run(mode: i32, powered: bool) -> bool {
    match mode { 1 => powered, 2 => !powered, _ => true }
}
pub fn allows_run(game: &mut Game<'_>, pos: [i32; 3], entity: &ModBlockEntity) -> bool {
    let mode = number(&entity.nbt, "RedstoneMode");
    if mode != 1 && mode != 2 { return true; }
    permits_run(mode, game.native_neighbor_signal(game.native_dimension(), pos) > 0)
}

fn stack_max(items: &Items, stack: &ItemStack) -> u32 {
    let data = stack.data();
    if data.is_some_and(|d| d.other("!minecraft:max_stack_size").is_some()) { return 1; }
    if let Some(Tag::Int(n)) = data.and_then(|d| d.other("minecraft:max_stack_size"))
        && (1..=99).contains(n) { return (*n as u32).min(64); }
    items.get(&stack.item).map_or(64, |item| item.max_stack).clamp(1, 64)
}
fn comparator_slots(entity: &ModBlockEntity) -> Vec<usize> {
    // Use the active inventory, not Java's accidental unbraced addition of
    // inactive fuel/output slots in factory and generator mode.
    match machine(entity) {
        Machine::Furnace => vec![0, 1, 2],
        Machine::Generator => vec![6],
        Machine::Factory => factory_inputs(entity).iter().flat_map(|&slot| [slot, slot + 6]).collect(),
    }
}
fn fullness(entity: &ModBlockEntity, max: impl Fn(&ItemStack) -> u32) -> i32 {
    let slots = comparator_slots(entity);
    let mut total = 0.0f32;
    let mut occupied = false;
    for &slot in &slots {
        if let Some(stack) = entity.items.get(slot).and_then(Option::as_ref).filter(|s| s.count > 0) {
            total += stack.count as f32 / max(stack).clamp(1, 64) as f32;
            occupied = true;
        }
    }
    ((total / slots.len() as f32 * 14.0).floor() as i32 + i32::from(occupied)).clamp(0, 15)
}
pub fn comparator(entity: &ModBlockEntity, items: &Items) -> i32 { fullness(entity, |stack| stack_max(items, stack)) }
pub fn emitted_signal(entity: &ModBlockEntity, items: &Items) -> i32 {
    match number(&entity.nbt, "RedstoneMode") {
        3 => comparator(entity, items),
        4 => (comparator(entity, items) - number(&entity.nbt, "RedstoneValue").clamp(0, 15)).max(0),
        _ => 0,
    }
}

/// Call after the engine restores a detached entity, never during tick_io:
/// neighbor queries must observe the actual inventory and settings.
pub fn after_restore(game: &mut Game<'_>, pos: (i32, i32, i32)) {
    if !game.native_on(runtime::ID) { return; }
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&pos) else { return; };
    if Tier::from_block(&entity.id).is_none() { return; }
    let analog = comparator(entity, game.items());
    let signal = emitted_signal(entity, game.items());
    let analog_changed = number(&entity.nbt, "LastComparator") != analog;
    let signal_changed = number(&entity.nbt, "LastSignal") != signal;
    if !analog_changed && !signal_changed { return; }
    if let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get_mut(&pos) {
        put(&mut entity.nbt, "LastComparator", analog);
        put(&mut entity.nbt, "LastSignal", signal);
    }
    game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    let position = [pos.0, pos.1, pos.2];
    let dimension = game.native_dimension();
    if analog_changed { game.native_update_output_neighbors(dimension, position); }
    if signal_changed { game.native_update_signal_neighbors(dimension, position); }
}

fn settings_snapshot(entity: &ModBlockEntity, facing: &str) -> Compound {
    let mut settings = Compound::new();
    for key in SETTINGS { put(&mut settings, key, number(&entity.nbt, key)); }
    settings.insert("Facing".into(), Tag::String(facing.into()));
    settings
}
fn apply_settings(entity: &mut ModBlockEntity, settings: &Compound) {
    for (index, key) in SETTINGS.into_iter().enumerate() {
        let max = match index { 0..=5 | 8 => 4, 9 => 15, _ => 1 };
        put(&mut entity.nbt, key, number(settings, key).clamp(0, max));
    }
}
fn feedback(game: &mut Game<'_>, message: &str) {
    if let Some(player) = game.native_acting_player() {
        game.native_tell(player.uuid, Text::literal(message));
    } else if game.native_window() { game.native_chat(message); }
}
pub fn setting_copy(game: &mut Game<'_>, pos: [i32; 3], held: &ItemStack, sneak: bool) -> bool {
    if &*held.item != "ironfurnaces:item_copy" { return false; }
    let key = (pos[0], pos[1], pos[2]);
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&key) else { return false; };
    if Tier::from_block(&entity.id).is_none() { return false; }
    if sneak {
        let Some(Tag::Compound(settings)) = held.custom_data().and_then(|data| data.get(&JavaString::from(COPY_KEY))) else {
            feedback(game, "Copy furnace settings with a normal right click first.");
            return true;
        };
        let settings = settings.clone();
        game.native_with_mod_block_entity(pos, |entity| apply_settings(entity, &settings));
        if let Some(Tag::String(facing)) = settings.get(&JavaString::from("Facing")) {
            let facing = facing.to_string_lossy();
            if matches!(facing.as_str(), "north" | "south" | "west" | "east") {
                game.native_set_block_property(pos, "facing", &facing);
            }
        }
        game.native_block_entity_sync(pos);
        after_restore(game, key);
        feedback(game, "Settings applied");
    } else {
        let facing = game.native_block_property(pos, "facing").unwrap_or_else(|| "north".into());
        let settings = settings_snapshot(entity, &facing);
        let mut custom = held.custom_data().cloned().unwrap_or_default();
        custom.insert(COPY_KEY.into(), Tag::Compound(settings));
        let mut updated = held.clone();
        updated.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
        *game.held_mut() = Some(updated);
        feedback(game, "Settings copied");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entity(mode: Machine) -> ModBlockEntity {
        let mut entity = ModBlockEntity::default();
        entity.id = "ironfurnaces:iron_furnace".into();
        entity.items.resize(19, None);
        let augment = match mode { Machine::Factory => Some("ironfurnaces:augment_factory"),
            Machine::Generator => Some("ironfurnaces:augment_generator"), Machine::Furnace => None };
        entity.items[5] = augment.map(|id| ItemStack::new(id, 1));
        entity
    }
    #[test]
    fn disabled_faces_and_augment_slots_never_allow_automation() {
        for mode in [Machine::Furnace, Machine::Factory, Machine::Generator] {
            let mut entity = entity(mode);
            for slot in 0..19 { assert!(!can_insert_through(&entity, slot, Some(Direction::Up))); }
            for value in 0..=4 {
                put(&mut entity.nbt, "Side1", value);
                for slot in [3,4,5] {
                    assert!(!can_insert_through(&entity, slot, Some(Direction::Up)));
                    assert!(!can_extract_through(&entity, slot, &ItemStack::new("minecraft:bucket",1), Direction::Up));
                }
            }
        }
    }
    #[test]
    fn fuel_face_only_extracts_spent_empty_bucket() {
        for (mode, slot) in [(Machine::Furnace,1), (Machine::Generator,6)] {
            let mut entity = entity(mode);
            put(&mut entity.nbt, "Side0", 4);
            assert!(can_insert_through(&entity, slot, Some(Direction::Down)));
            assert!(can_extract_through(&entity, slot, &ItemStack::new("minecraft:bucket",1), Direction::Down));
            for id in ["minecraft:coal", "minecraft:lava_bucket", "ironfurnaces:rainbow_coal"] {
                assert!(!can_extract_through(&entity, slot, &ItemStack::new(id,1), Direction::Down));
            }
        }
    }
    #[test]
    fn comparator_only_counts_active_slots_and_respects_nonstackable_fuel() {
        let mut factory = entity(Machine::Factory);
        for slot in [9,10,15,16] { factory.items[slot] = Some(ItemStack::new("minecraft:stone",64)); }
        assert_eq!(fullness(&factory, |_|64),15);
        let mut generator = entity(Machine::Generator);
        generator.items[6] = Some(ItemStack::new("minecraft:lava_bucket",1));
        assert_eq!(fullness(&generator, |_|1),15);
        generator.items[6] = None;
        generator.items[1] = Some(ItemStack::new("minecraft:stone",64));
        assert_eq!(fullness(&generator, |_|64),0);
    }
    #[test]
    fn settings_copy_preserves_inventory_and_clamps_values() {
        let mut source = entity(Machine::Factory);
        for (i,key) in SETTINGS.iter().enumerate() { put(&mut source.nbt,key,i as i32); }
        let settings = settings_snapshot(&source,"west");
        let mut destination = entity(Machine::Generator);
        let before = destination.items.clone();
        apply_settings(&mut destination,&settings);
        assert_eq!(destination.items,before);
        assert_eq!(number(&destination.nbt,"Side5"),4);
        assert_eq!(number(&destination.nbt,"RedstoneMode"),4);
        assert_eq!(number(&destination.nbt,"RedstoneValue"),9);
        assert_eq!(number(&destination.nbt,"AutoSplit"),1);
    }
    #[test]
    fn redstone_high_low_and_output_modes() {
        assert!(permits_run(1,true)); assert!(!permits_run(1,false));
        assert!(permits_run(2,false)); assert!(!permits_run(2,true));
        for mode in [0,3,4] { assert!(permits_run(mode,false)); assert!(permits_run(mode,true)); }
    }
}
