fn signal(level: &GameLevel<'_>, pos: BlockPos, value: i32) -> i32 {
    crate::game::ironfurnaces_compat::signal(level, pos, value, false)
}
