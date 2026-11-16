// SPDX-License-Identifier: Apache-2.0
use crate::{
    config,
    energy_rules::MachineMode,
    rules::{EfficiencyAugment, RecipeMode, Tier},
    state::*,
};
use mod_api::{Host, ModBlockEntityView};
const HEATER: &str = "ironfurnaces:heater";
const SETTINGS: [&str; 12] = [
    "Side0",
    "Side1",
    "Side2",
    "Side3",
    "Side4",
    "Side5",
    "AutoInput",
    "AutoOutput",
    "RedstoneMode",
    "RedstoneValue",
    "AugmentTab",
    "AutoSplit",
];
pub(super) fn display_pair(current: i32, total: i32) -> [i32; 2] {
    if total <= 0 {
        return [0, 1];
    }
    let shown = total.min(i32::from(i16::MAX));
    [
        (i64::from(current.clamp(0, total)) * i64::from(shown) / i64::from(total)) as i32,
        shown,
    ]
}
pub fn refresh_data(host: &dyn Host, e: &mut ModBlockEntityView<'_>) {
    e.data.resize(64, 0);
    let heater = e.id() == HEATER;
    let tier = Tier::from_block(e.id());
    let generator = !heater && mode(host, e) == MachineMode::Generator;
    let burn = display_pair(
        number(
            &e.nbt,
            if generator {
                "GeneratorRemaining"
            } else {
                "BurnTime"
            },
        ),
        number(
            &e.nbt,
            if generator {
                "GeneratorTotal"
            } else {
                "BurnTotal"
            },
        ),
    );
    let cook = display_pair(number(&e.nbt, "CookTime"), number(&e.nbt, "CookTotal"));
    e.data[0..2].copy_from_slice(&burn);
    e.data[2..4].copy_from_slice(&cook);
    e.data[4] = mode(host, e) as i32;
    e.data[5] = tier.map_or(0, |t| config::storage_tier(t) as i32);
    let cap = tier.map_or(config::heater_capacity(), config::energy_capacity);
    for (i, n) in [(6, number(&e.nbt, "energy").max(0) as u32), (8, cap)] {
        e.data[i] = (n & 65535) as i32;
        e.data[i + 1] = (n >> 16) as i32;
    }
    for i in 0..6 {
        let v = display_pair(
            number(&e.nbt, &format!("LaneProgress{i}")),
            number(&e.nbt, &format!("LaneTotal{i}")),
        );
        e.data[10 + i * 2..12 + i * 2].copy_from_slice(&v);
    }
    for (i, k) in SETTINGS.iter().enumerate() {
        e.data[22 + i] = number(&e.nbt, k);
    }
    e.data[34] = match recipe_mode(host, e) {
        RecipeMode::Smelting => 0,
        RecipeMode::Smoking => 1,
        RecipeMode::Blasting => 2,
    };
    e.data[35] = match efficiency(host, e) {
        EfficiencyAugment::None => 0,
        EfficiencyAugment::Speed => 1,
        EfficiencyAugment::Fuel => 2,
    };
    let rate = number(&e.nbt, "GenerationRate").max(0) as u32;
    e.data[36] = (rate & 65535) as i32;
    e.data[40] = (rate >> 16) as i32;
    e.data[37] = number(&e.nbt, "RainbowGen");
    e.data[38] = i32::from(heater);
    e.data[39] = number(&e.nbt, "Skin");
}

pub fn tick(host: &mut dyn mod_api::ServerHost, pos: [i32; 3], e: &mut ModBlockEntityView<'_>) {
    initialize(host, e);
    if e.id() == HEATER {
        crate::power::heater(host, pos, e);
        refresh_data(host, e);
        return;
    }
    let Some(tier) = Tier::from_block(e.id()) else {
        return;
    };
    let pending = number(&e.nbt, "PendingExtraction");
    e.nbt.remove(&"PendingExtraction".into());
    let count: i32 = [2, 13, 14, 15, 16, 17, 18]
        .into_iter()
        .filter_map(|i| e.items.get(i).and_then(Option::as_ref))
        .map(|s| s.count().min(i32::MAX as u32) as i32)
        .sum();
    if pending > count {
        award_experience(host, pos, e);
    }
    if let Some(owner) = crate::network::owner(&e.nbt) {
        crate::network::track(host, pos, owner);
    }
    let signatures: [String; 3] = std::array::from_fn(|i| item(host, e, 3 + i));
    let old = ["AugmentRecipe", "AugmentEfficiency", "AugmentMachine"]
        .map(|k| string(&e.nbt, k).unwrap_or_default());
    if e.nbt.contains_key(&"AugmentMachine".into()) && signatures != old {
        if signatures[2] != old[2]
            || (mode(host, e) == MachineMode::Generator && signatures[0] != old[0])
        {
            if let Some(level) = host.level(host.dimension()) {
                for i in [0, 1, 2, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18] {
                    if let Some(s) = e.items[i].take() {
                        let _ = level.spawn_item(
                            host,
                            f64::from(pos[0]) + 0.5,
                            f64::from(pos[1]) + 0.5,
                            f64::from(pos[2]) + 0.5,
                            s,
                            10,
                        );
                    }
                }
            }
        }
        for key in [
            "CookTime",
            "BurnTime",
            "BurnTotal",
            "GeneratorRemaining",
            "GeneratorTotal",
        ] {
            put(&mut e.nbt, key, 0);
        }
        for i in 0..6 {
            put(&mut e.nbt, &format!("LaneProgress{i}"), 0);
            put(&mut e.nbt, &format!("LaneReserved{i}"), 0);
        }
    }
    for (key, value) in ["AugmentRecipe", "AugmentEfficiency", "AugmentMachine"]
        .into_iter()
        .zip(signatures)
    {
        e.nbt
            .insert(key.into(), mod_api::Tag::String(value.as_str().into()));
    }
    let signal = host
        .level(host.dimension())
        .is_some_and(|l| l.has_neighbor_signal(host, pos));
    let run = match number(&e.nbt, "RedstoneMode") {
        1 => signal,
        2 => !signal,
        _ => true,
    };
    if run {
        crate::inventory::tick_io(host, pos, e);
    }
    match mode(host, e) {
        MachineMode::Furnace => crate::cooking::furnace(host, pos, e, tier, run),
        MachineMode::Factory => crate::cooking::factory(host, e, tier, run),
        MachineMode::Generator => {
            if run {
                crate::power::generator(host, pos, e, tier)
            } else {
                put(&mut e.nbt, "GenerationRate", 0);
                put(&mut e.nbt, "RainbowGen", 0);
            }
        }
    }
    refresh_data(host, e);
    if tier == Tier::Rainbow {
        host.set_block_property(
            pos,
            "rainbow",
            if number(&e.nbt, "RainbowGen") > 0 {
                "true"
            } else {
                "false"
            },
        );
    }
    let active = match mode(host, e) {
        MachineMode::Furnace => number(&e.nbt, "BurnTime") > 0,
        MachineMode::Generator => number(&e.nbt, "GenerationRate") > 0,
        MachineMode::Factory => run && number(&e.nbt, "FactoryActive") > 0,
    };
    for (key, value) in [
        ("lit", if active { "true" } else { "false" }.to_owned()),
        ("type", e.data[34].to_string()),
        ("jovial", number(&e.nbt, "Skin").clamp(0, 2).to_string()),
    ] {
        if host
            .level(host.dimension())
            .and_then(|l| l.block_property(host, pos, key))
            .as_deref()
            != Some(&value)
        {
            host.set_block_property(pos, key, &value);
        }
    }
}

fn award_experience(
    host: &mut dyn mod_api::ServerHost,
    pos: [i32; 3],
    e: &mut ModBlockEntityView<'_>,
) {
    use mod_api::{RandomSource, Tag};
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    let Some(Tag::Compound(used)) = e.nbt.remove(&"RecipesUsed".into()) else {
        return;
    };
    let recipes: Vec<_> = [
        "minecraft:smelting",
        "minecraft:blasting",
        "minecraft:smoking",
    ]
    .into_iter()
    .flat_map(|kind| host.recipes_of_type(kind).to_vec())
    .collect();
    let mut random = level.random(host);
    let mut points = 0u32;
    for (id, value) in used {
        let count = match value {
            Tag::Int(n) => n.max(0) as u32,
            _ => 0,
        };
        let xp = recipes
            .iter()
            .find(|r| r.id == id.to_string_lossy())
            .and_then(|r| r.experience)
            .unwrap_or(0.0);
        let total = count as f32 * xp;
        let whole = total.floor().max(0.0) as u32;
        let fraction = total - total.floor();
        points = points.saturating_add(
            whole
                + u32::from(
                    fraction > 0.0 && random.as_mut().is_some_and(|r| r.next_float() < fraction),
                ),
        );
    }
    if let Some(random) = random {
        level.set_random(host, random);
    }
    level.award_experience(
        host,
        f64::from(pos[0]) + 0.5,
        f64::from(pos[1]) + 0.5,
        f64::from(pos[2]) + 0.5,
        points,
    );
}
