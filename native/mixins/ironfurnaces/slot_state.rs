// Kept in the shared menu crate: menu rules are queried without a Game host.
use std::{collections::HashSet, ops::Range};

struct Context {
    mode: i32,
    active: Range<usize>,
    recipes: Option<HashSet<String>>,
    fuels: HashSet<String>,
    generator_fuels: HashSet<String>,
}
std::thread_local! {
    static CONTEXT: std::cell::RefCell<Option<Context>> = const { std::cell::RefCell::new(None) };
}

pub fn set_context(mode: i32, active: Range<usize>, recipe_items: HashSet<String>,
    fuels: HashSet<String>, generator_fuels: HashSet<String>) {
    CONTEXT.with(|current| *current.borrow_mut() = Some(Context {
        mode, active, recipes: Some(recipe_items), fuels, generator_fuels,
    }));
}

// Retained for existing packet hooks during an in-place native rebuild.
pub fn set_fuels(fuels: HashSet<String>) {
    CONTEXT.with(|current| *current.borrow_mut() = Some(Context {
        mode: 0, active: 0..0, recipes: None, fuels, generator_fuels: HashSet::new(),
    }));
}

pub fn clear_context() { CONTEXT.with(|current| *current.borrow_mut() = None); }

pub fn accepts(item: &str) -> bool {
    CONTEXT.with(|context| context.borrow().as_ref().is_none_or(|context| context.fuels.contains(item)))
}

pub fn slot_limit(menu: &str, slot: usize, item: &str) -> Option<u32> {
    if menu == "ironfurnaces:heater" || matches!(menu,"ironfurnaces:furnace" | "ironfurnaces:machine") &&
        ((3..=5).contains(&slot) || item == "ironfurnaces:item_heater" || slot == 1 && item == "minecraft:bucket")
    { Some(1) } else { None }
}

pub fn may_place(menu: &str, slot: usize, item: &str) -> bool {
    if menu == "ironfurnaces:heater" { return slot == 0 && item == "ironfurnaces:item_heater"; }
    if !matches!(menu,"ironfurnaces:furnace" | "ironfurnaces:machine") { return true; }
    match slot {
        3 => return matches!(item,"ironfurnaces:augment_smoking" | "ironfurnaces:augment_blasting"),
        4 => return matches!(item,"ironfurnaces:augment_speed" | "ironfurnaces:augment_fuel"),
        5 => return matches!(item,"ironfurnaces:augment_factory" | "ironfurnaces:augment_generator"),
        2 | 13..=18 => return false,
        _ => {}
    }
    CONTEXT.with(|context| {
        let context = context.borrow();
        // Client prediction has no authoritative registry. The server always
        // refreshes its context immediately before evaluating this menu click.
        let Some(context) = context.as_ref() else { return matches!(slot,0 | 1 | 6..=12); };
        match context.mode {
            0 => match slot {
                0 => context.recipes.as_ref().is_none_or(|recipes| recipes.contains(item)),
                1 => matches!(item,"minecraft:bucket" | "ironfurnaces:item_heater") || context.fuels.contains(item),
                _ => false,
            },
            1 => (7..=12).contains(&slot) && context.active.contains(&(slot-7)) &&
                context.recipes.as_ref().is_none_or(|recipes| recipes.contains(item)),
            2 => slot == 6 && context.generator_fuels.contains(item),
            _ => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set(mode:i32, active:Range<usize>) {
        set_context(mode,active,HashSet::from(["minecraft:raw_iron".into()]),
            HashSet::from(["minecraft:coal".into()]),HashSet::from(["minecraft:diamond".into()]));
    }
    #[test]
    fn only_active_factory_inputs_accept_the_current_recipe() {
        set(1,2..4);
        for slot in 0..19 {
            assert_eq!(may_place("ironfurnaces:machine",slot,"minecraft:raw_iron"), matches!(slot,9 | 10));
        }
        assert!(!may_place("ironfurnaces:machine",9,"minecraft:dirt"));
        clear_context();
    }
    #[test]
    fn generator_and_ordinary_fuel_slots_cannot_be_confused() {
        set(2,0..6);
        assert!(may_place("ironfurnaces:furnace",6,"minecraft:diamond"));
        assert!(!may_place("ironfurnaces:furnace",6,"minecraft:coal"));
        set_fuels(HashSet::from(["minecraft:coal".into()]));
        assert!(accepts("minecraft:coal"));
        assert!(!accepts("minecraft:dirt"));
        assert!(!may_place("ironfurnaces:furnace",1,"minecraft:diamond"));
        set(0,0..6);
        assert!(may_place("ironfurnaces:furnace",1,"minecraft:coal"));
        assert!(!may_place("ironfurnaces:furnace",1,"minecraft:dirt"));
        assert!(!may_place("ironfurnaces:furnace",6,"minecraft:coal"));
        clear_context();
    }
    #[test]
    fn augment_channels_results_and_linker_limits_are_enforced_without_context() {
        clear_context();
        assert!(may_place("ironfurnaces:furnace",3,"ironfurnaces:augment_smoking"));
        assert!(!may_place("ironfurnaces:furnace",3,"ironfurnaces:augment_factory"));
        assert!(may_place("ironfurnaces:furnace",5,"ironfurnaces:augment_factory"));
        assert!(!may_place("ironfurnaces:furnace",2,"minecraft:iron_ingot"));
        assert!(!may_place("ironfurnaces:furnace",18,"minecraft:iron_ingot"));
        assert!(!may_place("ironfurnaces:machine",18,"minecraft:iron_ingot"));
        assert_eq!(slot_limit("ironfurnaces:machine",5,"ironfurnaces:augment_factory"),Some(1));
        assert_eq!(slot_limit("ironfurnaces:furnace",5,"ironfurnaces:augment_factory"),Some(1));
        assert_eq!(slot_limit("ironfurnaces:furnace",1,"ironfurnaces:item_heater"),Some(1));
        assert!(may_place("ironfurnaces:heater",0,"ironfurnaces:item_heater"));
        assert!(!may_place("ironfurnaces:heater",0,"minecraft:coal"));
    }
    #[test]
    fn authoritative_rules_do_not_leak_into_client_prediction() {
        set(2,0..6);
        assert!(!may_place("ironfurnaces:machine",0,"minecraft:raw_iron"));
        std::thread::spawn(|| {
            assert!(may_place("ironfurnaces:machine",0,"minecraft:raw_iron"));
            assert!(!may_place("ironfurnaces:machine",2,"minecraft:iron_ingot"));
        }).join().unwrap();
        clear_context();
        assert!(may_place("ironfurnaces:machine",0,"minecraft:raw_iron"));
    }
}
