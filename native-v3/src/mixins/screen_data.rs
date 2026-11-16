fn screen_data(screen: &mut OpenNativeScreen, window: i32, id: i16, value: i16) {
    if window != screen.window
        || !iron_furnaces_horizon::ui::owns(&screen.block)
        || !matches!(id, 4 | 5 | 32)
    {
        return;
    }
    let mut data = screen.data.clone();
    let index = id as usize;
    if data.len() <= index {
        data.resize(index + 1, 0);
    }
    data[index] = i32::from(value);
    let next = iron_furnaces_horizon::ui::layout(&screen.block, &data);
    if screen.screen != next {
        screen.screen = next;
        screen.layout_revision = screen.layout_revision.wrapping_add(1);
    }
}
