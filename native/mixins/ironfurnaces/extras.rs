// Copyright 2026 Astryxion; Copyright 2026 Iron Furnaces Horizon port contributors.
// SPDX-License-Identifier: Apache-2.0
use super::{Game, iron_furnaces_rules::Tier, ironfurnaces as runtime, ironfurnaces_config as config};
use horizon_client::{containers::{BlockEntity, ModBlockEntity, ModEnergyBuffer}, items::ItemStack};
use horizon_world::nbt::{Compound, JavaString, Tag};
use std::sync::Arc;

const HEATER: &str = "ironfurnaces:heater";
const ENERGY: &str = "ironfurnaces:energy";
fn number(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&JavaString::from(key)) { Some(Tag::Int(n)) => *n, _ => 0 }
}
fn owns(block: &str) -> bool { block == HEATER || Tier::from_block(block).is_some() }
fn mode(entity: &ModBlockEntity) -> i32 {
    match entity.items.get(5).and_then(Option::as_ref).map(|s| &*s.item) {
        Some("ironfurnaces:augment_factory") => 1,
        Some("ironfurnaces:augment_generator") => 2,
        _ => 0,
    }
}

/// Register a stable backing capability; each access uses the live descriptor below.
pub fn register_energy(block: &str, buffer: &mut Option<ModEnergyBuffer>) {
    if owns(block) { *buffer = Some(ModEnergyBuffer {capacity:1,..Default::default()}); }
}
pub fn energy_at(game: &Game<'_>, dimension: &str, pos: [i32;3])
    -> std::ops::ControlFlow<Option<(Arc<ModEnergyBuffer>, i32)>>
{
    use std::ops::ControlFlow::{Break,Continue};
    let key = (pos[0],pos[1],pos[2]);
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&key) else { return Continue(()); };
    if !owns(&entity.id) { return Continue(()); }
    if !game.native_on(runtime::ID) || (game.serving.is_none() && game.options.dedicated.is_none()) || dimension != game.dim.id() { return Break(None); }
    let heater = entity.id == HEATER;
    let capacity = if heater { config::heater_capacity() } else {
        config::energy_capacity(Tier::from_block(&entity.id).unwrap())
    }.min(i32::MAX as u32) as i32;
    let machine = mode(entity);
    let mut buffer = ModEnergyBuffer { capacity, ..Default::default() };
    if heater {
        buffer.max_insert = capacity;
        buffer.insert_faces = [true;6];
    } else {
        for side in 0..6 {
            let setting = number(&entity.nbt, &format!("Side{side}"));
            buffer.insert_faces[side] = machine == 1;
            buffer.extract_faces[side] = machine == 2 && matches!(setting,2|3);
        }
        if machine == 1 { buffer.max_insert = capacity; }
        if machine == 2 { buffer.max_extract = capacity; }
    }
    Break(Some((Arc::new(buffer), number(&entity.nbt,"energy").max(0))))
}

/// Called after successful placement, before the item in hand shrinks.
pub fn placed(game: &mut Game<'_>, stack: &ItemStack, pos: [i32;3]) {
    if !game.native_on(runtime::ID) || !owns(&stack.item) { return; }
    let owner = game.native_acting_id();
    let key = (pos[0],pos[1],pos[2]);
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get_mut(&key) else { return; };
    runtime::initialize(entity);
    if entity.id == HEATER {
        let amount = stack.custom_data().map_or(0, |tag| number(tag,ENERGY))
            .clamp(0,config::heater_capacity() as i32);
        entity.nbt.insert("energy".into(),Tag::Int(amount));
    } else {
        entity.nbt.insert("Owner".into(),Tag::String(owner.to_string().as_str().into()));
    }
    game.world.modified.insert((pos[0]>>4,pos[2]>>4));
    if stack.item.as_ref() != HEATER { super::ironfurnaces_power::track_furnace(game,pos,owner); }
}

/// The normal block-drop component attachment path carries energy, exactly once.
pub fn remember_drop(pending: &mut super::spill::Pending, pos: [i32;3], entity: &BlockEntity, block: &str) {
    if block != HEATER { return; }
    let BlockEntity::Mod(entity) = entity else { return; };
    let amount = number(&entity.nbt,"energy").clamp(0,config::heater_capacity() as i32);
    let mut custom = Compound::new();
    custom.insert(ENERGY.into(),Tag::Int(amount));
    let mut components = Compound::new();
    components.insert("minecraft:custom_data".into(),Tag::Compound(custom));
    pending.copied.insert(pos,(HEATER.to_owned(),components));
}

/// Java's explosion mixin removes the furnace and its inventory before dropping coal.
pub fn explosion(game: &mut Game<'_>, pos: [i32;3]) -> std::ops::ControlFlow<horizon_client::Result<()>> {
    use std::ops::ControlFlow::{Break,Continue};
    if !game.native_on(runtime::ID) || (game.serving.is_none() && game.options.dedicated.is_none()) { return Continue(()); }
    let state=game.world.block(pos[0],pos[1],pos[2]);
    if game.res.facts().get(state).name.as_ref()!="ironfurnaces:million_furnace" {return Continue(());}
    let dimension=game.dim.id().to_owned();
    game.world.block_entities.remove(&(pos[0],pos[1],pos[2]));
    game.world.forget_block_entity_source((pos[0],pos[1],pos[2]));
    if game.native_set_block(&dimension,pos,game.world.air,3) {
        let stack=ItemStack {item:Arc::from("ironfurnaces:rainbow_coal"),count:1,damage:0,extra:None};
        game.spawn_item_own(stack,[f64::from(pos[0]),f64::from(pos[1])+6.0,f64::from(pos[2])],10);
    }
    Break(Ok(()))
}

pub fn item_bar(stack: &ItemStack, bar: &mut Option<(i32,u32)>) {
    if stack.item.as_ref()=="ironfurnaces:rainbow_coal" {
        let uses=super::iron_furnaces_energy_rules::RAINBOW_COAL_USES;
        let remaining=uses.saturating_sub(stack.damage);
        let fraction=remaining as f32/uses as f32;
        // HSV(hue=f/3, saturation=1, value=1), matching the source item.
        let red=(((1.0-fraction)*2.0).min(1.0)*255.0) as u32;
        let green=((fraction*2.0).min(1.0)*255.0) as u32;
        *bar=Some(((remaining*13/uses) as i32,(red<<16)|(green<<8)));
        return;
    }
    if stack.item.as_ref()!=HEATER {return;}
    let energy=stack.custom_data().map_or(0,|tag|number(tag,ENERGY)).max(0);
    *bar=(energy>0).then_some((((i64::from(energy)*13)/i64::from(config::heater_capacity())).clamp(0,13) as i32,0xff0000));
}

pub fn item_energy(stack: &ItemStack) -> Option<i32> {
    (stack.item.as_ref()==HEATER).then(||stack.custom_data().map_or(0,|tag|number(tag,ENERGY)).max(0))
}

pub fn interact_heater(game: &mut Game<'_>, pos: [i32;3], canceled: &mut bool) {
    if *canceled || !game.native_on(runtime::ID) { return; }
    if game.keys.held(super::Mapping::Sneak) && (game.stack_in(game.selected).is_some() || game.offhand.is_some()) {return;}
    let state=game.world.block(pos[0],pos[1],pos[2]);
    if game.res.facts().get(state).name.as_ref()!=HEATER {return;}
    // Iron-block behavior has no OpenMenu effect; use the normal engine path
    // so locks, guest menu ownership and custom-menu synchronization apply.
    game.open_menu(pos);
    *canceled=true;
}
