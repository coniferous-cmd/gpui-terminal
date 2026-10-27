use ely_gpui_component::{
    forms::{Input, InputEvent, TextInput},
    theme::ControlSize,
};
use gpui::{
    AppContext, Context, Entity, EntityId, Focusable, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Window, div,
};

use super::Workspace;

impl Workspace {
    pub(super) fn tab_name_editor(
        &self,
        id: EntityId,
        input: &Entity<TextInput>,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let workspace = cx.entity();
        div()
            .id(("tab-name-editor", id))
            .w_full()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    workspace.update(cx, |this, cx| {
                        this.end_tab_rename(id, false, window, cx);
                    });
                }
            })
            .child(Input::new(input).size(ControlSize::Sm))
    }

    pub(super) fn rename_tab(&mut self, id: EntityId, window: &mut Window, cx: &mut Context<Self>) {
        let Some((group, at)) = self.locate_tab(id) else {
            return;
        };
        let tab = &self.sessions[group].tabs[at];
        if tab.editing.is_some() {
            return;
        }
        let name = tab.named().to_string();
        let input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(name.clone(), cx);
            input.select(0..name.len(), cx);
            input
        });
        let events = cx.subscribe_in(
            &input,
            window,
            move |this, _, event, window, cx| match event {
                InputEvent::Submit | InputEvent::Blur => {
                    this.end_tab_rename(id, true, window, cx);
                }
                InputEvent::Changed | InputEvent::Focus => {}
            },
        );
        window.focus(&input.focus_handle(cx), cx);
        self.sessions[group].tabs[at].editing = Some((input, events));
        log::debug!("app: tab {id:?} is renamed in place");
        cx.notify();
    }

    fn end_tab_rename(
        &mut self,
        id: EntityId,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((group, at)) = self.locate_tab(id) else {
            return;
        };
        let tab = &mut self.sessions[group].tabs[at];
        let Some((input, _)) = tab.editing.take() else {
            return;
        };
        let held = input.focus_handle(cx).is_focused(window);
        if keep {
            tab.name = input.read(cx).text().trim().to_string();
            log::info!("app: tab {id:?} custom name is {:?}", tab.name);
        } else {
            log::debug!("app: tab {id:?} keeps its name");
        }
        if held {
            window.focus(
                &self.sessions[self.active]
                    .current()
                    .terminal
                    .focus_handle(cx),
                cx,
            );
        }
        self.note(true, cx);
        self.retitle(window);
        cx.notify();
    }
}
