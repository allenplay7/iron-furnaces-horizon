// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_container(block: &str, container: &mut Option<horizon_client::containers::ModContainer>) {
    if container.is_none() { *container = crate::game::ironfurnaces::container(block); }
}
