use ely_gpui_component::theme::{ActiveTheme, Theme};
use gpui::{App, Global, SharedString};

use super::SettingsStore;

struct DefaultFamily(SharedString);

impl Global for DefaultFamily {}

pub fn family(cx: &App) -> SharedString {
    SettingsStore::get(cx)
        .ui_font_family
        .clone()
        .map(SharedString::from)
        .unwrap_or_else(|| cx.global::<DefaultFamily>().0.clone())
}

pub fn init(cx: &mut App) {
    cx.set_global(DefaultFamily(cx.theme().font_family.clone()));
    apply(cx);
    let mut selected = SettingsStore::get(cx).ui_font_family.clone();
    cx.observe_global::<SettingsStore>(move |cx| {
        let now = SettingsStore::get(cx).ui_font_family.clone();
        if now != selected {
            selected = now;
            apply(cx);
        }
    })
    .detach();
}

fn apply(cx: &mut App) {
    let family = family(cx);
    log::info!("interface font: family -> {family:?}");
    Theme::update(cx, |theme| theme.font_family = family);
}
