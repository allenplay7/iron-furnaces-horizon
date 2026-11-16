// Copyright 2026 Astryxion; Copyright 2026 Iron Furnaces Horizon port contributors.
// SPDX-License-Identifier: Apache-2.0
// Adapted from Fabric 26.3 at e1fabfe21abf5b11de1efb2b2f773d5561539d4c. See NOTICE.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tier { Copper, Iron, Silver, Gold, Diamond, Emerald, Crystal, Obsidian, Netherite, Rainbow, Allthemodium, Vibranium, Unobtainium }
pub const TIERS: [Tier; 13] = [Tier::Copper,Tier::Iron,Tier::Silver,Tier::Gold,Tier::Diamond,Tier::Emerald,Tier::Crystal,Tier::Obsidian,Tier::Netherite,Tier::Rainbow,Tier::Allthemodium,Tier::Vibranium,Tier::Unobtainium];
impl Tier {
    pub fn from_block(block: &str) -> Option<Self> { TIERS.into_iter().find(|tier| tier.block_id() == block) }
    pub const fn block_id(self) -> &'static str {
        match self {
            Self::Copper => "ironfurnaces:copper_furnace", Self::Iron => "ironfurnaces:iron_furnace",
            Self::Silver => "ironfurnaces:silver_furnace", Self::Gold => "ironfurnaces:gold_furnace",
            Self::Diamond => "ironfurnaces:diamond_furnace", Self::Emerald => "ironfurnaces:emerald_furnace",
            Self::Crystal => "ironfurnaces:crystal_furnace", Self::Obsidian => "ironfurnaces:obsidian_furnace",
            Self::Netherite => "ironfurnaces:netherite_furnace", Self::Rainbow => "ironfurnaces:million_furnace",
            Self::Allthemodium => "ironfurnaces:allthemodium_furnace", Self::Vibranium => "ironfurnaces:vibranium_furnace",
            Self::Unobtainium => "ironfurnaces:unobtainium_furnace",
        }
    }
    pub const fn base_ticks(self) -> u32 {
        match self {
            Self::Copper=>180, Self::Iron=>160, Self::Silver=>140, Self::Gold=>120, Self::Diamond=>80,
            Self::Emerald|Self::Crystal=>40, Self::Obsidian|Self::Rainbow=>20,
            Self::Netherite|Self::Allthemodium=>5, Self::Vibranium=>3, Self::Unobtainium=>1,
        }
    }
    pub fn cook_ticks(self, recipe_ticks:u32, augment:EfficiencyAugment)->u32 {
        let base=(u64::from(self.base_ticks())*u64::from(recipe_ticks)/200).max(1);
        let ticks=match augment { EfficiencyAugment::None=>base, EfficiencyAugment::Speed=>(base/2).max(1), EfficiencyAugment::Fuel=>(base*5).div_ceil(4) };
        ticks.min(i32::MAX as u64) as u32
    }
    pub const fn batch_limit(self)->u32 {
        match self { Self::Rainbow|Self::Unobtainium=>64, Self::Allthemodium=>16, Self::Vibranium=>32, _=>1 }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EfficiencyAugment { #[default] None, Speed, Fuel }
impl EfficiencyAugment {
    pub fn from_item(item:&str)->Option<Self> {
        match item { "ironfurnaces:augment_speed"=>Some(Self::Speed), "ironfurnaces:augment_fuel"=>Some(Self::Fuel), _=>None }
    }
    pub const fn item_id(self)->Option<&'static str> {
        match self { Self::None=>None, Self::Speed=>Some("ironfurnaces:augment_speed"), Self::Fuel=>Some("ironfurnaces:augment_fuel") }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RecipeMode { #[default] Smelting, Smoking, Blasting }
impl RecipeMode {
    pub fn from_item(item:&str)->Option<Self> {
        match item { "ironfurnaces:augment_smoking"=>Some(Self::Smoking), "ironfurnaces:augment_blasting"=>Some(Self::Blasting), _=>None }
    }
    pub const fn item_id(self)->Option<&'static str> {
        match self { Self::Smelting=>None, Self::Smoking=>Some("ironfurnaces:augment_smoking"), Self::Blasting=>Some("ironfurnaces:augment_blasting") }
    }
    pub const fn recipe_type(self)->&'static str {
        match self { Self::Smelting=>"minecraft:smelting", Self::Smoking=>"minecraft:smoking", Self::Blasting=>"minecraft:blasting" }
    }
}
// Use the unhalved furnace fuel duration even in smoking/blasting mode.
// Batch size intentionally does not affect burn duration. Zero means no burn.
pub fn fuel_ticks(vanilla:u32,cook:u32,augment:EfficiencyAugment)->u32 {
    let scaled=u64::from(vanilla)*u64::from(cook)/200;
    let ticks=match augment { EfficiencyAugment::None=>scaled, EfficiencyAugment::Speed=>scaled/2, EfficiencyAugment::Fuel=>scaled*2 };
    ticks.min(i32::MAX as u64) as u32
}
// Caller must also require output item/component equality.
pub fn batch_count(limit:u32,input:u32,output_count:u32,result_count:u32,max_stack:u32)->u32 {
    if result_count==0 { return 0; }
    let capacity=max_stack.min(64).saturating_sub(output_count);
    limit.min(input).min(capacity/result_count)
}
#[derive(Clone,Copy,Debug,Eq,PartialEq)]
pub struct Upgrade { pub source_block:&'static str, pub target:Tier }
pub fn upgrade(item:&str)->Option<Upgrade> {
    let (source_block,target)=match item {
        "ironfurnaces:upgrade_copper"=>("minecraft:furnace",Tier::Copper),
        "ironfurnaces:upgrade_iron"=>("minecraft:furnace",Tier::Iron),
        "ironfurnaces:upgrade_iron2"=>(Tier::Copper.block_id(),Tier::Iron),
        "ironfurnaces:upgrade_silver"=>(Tier::Copper.block_id(),Tier::Silver),
        "ironfurnaces:upgrade_silver2"=>(Tier::Iron.block_id(),Tier::Silver),
        "ironfurnaces:upgrade_gold"=>(Tier::Iron.block_id(),Tier::Gold),
        "ironfurnaces:upgrade_gold2"=>(Tier::Silver.block_id(),Tier::Gold),
        "ironfurnaces:upgrade_diamond"=>(Tier::Gold.block_id(),Tier::Diamond),
        "ironfurnaces:upgrade_emerald"=>(Tier::Diamond.block_id(),Tier::Emerald),
        "ironfurnaces:upgrade_crystal"=>(Tier::Diamond.block_id(),Tier::Crystal),
        "ironfurnaces:upgrade_obsidian"=>(Tier::Emerald.block_id(),Tier::Obsidian),
        "ironfurnaces:upgrade_obsidian2"=>(Tier::Crystal.block_id(),Tier::Obsidian),
        "ironfurnaces:upgrade_netherite"=>(Tier::Obsidian.block_id(),Tier::Netherite),
        "ironfurnaces:upgrade_allthemodium"=>(Tier::Netherite.block_id(),Tier::Allthemodium),
        "ironfurnaces:upgrade_vibranium"=>(Tier::Allthemodium.block_id(),Tier::Vibranium),
        "ironfurnaces:upgrade_unobtainium"=>(Tier::Vibranium.block_id(),Tier::Unobtainium),
        _=>return None,
    }; Some(Upgrade{source_block,target})
}
pub fn upgrade_target(item:&str,source_block:&str)->Option<Tier> {
    let upgrade=upgrade(item)?;
    (upgrade.source_block==source_block).then_some(upgrade.target)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn tiers_preserve_default_timing_and_identity() {
        for (tier,ticks) in TIERS.into_iter().zip([180,160,140,120,80,40,40,20,5,20,5,3,1]) {
            assert_eq!(Tier::from_block(tier.block_id()),Some(tier));
            assert_eq!(tier.cook_ticks(200,EfficiencyAugment::None),ticks);
        }
        assert_eq!(Tier::from_block("minecraft:furnace"),None);
        assert_eq!(Tier::from_block("othermod:iron_furnace"),None);
    }
    #[test] fn coal_economics_follow_augment_not_tier_speed() {
        for tier in TIERS {
            for (augment,operations) in [(EfficiencyAugment::None,8),(EfficiencyAugment::Speed,4),(EfficiencyAugment::Fuel,16)] {
                let cook=tier.cook_ticks(200,augment);
                assert_eq!(fuel_ticks(1600,cook,augment),cook*operations);
            }
        }
    }
    #[test] fn recipe_times_and_rounding_match_upstream() {
        assert_eq!(Tier::Iron.cook_ticks(100,EfficiencyAugment::None),80);
        assert_eq!(Tier::Gold.cook_ticks(400,EfficiencyAugment::None),240);
        assert_eq!(Tier::Netherite.cook_ticks(200,EfficiencyAugment::Speed),2);
        assert_eq!(Tier::Netherite.cook_ticks(200,EfficiencyAugment::Fuel),7);
        assert_eq!(Tier::Unobtainium.cook_ticks(100,EfficiencyAugment::None),1);
        assert_eq!(Tier::Unobtainium.cook_ticks(100,EfficiencyAugment::Speed),1);
        assert_eq!(Tier::Unobtainium.cook_ticks(100,EfficiencyAugment::Fuel),2);
    }
    #[test] fn zero_and_short_fuel_do_not_become_free_fuel() {
        assert_eq!(fuel_ticks(100,1,EfficiencyAugment::None),0);
        assert_eq!(fuel_ticks(100,1,EfficiencyAugment::Fuel),0);
        assert_eq!(fuel_ticks(300,1,EfficiencyAugment::Speed),0);
        assert_eq!(fuel_ticks(300,1,EfficiencyAugment::Fuel),2);
        assert_eq!(fuel_ticks(0,200,EfficiencyAugment::Fuel),0);
    }
    #[test] fn bulk_furnaces_respect_input_and_output_capacity() {
        assert_eq!(batch_count(Tier::Rainbow.batch_limit(),64,0,1,64),64);
        assert_eq!(batch_count(Tier::Allthemodium.batch_limit(),64,0,1,64),16);
        assert_eq!(batch_count(Tier::Vibranium.batch_limit(),64,0,1,64),32);
        assert_eq!(batch_count(Tier::Unobtainium.batch_limit(),7,0,1,64),7);
        assert_eq!(batch_count(64,64,60,2,64),2);
        assert_eq!(batch_count(64,64,0,1,16),16);
        assert_eq!(batch_count(64,64,0,1,99),64);
        assert_eq!(batch_count(64,64,64,1,64),0);
        assert_eq!(batch_count(64,64,65,1,64),0);
        assert_eq!(batch_count(64,64,0,0,64),0);
    }
    #[test] fn upgrades_require_exact_original_routes() {
        assert_eq!(upgrade_target("ironfurnaces:upgrade_iron","minecraft:furnace"),Some(Tier::Iron));
        assert_eq!(upgrade_target("ironfurnaces:upgrade_iron2",Tier::Copper.block_id()),Some(Tier::Iron));
        assert_eq!(upgrade_target("ironfurnaces:upgrade_iron",Tier::Copper.block_id()),None);
        assert_eq!(upgrade_target("ironfurnaces:upgrade_gold",Tier::Diamond.block_id()),None);
        assert_eq!(upgrade_target("ironfurnaces:upgrade_obsidian2",Tier::Crystal.block_id()),Some(Tier::Obsidian));
        assert_eq!(upgrade_target("ironfurnaces:upgrade_allthemodium",Tier::Netherite.block_id()),Some(Tier::Allthemodium));
        assert_eq!(upgrade_target("ironfurnaces:upgrade_vibranium",Tier::Allthemodium.block_id()),Some(Tier::Vibranium));
        assert_eq!(upgrade_target("ironfurnaces:upgrade_unobtainium",Tier::Vibranium.block_id()),Some(Tier::Unobtainium));
        assert_eq!(upgrade("ironfurnaces:upgrade_rainbow"),None);
    }
    #[test] fn augment_channels_and_roundtrips() {
        for augment in [EfficiencyAugment::Speed,EfficiencyAugment::Fuel] { assert_eq!(EfficiencyAugment::from_item(augment.item_id().unwrap()),Some(augment)); }
        for mode in [RecipeMode::Smoking,RecipeMode::Blasting] { assert_eq!(RecipeMode::from_item(mode.item_id().unwrap()),Some(mode)); }
        assert_eq!(EfficiencyAugment::None.item_id(),None);
        assert_eq!(RecipeMode::Smelting.item_id(),None);
        assert_eq!(RecipeMode::from_item("ironfurnaces:augment_speed"),None);
        assert_eq!(EfficiencyAugment::from_item("ironfurnaces:augment_smoking"),None);
        assert_eq!(RecipeMode::Smoking.recipe_type(),"minecraft:smoking");
    }
    #[test] fn extreme_data_cannot_overflow_signed_timers() {
        assert_eq!(Tier::Copper.cook_ticks(u32::MAX,EfficiencyAugment::Fuel),i32::MAX as u32);
        assert_eq!(fuel_ticks(u32::MAX,u32::MAX,EfficiencyAugment::Fuel),i32::MAX as u32);
    }
}
