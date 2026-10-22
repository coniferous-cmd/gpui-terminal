use gpui::{Context, EntityId, Window};

use super::Workspace;
use crate::term::{State, TerminalEvent};

impl Workspace {
    /// Routes a terminal report to its owning tab, including hidden ones.
    pub(super) fn session_event(
        &mut self,
        id: EntityId,
        event: &TerminalEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((index, tab_index)) = self.locate_tab(id) else {
            return;
        };
        let shown = index == self.active && tab_index == self.sessions[index].active;
        let tab = &mut self.sessions[index].tabs[tab_index];
        match event {
            TerminalEvent::Title(title) => {
                log::debug!("app: tab {id:?}'s program names it {title:?}");
                tab.title.clone_from(title);
            }
            TerminalEvent::Folder(folder) => {
                tab.folder.clone_from(folder);
                tab.reports = true;
                if shown {
                    self.files.update(cx, |files, cx| files.show(folder, cx));
                    self.git.update(cx, |git, cx| git.show(folder, cx));
                }
            }
            TerminalEvent::Bell if !shown => {
                log::debug!("app: a bell in hidden tab {id:?}");
                tab.bell = true;
            }
            TerminalEvent::Bell => {}
            TerminalEvent::Shows(Some(State::Blocked)) if !shown => {
                log::debug!("app: hidden tab {id:?} waits");
                tab.waiting = true;
            }
            TerminalEvent::Shows(_) => {}
            TerminalEvent::Exited(status) => {
                log::info!(
                    "app: tab {}'s program ended, {status}",
                    tab.terminal.read(cx).pid()
                );
                self.close_tab(index, tab_index, window, cx);
                return;
            }
        }
        if shown {
            self.retitle(window);
        }
        cx.notify();
    }

    /// A notification press finds its exact tab, not just its session.
    pub fn pressed(&mut self, tag: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let found = self
            .sessions
            .iter()
            .enumerate()
            .find_map(|(index, session)| {
                session.tabs.iter().enumerate().find_map(|(tab, entry)| {
                    let focus = entry.terminal.update(cx, |view, _| view.pressed(tag))?;
                    Some((index, tab, focus))
                })
            });
        let Some((index, tab, focus)) = found else {
            return false;
        };
        if focus {
            cx.activate(true);
            window.activate_window();
            self.show_terminal(index, tab, window, cx);
        }
        true
    }
}
