// Copyright 2026 Astryxion; Copyright 2026 Iron Furnaces Horizon port contributors.
// SPDX-License-Identifier: Apache-2.0
// Adapted from Fabric 26.3 at e1fabfe21abf5b11de1efb2b2f773d5561539d4c. See NOTICE.
// World access, item components, recipes and persistence belong in the adapter.
use super::iron_furnaces_rules::{EfficiencyAugment, RecipeMode, Tier};
use std::ops::Range;

pub const MAX_ENERGY: u32 = i32::MAX as u32;
pub const HEATER_CAPACITY: u32 = 1_000_000;
pub const WIRELESS_ENERGY_COST: u32 = 2_000;
pub const RAINBOW_GENERATION: u32 = 50_000;
pub const RAINBOW_COAL_USES: u32 = 5_120;
pub const RAINBOW_COAL_FUEL_TICKS: u32 = 200;
pub const FACTORY_LANES: usize = 6;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum MachineMode {
    #[default]
    Furnace = 0,
    Factory = 1,
    Generator = 2,
}

impl MachineMode {
    pub fn from_item(item: &str) -> Option<Self> {
        match item {
            "ironfurnaces:augment_factory" => Some(Self::Factory),
            "ironfurnaces:augment_generator" => Some(Self::Generator),
            _ => None,
        }
    }

    pub const fn item_id(self) -> Option<&'static str> {
        match self {
            Self::Furnace => None,
            Self::Factory => Some("ironfurnaces:augment_factory"),
            Self::Generator => Some("ironfurnaces:augment_generator"),
        }
    }

    pub const fn from_value(value: i32) -> Self {
        match value {
            1 => Self::Factory,
            2 => Self::Generator,
            _ => Self::Furnace,
        }
    }

    pub const fn can_receive_energy(self) -> bool { matches!(self, Self::Factory) }
    pub const fn can_extract_energy(self) -> bool { matches!(self, Self::Generator) }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AugmentChannel { Recipe, Efficiency, Machine }

pub fn augment_channel(item: &str) -> Option<AugmentChannel> {
    if RecipeMode::from_item(item).is_some() {
        Some(AugmentChannel::Recipe)
    } else if EfficiencyAugment::from_item(item).is_some() {
        Some(AugmentChannel::Efficiency)
    } else if MachineMode::from_item(item).is_some() {
        Some(AugmentChannel::Machine)
    } else {
        None
    }
}

pub const fn storage_tier(tier: Tier) -> u8 {
    match tier {
        Tier::Copper | Tier::Iron => 0,
        Tier::Silver | Tier::Gold => 1,
        _ => 2,
    }
}

pub const fn energy_capacity(tier: Tier) -> u32 {
    match storage_tier(tier) {
        0 => 80_000,
        1 => 200_000,
        _ => 1_000_000,
    }
}

// These are indexes within the six lanes, not inventory slot numbers.
// Upstream inputs are 7 + lane; corresponding outputs are 13 + lane.
pub const fn active_factory_lanes(tier: Tier) -> Range<usize> {
    match storage_tier(tier) {
        0 => 2..4,
        1 => 1..5,
        _ => 0..6,
    }
}

pub const fn base_generation_rate(tier: Tier) -> u32 {
    match tier {
        Tier::Copper | Tier::Iron => 40,
        Tier::Silver => 100,
        Tier::Gold => 160,
        Tier::Diamond => 240,
        Tier::Emerald => 320,
        Tier::Crystal => 360,
        Tier::Obsidian => 500,
        Tier::Netherite => 1_000,
        Tier::Rainbow | Tier::Allthemodium => 2_000,
        Tier::Vibranium => 3_000,
        Tier::Unobtainium => 5_000,
    }
}

pub fn adjusted_generation_rate(base: u32, augment: EfficiencyAugment) -> u32 {
    let base = u64::from(base);
    bounded_energy(match augment {
        EfficiencyAugment::None => base,
        EfficiencyAugment::Speed => base * 2,
        EfficiencyAugment::Fuel => base * 3 / 4,
    })
}

pub fn generation_rate(tier: Tier, augment: EfficiencyAugment) -> u32 {
    adjusted_generation_rate(base_generation_rate(tier), augment)
}

fn bounded_energy(value: u64) -> u32 { value.min(u64::from(MAX_ENERGY)) as u32 }

// Java represents fuel as double "burn" units and spends generation / 20.
// Keeping a finite integer FE budget avoids rounding drift on the last tick.
// The integer burn calculation (including augment division) happens first.
pub fn generator_fuel_energy(
    mode: RecipeMode,
    augment: EfficiencyAugment,
    vanilla_ticks: u32,
    nutrition: u32,
    blasting_energy: u32,
) -> u32 {
    let burn = match mode {
        RecipeMode::Smelting => u64::from(vanilla_ticks),
        RecipeMode::Smoking => u64::from(nutrition) * 800,
        RecipeMode::Blasting => u64::from(blasting_energy / 20),
    };
    let adjusted = match augment {
        EfficiencyAugment::None => burn,
        EfficiencyAugment::Speed => burn / 2,
        EfficiencyAugment::Fuel => burn * 2,
    };
    bounded_energy(adjusted * 20)
}

pub const BLASTING_FUELS: [(&str, u32); 12] = [
    ("minecraft:amethyst_shard", 40_000),
    ("minecraft:copper_ingot", 10_000),
    ("minecraft:diamond", 500_000),
    ("minecraft:emerald", 125_000),
    ("minecraft:gold_ingot", 40_000),
    ("minecraft:iron_ingot", 20_000),
    ("minecraft:lapis_lazuli", 40_000),
    ("minecraft:nether_star", 1_000_000),
    ("minecraft:netherite_ingot", 750_000),
    ("minecraft:prismarine_shard", 40_000),
    ("minecraft:quartz", 40_000),
    ("minecraft:redstone", 10_000),
];

pub fn default_blasting_energy(item: &str) -> Option<u32> {
    BLASTING_FUELS.iter().find_map(|(fuel, energy)| (*fuel == item).then_some(*energy))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratorStep {
    pub generated: u32,
    pub stored: u32,
    pub remaining: u32,
}

// A full destination pauses the fuel budget. A partially full destination
// consumes only energy actually produced, so neither overflow nor the final
// partial tick destroys fuel or creates extra energy.
pub fn generator_step(stored: u32, capacity: u32, remaining: u32, rate: u32) -> GeneratorStep {
    let capacity = capacity.min(MAX_ENERGY);
    let stored = stored.min(capacity);
    let generated = rate.min(remaining).min(capacity - stored);
    GeneratorStep { generated, stored: stored + generated, remaining: remaining - generated }
}

// Cost is per completed lane operation, including an entire tier batch.
pub fn factory_recipe_energy(recipe_ticks: u32, augment: EfficiencyAugment) -> u32 {
    let energy = u64::from(recipe_ticks) * 20;
    bounded_energy(match augment {
        EfficiencyAugment::None => energy,
        EfficiencyAugment::Speed => energy * 2,
        EfficiencyAugment::Fuel => energy / 2,
    })
}

// Progress is the count of already completed ticks. Upstream floors each
// ordinary tick's debit, then charges the remainder on the completion tick.
pub fn factory_tick_debit(cost: u32, duration: u32, progress: u32) -> u32 {
    if duration == 0 || progress >= duration { return 0; }
    let ordinary = cost / duration;
    if progress == duration - 1 { cost - ordinary * (duration - 1) } else { ordinary }
}

pub fn factory_remaining_energy(cost: u32, duration: u32, progress: u32) -> u32 {
    if duration == 0 || progress >= duration { return 0; }
    cost - (cost / duration) * progress
}

pub fn factory_reserved_energy(
    costs: &[u32; FACTORY_LANES],
    durations: &[u32; FACTORY_LANES],
    progress: &[u32; FACTORY_LANES],
) -> u32 {
    (0..FACTORY_LANES).filter(|&lane| progress[lane] > 0).fold(0, |sum, lane| {
        sum.saturating_add(factory_remaining_energy(costs[lane], durations[lane], progress[lane]))
    })
}

pub const fn factory_can_start(stored: u32, reserved: u32, cost: u32) -> bool {
    cost > 0 && stored >= reserved && stored - reserved >= cost
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FactoryStep {
    pub progress: u32,
    pub consumed: u32,
    pub stored: u32,
    pub completed: bool,
}

// Reservations keep simultaneous lanes from promising the same stored FE.
// If a caller restores an underfunded save, an active lane pauses safely.
pub fn factory_step(
    stored: u32,
    other_reserved: u32,
    progress: u32,
    duration: u32,
    cost: u32,
) -> FactoryStep {
    let paused = FactoryStep { progress, consumed: 0, stored, completed: false };
    if duration == 0 || cost == 0 || progress >= duration { return paused; }
    if progress == 0 && !factory_can_start(stored, other_reserved, cost) { return paused; }
    let consumed = factory_tick_debit(cost, duration, progress);
    if stored < consumed { return paused; }
    let completed = progress == duration - 1;
    FactoryStep {
        progress: if completed { 0 } else { progress + 1 },
        consumed,
        stored: stored - consumed,
        completed,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WirelessIgnition { pub burn_ticks: u32, pub energy_cost: u32 }

// Binding and loaded-block lookup are adapter responsibilities. Java applies
// no distance limit: it resolves the three stored coordinates in this level.
// Speed's 4000 FE prerequisite and 2000 FE debit intentionally differ.
pub fn wireless_ignition(
    stored: u32, cook_ticks: u32, augment: EfficiencyAugment,
) -> Option<WirelessIgnition> {
    let required = if augment == EfficiencyAugment::Speed { 4_000 } else { WIRELESS_ENERGY_COST };
    if stored < required { return None; }
    let burn_ticks = match augment {
        EfficiencyAugment::None => cook_ticks.min(MAX_ENERGY),
        EfficiencyAugment::Speed => (cook_ticks / 2).min(MAX_ENERGY),
        EfficiencyAugment::Fuel => bounded_energy(u64::from(cook_ticks) * 2),
    };
    (burn_ticks > 0).then_some(WirelessIgnition { burn_ticks, energy_cost: WIRELESS_ENERGY_COST })
}

pub fn rainbow_coal_after_use(damage: u32) -> Option<u32> {
    (damage < RAINBOW_COAL_USES - 1).then(|| damage + 1)
}

// Compute accepted FE before withdrawing it from the source. In particular,
// a generator cannot receive a refund because its receive limit is zero.
pub fn energy_transfer_amount(
    source_energy: u32, source_extract_limit: u32,
    target_energy: u32, target_capacity: u32, target_receive_limit: u32,
) -> u32 {
    source_energy.min(source_extract_limit)
        .min(target_capacity.saturating_sub(target_energy)).min(target_receive_limit)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum SideMode { #[default] Disabled = 0, Input = 1, Output = 2, Both = 3, Fuel = 4 }

impl SideMode {
    pub const fn from_value(value: i32) -> Self {
        match value { 1 => Self::Input, 2 => Self::Output, 3 => Self::Both, 4 => Self::Fuel, _ => Self::Disabled }
    }
    pub const fn allows_input(self) -> bool { matches!(self, Self::Input | Self::Both) }
    pub const fn allows_output(self) -> bool { matches!(self, Self::Output | Self::Both) }
    pub const fn allows_fuel(self) -> bool { matches!(self, Self::Fuel) }
}

pub const RAINBOW_REQUIRED_TIERS: [Tier; 9] = [
    Tier::Copper, Tier::Iron, Tier::Silver, Tier::Gold, Tier::Diamond,
    Tier::Emerald, Tier::Crystal, Tier::Obsidian, Tier::Netherite,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratorPresence {
    pub tier: Tier,
    pub running: bool,
    pub energy: u32,
    pub capacity: u32,
}

// Caller supplies the owner's generators in this level. The Java owner list
// has no radius filter; ATM tiers cannot substitute for one of the nine tiers.
pub fn rainbow_ready(
    owner_online: bool, owned_rainbow_generators: usize, generators: &[GeneratorPresence],
) -> bool {
    owner_online && owned_rainbow_generators <= 1 && RAINBOW_REQUIRED_TIERS.iter().all(|tier| {
        generators.iter().any(|generator| generator.tier == *tier && generator.running && generator.energy < generator.capacity)
    })
}

// The bonus goes straight to adjacent accepting output faces. It is separate
// from the Rainbow furnace's 2000 FE/t ordinary fuel budget and not stored.
pub fn rainbow_receiver_share(receivers: usize) -> u32 {
    if receivers == 0 { 0 } else { (u64::from(RAINBOW_GENERATION) / receivers as u64) as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::iron_furnaces_rules::TIERS;

    #[test]
    fn capacities_and_active_lanes_match_upstream_tiers() {
        for (tier, rate) in TIERS.into_iter().zip([40,40,100,160,240,320,360,500,1000,2000,2000,3000,5000]) {
            assert_eq!(generation_rate(tier, EfficiencyAugment::None), rate);
            let (capacity, lanes) = match tier {
                Tier::Copper | Tier::Iron => (80_000, 2..4),
                Tier::Silver | Tier::Gold => (200_000, 1..5),
                _ => (1_000_000, 0..6),
            };
            assert_eq!(energy_capacity(tier), capacity);
            assert_eq!(active_factory_lanes(tier), lanes);
        }
    }

    #[test]
    fn coal_yield_depends_on_augment_not_generator_tier() {
        for tier in TIERS {
            for (augment, yield_fe) in [(EfficiencyAugment::None,32_000), (EfficiencyAugment::Speed,16_000), (EfficiencyAugment::Fuel,64_000)] {
                let mut remaining = generator_fuel_energy(RecipeMode::Smelting, augment, 1600, 0, 0);
                let mut generated = 0;
                let mut ticks = 0;
                while remaining > 0 {
                    let step = generator_step(0, energy_capacity(tier), remaining, generation_rate(tier, augment));
                    generated += step.generated;
                    remaining = step.remaining;
                    ticks += 1;
                    assert!(ticks <= 2200);
                }
                assert_eq!(generated, yield_fe);
            }
        }
    }

    #[test]
    fn full_or_small_buffers_cannot_destroy_or_create_fuel_energy() {
        assert_eq!(generator_step(100, 100, 77, 40), GeneratorStep { generated: 0, stored: 100, remaining: 77 });
        let partial = generator_step(95, 100, 77, 40);
        assert_eq!(partial, GeneratorStep { generated: 5, stored: 100, remaining: 72 });
        assert_eq!(generator_step(0, 100, 7, 40), GeneratorStep { generated: 7, stored: 7, remaining: 0 });
    }

    #[test]
    fn generator_modes_preserve_source_rounding_and_food_energy() {
        assert_eq!(generator_fuel_energy(RecipeMode::Smoking, EfficiencyAugment::None, 0, 6, 0), 96_000);
        assert_eq!(generator_fuel_energy(RecipeMode::Smoking, EfficiencyAugment::None, 1600, 0, 0), 0);
        assert_eq!(generator_fuel_energy(RecipeMode::Blasting, EfficiencyAugment::None, 0, 0, 39), 20);
        assert_eq!(generator_fuel_energy(RecipeMode::Blasting, EfficiencyAugment::Speed, 0, 0, 39), 0);
        for (item, energy) in BLASTING_FUELS {
            assert_eq!(default_blasting_energy(item), Some(energy));
            assert_eq!(generator_fuel_energy(RecipeMode::Blasting, EfficiencyAugment::None, 0, 0, energy), energy);
        }
        assert_eq!(default_blasting_energy("minecraft:raw_iron"), None);
    }

    fn run_six_lanes(mut stored: u32, duration: u32, cost: u32) -> (u32, [u32; 6], u32) {
        let mut progress = [0; 6];
        let mut completed = [0; 6];
        let mut consumed = 0;
        for _ in 0..duration {
            for lane in 0..6 {
                if completed[lane] > 0 { continue; }
                let all_reserved = factory_reserved_energy(&[cost;6], &[duration;6], &progress);
                let own_reserved = if progress[lane] > 0 { factory_remaining_energy(cost, duration, progress[lane]) } else { 0 };
                let step = factory_step(stored, all_reserved - own_reserved, progress[lane], duration, cost);
                progress[lane] = step.progress;
                stored = step.stored;
                consumed += step.consumed;
                completed[lane] += u32::from(step.completed);
            }
        }
        (stored, completed, consumed)
    }

    #[test]
    fn six_factory_lanes_cannot_reserve_the_same_scarce_energy() {
        assert_eq!(run_six_lanes(4000,160,4000), (0,[1,0,0,0,0,0],4000));
        assert_eq!(run_six_lanes(24_000,160,4000), (0,[1,1,1,1,1,1],24_000));
    }

    #[test]
    fn factory_partial_debits_and_last_tick_charge_exact_recipe_cost() {
        for tier in TIERS {
            for augment in [EfficiencyAugment::None,EfficiencyAugment::Speed,EfficiencyAugment::Fuel] {
                let duration = tier.cook_ticks(200, augment);
                let cost = factory_recipe_energy(200, augment);
                let mut spent = 0;
                for progress in 0..duration {
                    assert_eq!(factory_remaining_energy(cost,duration,progress), cost-spent);
                    spent += factory_tick_debit(cost,duration,progress);
                }
                assert_eq!(spent, cost);
                assert_eq!(factory_remaining_energy(cost,duration,duration), 0);
            }
        }
        assert_eq!(factory_recipe_energy(200,EfficiencyAugment::None), 4000);
        assert_eq!(factory_recipe_energy(200,EfficiencyAugment::Speed), 8000);
        assert_eq!(factory_recipe_energy(200,EfficiencyAugment::Fuel), 2000);
    }

    #[test]
    fn factory_stalls_if_restored_progress_has_no_energy() {
        let stalled = factory_step(0,0,80,160,4000);
        assert_eq!(stalled, FactoryStep { progress:80, consumed:0, stored:0, completed:false });
        assert!(!factory_can_start(3999,0,4000));
        assert!(!factory_can_start(8000,4001,4000));
        assert!(factory_can_start(8000,4000,4000));
        assert!(!factory_can_start(10,20,1));
    }

    #[test]
    fn wireless_ignition_obeys_cost_and_the_speed_prerequisite() {
        assert_eq!(wireless_ignition(1999,160,EfficiencyAugment::None), None);
        assert_eq!(wireless_ignition(2000,160,EfficiencyAugment::None), Some(WirelessIgnition { burn_ticks:160, energy_cost:2000 }));
        assert_eq!(wireless_ignition(2000,200,EfficiencyAugment::Fuel), Some(WirelessIgnition { burn_ticks:400, energy_cost:2000 }));
        assert_eq!(wireless_ignition(3999,80,EfficiencyAugment::Speed), None);
        assert_eq!(wireless_ignition(4000,80,EfficiencyAugment::Speed), Some(WirelessIgnition { burn_ticks:40, energy_cost:2000 }));
        assert_eq!(wireless_ignition(4000,1,EfficiencyAugment::Speed), None);
        assert_eq!(wireless_ignition(2000,0,EfficiencyAugment::Fuel), None);
    }

    #[test]
    fn transfer_only_debits_energy_the_destination_accepts() {
        assert_eq!(energy_transfer_amount(1000,1000,995,1000,1000), 5);
        assert_eq!(energy_transfer_amount(1000,1000,0,1000,0), 0);
        assert_eq!(energy_transfer_amount(1000,3,0,1000,1000), 3);
        assert_eq!(energy_transfer_amount(2,1000,0,1000,1000), 2);
        assert_eq!(energy_transfer_amount(1000,1000,1001,1000,1000), 0);
    }

    #[test]
    fn rainbow_requires_each_distinct_active_tier_and_single_owned_generator() {
        let mut contributors: Vec<_> = RAINBOW_REQUIRED_TIERS.into_iter().map(|tier| GeneratorPresence {
            tier, running:true, energy:1, capacity:energy_capacity(tier),
        }).collect();
        assert!(rainbow_ready(true,1,&contributors));
        assert!(!rainbow_ready(false,1,&contributors));
        assert!(!rainbow_ready(true,2,&contributors));
        contributors[6].tier = Tier::Unobtainium;
        assert!(!rainbow_ready(true,1,&contributors));
        contributors[6].tier = Tier::Crystal;
        contributors[0].energy = contributors[0].capacity;
        assert!(!rainbow_ready(true,1,&contributors));
        contributors[0].energy = 0;
        contributors[0].running = false;
        assert!(!rainbow_ready(true,1,&contributors));
        assert_eq!(rainbow_receiver_share(0),0);
        assert_eq!(rainbow_receiver_share(1),50_000);
        assert_eq!(rainbow_receiver_share(3),16_666);
    }

    #[test]
    fn rainbow_coal_is_consumed_after_exactly_5120_uses() {
        let mut damage = 0;
        let mut uses = 0;
        loop {
            uses += 1;
            match rainbow_coal_after_use(damage) { Some(next) => damage=next, None => break }
        }
        assert_eq!(uses,RAINBOW_COAL_USES);
        assert_eq!(rainbow_coal_after_use(u32::MAX),None);
        assert_eq!(generator_fuel_energy(RecipeMode::Smelting,EfficiencyAugment::None,RAINBOW_COAL_FUEL_TICKS,0,0),4000);
    }

    #[test]
    fn augment_channels_and_energy_direction_are_independent() {
        assert_eq!(augment_channel("ironfurnaces:augment_smoking"),Some(AugmentChannel::Recipe));
        assert_eq!(augment_channel("ironfurnaces:augment_fuel"),Some(AugmentChannel::Efficiency));
        assert_eq!(augment_channel("ironfurnaces:augment_generator"),Some(AugmentChannel::Machine));
        assert_eq!(augment_channel("minecraft:coal"),None);
        assert_eq!(MachineMode::from_item("ironfurnaces:augment_factory"),Some(MachineMode::Factory));
        assert!(MachineMode::Factory.can_receive_energy());
        assert!(!MachineMode::Factory.can_extract_energy());
        assert!(!MachineMode::Generator.can_receive_energy());
        assert!(MachineMode::Generator.can_extract_energy());
        assert!(!SideMode::Fuel.allows_output());
        assert!(SideMode::Both.allows_output());
    }
}
