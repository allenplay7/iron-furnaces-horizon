// SPDX-License-Identifier: Apache-2.0
// Adapted from the validated 0.2 processing implementation for typed API stacks.
use crate::{
    config, energy_rules as energy,
    rules::{self, Tier},
    state::*,
};
use mod_api::{Compound, Host, ItemStack, ModBlockEntityView, Recipe, Tag};

fn recipe(host: &dyn Host, entity: &ModBlockEntityView<'_>, input: usize) -> Option<Recipe> {
    let stack = entity.items.get(input)?.as_ref()?;
    host.recipes_of_type(recipe_mode(host, entity).recipe_type())
        .iter()
        .find(|r| {
            r.results.len() == 1 && r.ingredients.len() == 1 && r.ingredients[0].test(host, stack)
        })
        .cloned()
}
fn count(
    host: &dyn Host,
    e: &ModBlockEntityView<'_>,
    input: usize,
    output: usize,
    tier: Tier,
    result: &ItemStack,
) -> u32 {
    if !e.items[output].as_ref().is_none_or(|s| same(s, result)) {
        return 0;
    }
    rules::batch_count(
        config::batch_limit(tier),
        e.items[input].as_ref().map_or(0, ItemStack::count),
        e.items[output].as_ref().map_or(0, ItemStack::count),
        result.count(),
        maximum(host, result),
    )
}
fn finish(
    host: &dyn Host,
    e: &mut ModBlockEntityView<'_>,
    input: usize,
    output: usize,
    r: &Recipe,
    count: u32,
) {
    let result = &r.results[0];
    let wet = item(host, e, input) == "minecraft:wet_sponge";
    let added = result.count() * count;
    if let Some(s) = e.items[output].as_mut() {
        s.grow(added).expect("batch capacity checked");
    } else {
        let mut s = result.clone();
        s.set_count(added).expect("nonempty result");
        e.items[output] = Some(s);
    }
    consume(&mut e.items[input], count);
    if input == 0 && wet && item(host, e, 1) == "minecraft:bucket" {
        e.items[1] = stack(host, "minecraft:water_bucket", 1);
    }
    let mut used = match e.nbt.remove(&("RecipesUsed").into()) {
        Some(Tag::Compound(v)) => v,
        _ => Compound::new(),
    };
    let old = number(&used, &r.id).max(0);
    put(&mut used, &r.id, old.saturating_add(count as i32));
    e.nbt.insert("RecipesUsed".into(), Tag::Compound(used));
}
pub fn consume_fuel(
    host: &mut dyn mod_api::ServerHost,
    pos: [i32; 3],
    e: &mut ModBlockEntityView<'_>,
    slot: usize,
) -> bool {
    let Some(fuel) = e.items[slot].clone() else {
        return false;
    };
    if fuel.item().id(host) == "ironfurnaces:rainbow_coal" {
        if fuel.damage() >= energy::RAINBOW_COAL_USES {
            return false;
        }
        let damage = fuel.damage() + 1;
        if damage >= energy::RAINBOW_COAL_USES {
            consume(&mut e.items[slot], 1);
        } else {
            let mut fuel = fuel;
            fuel.set_damage(damage);
            e.items[slot] = Some(fuel);
        }
    } else {
        let remainder = host.item_crafting_remainder(&fuel);
        consume(&mut e.items[slot], 1);
        if e.items[slot].is_none() {
            e.items[slot] = remainder;
        } else if let (Some(level), Some(remainder)) = (host.level(host.dimension()), remainder) {
            let _ = level.spawn_item(
                host,
                f64::from(pos[0]) + 0.5,
                f64::from(pos[1]) + 0.5,
                f64::from(pos[2]) + 0.5,
                remainder,
                10,
            );
        }
    }
    true
}
pub fn furnace(
    host: &mut dyn mod_api::ServerHost,
    pos: [i32; 3],
    e: &mut ModBlockEntityView<'_>,
    tier: Tier,
    run: bool,
) {
    if !run {
        return;
    }
    let augment = efficiency(host, e);
    let mut burn = number(&e.nbt, "BurnTime").saturating_sub(1).max(0);
    let mut total_burn = number(&e.nbt, "BurnTotal").max(0);
    let mut progress = number(&e.nbt, "CookTime").max(0);
    let recipe = recipe(host, e, 0);
    let total = config::cook_ticks(
        tier,
        recipe.as_ref().and_then(|r| r.cooking_time).unwrap_or(200),
        augment,
    ) as i32;
    if !fingerprint(host, e, 0, "", recipe.as_ref().map_or("", |r| &r.id)) {
        progress = 0;
    }
    if let Some(r) = recipe {
        let n = count(host, e, 0, 2, tier, &r.results[0]);
        if n > 0 {
            if burn == 0 {
                if let Some((ticks, full)) =
                    crate::power::ignite(host, pos, e, total as u32, augment)
                {
                    burn = ticks as i32;
                    total_burn = full as i32;
                } else if let Some(fuel) = e.items[1].clone() {
                    let ticks = if fuel.item().id(host) == "ironfurnaces:rainbow_coal"
                        && fuel.damage() < energy::RAINBOW_COAL_USES
                    {
                        energy::RAINBOW_COAL_FUEL_TICKS
                    } else {
                        host.item_burn_time(fuel.item()).unwrap_or(0).max(0) as u32
                    };
                    let ticks = rules::fuel_ticks(ticks, total as u32, augment) as i32;
                    if ticks > 0 && consume_fuel(host, pos, e, 1) {
                        burn = ticks;
                        total_burn = ticks;
                    }
                }
            }
            if burn > 0 {
                progress += 1;
                if progress >= total {
                    progress = 0;
                    finish(host, e, 0, 2, &r, n);
                }
            } else {
                progress = (progress - 2).max(0);
            }
        } else {
            progress = 0;
        }
    } else {
        progress = 0;
    }
    for (key, value) in [
        ("BurnTime", burn),
        ("BurnTotal", total_burn),
        ("CookTime", progress),
        ("CookTotal", total),
    ] {
        put(&mut e.nbt, key, value);
    }
}
fn split(host: &dyn Host, e: &mut ModBlockEntityView<'_>, tier: Tier) {
    let lanes: Vec<usize> = config::active_factory_lanes(tier).map(|i| 7 + i).collect();
    let mut done = std::collections::HashSet::new();
    for &slot in &lanes {
        if done.contains(&slot) {
            continue;
        }
        let Some(sample) = e.items[slot].clone() else {
            continue;
        };
        let targets: Vec<usize> = lanes
            .iter()
            .copied()
            .filter(|i| !done.contains(i) && e.items[*i].as_ref().is_none_or(|s| same(s, &sample)))
            .collect();
        let total: u32 = targets
            .iter()
            .map(|i| e.items[*i].as_ref().map_or(0, ItemStack::count))
            .sum();
        let max = maximum(host, &sample);
        let each = (total / targets.len() as u32).min(max);
        let mut extra = total - each * targets.len() as u32;
        for i in targets {
            let n = (each + u32::from(extra > 0)).min(max);
            extra = extra.saturating_sub(n - each);
            e.items[i] = if n == 0 {
                None
            } else {
                let mut s = sample.clone();
                s.set_count(n).unwrap();
                Some(s)
            };
            done.insert(i);
        }
    }
}
pub fn factory(host: &dyn Host, e: &mut ModBlockEntityView<'_>, tier: Tier, run: bool) {
    put(&mut e.nbt, "FactoryActive", 0);
    if !run {
        return;
    }
    if number(&e.nbt, "AutoSplit") != 0 {
        split(host, e, tier);
    }
    let augment = efficiency(host, e);
    let mut stored = number(&e.nbt, "energy")
        .max(0)
        .min(config::energy_capacity(tier) as i32) as u32;
    let active = config::active_factory_lanes(tier);
    for lane in 0..6 {
        if !active.contains(&lane) {
            put(&mut e.nbt, &format!("LaneReserved{lane}"), 0);
            put(&mut e.nbt, &format!("LaneProgress{lane}"), 0);
        }
    }
    let mut reserved = (0..6)
        .map(|i| number(&e.nbt, &format!("LaneReserved{i}")).max(0) as u32)
        .fold(0, u32::saturating_add);
    for lane in active {
        let input = 7 + lane;
        let output = 13 + lane;
        let pk = format!("LaneProgress{lane}");
        let tk = format!("LaneTotal{lane}");
        let ck = format!("LaneCost{lane}");
        let rk = format!("LaneReserved{lane}");
        let mut progress = number(&e.nbt, &pk).max(0) as u32;
        let mut remaining = number(&e.nbt, &rk).max(0) as u32;
        let r = recipe(host, e, input);
        let total = config::cook_ticks(
            tier,
            r.as_ref().and_then(|r| r.cooking_time).unwrap_or(200),
            augment,
        );
        let same = fingerprint(
            host,
            e,
            input,
            &format!("Lane{lane}"),
            r.as_ref().map_or("", |r| &r.id),
        );
        let n = r
            .as_ref()
            .map_or(0, |r| count(host, e, input, output, tier, &r.results[0]));
        if !same || n == 0 || progress >= total {
            reserved = reserved.saturating_sub(remaining);
            remaining = 0;
            progress = 0;
        }
        if let Some(r) = r
            && n > 0
        {
            if progress == 0 && remaining == 0 {
                let cost = energy::factory_recipe_energy(r.cooking_time.unwrap_or(200), augment);
                if energy::factory_can_start(stored, reserved, cost) {
                    remaining = cost;
                    reserved = reserved.saturating_add(cost);
                    put(&mut e.nbt, &ck, cost as i32);
                }
            }
            if remaining > 0 {
                let debit = remaining / (total - progress);
                if stored >= debit {
                    stored -= debit;
                    remaining -= debit;
                    reserved = reserved.saturating_sub(debit);
                    progress += 1;
                    put(&mut e.nbt, "FactoryActive", 1);
                    if progress >= total {
                        progress = 0;
                        remaining = 0;
                        finish(host, e, input, output, &r, n);
                    }
                }
            }
        }
        put(&mut e.nbt, &pk, progress as i32);
        put(&mut e.nbt, &tk, total as i32);
        put(&mut e.nbt, &rk, remaining as i32);
    }
    put(&mut e.nbt, "energy", stored as i32);
}
