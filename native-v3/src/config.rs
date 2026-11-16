// SPDX-License-Identifier: Apache-2.0
// Compatible with the Fabric 26.3 ironfurnaces.json configuration layout.
use super::{
    energy_rules as energy,
    rules::{EfficiencyAugment, TIERS, Tier},
};
use serde_json::{Value, json};
use std::sync::{OnceLock, RwLock};

fn settings() -> &'static RwLock<Value> {
    static CONFIG: OnceLock<RwLock<Value>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(Value::Null))
}
fn key(tier: Tier) -> &'static str {
    tier.block_id().split(':').nth(1).unwrap_or("iron_furnace")
}
fn modded(tier: Tier) -> bool {
    matches!(
        tier,
        Tier::Allthemodium | Tier::Vibranium | Tier::Unobtainium
    )
}
fn group(tier: Tier, field: &str) -> (&'static str, &'static str) {
    if modded(tier) {
        ("modded_furnaces", key(tier))
    } else if tier == Tier::Rainbow && field != "tier" {
        ("furnaces", "rainbow_furnace")
    } else {
        ("furnaces", key(tier))
    }
}
fn integer(category: &str, name: &str, field: &str, default: u32, min: u32, max: u32) -> u32 {
    settings()
        .read()
        .ok()
        .and_then(|s| s.get(category)?.get(name)?.get(field)?.as_i64())
        .map(|v| v.clamp(i64::from(min), i64::from(max)) as u32)
        .unwrap_or(default)
}
fn tier_value(tier: Tier, field: &str, default: u32, min: u32, max: u32) -> u32 {
    let (category, name) = group(tier, field);
    integer(category, name, field, default, min, max)
}
pub fn storage_tier(tier: Tier) -> u8 {
    tier_value(tier, "tier", energy::storage_tier(tier).into(), 0, 2) as u8
}
pub fn active_factory_lanes(tier: Tier) -> std::ops::Range<usize> {
    match storage_tier(tier) {
        0 => 2..4,
        1 => 1..5,
        _ => 0..6,
    }
}
pub fn energy_capacity(tier: Tier) -> u32 {
    let st = storage_tier(tier);
    integer(
        "furnaces",
        "energy",
        &format!("tier_{st}"),
        [80_000, 200_000, 1_000_000][st as usize],
        4000,
        i32::MAX as u32,
    )
}
pub fn heater_capacity() -> u32 {
    1_000_000
}
pub fn generation_rate(tier: Tier, augment: EfficiencyAugment) -> u32 {
    let base = tier_value(
        tier,
        "generation",
        energy::generation_rate(tier, EfficiencyAugment::None),
        1,
        100_000,
    );
    match augment {
        EfficiencyAugment::None => base,
        EfficiencyAugment::Speed => base * 2,
        EfficiencyAugment::Fuel => base * 3 / 4,
    }
}
pub fn rainbow_output() -> u32 {
    integer(
        "furnaces",
        "rainbow_furnace",
        "rainbow_generation",
        50_000,
        1,
        100_000_000,
    )
}
pub fn cook_ticks(tier: Tier, recipe: u32, augment: EfficiencyAugment) -> u32 {
    let base = tier_value(
        tier,
        "speed",
        tier.base_ticks(),
        if modded(tier) { 1 } else { 2 },
        72_000,
    );
    let ticks = (u64::from(base) * u64::from(recipe) / 200).max(1);
    let ticks = match augment {
        EfficiencyAugment::None => ticks,
        EfficiencyAugment::Speed => (ticks / 2).max(1),
        EfficiencyAugment::Fuel => (ticks * 5).div_ceil(4),
    };
    ticks.min(i32::MAX as u64) as u32
}
pub fn batch_limit(tier: Tier) -> u32 {
    if modded(tier) {
        tier_value(tier, "mult", tier.batch_limit(), 1, 64)
    } else {
        tier.batch_limit()
    }
}
pub fn light_updates() -> bool {
    !settings()
        .read()
        .ok()
        .and_then(|s| s.get("misc")?.get("lightupdates")?.as_bool())
        .unwrap_or(false)
}
pub fn default_text() -> String {
    let mut root = json!({"general":{"show_errors":true},"misc":{"lightupdates":false},
        "furnaces":{"energy":{"tier_0":80000,"tier_1":200000,"tier_2":1000000},"recipeMaxXPLevel":100},"modded_furnaces":{}});
    for tier in TIERS {
        let category = if modded(tier) {
            "modded_furnaces"
        } else {
            "furnaces"
        };
        let mut value = json!({"tier":energy::storage_tier(tier),"speed":tier.base_ticks(),"generation":energy::generation_rate(tier,EfficiencyAugment::None)});
        if modded(tier) {
            value["mult"] = json!(tier.batch_limit());
        }
        if tier == Tier::Rainbow {
            value.as_object_mut().unwrap().remove("tier");
            value["rainbow_generation"] = json!(50_000);
            root[category]["rainbow_furnace"] = value;
            root[category]["million_furnace"] = json!({"tier":2});
        } else {
            root[category][key(tier)] = value;
        }
    }
    serde_json::to_string_pretty(&root).unwrap() + "\n"
}

pub fn load_text(text: &str) -> Result<(), serde_json::Error> {
    let value: Value = serde_json::from_str(text)?;
    if value.is_object() {
        if let Ok(mut current) = settings().write() {
            *current = value;
        }
    }
    Ok(())
}
pub fn reset() {
    if let Ok(mut current) = settings().write() {
        *current = Value::Null;
    }
}
