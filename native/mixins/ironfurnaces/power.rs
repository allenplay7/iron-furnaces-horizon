// Apache-2.0; see LICENSE and NOTICE. Server-side power integration.
use super::{Game, iron_furnaces_energy_rules as rules, iron_furnaces_rules::{EfficiencyAugment, RecipeMode, Tier}, ironfurnaces as furnace, ironfurnaces_config as config};
use horizon_client::{containers::{BlockEntity, ModBlockEntity}, crafting::Ingredient, items::ItemStack};
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};
use std::collections::BTreeSet;

pub const HEATER_ID: &str = "ironfurnaces:heater";
pub const GENERATOR_FUEL_SLOT: usize = 6;
const OWNER_LOCATIONS: &str = "ironfurnaces:furnaces";

fn nbt_key(name: &str) -> JavaString { name.into() }
fn saved_tick(entity: &ModBlockEntity, key: &str) -> Option<i64> {
    match entity.nbt.get(&nbt_key(key)) { Some(Tag::Long(value)) => Some(*value), _ => None }
}
fn parse_owner(owner: &str) -> Option<u128> {
    // Placement hooks use the native u128's decimal representation; also
    // accept a conventional UUID for administration and imported state.
    if !owner.contains('-') && let Ok(value) = owner.parse::<u128>() { return Some(value); }
    let compact = owner.replace('-', "");
    (compact.len() == 32).then(|| u128::from_str_radix(&compact, 16).ok()).flatten()
}

fn blasting_energy(game: &Game<'_>, stack: &ItemStack) -> u32 {
    if let Some(recipes) = game.recipes().custom.get("ironfurnaces:generator_blasting") {
        // The engine keeps these in RecipeManager's registration order and
        // replaces them on /reload. Tags and custom ingredient tests work too.
        return recipes.iter().find_map(|recipe| {
            let ingredient = Ingredient::parse(recipe.json.get("ingredient")?, game.items())?;
            ingredient.test(stack).then(|| recipe.json.get("energy")
                .and_then(serde_json::Value::as_u64).unwrap_or(0).min(i32::MAX as u64) as u32)
        }).unwrap_or(0);
    }
    rules::default_blasting_energy(&stack.item).unwrap_or(0)
}

pub(super) fn generator_fuel_energy(game: &Game<'_>, entity: &ModBlockEntity, stack: &ItemStack) -> u32 {
    if stack.count == 0 { return 0; }
    let mode = furnace::mode_from(entity);
    let augment = furnace::efficiency_from(entity);
    let (fuel, nutrition, blasting) = match mode {
        RecipeMode::Smelting => {
            let fuel = if &*stack.item == "ironfurnaces:rainbow_coal" {
                if stack.damage >= rules::RAINBOW_COAL_USES { 0 }
                else { rules::RAINBOW_COAL_FUEL_TICKS }
            } else { game.res.providers().burn_time(game.items(), stack, false).max(0) as u32 };
            (fuel, 0, 0)
        }
        RecipeMode::Smoking => {
            let default = game.items().get(&stack.item).and_then(|item| item.food);
            let nutrition = horizon_client::item_sync::food(stack, default).map_or(0, |food| food.0);
            (0, nutrition, 0)
        }
        RecipeMode::Blasting => (0, 0, blasting_energy(game, stack)),
    };
    rules::generator_fuel_energy(mode, augment, fuel, nutrition, blasting)
}

pub(super) fn accepts_generator(game: &Game<'_>, entity: &ModBlockEntity, stack: &ItemStack) -> bool {
    generator_fuel_energy(game, entity, stack) > 0
}

pub(super) fn tick_generator(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity, tier: Tier) {
    let capacity = config::energy_capacity(tier);
    let mut stored = furnace::energy(entity).max(0) as u32;
    stored = stored.min(capacity);
    let mut remaining = furnace::number(&entity.nbt, "GeneratorRemaining").max(0) as u32;
    if stored < capacity && remaining == 0 {
        let budget = entity.items.get(GENERATOR_FUEL_SLOT).and_then(Option::as_ref)
            .map_or(0, |stack| generator_fuel_energy(game, entity, stack));
        if budget > 0 && furnace::consume_fuel(game, pos, entity, GENERATOR_FUEL_SLOT) {
            remaining = budget;
            furnace::put(&mut entity.nbt, "GeneratorTotal", budget as i32);
        }
    }
    let rate = config::generation_rate(tier, furnace::efficiency_from(entity));
    let step = rules::generator_step(stored, capacity, remaining, rate);
    furnace::put(&mut entity.nbt, "energy", step.stored as i32);
    furnace::put(&mut entity.nbt, "GeneratorRemaining", step.remaining as i32);
    entity.nbt.insert("GeneratorLastTick".into(), Tag::Long(game.world.game_time));

    if let Some(owner) = furnace::string(&entity.nbt, "Owner").and_then(|owner| parse_owner(&owner)) {
        let owner_text = horizon_protocol::crypto::uuid_string(owner);
        if furnace::string(&entity.nbt, "NetworkRegistered").as_deref() != Some(owner_text.as_str())
            && track_furnace(game, pos, owner)
        {
            entity.nbt.insert("NetworkRegistered".into(), Tag::String(owner_text.as_str().into()));
        }
        if saved_tick(entity, "NetworkScanTick").is_none_or(|last| game.world.game_time < last || game.world.game_time - last >= 20) {
            reconcile_owner_chunks(game, pos, entity, owner);
            entity.nbt.insert("NetworkScanTick".into(), Tag::Long(game.world.game_time));
        }
    }

    let rainbow = tier == Tier::Rainbow && rainbow_ready(game, pos, entity);
    let previous_rainbow = furnace::number(&entity.nbt, "RainbowGen") != 0;
    furnace::put(&mut entity.nbt, "RainbowGen", i32::from(rainbow));
    let bonus = if rainbow { emit_rainbow(game, pos, entity) } else { 0 };
    furnace::put(&mut entity.nbt, "GenerationRate", step.generated.saturating_add(bonus).min(i32::MAX as u32) as i32);
    emit_stored(game, pos, entity);
    // The common adapter writes lit/type/rainbow after updating menu data.
    if rainbow != previous_rainbow { game.native_block_entity_sync(pos); }
}

fn neighbor(pos: [i32; 3], direction: Direction) -> [i32; 3] {
    let offset = direction.normal();
    [pos[0].saturating_add(offset[0]), pos[1].saturating_add(offset[1]), pos[2].saturating_add(offset[2])]
}

fn emit_stored(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let mut stored = furnace::energy(entity).max(0);
    for direction in Direction::ALL {
        if stored <= 0 { break; }
        if !matches!(furnace::side(entity, direction as usize), 2 | 3) { continue; }
        let target = neighbor(pos, direction);
        let accepted = game.native_energy_insert(target, direction.opposite(), stored, false).clamp(0, stored);
        stored -= accepted;
    }
    furnace::put(&mut entity.nbt, "energy", stored);
}

fn emit_rainbow(game: &mut Game<'_>, pos: [i32; 3], entity: &ModBlockEntity) -> u32 {
    let mut destinations = Vec::new();
    for direction in Direction::ALL {
        if !matches!(furnace::side(entity, direction as usize), 2 | 3) { continue; }
        let target = neighbor(pos, direction);
        if game.native_energy_insert(target, direction.opposite(), 1, true) > 0 {
            destinations.push((target, direction.opposite()));
        }
    }
    if destinations.is_empty() { return 0; }
    let share = config::rainbow_output() / destinations.len() as u32;
    destinations.into_iter().map(|(target, face)| game.native_energy_insert(target, face, share as i32, false).max(0) as u32).sum()
}

fn owner_online(game: &Game<'_>, owner: u128) -> bool {
    if let Some(host) = game.lan.as_ref() {
        host.hub.guests.iter().any(|guest| guest.placed && !guest.released && guest.uuid == owner && guest.dimension == game.dim.id())
    } else {
        game.player.is_some() && game.level.player_uuid.map(horizon_client::lan::playerdata::ints_uuid) == Some(owner)
    }
}

fn same_owner(entity: &ModBlockEntity, owner: u128) -> bool {
    furnace::string(&entity.nbt, "Owner").as_deref().and_then(parse_owner) == Some(owner)
}

fn rainbow_ready(game: &Game<'_>, pos: [i32; 3], entity: &ModBlockEntity) -> bool {
    let Some(owner) = furnace::string(&entity.nbt, "Owner").as_deref().and_then(parse_owner) else { return false; };
    if !owner_online(game, owner) { return false; }
    let mut rainbow_generators = 1; // This entity is detached while ticking.
    let mut generators = Vec::new();
    for (at, block_entity) in &game.world.block_entities {
        if *at == (pos[0], pos[1], pos[2]) { continue; }
        let BlockEntity::Mod(other) = block_entity else { continue; };
        let Some(tier) = Tier::from_block(&other.id) else { continue; };
        if !same_owner(other, owner) || furnace::machine_mode(other) != rules::MachineMode::Generator { continue; }
        if tier == Tier::Rainbow { rainbow_generators += 1; }
        let fresh = saved_tick(other, "GeneratorLastTick").is_some_and(|last| last >= game.world.game_time.saturating_sub(1));
        generators.push(rules::GeneratorPresence {
            tier, running: fresh && furnace::number(&other.nbt, "GeneratorRemaining") > 0,
            energy: furnace::energy(other).max(0) as u32, capacity: config::energy_capacity(tier),
        });
    }
    rules::rainbow_ready(true, rainbow_generators, &generators)
}

// Persist the placement index on its owner, like Fabric's PlayerFurnacesList.
// Keep other mods' persistent keys, and keep dimensions distinct so coordinates
// from another level do not accidentally load unrelated chunks here.
pub(super) fn track_furnace(game: &mut Game<'_>, pos: [i32; 3], owner: u128) -> bool {
    let Some(Tag::Compound(mut data)) = game.native_player_persistent_data(owner) else { return false; };
    let dimension = game.dim.id();
    let entry = data.entry(OWNER_LOCATIONS.into()).or_insert_with(|| Tag::Compound(Compound::new()));
    if !matches!(entry, Tag::Compound(_)) { *entry = Tag::Compound(Compound::new()); }
    let Tag::Compound(levels) = entry else { return false; };
    let entry = levels.entry(dimension.into()).or_insert_with(|| Tag::IntArray(Vec::new()));
    if !matches!(entry, Tag::IntArray(_)) { *entry = Tag::IntArray(Vec::new()); }
    let Tag::IntArray(positions) = entry else { return false; };
    if positions.chunks_exact(3).any(|point| point == pos) { return true; }
    positions.extend_from_slice(&pos);
    game.native_set_player_persistent_data(owner, Tag::Compound(data))
}

fn owner_positions(game: &Game<'_>, owner: u128) -> Vec<[i32; 3]> {
    let Some(Tag::Compound(data)) = game.native_player_persistent_data(owner) else { return Vec::new(); };
    let Some(Tag::Compound(levels)) = data.get(&nbt_key(OWNER_LOCATIONS)) else { return Vec::new(); };
    let Some(Tag::IntArray(positions)) = levels.get(&nbt_key(game.dim.id())) else { return Vec::new(); };
    positions.chunks_exact(3).map(|point| [point[0], point[1], point[2]]).collect()
}

fn store_owner_positions(game: &mut Game<'_>, owner: u128, positions: &[[i32; 3]]) {
    let Some(Tag::Compound(mut data)) = game.native_player_persistent_data(owner) else { return; };
    let Some(Tag::Compound(levels)) = data.get_mut(&nbt_key(OWNER_LOCATIONS)) else { return; };
    levels.insert(game.dim.id().into(),Tag::IntArray(positions.iter().flatten().copied().collect()));
    game.native_set_player_persistent_data(owner,Tag::Compound(data));
}

fn ticket_owner(owner: u128) -> String { format!("ironfurnaces:rainbow/{}", horizon_protocol::crypto::uuid_string(owner)) }
fn wireless_ticket_owner(pos: [i32; 3]) -> String { format!("ironfurnaces:wireless/{},{},{}",pos[0],pos[1],pos[2]) }

fn release_tickets(game: &mut Game<'_>, key: &str) {
    for chunk in game.native_forced_chunks(key) { game.native_force_chunk(key, chunk, false); }
}

pub(super) fn release_owner_chunks(game: &mut Game<'_>, owner: u128) {
    let key = ticket_owner(owner);
    release_tickets(game,&key);
}

// Run from native_level_tick_post, after every detached entity is restored.
// Even a removed final machine, an offline owner or a disabled mod releases
// its tickets. Restrict cleanup to this module's owners in the current level.
pub(super) fn maintenance(game: &mut Game<'_>, dimension: &str) {
    if dimension != game.dim.id() { return; }
    let enabled = game.native_on(furnace::ID);
    let tickets: BTreeSet<_> = game.mod_chunk_tickets.get(&game.dim).into_iter()
        .flat_map(|chunks| chunks.values()).flat_map(|owners| owners.iter())
        .filter(|key| key.starts_with("ironfurnaces:rainbow/") || key.starts_with("ironfurnaces:wireless/")).cloned().collect();
    for key in tickets {
        if !enabled { release_tickets(game,&key); continue; }
        if let Some(position) = key.strip_prefix("ironfurnaces:wireless/") {
            let point = position.split(',').map(str::parse::<i32>).collect::<Result<Vec<_>,_>>().ok();
            let pending = point.filter(|point| point.len()==3).and_then(|point| game.world.block_entities.get(&(point[0],point[1],point[2])))
                .is_some_and(|block| matches!(block,BlockEntity::Mod(other)
                    if Tier::from_block(&other.id).is_some() && furnace::machine_mode(other)==rules::MachineMode::Furnace
                    && saved_tick(other,"WirelessLoadTick").is_some_and(|last|last>=game.world.game_time.saturating_sub(1))
                    && other.items.get(1).and_then(Option::as_ref).and_then(heater_link).is_some()));
            // Pending requests are renewed only while this loaded furnace
            // needs an ignition. Broken/unloaded furnaces and removed cards
            // cannot keep their remote source loaded indefinitely.
            if !pending { release_tickets(game,&key); }
            continue;
        }
        let Some(owner) = key.strip_prefix("ironfurnaces:rainbow/").and_then(parse_owner) else { continue; };
        let has_rainbow = game.world.block_entities.values().any(|block| {
            matches!(block,BlockEntity::Mod(other) if other.id == Tier::Rainbow.block_id() && same_owner(other,owner)
                && furnace::machine_mode(other) == rules::MachineMode::Generator)
        });
        if !owner_online(game,owner) || !has_rainbow { release_owner_chunks(game,owner); }
    }
}

fn reconcile_owner_chunks(game: &mut Game<'_>, pos: [i32; 3], entity: &ModBlockEntity, owner: u128) {
    let own_rainbow = entity.id == Tier::Rainbow.block_id() && furnace::machine_mode(entity) == rules::MachineMode::Generator;
    let loaded_rainbow = own_rainbow || game.world.block_entities.values().any(|block| {
        matches!(block, BlockEntity::Mod(other) if other.id == Tier::Rainbow.block_id() && same_owner(other, owner)
            && furnace::machine_mode(other) == rules::MachineMode::Generator)
    });
    if !owner_online(game, owner) || !loaded_rainbow { release_owner_chunks(game, owner); return; }
    let mut wanted = BTreeSet::new();
    let positions = owner_positions(game, owner);
    let mut valid_positions = Vec::new();
    for point in positions.iter().copied() {
        if point != pos && game.world.loaded(point[0], point[2]) {
            // Broken/reassigned furnaces need no ticket, even if their old
            // position is still in an offline owner's placement history.
            let valid = matches!(game.world.block_entities.get(&(point[0],point[1],point[2])),
                Some(BlockEntity::Mod(other)) if Tier::from_block(&other.id).is_some() && same_owner(other, owner));
            if !valid { continue; }
        }
        valid_positions.push(point);
        wanted.insert([point[0] >> 4, point[2] >> 4]);
    }
    if valid_positions.len() != positions.len() { store_owner_positions(game,owner,&valid_positions); }
    let key = ticket_owner(owner);
    for chunk in game.native_forced_chunks(&key) {
        if !wanted.contains(&chunk) { game.native_force_chunk(&key, chunk, false); }
    }
    for chunk in wanted { game.native_force_chunk(&key, chunk, true); }
}

pub(super) fn tick_heater(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let stored = (furnace::energy(entity).max(0) as u32).min(config::heater_capacity());
    furnace::put(&mut entity.nbt, "energy", stored as i32);
    let Some(stack) = entity.items.first_mut().and_then(Option::as_mut) else { return; };
    if &*stack.item != "ironfurnaces:item_heater" { return; }
    let mut bound = match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        Some(Tag::Compound(data)) => data.clone(), _ => Compound::new(),
    };
    let mut changed = false;
    for (field, value) in ["HeaterPosX","HeaterPosY","HeaterPosZ"].into_iter().zip(pos) {
        if !matches!(bound.get(&nbt_key(field)),Some(Tag::Int(previous)) if *previous == value) {
            furnace::put(&mut bound, field, value); changed = true;
        }
    }
    if changed {
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(bound)));
        game.native_block_entity_sync(pos);
    }
}

pub(super) fn heater_link(stack: &ItemStack) -> Option<[i32; 3]> {
    if &*stack.item != "ironfurnaces:item_heater" { return None; }
    let Some(Tag::Compound(bound)) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return None; };
    let coordinate = |field: &str| match bound.get(&nbt_key(field)) {
        Some(Tag::Int(value)) => Some(*value), Some(Tag::Short(value)) => Some(i32::from(*value)), _ => None,
    };
    Some([coordinate("HeaterPosX")?, coordinate("HeaterPosY")?, coordinate("HeaterPosZ")?])
}

pub(super) fn wireless_ignite(
    game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity,
    cook: u32, efficiency: EfficiencyAugment,
) -> Option<(u32, u32)> {
    let linked = heater_link(entity.items.get(1)?.as_ref()?)?;
    if linked == pos { return None; }
    let key = wireless_ticket_owner(pos);
    let chunk = [linked[0]>>4,linked[2]>>4];
    for previous in game.native_forced_chunks(&key) {
        if previous != chunk { game.native_force_chunk(&key,previous,false); }
    }
    if !game.world.loaded(linked[0],linked[2]) {
        // Java's Level.getBlockEntity loads its chunk. Request that source
        // asynchronously here, then retry on subsequent furnace ticks.
        game.native_force_chunk(&key,chunk,true);
        entity.nbt.insert("WirelessLoadTick".into(),Tag::Long(game.world.game_time));
        return None;
    }
    let ignition = game.native_with_mod_block_entity(linked, |heater| {
        if heater.id != HEATER_ID { return None; }
        let stored = (furnace::energy(heater).max(0) as u32).min(config::heater_capacity());
        let ignition = rules::wireless_ignition(stored, cook, efficiency)?;
        furnace::put(&mut heater.nbt, "energy", (stored - ignition.energy_cost) as i32);
        Some(ignition)
    }).flatten();
    // Once the lookup finishes, normal streaming owns the source again.
    // Failed links and insufficient energy release their tickets too.
    release_tickets(game,&key);
    let ignition = ignition?;
    game.native_block_entity_sync(linked);
    Some((ignition.burn_ticks, ignition.burn_ticks))
}
