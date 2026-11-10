// The mod's row in Options > Mods, with the original's description. The
// row's own on/off switch is the game's: a native mod's line says `false`
// when off, and the launcher's next build leaves the mod out. No keys —
// the mod's one toggle lives on the settings page, so it survives without
// a binding to remember.
fn nofall_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::nofall as nofall;
    mods.push(crate::game::native_hooks::NativeMod {
        id: nofall::ID.into(),
        title: format!("NoFall {}", nofall::VERSION),
        description: nofall::DESCRIPTION.into(),
        keys: vec![],
    });
}
