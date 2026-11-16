fn ironfurnaces_power_maintenance(game:&mut crate::game::Game<'_>,dimension:&str) {
    horizon_client::menu::ironfurnaces_slots::clear_context();
    crate::game::ironfurnaces_power::maintenance(game,dimension);
}
