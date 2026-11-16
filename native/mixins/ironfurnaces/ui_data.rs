// HEAD on native_hooks::OpenNativeScreen::set_data. The original method then
// stores the same value; do not consume a stale window's update.
fn ironfurnaces_ui_data(screen: &mut OpenNativeScreen, window: i32, id: i16, value: i16) {
    if window != screen.window || !crate::game::ironfurnaces_ui::owns(&screen.block) { return; }
    if !matches!(id,4|5|32) { return; }
    let Ok(index @ 0..64) = usize::try_from(id) else { return; };
    let mut data = screen.data.clone();
    if data.len() <= index { data.resize(index+1,0); }
    data[index] = i32::from(value);
    screen.screen = crate::game::ironfurnaces_ui::layout(&screen.block,&data);
}
