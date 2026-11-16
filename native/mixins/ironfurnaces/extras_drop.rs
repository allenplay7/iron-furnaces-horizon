fn ironfurnaces_drop(pending:&mut Pending,pos:[i32;3],entity:&horizon_client::containers::BlockEntity,block:&str) {
    crate::game::ironfurnaces_extras::remember_drop(pending,pos,entity,block);
}
