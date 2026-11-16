// SPDX-License-Identifier: Apache-2.0
// Typed energy handlers preserve the original FE accounting and saved energy key.
use crate::{
    config, cooking, energy_rules as rules,
    rules::{EfficiencyAugment, RecipeMode, Tier},
    state::*,
};
use mod_api::{
    BlockSide, Compound, Direction, ENERGY, EnergyHandler, Host, HostTransaction, ItemStack,
    ModBlockEntityView, ServerHost, Tag, Transaction,
};

pub fn fuel_energy(host: &dyn Host, e: &ModBlockEntityView<'_>, stack: &ItemStack) -> u32 {
    let id = stack.item().id(host);
    let (fuel, food, blasting) = match recipe_mode(host, e) {
        RecipeMode::Smelting => (
            if id == "ironfurnaces:rainbow_coal" {
                if stack.damage() < rules::RAINBOW_COAL_USES {
                    rules::RAINBOW_COAL_FUEL_TICKS
                } else {
                    0
                }
            } else {
                host.item_burn_time(stack.item()).unwrap_or(0).max(0) as u32
            },
            0,
            0,
        ),
        RecipeMode::Smoking => {
            let patch = stack.components().map(|p| p.as_nbt());
            let food = if patch.is_some_and(|p| p.contains_key(&("!minecraft:food").into())) {
                None
            } else {
                patch
                    .and_then(|p| p.get(&("minecraft:food").into()))
                    .cloned()
                    .or_else(|| host.item_default_component(stack.item(), "minecraft:food"))
            };
            let n = match food {
                Some(Tag::Compound(p)) => number(&p, "nutrition").max(0) as u32,
                _ => 0,
            };
            (0, n, 0)
        }
        RecipeMode::Blasting => {
            let recipes = host.recipes_of_type("ironfurnaces:generator_blasting");
            let n = recipes
                .iter()
                .find_map(|r| {
                    let ingredient = host
                        .recipe_ingredient(&r.json.get("ingredient")?.to_string())
                        .ok()??;
                    ingredient.test(host, stack).then(|| {
                        r.json
                            .get("energy")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0)
                            .min(i32::MAX as u64) as u32
                    })
                })
                .unwrap_or(0);
            (0, 0, n)
        }
    };
    rules::generator_fuel_energy(
        recipe_mode(host, e),
        efficiency(host, e),
        fuel,
        food,
        blasting,
    )
}

pub fn generator(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
    e: &mut ModBlockEntityView<'_>,
    tier: Tier,
) {
    let capacity = config::energy_capacity(tier);
    let stored = (number(&e.nbt, "energy").max(0) as u32).min(capacity);
    let mut remaining = number(&e.nbt, "GeneratorRemaining").max(0) as u32;
    if stored < capacity && remaining == 0 {
        let budget = e.items[6].as_ref().map_or(0, |s| fuel_energy(host, e, s));
        if budget > 0 && cooking::consume_fuel(host, pos, e, 6) {
            remaining = budget;
            put(&mut e.nbt, "GeneratorTotal", budget as i32);
        }
    }
    let step = rules::generator_step(
        stored,
        capacity,
        remaining,
        config::generation_rate(tier, efficiency(host, e)),
    );
    put(&mut e.nbt, "GeneratorRemaining", step.remaining as i32);
    put(&mut e.nbt, "energy", step.stored as i32);
    put(&mut e.nbt, "GenerationRate", step.generated as i32);
    if let Some(level) = host.level(host.dimension()) {
        e.nbt.insert(
            "GeneratorLastTick".into(),
            Tag::Long(level.game_time(host).unwrap_or(0)),
        );
        let rainbow = tier == Tier::Rainbow && crate::network::ready(host, pos, e);
        put(&mut e.nbt, "RainbowGen", i32::from(rainbow));
        let mut bonus = 0u32;
        if rainbow {
            let mut receivers = Vec::new();
            for side in Direction::ALL {
                if !matches!(number(&e.nbt, &format!("Side{}", side as usize)), 2 | 3) {
                    continue;
                }
                let offset = side.normal();
                let to = std::array::from_fn(|i| pos[i].saturating_add(offset[i]));
                if let Some(handler) = level.capability(host, &ENERGY, to, Some(side.opposite())) {
                    let probe: Result<(), u32> =
                        host.transaction(|host, tx| Err(handler.insert(host, tx, 1)));
                    if probe == Err(1) {
                        receivers.push(handler);
                    }
                }
            }
            let share = if receivers.is_empty() {
                0
            } else {
                config::rainbow_output() / receivers.len() as u32
            };
            for receiver in receivers {
                let moved: Result<u32, ()> = host.transaction(|host, tx| {
                    let n = receiver.insert(host, tx, share);
                    if n <= share { Ok(n) } else { Err(()) }
                });
                bonus = bonus.saturating_add(moved.unwrap_or(0));
            }
        }
        put(
            &mut e.nbt,
            "GenerationRate",
            step.generated.saturating_add(bonus).min(i32::MAX as u32) as i32,
        );
        let mut stored = step.stored;
        for side in Direction::ALL {
            if !matches!(number(&e.nbt, &format!("Side{}", side as usize)), 2 | 3) {
                continue;
            }
            let offset = side.normal();
            let to = std::array::from_fn(|i| pos[i].saturating_add(offset[i]));
            if let Some(handler) = level.capability(host, &ENERGY, to, Some(side.opposite())) {
                let moved: Result<u32, ()> = host.transaction(|host, tx| {
                    let n = handler.insert(host, tx, stored);
                    if n <= stored { Ok(n) } else { Err(()) }
                });
                stored -= moved.unwrap_or(0);
            }
        }
        put(&mut e.nbt, "energy", stored as i32);
    }
}

pub fn heater(host: &dyn Host, pos: [i32; 3], e: &mut ModBlockEntityView<'_>) {
    let stored = (number(&e.nbt, "energy").max(0) as u32).min(config::heater_capacity());
    put(&mut e.nbt, "energy", stored as i32);
    let Some(s) = e.items[0]
        .as_mut()
        .filter(|s| s.item().id(host) == "ironfurnaces:item_heater")
    else {
        return;
    };
    let mut patch = s
        .components()
        .map(|p| p.as_nbt().clone())
        .unwrap_or_default();
    let mut data = match patch.remove(&("minecraft:custom_data").into()) {
        Some(Tag::Compound(d)) => d,
        _ => Compound::new(),
    };
    for (key, value) in ["HeaterPosX", "HeaterPosY", "HeaterPosZ"]
        .into_iter()
        .zip(pos)
    {
        put(&mut data, key, value);
    }
    patch.insert("minecraft:custom_data".into(), Tag::Compound(data));
    let _ = s.set_components(patch);
}

pub fn link(host: &dyn Host, s: &ItemStack) -> Option<[i32; 3]> {
    if s.item().id(host) != "ironfurnaces:item_heater" {
        return None;
    }
    let Tag::Compound(data) = s
        .components()?
        .as_nbt()
        .get(&("minecraft:custom_data").into())?
    else {
        return None;
    };
    let read = |k: &str| match data.get(&(k).into()) {
        Some(Tag::Int(n)) => Some(*n),
        Some(Tag::Short(n)) => Some(i32::from(*n)),
        _ => None,
    };
    Some([
        read("HeaterPosX")?,
        read("HeaterPosY")?,
        read("HeaterPosZ")?,
    ])
}

pub fn ignite(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
    e: &mut ModBlockEntityView<'_>,
    cook: u32,
    augment: EfficiencyAugment,
) -> Option<(u32, u32)> {
    let target = link(host, e.items.get(1)?.as_ref()?)?;
    if target == pos {
        return None;
    }
    let level = host.level(host.dimension())?;
    if !level.is_loaded(host, target) {
        crate::network::wireless_load(host, level, pos, target);
        e.nbt.insert(
            "WirelessLoadTick".into(),
            Tag::Long(level.game_time(host).unwrap_or(0)),
        );
        return None;
    }
    if let Some(&controller) = crate::network::WIRELESS.get() {
        crate::network::release(host, level, controller, mod_api::TicketOwner::Block(pos));
    }
    let handle = host.block_entity(level, target)?;
    if level.block(host, target)?.id(host) != "ironfurnaces:heater" {
        return None;
    }
    let data = handle.data(host)?;
    let stored = number(&data, "energy").max(0) as u32;
    let ignition = rules::wireless_ignition(stored.min(config::heater_capacity()), cook, augment)?;
    let mut update = Compound::new();
    put(
        &mut update,
        "energy",
        (stored - ignition.energy_cost) as i32,
    );
    handle.merge_data(host, update).ok().filter(|v| *v)?;
    Some((ignition.burn_ticks, ignition.burn_ticks))
}

pub struct MachineEnergy(pub BlockSide, pub Option<Tier>);
impl MachineEnergy {
    fn read(
        &self,
        host: &dyn Host,
    ) -> Option<(mod_api::BlockEntityRef, Compound, rules::MachineMode)> {
        let entity = host.block_entity(self.0.level, self.0.pos)?;
        let data = entity.data(host)?;
        let mode = entity
            .item_slot(host, 5)
            .ok()
            .flatten()
            .map(|s| s.item().id(host))
            .and_then(|id| rules::MachineMode::from_item(&id))
            .unwrap_or_default();
        Some((entity, data, mode))
    }
}
impl EnergyHandler for MachineEnergy {
    fn amount(&self, host: &mut dyn ServerHost) -> u32 {
        self.read(host)
            .map_or(0, |(_, d, _)| number(&d, "energy").max(0) as u32)
    }
    fn capacity(&self, _: &mut dyn ServerHost) -> u32 {
        self.1
            .map_or_else(config::heater_capacity, config::energy_capacity)
    }
    fn insert(&self, host: &mut dyn ServerHost, _tx: &Transaction, amount: u32) -> u32 {
        let Some((entity, data, mode)) = self.read(host) else {
            return 0;
        };
        if self.1.is_some() && mode != rules::MachineMode::Factory {
            return 0;
        }
        let stored = number(&data, "energy").max(0) as u32;
        let moved = amount.min(self.capacity(host).saturating_sub(stored));
        let mut update = Compound::new();
        put(&mut update, "energy", stored.saturating_add(moved) as i32);
        if moved > 0 && matches!(entity.merge_data(host, update), Ok(true)) {
            moved
        } else {
            0
        }
    }
    fn extract(&self, host: &mut dyn ServerHost, _tx: &Transaction, amount: u32) -> u32 {
        let Some((entity, data, mode)) = self.read(host) else {
            return 0;
        };
        if self.1.is_none() || mode != rules::MachineMode::Generator {
            return 0;
        }
        if self
            .0
            .side
            .is_some_and(|s| !matches!(number(&data, &format!("Side{}", s as usize)), 2 | 3))
        {
            return 0;
        }
        let stored = number(&data, "energy").max(0) as u32;
        let moved = amount.min(stored);
        let mut update = Compound::new();
        put(&mut update, "energy", (stored - moved) as i32);
        if moved > 0 && matches!(entity.merge_data(host, update), Ok(true)) {
            moved
        } else {
            0
        }
    }
}
