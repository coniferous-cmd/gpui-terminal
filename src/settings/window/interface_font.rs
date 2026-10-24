use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::FontPicker,
    settings::{SettingsRow, SettingsSection},
    theme::ActiveTheme,
};
use gpui::{App, Context, Entity, ParentElement, Styled, Window, div};

use super::{SettingsWindow, set};
use crate::settings::{SettingsStore, interface_font};

impl SettingsWindow {
    pub(super) fn interface_font(&self, cx: &Context<Self>) -> SettingsSection {
        let this = cx.entity();
        let picker = FontPicker::new("ui_font_family", &self.ui_font)
            .selected(interface_font::family(cx))
            .on_change({
                let this = this.clone();
                move |family, window, cx| {
                    let family = family.to_string();
                    let saved = set(
                        &this,
                        format!("ui_font_family is {family:?}"),
                        cx,
                        |settings| {
                            settings.ui_font_family = Some(family);
                        },
                    );
                    if !saved {
                        synchronize(&this, window, cx);
                    }
                }
            });
        let reset = Button::new("restore_ui_font", "Restore default")
            .variant(ButtonVariant::Ghost)
            .disabled(SettingsStore::get(cx).ui_font_family.is_none())
            .on_click(move |_, window, cx| {
                set(&this, "ui_font_family is default".into(), cx, |settings| {
                    settings.ui_font_family = None;
                });
                synchronize(&this, window, cx);
            });
        SettingsSection::new("Interface font")
            .row(
                SettingsRow::new("Family").control(div().w(cx.theme().prose_width()).child(picker)),
            )
            .row(SettingsRow::new("Default").control(reset))
    }
}

fn synchronize(this: &Entity<SettingsWindow>, window: &mut Window, cx: &mut App) {
    let saved = interface_font::family(cx);
    let input = this.read(cx).ui_font.clone();
    input.update(cx, |input, cx| input.set_text(saved, cx));
    window.blur(cx);
}
