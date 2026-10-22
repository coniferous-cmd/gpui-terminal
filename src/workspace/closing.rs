use ely_gpui_component::overlays::ConfirmDialog;
use gpui::{Context, Window};

use super::Workspace;

impl Workspace {
    /// Closing a group asks before hanging up multiple terminals.
    pub fn request_close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(index) else {
            return;
        };
        if session.tabs.len() == 1 {
            self.close(index, window, cx);
        } else {
            log::debug!("app: closing session {:?} needs confirmation", session.id);
            self.closing = Some(session.id);
            cx.notify();
        }
    }

    pub(super) fn close_dialog(&self, cx: &Context<Self>) -> Option<ConfirmDialog> {
        let id = self.closing?;
        let session = self.sessions.iter().find(|session| session.id == id)?;
        let cancel = cx.entity();
        let confirm = cx.entity();
        Some(
            ConfirmDialog::new(
                ("close-session-dialog", id),
                format!("Close {}?", session.named()),
                format!(
                    "This will close all {} tabs and stop their programs.",
                    session.tabs.len()
                ),
                move |_, cx| {
                    cancel.update(cx, |this, cx| {
                        this.closing = None;
                        cx.notify();
                    })
                },
            )
            .confirm("Close session")
            .destructive()
            .on_confirm(move |window, cx| {
                confirm.update(cx, |this, cx| {
                    this.closing = None;
                    if let Some(index) = this.sessions.iter().position(|session| session.id == id) {
                        this.close(index, window, cx);
                    }
                });
            }),
        )
    }
}
