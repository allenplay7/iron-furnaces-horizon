// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_screen(block: &str, screen: &mut Option<crate::game::native_hooks::NativeScreen>) {
    if screen.is_none() { *screen = crate::game::ironfurnaces::screen(block); }
}
