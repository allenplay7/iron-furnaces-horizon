fn ironfurnaces_energy_at(game:&crate::game::Game<'_>,dimension:&str,pos:[i32;3]) -> core::ops::ControlFlow<Option<(Arc<ModEnergyBuffer>,i32)>> {
    crate::game::ironfurnaces_extras::energy_at(game,dimension,pos)
}
