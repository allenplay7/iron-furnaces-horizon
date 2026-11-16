// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod { id: crate::game::ironfurnaces::ID.into(), title: "Iron Furnaces Native".into(), description: "Furnaces, electric factories, generators, wireless heating and automation".into(), keys: vec![] });
}
