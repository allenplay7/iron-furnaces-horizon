// SPDX-License-Identifier: Apache-2.0
// Narrow engine compatibility for removal and redstone services without typed counterparts.
use super::{Game, GameLevel};
use horizon_client::{
    containers::{BlockEntity, ModBlockEntity},
    items::{ItemStack, Items},
};
use horizon_core::{
    nbt::{Compound, Tag},
    position::BlockPos,
};
use iron_furnaces_horizon::{config, rules::Tier};
fn number(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(n)) => *n,
        _ => 0,
    }
}
fn mode(e: &ModBlockEntity) -> i32 {
    match e
        .items
        .get(5)
        .and_then(Option::as_ref)
        .map(|s| s.item.as_ref())
    {
        Some("ironfurnaces:augment_factory") => 1,
        Some("ironfurnaces:augment_generator") => 2,
        _ => 0,
    }
}
fn max(items: &Items, s: &ItemStack) -> u32 {
    if s.data()
        .is_some_and(|d| d.other("!minecraft:max_stack_size").is_some())
    {
        return 1;
    }
    if let Some(Tag::Int(n)) = s.data().and_then(|d| d.other("minecraft:max_stack_size")) {
        return (*n).clamp(1, 64) as u32;
    }
    items.get(&s.item).map_or(64, |i| i.max_stack).clamp(1, 64)
}
pub fn comparator(e: &ModBlockEntity, items: &Items) -> i32 {
    let Some(tier) = Tier::from_block(e.id()) else {
        return 0;
    };
    let slots: Vec<usize> = match mode(e) {
        1 => config::active_factory_lanes(tier)
            .flat_map(|i| [i + 7, i + 13])
            .collect(),
        2 => vec![6],
        _ => vec![0, 1, 2],
    };
    let mut total = 0f32;
    let mut occupied = false;
    for &slot in &slots {
        if let Some(s) = e.items.get(slot).and_then(Option::as_ref) {
            total += s.count as f32 / max(items, s) as f32;
            occupied = true;
        }
    }
    ((total / slots.len() as f32 * 14.0).floor() as i32 + i32::from(occupied)).clamp(0, 15)
}
pub fn emitted(e: &ModBlockEntity, items: &Items) -> i32 {
    match number(&e.nbt, "RedstoneMode") {
        3 => comparator(e, items),
        4 => (comparator(e, items) - number(&e.nbt, "RedstoneValue").clamp(0, 15)).max(0),
        _ => 0,
    }
}
pub fn signal(level: &GameLevel<'_>, pos: BlockPos, vanilla: i32, analog: bool) -> i32 {
    if let Some(BlockEntity::Mod(e)) = level.world.block_entities.get(&(pos.x, pos.y, pos.z)) {
        if Tier::from_block(e.id()).is_some() {
            return if analog {
                comparator(e, level.items)
            } else {
                emitted(e, level.items)
            };
        }
    }
    vanilla
}
pub fn restored(game: &mut Game<'_>, pos: (i32, i32, i32)) {
    let Some(BlockEntity::Mod(e)) = game.world.block_entities.get(&pos) else {
        return;
    };
    if Tier::from_block(e.id()).is_none() {
        return;
    }
    let analog = comparator(e, game.items());
    let signal = emitted(e, game.items());
    let ac = number(&e.nbt, "LastComparator") != analog;
    let sc = number(&e.nbt, "LastSignal") != signal;
    if !ac && !sc {
        return;
    }
    if let Some(BlockEntity::Mod(e)) = game.world.block_entities.get_mut(&pos) {
        e.nbt.insert("LastComparator".into(), Tag::Int(analog));
        e.nbt.insert("LastSignal".into(), Tag::Int(signal));
    }
    game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    let Some(level) = game.level(game.dim.id()) else { return; };
    let point = [pos.0, pos.1, pos.2];
    let state = game.world.block(pos.0,pos.1,pos.2);
    if ac {
        let _=game.level_update_neighbors(level,point,state,horizon_core::neighbour_update::NeighbourUpdate::OutputSignal);
    }
    if sc {
        let _=game.level_update_neighbors(level,point,state,horizon_core::neighbour_update::NeighbourUpdate::Neighbours);
    }
}
pub fn remove(level: &mut GameLevel<'_>, pos: BlockPos, side_effects: bool) {
    if !side_effects {
        return;
    }
    let Some(BlockEntity::Mod(e)) = level.world.block_entities.get_mut(&(pos.x, pos.y, pos.z))
    else {
        return;
    };
    if Tier::from_block(e.id()).is_none() {
        return;
    }
    if let Some(Tag::Compound(used)) = e.nbt.remove(&"RecipesUsed".into()) {
        for (id, value) in used {
            let count = match value {
                Tag::Int(n) => n.max(0) as u32,
                _ => 0,
            };
            let xp = level.recipes.cooking_experience(&id.to_string_lossy());
            let points =
                horizon_client::containers::create_experience(count, xp, &mut level.world.random);
            level.award_orbs(
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                points,
            );
        }
    }
}
