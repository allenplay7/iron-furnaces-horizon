// SPDX-License-Identifier: Apache-2.0
use crate::{
    config,
    energy_rules::{AugmentChannel, MachineMode, augment_channel},
    power,
    rules::{RecipeMode, Tier},
    state::*,
};
use mod_api::item::containers::ModBlockEntityFields;
use mod_api::{
    BlockSide, Direction, Host, HostTransaction, ITEMS, ItemResource, ItemStack, ModBlockEntity,
    ModBlockEntityView, ResourceHandler, ServerHost, Transaction,
};

pub fn accepts(host: &dyn Host, e: &ModBlockEntityView<'_>, slot: usize, s: &ItemStack) -> bool {
    let id = s.item().id(host);
    if e.id() == "ironfurnaces:heater" {
        return slot == 0 && id == "ironfurnaces:item_heater";
    }
    if (3..=5).contains(&slot) {
        return augment_channel(&id).map(|c| match c {
            AugmentChannel::Recipe => 3,
            AugmentChannel::Efficiency => 4,
            AugmentChannel::Machine => 5,
        }) == Some(slot);
    }
    match mode(host, e) {
        MachineMode::Furnace => match slot {
            0 => recipe_input(host, recipe_mode(host, e), s),
            1 => {
                host.item_burn_time(s.item()).unwrap_or(0) > 0
                    || matches!(
                        id.as_str(),
                        "ironfurnaces:item_heater"
                            | "ironfurnaces:rainbow_coal"
                            | "minecraft:bucket"
                    )
            }
            _ => false,
        },
        MachineMode::Factory => {
            (7..13).contains(&slot)
                && Tier::from_block(e.id())
                    .is_some_and(|t| config::active_factory_lanes(t).contains(&(slot - 7)))
                && recipe_input(host, recipe_mode(host, e), s)
        }
        MachineMode::Generator => slot == 6 && power::fuel_energy(host, e, s) > 0,
    }
}
fn recipe_input(host: &dyn Host, mode: RecipeMode, s: &ItemStack) -> bool {
    host.recipes_of_type(mode.recipe_type())
        .iter()
        .any(|r| r.ingredients.len() == 1 && r.ingredients[0].test(host, s))
}
pub fn limit(host: &dyn Host, slot: usize, s: &ItemStack) -> u32 {
    if (3..=5).contains(&slot)
        || matches!(
            s.item().id(host).as_str(),
            "ironfurnaces:item_heater" | "minecraft:bucket"
        )
    {
        1
    } else {
        maximum(host, s)
    }
}
pub fn side(e: &ModBlockEntityView<'_>, face: Direction) -> i32 {
    number(&e.nbt, &format!("Side{}", face as usize)).clamp(0, 4)
}
fn input_face(
    host: &dyn Host,
    e: &ModBlockEntityView<'_>,
    slot: usize,
    face: Option<Direction>,
) -> bool {
    if e.id() == "ironfurnaces:heater" {
        return slot == 0;
    }
    let Some(face) = face else {
        return true;
    };
    match (mode(host, e), side(e, face)) {
        (MachineMode::Furnace, 1 | 3) => slot == 0 || slot == 1,
        (MachineMode::Furnace, 4) => slot == 1,
        (MachineMode::Factory, 1 | 3) => (7..13).contains(&slot),
        (MachineMode::Generator, 4) => slot == 6,
        _ => false,
    }
}
fn output_face(
    host: &dyn Host,
    e: &ModBlockEntityView<'_>,
    slot: usize,
    s: &ItemStack,
    face: Option<Direction>,
) -> bool {
    if e.id() == "ironfurnaces:heater" {
        return slot == 0;
    }
    let Some(face) = face else {
        return true;
    };
    match (mode(host, e), side(e, face)) {
        (MachineMode::Furnace, 2 | 3) => slot == 2,
        (MachineMode::Furnace, 4) => slot == 1 && s.item().id(host) == "minecraft:bucket",
        (MachineMode::Factory, 2 | 3) => {
            (13..19).contains(&slot)
                && Tier::from_block(e.id())
                    .is_some_and(|t| config::active_factory_lanes(t).contains(&(slot - 13)))
        }
        (MachineMode::Generator, 4) => slot == 6 && s.item().id(host) == "minecraft:bucket",
        _ => false,
    }
}
pub fn snapshot(
    host: &dyn Host,
    at: BlockSide,
) -> Option<(mod_api::BlockEntityRef, ModBlockEntity)> {
    let handle = host.block_entity(at.level, at.pos)?;
    let id = at.level.block(host, at.pos)?.id(host);
    let slots = if id == "ironfurnaces:heater" { 1 } else { 19 };
    let mut fields = ModBlockEntityFields::default();
    fields.nbt = handle.data(host)?;
    fields.data = handle.data_slots(host).unwrap_or_default();
    fields.items = (0..slots)
        .map(|i| handle.item_slot(host, i).ok().flatten())
        .collect();
    Some((handle, ModBlockEntity::new(id, fields)))
}
pub struct MachineItems(pub BlockSide);
impl ResourceHandler<ItemResource> for MachineItems {
    fn size(&self, host: &mut dyn ServerHost) -> usize {
        snapshot(host, self.0).map_or(0, |(_, e)| e.items.len())
    }
    fn resource(&self, host: &mut dyn ServerHost, index: usize) -> Option<ItemResource> {
        snapshot(host, self.0)?
            .1
            .items
            .get(index)?
            .as_ref()
            .map(ItemResource::of)
    }
    fn amount(&self, host: &mut dyn ServerHost, index: usize) -> u32 {
        snapshot(host, self.0)
            .and_then(|(_, e)| e.items.get(index).cloned().flatten())
            .map_or(0, |s| s.count())
    }
    fn capacity(&self, host: &mut dyn ServerHost, index: usize, r: &ItemResource) -> u32 {
        if index < self.size(host) {
            limit(host, index, r.as_stack())
        } else {
            0
        }
    }
    fn is_valid(&self, host: &mut dyn ServerHost, index: usize, r: &ItemResource) -> bool {
        let Some((_, mut e)) = snapshot(host, self.0) else {
            return false;
        };
        let e = ModBlockEntityView::new(&mut e);
        input_face(host, &e, index, self.0.side) && accepts(host, &e, index, r.as_stack())
    }
    fn insert(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        index: usize,
        r: &ItemResource,
        amount: u32,
    ) -> u32 {
        let Some((handle, mut saved)) = snapshot(host, self.0) else {
            return 0;
        };
        let e = ModBlockEntityView::new(&mut saved);
        if !input_face(host, &e, index, self.0.side) || !accepts(host, &e, index, r.as_stack()) {
            return 0;
        }
        let Some(slot) = e.items.get(index) else {
            return 0;
        };
        if slot.as_ref().is_some_and(|s| !same(s, r.as_stack())) {
            return 0;
        }
        let count = slot.as_ref().map_or(0, ItemStack::count);
        let moved = amount.min(limit(host, index, r.as_stack()).saturating_sub(count));
        if moved == 0 {
            return 0;
        }
        let mut s = r.as_stack().clone();
        let _ = s.set_count(count + moved);
        if matches!(
            handle.set_item_slot(host, tx, index as u32, Some(s)),
            Ok(true)
        ) {
            let _ = handle.set_changed(host);
            moved
        } else {
            0
        }
    }
    fn extract(
        &self,
        host: &mut dyn ServerHost,
        tx: &Transaction,
        index: usize,
        r: &ItemResource,
        amount: u32,
    ) -> u32 {
        let Some((handle, mut saved)) = snapshot(host, self.0) else {
            return 0;
        };
        let e = ModBlockEntityView::new(&mut saved);
        let Some(s) = e.items.get(index).and_then(Option::as_ref) else {
            return 0;
        };
        if !same(s, r.as_stack()) || !output_face(host, &e, index, s, self.0.side) {
            return 0;
        }
        let moved = amount.min(s.count());
        if moved == 0 {
            return 0;
        }
        let next = s.clone().shrink(moved);
        if matches!(handle.set_item_slot(host, tx, index as u32, next), Ok(true)) {
            let _ = handle.set_changed(host);
            moved
        } else {
            0
        }
    }
}

pub fn tick_io(host: &mut dyn ServerHost, pos: [i32; 3], e: &mut ModBlockEntityView<'_>) {
    if number(&e.nbt, "AutoInput") == 0 && number(&e.nbt, "AutoOutput") == 0 {
        return;
    }
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    for face in Direction::ALL {
        let delta = face.normal();
        let target = std::array::from_fn(|i| pos[i].saturating_add(delta[i]));
        let Some(storage) = level.capability(host, &ITEMS, target, Some(face.opposite())) else {
            continue;
        };
        if number(&e.nbt, "AutoInput") != 0 {
            for j in 0..storage.size(host) {
                let Some(r) = storage.resource(host, j) else {
                    continue;
                };
                for i in 0..e.items.len() {
                    if !input_face(host, e, i, Some(face)) || !accepts(host, e, i, r.as_stack()) {
                        continue;
                    }
                    if mode(host, e) == MachineMode::Furnace
                        && i == 1
                        && (matches!(side(e, face), 1 | 3)
                            || r.as_stack().item().id(host) == "minecraft:bucket")
                    {
                        continue;
                    }
                    if e.items[i].as_ref().is_some_and(|s| !same(s, r.as_stack())) {
                        continue;
                    }
                    let count = e.items[i].as_ref().map_or(0, ItemStack::count);
                    let want = limit(host, i, r.as_stack()).saturating_sub(count);
                    if want == 0 {
                        continue;
                    }
                    let moved: Result<u32, ()> = host.transaction(|host, tx| {
                        let n = storage.extract(host, tx, j, &r, want);
                        if n <= want { Ok(n) } else { Err(()) }
                    });
                    let n = moved.unwrap_or(0);
                    if n > 0 {
                        let mut s = r.as_stack().clone();
                        let _ = s.set_count(count + n);
                        e.items[i] = Some(s);
                    }
                }
            }
        }
        if number(&e.nbt, "AutoOutput") != 0 {
            for i in 0..e.items.len() {
                let Some(s) = e.items[i].clone() else {
                    continue;
                };
                let remainder = side(e, face) == 4
                    && ((mode(host, e) == MachineMode::Furnace && i == 1)
                        || (mode(host, e) == MachineMode::Generator && i == 6))
                    && (matches!(
                        s.item().id(host).as_str(),
                        "minecraft:bucket" | "minecraft:water_bucket"
                    ) || !accepts(host, e, i, &s));
                if !remainder && !output_face(host, e, i, &s, Some(face)) {
                    continue;
                }
                let r = ItemResource::of(&s);
                let mut remaining = s.count();
                for j in 0..storage.size(host) {
                    let moved: Result<u32, ()> = host.transaction(|host, tx| {
                        let n = storage.insert(host, tx, j, &r, remaining);
                        if n <= remaining { Ok(n) } else { Err(()) }
                    });
                    let n = moved.unwrap_or(0);
                    consume(&mut e.items[i], n);
                    remaining -= n;
                    if remaining == 0 {
                        break;
                    }
                }
            }
        }
    }
}
