// SPDX-License-Identifier: Apache-2.0
use crate::{config, energy_rules::AugmentChannel, network, rules, state::*};
use mod_api::*;
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
pub fn custom(s: &ItemStack) -> Compound {
    match s
        .components()
        .and_then(|p| p.as_nbt().get(&("minecraft:custom_data").into()))
    {
        Some(Tag::Compound(d)) => d.clone(),
        _ => Compound::new(),
    }
}
pub fn set_custom(s: &mut ItemStack, data: Compound) {
    let mut patch = s
        .components()
        .map(|p| p.as_nbt().clone())
        .unwrap_or_default();
    patch.insert("minecraft:custom_data".into(), Tag::Compound(data));
    let _ = s.set_components(patch);
}
pub fn placed(host: &mut dyn ServerHost, e: &OnBlockPlacedByEvent<'_>) {
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    let Some(entity) = host.block_entity(level, e.pos) else {
        return;
    };
    let id = e.stack.item().id(host);
    let mut data = Compound::new();
    if id == "ironfurnaces:heater" {
        put(
            &mut data,
            "energy",
            number(&custom(e.stack), "ironfurnaces:energy")
                .clamp(0, config::heater_capacity() as i32),
        );
    } else if rules::Tier::from_block(&id).is_some() {
        if let Some(player) = e.placer {
            data.insert(
                "Owner".into(),
                Tag::String(player.uuid().to_string().as_str().into()),
            );
            network::track(host, e.pos, player.uuid());
        }
    } else {
        return;
    }
    let _ = entity.merge_data(host, data);
}
pub fn interact(host: &mut dyn ServerHost, click: &BlockClick<'_>) -> Option<InteractionResult> {
    let level = host.level(host.dimension())?;
    let player = host.player(click.player)?;
    let block = level.block(host, click.pos)?.id(host);
    let sneaking = player.entity().is_crouching(host).unwrap_or(false);
    let held = click.stack;
    if let Some(s) = held {
        let id = s.item().id(host);
        if let Some(tier) = rules::upgrade_target(&id, &block) {
            return upgrade(host, click, tier);
        }
    }
    if !crate::ui::owns(&block) {
        return None;
    }
    let entity = host.block_entity(level, click.pos)?;
    if let Some(s) = held {
        let id = s.item().id(host);
        if id == "ironfurnaces:item_copy" && block != "ironfurnaces:heater" {
            if sneaking {
                if let Some(Tag::Compound(settings)) =
                    custom(s).get(&("ironfurnaces:settings").into())
                {
                    let mut data = Compound::new();
                    for (i, key) in SETTINGS.into_iter().enumerate() {
                        let max = match i {
                            0..=5 | 8 => 4,
                            9 => 15,
                            _ => 1,
                        };
                        put(&mut data, key, number(settings, key).clamp(0, max));
                    }
                    let _ = entity.merge_data(host, data);
                    if let Some(facing) = string(settings, "Facing") {
                        if matches!(facing.as_str(), "north" | "south" | "east" | "west") {
                            host.set_block_property(click.pos, "facing", &facing);
                        }
                    }
                    let _ = player.tell(host, Text::literal("Settings applied"));
                } else {
                    let _ = player.tell(
                        host,
                        Text::literal("Copy furnace settings with a normal right click first."),
                    );
                }
            } else {
                let data = entity.data(host)?;
                let mut settings = Compound::new();
                for key in SETTINGS {
                    put(&mut settings, key, number(&data, key));
                }
                settings.insert(
                    "Facing".into(),
                    Tag::String(
                        level
                            .block_property(host, click.pos, "facing")
                            .unwrap_or_else(|| "north".into())
                            .as_str()
                            .into(),
                    ),
                );
                let mut custom = custom(s);
                custom.insert("ironfurnaces:settings".into(), Tag::Compound(settings));
                // PlayerHand has infinite-material semantics in creative mode.
                // Editing a tool must replace its actual slot, without creating a copy.
                let hand = ItemPlace::PlayerSlot {
                    player: click.player,
                    slot: match click.hand {
                        Hand::Main => player.selected_slot(host)? as i32,
                        Hand::Off => -106,
                    },
                };
                let result: Result<(), ()> = host.transaction(|host, tx| {
                    if hand.edit(host, tx, |s| set_custom(s, custom)) {
                        Ok(())
                    } else {
                        Err(())
                    }
                });
                result.ok()?;
                let _ = player.tell(host, Text::literal("Settings copied"));
            }
            return Some(InteractionResult::SuccessServer);
        }
        if block != "ironfurnaces:heater" {
            if let Some(skin) = match id.as_str() {
                "ironfurnaces:item_spooky" => Some(1),
                "ironfurnaces:item_xmas" => Some(2),
                _ => None,
            } {
                let mut data = Compound::new();
                put(&mut data, "Skin", skin);
                let _ = entity.merge_data(host, data);
                host.set_block_property(click.pos, "jovial", &skin.to_string());
                return Some(InteractionResult::SuccessServer);
            }
            if let Some(channel) = crate::energy_rules::augment_channel(&id) {
                let slot = match channel {
                    AugmentChannel::Recipe => 3,
                    AugmentChannel::Efficiency => 4,
                    AugmentChannel::Machine => 5,
                };
                let previous = entity.item_slot(host, slot).ok().flatten();
                if previous.as_ref().is_some_and(|p| same(p, s)) {
                    return Some(InteractionResult::SuccessServer);
                }
                let hand = ItemPlace::PlayerHand {
                    player: click.player,
                    hand: click.hand,
                };
                let mut installed = s.clone();
                let _ = installed.set_count(1);
                let result: Result<(), ()> = host.transaction(|host, tx| {
                    if !player.is_creative(host).unwrap_or(false)
                        && hand.extract(host, tx, &ItemResource::of(s), 1) != 1
                    {
                        return Err(());
                    }
                    entity
                        .set_item_slot(host, tx, slot, Some(installed))
                        .map_err(|_| ())?;
                    if let Some(previous) = previous {
                        level
                            .spawn_item(
                                host,
                                f64::from(click.pos[0]) + 0.5,
                                f64::from(click.pos[1]) + 0.5,
                                f64::from(click.pos[2]) + 0.5,
                                previous,
                                10,
                            )
                            .map_err(|_| ())?;
                    }
                    Ok(())
                });
                if result.is_ok() {
                    host.block_entity_sync(click.pos);
                    return Some(InteractionResult::SuccessServer);
                }
            }
        }
        if sneaking {
            return None;
        }
    } else if sneaking && block != "ironfurnaces:heater" {
        let mut data = Compound::new();
        put(&mut data, "Skin", 0);
        let _ = entity.merge_data(host, data);
        host.set_block_property(click.pos, "jovial", "0");
        return Some(InteractionResult::SuccessServer);
    }
    player
        .open_menu(host, entity)
        .ok()
        .filter(|v| *v)
        .map(|_| InteractionResult::Consume)
}

fn upgrade(
    host: &mut dyn ServerHost,
    click: &BlockClick<'_>,
    target: rules::Tier,
) -> Option<InteractionResult> {
    let level = host.level(host.dimension())?;
    let player = host.player(click.player)?;
    let held = click.stack?;
    let source = level.block(host, click.pos)?.id(host);
    let old = host.block_entity(level, click.pos)?;
    let original = old.data(host)?;
    let original_state = level.block_state(host, click.pos)?;
    let mut data = original.clone();
    let slots = if source == "minecraft:furnace" { 3 } else { 19 };
    // Internal typed slots are only available on mod holders; vanilla inventories use their saved NBT.
    let items = saved_items(host, &data, slots)?;
    if source == "minecraft:furnace" {
        for (from, to) in [
            ("lit_time_remaining", "BurnTime"),
            ("lit_total_time", "BurnTotal"),
            ("cooking_time_spent", "CookTime"),
            ("cooking_total_time", "CookTotal"),
        ] {
            let n = number(&data, from);
            put(&mut data, to, n);
        }
    }
    let mut saved = ModBlockEntity::new(target.block_id().into(), Default::default());
    saved.nbt = data;
    saved.items = items;
    let mut e = ModBlockEntityView::new(&mut saved);
    initialize(host, &mut e);
    let augment = efficiency(host, &e);
    let old_total = number(&e.nbt, "CookTotal").max(1);
    let recipe = e.items[0].as_ref().and_then(|s| {
        host.recipes_of_type(recipe_mode(host, &e).recipe_type())
            .iter()
            .find(|r| r.ingredients.len() == 1 && r.ingredients[0].test(host, s))
            .cloned()
    });
    let new_total = config::cook_ticks(
        target,
        recipe.as_ref().and_then(|r| r.cooking_time).unwrap_or(200),
        augment,
    ) as i32;
    for key in ["BurnTime", "BurnTotal", "CookTime"] {
        let n = number(&e.nbt, key).max(0);
        put(
            &mut e.nbt,
            key,
            (i64::from(n) * i64::from(new_total) / i64::from(old_total)).min(i64::from(i32::MAX))
                as i32,
        );
    }
    put(&mut e.nbt, "CookTotal", new_total);
    if source == "minecraft:furnace" {
        fingerprint(host, &mut e, 0, "", recipe.as_ref().map_or("", |r| &r.id));
    }
    for i in 0..6 {
        let pk = format!("LaneProgress{i}");
        let tk = format!("LaneTotal{i}");
        let ot = number(&e.nbt, &tk).max(1);
        let p = number(&e.nbt, &pk).max(0);
        let recipe = e.items[7 + i].as_ref().and_then(|s| {
            host.recipes_of_type(recipe_mode(host, &e).recipe_type())
                .iter()
                .find(|r| r.ingredients.len() == 1 && r.ingredients[0].test(host, s))
                .cloned()
        });
        let nt = config::cook_ticks(
            target,
            recipe.as_ref().and_then(|r| r.cooking_time).unwrap_or(200),
            augment,
        ) as i32;
        put(
            &mut e.nbt,
            &pk,
            (i64::from(p) * i64::from(nt) / i64::from(ot)).min(i64::from(nt - 1)) as i32,
        );
        put(&mut e.nbt, &tk, nt);
    }
    if string(&e.nbt, "Owner").is_none() {
        e.nbt.insert(
            "Owner".into(),
            Tag::String(click.player.to_string().as_str().into()),
        );
    }
    let facing = level
        .block_property(host, click.pos, "facing")
        .unwrap_or_else(|| "north".into());
    let lit = level
        .block_property(host, click.pos, "lit")
        .unwrap_or_else(|| "false".into());
    let next = host.block_state_id(&format!("{}[facing={facing},lit={lit}]", target.block_id()))?;
    let mut replacement = e.nbt.clone();
    for k in ["id", "x", "y", "z"] {
        replacement.remove(&k.into());
    }
    replacement.insert("Items".into(), encode_items(host, &e.items)?);
    let hand = ItemPlace::PlayerHand {
        player: click.player,
        hand: click.hand,
    };
    let result: Result<(), ()> = host.transaction(|host, tx| {
        if !player.is_creative(host).unwrap_or(false)
            && hand.extract(host, tx, &ItemResource::of(held), 1) != 1
        {
            return Err(());
        }
        // Skip removal side effects: the entire saved inventory and XP move to the new tier.
        if !level
            .set_block(
                host,
                click.pos,
                next,
                BlockUpdateFlags::NEIGHBORS
                    | BlockUpdateFlags::CLIENTS
                    | BlockUpdateFlags::SKIP_BLOCK_ENTITY_SIDE_EFFECTS,
            )
            .map_err(|_| ())?
        {
            return Err(());
        }
        let new = host.block_entity(level, click.pos).ok_or(())?;
        if !new.merge_data(host, replacement).map_err(|_| ())? {
            return Err(());
        }
        Ok(())
    });
    if result.is_ok() {
        network::track(host, click.pos, click.player);
        host.block_entity_sync(click.pos);
        Some(InteractionResult::SuccessServer)
    } else {
        // Block placement is not part of the item transaction journal. Restore it explicitly on refusal.
        let _ = level.set_block(
            host,
            click.pos,
            original_state,
            BlockUpdateFlags::NEIGHBORS
                | BlockUpdateFlags::CLIENTS
                | BlockUpdateFlags::SKIP_BLOCK_ENTITY_SIDE_EFFECTS,
        );
        if let Some(restored) = host.block_entity(level, click.pos) {
            let _ = restored.merge_data(host, original);
        }
        Some(InteractionResult::Fail)
    }
}

pub fn bar(host: &dyn Host, s: &ItemStack) -> Option<(i32, u32)> {
    match s.item().id(host).as_str() {
        "ironfurnaces:rainbow_coal" => {
            let uses = crate::energy_rules::RAINBOW_COAL_USES;
            let remaining = uses.saturating_sub(s.damage());
            let f = remaining as f32 / uses as f32;
            let red = (((1.0 - f) * 2.0).min(1.0) * 255.0) as u32;
            let green = ((f * 2.0).min(1.0) * 255.0) as u32;
            Some(((remaining * 13 / uses) as i32, (red << 16) | (green << 8)))
        }
        "ironfurnaces:heater" => {
            let energy = number(&custom(s), "ironfurnaces:energy").max(0);
            (energy > 0).then_some((
                ((i64::from(energy) * 13) / i64::from(config::heater_capacity())).clamp(0, 13)
                    as i32,
                0xff0000,
            ))
        }
        _ => None,
    }
}
