//! The layout the sessions stand in, noted to the snapshot at every change that moves it.

use gpui::Context;

use super::Workspace;
use crate::sessions::state::{SavedSession, SavedTab, SessionStore, Snapshot};

impl Workspace {
    /// Notes the layout as it stands now, writing it when `write` is set.
    pub(super) fn note(&mut self, write: bool, cx: &mut Context<Self>) {
        let snapshot = self.layout();
        SessionStore::note(cx, snapshot, write);
    }

    /// The sessions, their tabs and their folders; a tab whose folder is not text is left out.
    fn layout(&self) -> Snapshot {
        let mut sessions = Vec::with_capacity(self.sessions.len());
        let mut active = 0;
        for (index, session) in self.sessions.iter().enumerate() {
            let mut tabs = Vec::with_capacity(session.tabs.len());
            let mut shown = 0;
            for (at, tab) in session.tabs.iter().enumerate() {
                let Some(mut saved) = SavedTab::new(&tab.folder) else {
                    continue;
                };
                if at == session.active {
                    shown = tabs.len();
                }
                saved.name = tab.name.clone();
                tabs.push(saved);
            }
            if tabs.is_empty() {
                log::info!("app: session {:?} keeps no tab", session.id);
                continue;
            }
            let Some(saved) = SavedSession::new(&session.cwd, session.name.clone(), shown, tabs)
            else {
                continue;
            };
            if index == self.active {
                active = sessions.len();
            }
            sessions.push(saved);
        }
        Snapshot { active, sessions }
    }
}
