// Copyright 2026 Iron Furnaces Horizon port contributors.
// SPDX-License-Identifier: Apache-2.0
// Unofficial adaptation of Astryxion's Iron Furnaces; see LICENSE and NOTICE.
use mod_api::*;

pub mod config;
mod cooking;
pub mod energy_rules;
mod inventory;
mod network;
mod power;
pub mod rules;
mod runtime;
mod state;
mod tools;
pub mod ui;

#[derive(Default)]
struct IronFurnaces;
#[mod_api::horizon_mod]
impl Hooks for IronFurnaces {
    fn item_tooltip(&self, h: &dyn ClientHost, e: &mut ItemTooltipEvent<'_>) {
        ui::item_tooltip(h, e.stack, e.lines);
    }
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        r.register_recipe_type("ironfurnaces:generator_blasting");
        r.register_raw_recipe_serializer(
            "ironfurnaces:generator_blasting",
            "ironfurnaces:generator_blasting",
        );
        let _ = network::RAINBOW.set(r.register_ticket_controller("rainbow", false));
        let _ = network::WIRELESS.set(r.register_ticket_controller("wireless", false));
        for id in rules::TIERS
            .into_iter()
            .map(rules::Tier::block_id)
            .chain(std::iter::once("ironfurnaces:heater"))
        {
            let block = r.block_key(id);
            // Registering the entity part also attaches its BlockPart callbacks.
            let item = r.item_key(id);
            r.item_part(item, Tool);
            let kind = r.block_entity_type(id).blocks(&[id]).build();
            r.block_entity_part(&kind, Machine)
                .expect("unique machine part");
            let heater = id == "ironfurnaces:heater";
            let slots = if heater { 1 } else { 19 };
            r.block_container(
                &block,
                ModContainer {
                    slots,
                    faces: std::array::from_fn(|_| (0..slots).collect()),
                    inputs: if heater {
                        vec![0]
                    } else {
                        vec![0, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
                    },
                    outputs: if heater {
                        vec![0]
                    } else {
                        vec![1, 2, 6, 13, 14, 15, 16, 17, 18]
                    },
                    menu: Some(
                        if heater {
                            "ironfurnaces:heater"
                        } else {
                            "ironfurnaces:machine"
                        }
                        .into(),
                    ),
                },
            );
            let tier = rules::Tier::from_block(id);
            r.block_entity_capability_before_game(&ENERGY, &kind, move |_, at| {
                Some(Box::new(power::MachineEnergy(at, tier)))
            });
            r.block_entity_capability_before_game(&ITEMS, &kind, |_, at| {
                Some(Box::new(inventory::MachineItems(at)))
            });
        }
        for id in OWN_ITEMS {
            let item = r.item_key(id);
            r.item_part(item, Tool);
        }
        r.menu_part("ironfurnaces:machine", MachineMenu);
        r.menu_part("ironfurnaces:heater", MachineMenu);
    }
    fn explosion_detonate(&self, h: &mut dyn ServerHost, e: &mut ExplosionDetonateEvent<'_>) {
        let Some(level) = h.level(e.dimension) else {
            return;
        };
        e.blocks.retain(|&pos| {
            if level.block(h, pos).map(|b| b.id(h)).as_deref()
                != Some("ironfurnaces:million_furnace")
            {
                return true;
            }
            let Some(air) = h.block_state_id("minecraft:air") else {
                return true;
            };
            if level
                .set_block(
                    h,
                    pos,
                    air,
                    BlockUpdateFlags::NEIGHBORS
                        | BlockUpdateFlags::CLIENTS
                        | BlockUpdateFlags::SKIP_BLOCK_ENTITY_SIDE_EFFECTS,
                )
                .unwrap_or(false)
            {
                if let Some(s) = state::stack(h, "ironfurnaces:rainbow_coal", 1) {
                    let _ = level.spawn_item(
                        h,
                        f64::from(pos[0]),
                        f64::from(pos[1]) + 6.0,
                        f64::from(pos[2]),
                        s,
                        10,
                    );
                }
                false
            } else {
                true
            }
        });
    }
    fn level_tick_post(&self, h: HostSide, e: &mut LevelTickPostEvent) {
        if let HostSide::Server(h) = h {
            network::maintenance(h, e.level);
        }
    }
    fn register_configs(&self, _: HostSide, e: &mut RegisterConfigsEvent<'_>) {
        e.configs.push(
            Config::new("ironfurnaces", ConfigKind::Server, &config::default_text())
                .file("ironfurnaces.json"),
        );
    }
    fn config_loading(&self, _: HostSide, e: &mut ConfigLoadingEvent<'_>) {
        if e.config.mod_id == "ironfurnaces" {
            let _ = config::load_text(&e.config.text);
        }
    }
    fn config_reloading(&self, _: HostSide, e: &mut ConfigReloadingEvent<'_>) {
        if e.config.mod_id == "ironfurnaces" {
            let _ = config::load_text(&e.config.text);
        }
    }
    fn config_unloading(&self, _: &mut dyn ServerHost, e: &mut ConfigUnloadingEvent<'_>) {
        if e.config.mod_id == "ironfurnaces" {
            config::reset();
        }
    }
}
struct Machine;
#[mod_api::horizon_part]
impl BlockPart for Machine {
    fn has_analog_output(&self, _: &dyn LevelHost, e: &mut HasAnalogOutputEvent) {
        e.output = true;
    }
    fn is_signal_source(&self, _: &dyn LevelHost, e: &mut IsSignalSourceEvent) {
        e.source = true;
    }
    fn use_item_on(
        &self,
        h: &mut dyn ServerHost,
        e: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        tools::interact(h, e.click)
    }
    fn use_without_item(
        &self,
        h: &mut dyn ServerHost,
        e: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        tools::interact(h, e.click)
    }
}
#[mod_api::horizon_part]
impl BlockEntityPart for Machine {
    fn block_entity(&self, _: LevelSide, _: &mut BlockEntityEvent<'_>) -> Option<Compound> {
        let mut nbt = Compound::new();
        state::put(&mut nbt, "Version", 2);
        for (side, value) in [2, 1, 4, 4, 4, 4].into_iter().enumerate() {
            state::put(&mut nbt, &format!("Side{side}"), value);
        }
        Some(nbt)
    }
    fn screen(&self, _: &dyn Host, e: &mut ScreenEvent<'_>) -> Option<Screen> {
        Some(ui::layout(e.block, &[]))
    }
    fn screen_draw(&self, h: &dyn ClientHost, e: &mut ScreenDrawEvent<'_>) {
        ui::draw(h, e.block, e.data, e.draw, e.origin);
    }
    fn screen_click(&self, _: &dyn ClientHost, e: &mut ScreenClickEvent<'_>) -> Option<i32> {
        let mut out = None;
        ui::click(e.block, e.data, e.at, e.right, &mut out);
        out
    }
    fn screen_tooltip(&self, h: &dyn ClientHost, e: &mut ScreenTooltipEvent<'_>) {
        ui::tooltip(h, e.block, e.data, e.at, e.lines);
    }
    fn block_entity_ticks(&self, _: &dyn Host, e: &mut BlockEntityTicksEvent<'_>) {
        e.ticks = true;
    }
    fn block_entity_tick(&self, h: &mut dyn ServerHost, e: &mut BlockEntityTickEvent<'_>) {
        runtime::tick(h, e.pos, &mut e.entity);
    }
    fn menu_button(&self, h: &mut dyn ServerHost, e: &mut MenuButtonEvent<'_>) {
        ui::button(&mut e.entity, e.context.id);
        runtime::refresh_data(h, &mut e.entity);
    }
}
struct MachineMenu;
#[mod_api::horizon_part]
impl MenuPart for MachineMenu {
    fn menu_slot_rules(&self, h: &dyn Host, e: &mut MenuSlotRulesEvent<'_>) {
        if e.container != MenuContainer::Main {
            return;
        }
        let id = e
            .context
            .pos
            .and_then(|p| h.level(h.dimension()).and_then(|l| l.block(h, p)))
            .map(|b| b.id(h))
            .unwrap_or_else(|| {
                if e.menu == "ironfurnaces:heater" {
                    "ironfurnaces:heater"
                } else {
                    "ironfurnaces:iron_furnace"
                }
                .into()
            });
        let mut saved = ModBlockEntity::new(id, Default::default());
        saved.items = e.context.slots.first().cloned().unwrap_or_default();
        let mut view = ModBlockEntityView::new(&mut saved);
        state::initialize(h, &mut view);
        e.may_place = inventory::accepts(h, &view, e.slot, e.stack);
        e.max_stack = inventory::limit(h, e.slot, e.stack);
    }
}

struct Tool;
#[mod_api::horizon_part]
impl ItemPart for Tool {
    fn on_item_use_first(
        &self,
        h: &mut dyn ServerHost,
        e: &mut OnItemUseFirstEvent<'_>,
    ) -> Option<InteractionResult> {
        let id = e.stack.item().id(h);
        // Sneak-use bypasses block interaction, so the linker's apply action
        // must run through the item's first-use callback as upgrades do.
        if rules::upgrade(&id).is_some() || id == "ironfurnaces:item_copy" {
            tools::interact(h, e.click)
        } else {
            None
        }
    }
    fn on_block_placed_by(&self, h: &mut dyn ServerHost, e: &mut OnBlockPlacedByEvent<'_>) {
        tools::placed(h, e);
    }
    fn item_bar(&self, h: &dyn ClientHost, e: &mut ItemBarEvent<'_>) {
        if let Some(bar) = tools::bar(h, e.stack) {
            *e.bar = Some(bar);
        }
    }
}
const OWN_ITEMS: &[&str] = &[
    "ironfurnaces:augment_blasting",
    "ironfurnaces:augment_factory",
    "ironfurnaces:augment_fuel",
    "ironfurnaces:augment_generator",
    "ironfurnaces:augment_smoking",
    "ironfurnaces:augment_speed",
    "ironfurnaces:item_copy",
    "ironfurnaces:item_heater",
    "ironfurnaces:item_spooky",
    "ironfurnaces:item_xmas",
    "ironfurnaces:rainbow_coal",
    "ironfurnaces:rainbow_core",
    "ironfurnaces:rainbow_plating",
    "ironfurnaces:upgrade_allthemodium",
    "ironfurnaces:upgrade_copper",
    "ironfurnaces:upgrade_crystal",
    "ironfurnaces:upgrade_diamond",
    "ironfurnaces:upgrade_emerald",
    "ironfurnaces:upgrade_gold",
    "ironfurnaces:upgrade_gold2",
    "ironfurnaces:upgrade_iron",
    "ironfurnaces:upgrade_iron2",
    "ironfurnaces:upgrade_netherite",
    "ironfurnaces:upgrade_obsidian",
    "ironfurnaces:upgrade_obsidian2",
    "ironfurnaces:upgrade_silver",
    "ironfurnaces:upgrade_silver2",
    "ironfurnaces:upgrade_unobtainium",
    "ironfurnaces:upgrade_vibranium",
];

#[cfg(test)]
mod tests;
