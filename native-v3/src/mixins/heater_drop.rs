fn heater_drop(pending: &mut Pending, pos: [i32; 3], entity: &BlockEntity, block: &str) {
    if block != "ironfurnaces:heater" {
        return;
    }
    let BlockEntity::Mod(e) = entity else {
        return;
    };
    let energy = match e.nbt.get(&"energy".into()) {
        Some(Tag::Int(n)) => *n,
        _ => 0,
    };
    let mut data = Compound::new();
    data.insert(
        "ironfurnaces:energy".into(),
        Tag::Int(energy.clamp(0, iron_furnaces_horizon::config::heater_capacity() as i32)),
    );
    let mut patch = Compound::new();
    patch.insert("minecraft:custom_data".into(), Tag::Compound(data));
    pending.copied.insert(pos, (block.into(), patch));
}
