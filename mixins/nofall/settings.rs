// The mod's settings page: the runtime's single row, in the state the
// setting file holds (read fresh from disk, so the page always shows what
// the ticks act on).
fn nofall_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::nofall as nofall;
    if mod_id != nofall::ID {
        return;
    }
    let game = &*game;
    let on = nofall::setting_on(|key| game.native_setting(nofall::ID, key));
    buttons.push(nofall::label(on));
}
