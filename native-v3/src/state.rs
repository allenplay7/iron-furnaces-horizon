// SPDX-License-Identifier: Apache-2.0
// Save keys deliberately match the 0.2 adapter for world migration.
use crate::energy_rules::MachineMode;
use crate::rules::{EfficiencyAugment, RecipeMode};
use mod_api::{Compound, Host, Identifier, ItemStack, ModBlockEntityView, Tag};

pub fn number(nbt: &Compound, name: &str) -> i32 {
    match nbt.get(&(name).into()) {
        Some(Tag::Int(n)) => *n,
        Some(Tag::Short(n)) => i32::from(*n),
        _ => 0,
    }
}
pub fn put(nbt: &mut Compound, name: &str, n: i32) {
    nbt.insert(name.into(), Tag::Int(n));
}
pub fn string(nbt: &Compound, name: &str) -> Option<String> {
    match nbt.get(&(name).into()) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}
pub fn stack(host: &dyn Host, id: &str, count: u32) -> Option<ItemStack> {
    ItemStack::new(host.item_key(&Identifier::parse(id).ok()?)?, count)
}
pub fn item(host: &dyn Host, entity: &ModBlockEntityView<'_>, slot: usize) -> String {
    entity
        .items
        .get(slot)
        .and_then(Option::as_ref)
        .map(|s| s.item().id(host))
        .unwrap_or_default()
}
pub fn recipe_mode(host: &dyn Host, entity: &ModBlockEntityView<'_>) -> RecipeMode {
    RecipeMode::from_item(&item(host, entity, 3)).unwrap_or_default()
}
pub fn efficiency(host: &dyn Host, entity: &ModBlockEntityView<'_>) -> EfficiencyAugment {
    EfficiencyAugment::from_item(&item(host, entity, 4)).unwrap_or_default()
}
pub fn mode(host: &dyn Host, entity: &ModBlockEntityView<'_>) -> MachineMode {
    MachineMode::from_item(&item(host, entity, 5)).unwrap_or_default()
}
pub fn initialize(host: &dyn Host, entity: &mut ModBlockEntityView<'_>) {
    let heater = entity.id() == "ironfurnaces:heater";
    let count = entity.items.len().max(if heater { 1 } else { 19 });
    entity.items.resize(count, None);
    // 0.2 used bare vanilla recipe IDs. Keep paid work and earned XP when the
    // typed recipe API starts returning fully qualified identifiers.
    if !heater && number(&entity.nbt, "RecipeIdsVersion") < 1 {
        for key in std::iter::once("Recipe".to_owned())
            .chain((0..6).map(|lane| format!("Lane{lane}Recipe")))
        {
            if let Some(id) = string(&entity.nbt, &key) {
                if !id.is_empty() && !id.contains(':') {
                    entity.nbt.insert(
                        key.as_str().into(),
                        Tag::String(format!("minecraft:{id}").as_str().into()),
                    );
                }
            }
        }
        if let Some(Tag::Compound(used)) = entity.nbt.get_mut(&"RecipesUsed".into()) {
            let legacy: Vec<_> = used
                .keys()
                .filter(|key| {
                    let id = key.to_string_lossy();
                    !id.is_empty() && !id.contains(':')
                })
                .cloned()
                .collect();
            for key in legacy {
                if let Some(Tag::Int(count)) = used.remove(&key) {
                    let qualified = format!("minecraft:{}", key.to_string_lossy());
                    let total = number(used, &qualified).saturating_add(count);
                    put(used, &qualified, total);
                }
            }
        }
        put(&mut entity.nbt, "RecipeIdsVersion", 1);
    }
    if number(&entity.nbt, "Version") < 2 {
        if !heater {
            for (key, slot) in [("Mode", 3), ("Efficiency", 4)] {
                if let Some(Tag::String(id)) = entity.nbt.remove(&(key).into()) {
                    if entity.items[slot].is_none() {
                        entity.items[slot] = stack(host, &id.to_string_lossy(), 1);
                    }
                }
            }
            for (i, value) in [2, 1, 4, 4, 4, 4].into_iter().enumerate() {
                let key = format!("Side{i}");
                if !entity.nbt.contains_key(&(key.as_str()).into()) {
                    put(&mut entity.nbt, &key, value);
                }
            }
        }
        put(&mut entity.nbt, "Version", 2);
    }
    entity.data.resize(64, 0);
}
pub fn same(a: &ItemStack, b: &ItemStack) -> bool {
    a.same_item(b) && a.same_components(b)
}
pub fn maximum(host: &dyn Host, stack: &ItemStack) -> u32 {
    if let Some(patch) = stack.components() {
        let patch = patch.as_nbt();
        if patch.contains_key(&("!minecraft:max_stack_size").into()) {
            return 1;
        }
        if let Some(Tag::Int(n)) = patch.get(&("minecraft:max_stack_size").into()) {
            return (*n).clamp(1, 64) as u32;
        }
    }
    host.max_stack_size(stack.item()).unwrap_or(64).clamp(1, 64)
}
pub fn consume(slot: &mut Option<ItemStack>, count: u32) {
    *slot = slot.take().and_then(|s| s.shrink(count));
}
pub fn saved_items(host: &dyn Host, nbt: &Compound, size: usize) -> Option<Vec<Option<ItemStack>>> {
    let mut items = vec![None; size];
    if let Some(Tag::List(list)) = nbt.get(&"Items".into()) {
        for entry in list {
            let Tag::Compound(entry) = entry else {
                return None;
            };
            let index = match entry.get(&"Slot".into()) {
                Some(Tag::Byte(n)) => *n as u8 as usize,
                Some(Tag::Int(n)) if *n >= 0 => *n as usize,
                _ => return None,
            };
            if index >= size {
                return None;
            }
            items[index] = Some(ItemStack::from_nbt(host, entry)?);
        }
    }
    Some(items)
}
pub fn encode_items(host: &dyn Host, items: &[Option<ItemStack>]) -> Option<Tag> {
    let mut list = Vec::new();
    for (index, stack) in items.iter().enumerate() {
        if let Some(stack) = stack {
            let mut data = stack.to_nbt(host)?;
            data.insert("Slot".into(), Tag::Byte(index as i8));
            list.push(Tag::Compound(data));
        }
    }
    Some(Tag::List(list))
}
pub fn fingerprint(
    host: &dyn Host,
    entity: &mut ModBlockEntityView<'_>,
    slot: usize,
    prefix: &str,
    recipe: &str,
) -> bool {
    let tag = entity.items[slot].as_ref().and_then(|s| {
        let mut s = s.clone();
        s.set_count(1).ok()?;
        let mut nbt = s.to_nbt(host)?;
        nbt.insert("Slot".into(), Tag::Byte(0));
        Some(Tag::List(vec![Tag::Compound(nbt)]))
    });
    let input_key = (format!("{prefix}Input").as_str()).into();
    let recipe_key = format!("{prefix}Recipe");
    let same = entity.nbt.get(&input_key) == tag.as_ref()
        && string(&entity.nbt, &recipe_key).as_deref().unwrap_or("") == recipe;
    if let Some(tag) = tag {
        entity.nbt.insert(input_key, tag);
    } else {
        entity.nbt.remove(&input_key);
    }
    entity
        .nbt
        .insert(recipe_key.as_str().into(), Tag::String(recipe.into()));
    same
}
