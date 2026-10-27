use ely_gpui_component::{
    primitives::IconName,
    theme::{ActiveTheme, ControlSize},
};
use gpui::{
    AppContext, Context, EntityId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, rems,
};

use super::{
    Workspace,
    tab_bar::{TabBar, WindowTab},
};
use crate::{
    sessions::{self, Session, Tab, orb::Look},
    settings::{SettingsStore, StartIn},
    term::{
        Events, Launch, Pty, TerminalView, config,
        shell::{ShellIntegration, launch},
        spawn,
    },
};

impl Workspace {
    pub(super) fn make_tab(
        &mut self,
        pty: Pty,
        events: Events,
        launch: Launch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Tab {
        let terminal = cx.new(|cx| TerminalView::new(pty, events, window, cx));
        let id = terminal.entity_id();
        let events = cx.subscribe_in(&terminal, window, move |this, _, event, window, cx| {
            this.session_event(id, event, window, cx)
        });
        log::info!(
            "app: tab {} opens in {}",
            terminal.read(cx).pid(),
            launch.cwd.display()
        );
        Tab {
            terminal,
            title: String::new(),
            name: String::new(),
            editing: None,
            folder: launch.cwd,
            reports: launch.reports,
            bell: false,
            waiting: false,
            _events: events,
        }
    }

    pub(super) fn add_session(&mut self, name: String, active: usize, tabs: Vec<Tab>) {
        let Some(first) = tabs.first() else {
            log::error!("app: a session with no tab is not opened");
            return;
        };
        let id = first.terminal.entity_id();
        log::info!("app: session {id:?} opens with {} tabs", tabs.len());
        self.sessions.push(Session {
            id,
            runtime_id: crate::cli::identity::next('s'),
            tabs,
            active,
            name,
            editing: None,
            look: None,
        });
    }

    pub(super) fn new_terminal(
        &mut self,
        group: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.create_terminal(group, None, String::new(), window, cx) {
            log::error!("app: no new terminal: {error:#}");
        }
    }

    pub(super) fn create_terminal(
        &mut self,
        group: bool,
        folder: Option<std::path::PathBuf>,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let settings = SettingsStore::get(cx);
        let folder = match folder {
            Some(folder) => folder,
            None => match settings.start_in {
                StartIn::Session if !self.sessions.is_empty() => {
                    self.sessions[self.active].current().folder.clone()
                }
                _ => dirs::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("app: no home folder, so no new terminal"))?,
            },
        };
        anyhow::ensure!(
            folder.is_dir(),
            "app: working directory is not a directory: {}",
            folder.display()
        );
        let launch = launch(&settings.shell, folder, &cx.global::<ShellIntegration>().0)?;
        let (pty, events) = spawn(&launch, (80, 24), (8, 16), config(settings))?;
        let tab = self.make_tab(pty, events, launch, window, cx);
        if group {
            self.add_session(name.trim().to_string(), 0, vec![tab]);
            self.select(self.sessions.len() - 1, window, cx);
        } else {
            let session = &mut self.sessions[self.active];
            session.tabs.push(tab);
            let index = session.tabs.len() - 1;
            self.select_tab(index, window, cx);
        }
        self.note(true, cx);
        Ok(())
    }

    pub(super) fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let session = &mut self.sessions[self.active];
        if index >= session.tabs.len() {
            return;
        }
        if index != session.active {
            session.current().terminal.update(cx, |view, cx| {
                view.let_go(cx);
                view.set_front(false);
            });
        }
        session.active = index;
        log::debug!("app: session {:?} shows tab {}", session.id, index + 1);
        self.select(self.active, window, cx);
    }

    pub(super) fn show_terminal(
        &mut self,
        group: usize,
        tab: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if group == self.active {
            self.select_tab(tab, window, cx);
        } else if let Some(session) = self.sessions.get_mut(group) {
            if tab >= session.tabs.len() {
                return;
            }
            session.active = tab;
            self.select(group, window, cx);
        }
    }

    pub(super) fn close_tab(
        &mut self,
        session: usize,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.sessions.get_mut(session) else {
            return;
        };
        if index >= group.tabs.len() {
            return;
        }
        let tab = group.tabs.remove(index);
        log::info!("app: tab {} closes", tab.terminal.read(cx).pid());
        let shown = group.active == index && self.active == session;
        let Some(next) = sessions::after_close(group.active, index, group.tabs.len()) else {
            self.close(session, window, cx);
            return;
        };
        group.active = next;
        if shown {
            self.select(session, window, cx);
        } else {
            cx.notify();
        }
        self.note(true, cx);
    }

    pub(super) fn step_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let group = &self.sessions[self.active];
        let count = group.tabs.len();
        let by = if forward { 1 } else { count - 1 };
        self.select_tab((group.active + by) % count, window, cx);
    }

    pub(super) fn locate_tab(&self, id: EntityId) -> Option<(usize, usize)> {
        self.sessions
            .iter()
            .enumerate()
            .find_map(|(group, session)| {
                session
                    .tabs
                    .iter()
                    .position(|tab| tab.terminal.entity_id() == id)
                    .map(|tab| (group, tab))
            })
    }

    pub(super) fn tab_bar(&self, cx: &Context<Self>) -> impl IntoElement + use<> {
        let session = &self.sessions[self.active];
        let group = session.id;
        let count = session.tabs.len() as f32;
        let minimum = cx.theme().tab_width().0 * count
            + rems(0.25) * count
            + cx.theme().control_height(ControlSize::Sm);
        let ids: Vec<_> = session
            .tabs
            .iter()
            .map(|tab| tab.terminal.entity_id())
            .collect();
        let mut bar =
            TabBar::new(("terminal-tabs", group)).selected(format!("{:?}", ids[session.active]));
        for (index, tab) in session.tabs.iter().enumerate() {
            let title = sessions::one_line(tab.named());
            let icon = if tab.waiting {
                IconName::CircleAlert
            } else if tab.bell {
                IconName::Bell
            } else {
                match tab.looks(cx) {
                    Some(Look::Waiting) => IconName::CircleAlert,
                    Some(Look::Error) => IconName::CircleX,
                    Some(Look::Working(_)) => IconName::LoaderCircle,
                    Some(Look::Done) => IconName::CircleCheck,
                    _ => IconName::Terminal,
                }
            };
            let mut item = WindowTab::new(format!("{:?}", ids[index]), title).icon(icon);
            if let Some((input, _)) = &tab.editing {
                item = item.editor(self.tab_name_editor(ids[index], input, cx));
            }
            bar = bar.tab(item);
        }
        let select = cx.entity();
        let close = cx.entity();
        let add = cx.entity();
        let reorder = cx.entity();
        let close_ids = ids.clone();
        let rename_ids = ids.clone();
        let rename = cx.entity();
        let bar = bar
            .on_rename(move |id, window, cx| {
                if let Some(key) = rename_ids
                    .iter()
                    .find(|key| format!("{key:?}") == id.as_ref())
                {
                    rename.update(cx, |this, cx| this.rename_tab(*key, window, cx));
                }
            })
            .on_select(move |id, window, cx| {
                let Some(index) = ids.iter().position(|key| format!("{key:?}") == id.as_ref())
                else {
                    return;
                };
                select.update(cx, |this, cx| {
                    if let Some((group, tab)) = this.locate_tab(ids[index]) {
                        this.show_terminal(group, tab, window, cx);
                    }
                });
            })
            .on_close(move |id, window, cx| {
                let Some(index) = close_ids
                    .iter()
                    .position(|key| format!("{key:?}") == id.as_ref())
                else {
                    return;
                };
                close.update(cx, |this, cx| {
                    if let Some((group, tab)) = this.locate_tab(close_ids[index]) {
                        this.close_tab(group, tab, window, cx);
                    }
                });
            })
            .on_add(move |window, cx| {
                add.update(cx, |this, cx| {
                    if this.sessions[this.active].id == group {
                        this.new_terminal(false, window, cx);
                    }
                });
            })
            .on_reorder(move |from, to, _, cx| {
                reorder.update(cx, |this, cx| {
                    let Some(session) =
                        this.sessions.iter_mut().find(|session| session.id == group)
                    else {
                        return;
                    };
                    if from >= session.tabs.len() || to >= session.tabs.len() {
                        return;
                    }
                    let tab = session.tabs.remove(from);
                    session.tabs.insert(to, tab);
                    session.active = sessions::after_move(session.active, from, to);
                    log::debug!("app: session {group:?} moves tab {from} to {to}");
                    this.note(true, cx);
                    cx.notify();
                });
            });
        div()
            .id(("tab-scroll", group))
            .w_full()
            .flex_none()
            .overflow_x_scroll()
            .p_2()
            .child(div().min_w(minimum).child(bar))
    }
}
