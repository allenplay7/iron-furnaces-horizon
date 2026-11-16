fn remove(level: &mut GameLevel<'_>, pos: BlockPos, side_effects: bool) {
    crate::game::ironfurnaces_compat::remove(level, pos, side_effects)
}
