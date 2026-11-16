fn ironfurnaces_energy(block:&str,energy:&mut Option<horizon_client::containers::ModEnergyBuffer>) {
    crate::game::ironfurnaces_extras::register_energy(block,energy);
}
