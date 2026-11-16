// SPDX-License-Identifier: Apache-2.0
// RETURN Containers::signal, for the ordinary container/comparator query.
fn ironfurnaces_automation_comparator(value: i32, containers: &Containers<'_>, view: &View) -> i32 {
    if let Some((pos, _)) = view.locate(0)
        && let Some(BlockEntity::Mod(entity)) = containers.world.block_entities.get(&pos)
        && crate::game::iron_furnaces_rules::Tier::from_block(&entity.id).is_some()
    {
        return crate::game::ironfurnaces_automation::comparator(entity, containers.items);
    }
    value
}
