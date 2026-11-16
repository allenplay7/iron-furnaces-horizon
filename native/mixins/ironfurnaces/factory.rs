// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_factory(block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
    if entity.is_none() && (block == "ironfurnaces:heater" || crate::game::iron_furnaces_rules::Tier::from_block(block).is_some()) {
        let mut nbt = horizon_world::nbt::Compound::new();
        nbt.insert("Version".into(), horizon_world::nbt::Tag::Int(2));
        for (side,value) in [2,1,4,4,4,4].into_iter().enumerate() {
            nbt.insert(format!("Side{side}").as_str().into(), horizon_world::nbt::Tag::Int(value));
        }
        *entity = Some((block.to_owned(), nbt));
    }
}
