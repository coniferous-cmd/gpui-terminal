use gpui::{App, KeyBinding};

use super::{
    CONTEXT, CloseSession, CloseTab, LastTab, NewSession, NewTab, NextSession, NextTab,
    PreviousSession, PreviousTab, SelectTab, TogglePanel, ToggleSessions,
};

/// Tab keys stay familiar; groups get their own shortcuts.
pub fn bind_keys(cx: &mut App) {
    let (sessions, panel, new, close, digit, close_group) = if cfg!(target_os = "macos") {
        ("cmd-b", "cmd-alt-b", "cmd-t", "cmd-w", "cmd", "cmd-shift-w")
    } else {
        (
            "ctrl-shift-b",
            "ctrl-shift-alt-b",
            "ctrl-shift-t",
            "ctrl-shift-w",
            "alt",
            "ctrl-shift-alt-w",
        )
    };
    let context = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new(sessions, ToggleSessions, context),
        KeyBinding::new(panel, TogglePanel, context),
        KeyBinding::new(new, NewTab, context),
        KeyBinding::new(close, CloseTab, context),
        KeyBinding::new(close_group, CloseSession, context),
        KeyBinding::new("ctrl-tab", NextTab, context),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, context),
        KeyBinding::new(&format!("{digit}-9"), LastTab, context),
    ]);
    cx.bind_keys(
        (1..=8).map(|n| KeyBinding::new(&format!("{digit}-{n}"), SelectTab(n - 1), context)),
    );
    let modifier = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    cx.bind_keys([
        KeyBinding::new(&format!("{modifier}-shift-n"), NewSession, context),
        KeyBinding::new(
            &format!("{modifier}-shift-pageup"),
            PreviousSession,
            context,
        ),
        KeyBinding::new(&format!("{modifier}-shift-pagedown"), NextSession, context),
    ]);
}
