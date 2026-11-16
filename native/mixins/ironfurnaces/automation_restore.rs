// SPDX-License-Identifier: Apache-2.0
// RETURN Game::put_back_mod_block_entity, after the real entity is restored.
fn ironfurnaces_automation_restore(value: (), game: &mut crate::game::Game<'_>, pos: (i32, i32, i32)) {
    let _ = value;
    crate::game::ironfurnaces_automation::after_restore(game, pos);
}
