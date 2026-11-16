// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_interact(game: &mut crate::game::Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    crate::game::ironfurnaces::interact(game, pos, canceled);
    crate::game::ironfurnaces_extras::interact_heater(game, pos, canceled);
}
