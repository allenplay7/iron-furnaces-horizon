fn hidden(game: &Game<'_>, pos: Pos, size: usize, value: Vec<usize>) -> Vec<usize> {
    if let Some(crate::containers::BlockEntity::Mod(e)) = game.world.block_entities.get(&pos) {
        if iron_furnaces_horizon::ui::owns(e.id()) {
            return iron_furnaces_horizon::ui::hidden(e.id(), &e.data, size);
        }
    }
    value
}
