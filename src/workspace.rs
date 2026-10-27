use std::time::Instant;

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton, SegmentedControl},
    lists::List,
    primitives::{FocusScope, IconName},
    shell::{ColumnShell, ResizeBorder, ShellColumn},
    theme::{ActiveTheme, ControlSize},
    typography::Caption,
};
use gpui::{
    Action, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Subscription, Task, Window, actions, div, prelude::FluentBuilder,
};

use crate::{
    blur::Blur,
    files::Files,
    git::panel::{Acted, GitPanel},
    menus,
    sessions::{self, Session, orb},
    settings::{PanelTab, Settings, SettingsStore, window::OpenSettings},
};

mod closing;
mod control;
mod events;
mod keys;
mod rename;
pub mod restore;
mod save;
mod tab_bar;
mod tab_name;
mod tabs;
mod widths;

pub use keys::bind_keys;
pub use restore::{Opening, Started};
use widths::{Dragged, Side};

actions!(
    luna_terminal,
    [
        ToggleSessions,
        TogglePanel,
        NewSession,
        CloseSession,
        NextSession,
        PreviousSession,
        NewTab,
        CloseTab,
        NextTab,
        PreviousTab,
        LastTab
    ]
);

/// Shows a tab by position; an out-of-range binding passes to the program.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = luna_terminal, no_json)]
pub struct SelectTab(pub usize);

/// The key context of the main window's content.
pub const CONTEXT: &str = "Workspace";

/// The window's title while the program names none.
pub const TITLE: &str = "luna-terminal";

/// The main window's content: sessions, the terminal, and Git or Files, in Ely's columns.
pub struct Workspace {
    runtime_id: String,
    focus: FocusHandle,
    pub(crate) pointer: crate::pet::gaze::Pointer,
    sessions: Vec<Session>,
    /// The session shown.
    active: usize,
    closing: Option<gpui::EntityId>,
    files: Entity<Files>,
    git: Entity<GitPanel>,
    /// What the window shows of the desktop, and the blur behind it.
    background: Blur,
    /// Column widths a drag shows before the settings hold them.
    dragged: Dragged,
    /// Draws the window at each step of the status marks while one shown moves.
    ticking: Option<Task<()>>,
    _settings: Subscription,
    _acted: Subscription,
}

impl Workspace {
    /// The window's content holding the sessions `openings` describe, `active` shown.
    pub fn new(
        openings: Vec<Opening>,
        active: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let Some(first) = openings.first().and_then(|opening| opening.tabs.first()) else {
            unreachable!("app: a window opens with at least one tab");
        };
        let folder = first.launch.cwd.clone();
        let git = cx.new(|cx| GitPanel::new(folder.clone(), window, cx));
        // A change the Git panel made shows in Files' marks too.
        let acted = cx.subscribe(&git, |this, _, _: &Acted, cx| {
            let folder = this.sessions[this.active].current().folder.clone();
            this.files.update(cx, |files, cx| files.show(&folder, cx));
        });
        let mut workspace = Self {
            runtime_id: crate::cli::identity::next('w'),
            focus: cx.focus_handle(),
            pointer: Default::default(),
            sessions: Vec::with_capacity(openings.len()),
            active: 0,
            closing: None,
            files: cx.new(|_| Files::new(folder)),
            git,
            background: Blur::new(window, SettingsStore::get(cx)),
            dragged: Dragged::default(),
            ticking: None,
            _settings: cx.observe_global_in::<SettingsStore>(window, |this, window, cx| {
                this.background.follow(window, SettingsStore::get(cx));
                cx.notify();
            }),
            _acted: acted,
        };
        for opening in openings {
            let mut tabs = Vec::with_capacity(opening.tabs.len());
            for started in opening.tabs {
                let mut tab =
                    workspace.make_tab(started.pty, started.events, started.launch, window, cx);
                tab.name = started.name;
                tabs.push(tab);
            }
            workspace.add_session(opening.name, opening.active, tabs);
        }
        let active = active.min(workspace.sessions.len().saturating_sub(1));
        workspace.select(active, window, cx);
        workspace
    }

    /// Notes each session's mark `now`, and draws the window at each step while one shown moves.
    fn tick(&mut self, now: Instant, cx: &mut Context<Self>) {
        let mut moving = false;
        for session in &mut self.sessions {
            moving |= session.note(now, cx);
        }
        let moving = moving && SettingsStore::get(cx).show_sessions;
        if moving == self.ticking.is_some() {
            return;
        }
        log::debug!("app: status marks {}", if moving { "move" } else { "rest" });
        self.ticking = moving.then(|| {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(orb::wait(Instant::now()))
                        .await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
        });
    }

    /// Shows session `index`, clears its bell and its waiting mark, and gives its terminal the keys.
    pub fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.sessions.len() {
            return;
        }
        if index != self.active
            && let Some(hidden) = self.sessions.get(self.active)
        {
            hidden.current().terminal.update(cx, |view, cx| {
                view.let_go(cx);
                view.set_front(false);
            });
        }
        let session = &mut self.sessions[index];
        let tab = &mut session.tabs[session.active];
        tab.bell = false;
        tab.waiting = false;
        tab.terminal.update(cx, |view, _| view.set_front(true));
        window.focus(&tab.terminal.focus_handle(cx), cx);
        let folder = tab.folder.clone();
        self.files.update(cx, |files, cx| files.show(&folder, cx));
        self.git.update(cx, |git, cx| git.show(&folder, cx));
        self.active = index;
        log::debug!("app: session {} shown", index + 1);
        self.retitle(window);
        // The selection is kept in memory and written once, at quit.
        self.note(false, cx);
        cx.notify();
    }

    /// Closes session `index`; its program hangs up. The last one closes the window.
    pub fn close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.sessions.len() {
            return;
        }
        let session = self.sessions.remove(index);
        if self.closing == Some(session.id) {
            self.closing = None;
        }
        log::info!(
            "app: session {:?} closes with {} tabs",
            session.id,
            session.tabs.len()
        );
        let Some(next) = sessions::after_close(self.active, index, self.sessions.len()) else {
            log::info!("app: no session left; the window closes");
            // Nothing is kept but the empty layout, so the next start opens one home session.
            self.note(true, cx);
            window.remove_window();
            return;
        };
        let shown = index == self.active;
        self.active = next;
        if shown {
            self.select(self.active, window, cx);
        } else {
            cx.notify();
        }
        self.note(true, cx);
    }

    /// Its sessions, in their rows' order.
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// What the shown session is called.
    fn title(&self) -> &str {
        self.sessions.get(self.active).map_or(TITLE, Session::named)
    }

    fn retitle(&self, window: &mut Window) {
        let title = self.title();
        if window.window_title() != title {
            log::debug!("app: the title is {title:?}");
            window.set_window_title(title);
        }
    }

    /// Shows or hides a side column, as the settings keep it.
    fn toggle(&mut self, flip: fn(&mut Settings), cx: &mut Context<Self>) {
        match SettingsStore::update(cx, flip) {
            Ok(()) => {
                let settings = SettingsStore::get(cx);
                let shown = |on: bool| if on { "shown" } else { "hidden" };
                log::info!(
                    "app: sessions {}, right column {}",
                    shown(settings.show_sessions),
                    shown(settings.show_panel)
                );
            }
            Err(error) => log::error!("app: the columns stay: {error:#}"),
        }
    }

    /// Shows the next session, or the one before, round the ends.
    fn step(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.sessions.len();
        let by = if forward { 1 } else { count - 1 };
        self.select((self.active + by) % count, window, cx);
    }
}

/// Shows Git or Files in the right column, as the settings keep it.
fn show_tab(value: &SharedString, cx: &mut App) {
    let panel = match value.as_ref() {
        "git" => PanelTab::Git,
        "files" => PanelTab::Files,
        other => unreachable!("app: no {other} tab"),
    };
    if SettingsStore::get(cx).panel == panel {
        return;
    }
    match SettingsStore::update(cx, |settings| settings.panel = panel) {
        Ok(()) => log::info!("app: the right column shows {panel:?}"),
        Err(error) => log::error!("app: the right column stays: {error:#}"),
    }
}

/// A header button that hides or shows a column, saved.
fn column_toggle(
    workspace: &Entity<Workspace>,
    (id, icon): (&'static str, IconName),
    tip: String,
    flip: fn(&mut Settings),
) -> IconButton {
    let workspace = workspace.clone();
    IconButton::new(id, icon)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
        .tooltip(tip)
        .on_click(move |_, _, cx| workspace.update(cx, |this, cx| this.toggle(flip, cx)))
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.background.fit(window);
        let now = Instant::now();
        self.tick(now, cx);
        let settings = SettingsStore::get(cx);
        let workspace = cx.entity();
        let tip =
            |shown: bool, what: &str| format!("{} {what}", if shown { "Hide" } else { "Show" });
        // It leads the first column or ends the last.
        let left = column_toggle(
            &workspace,
            ("toggle-sessions", IconName::PanelLeft),
            tip(settings.show_sessions, "sessions"),
            |settings| settings.show_sessions = !settings.show_sessions,
        );
        let right = column_toggle(
            &workspace,
            ("toggle-panel", IconName::PanelRight),
            tip(settings.show_panel, "Git and Files"),
            |settings| settings.show_panel = !settings.show_panel,
        );
        // No menu bar off macOS: a gear opens Settings.
        let gear = (!cfg!(target_os = "macos")).then(|| {
            IconButton::new("open-settings", IconName::Settings)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip("Settings")
                .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenSettings), cx))
        });
        let end = div()
            .flex()
            .items_center()
            .gap_2()
            .children(gear)
            .child(right);
        let mut shell = ColumnShell::new("workspace").glass();
        let shown = self
            .sessions
            .get(self.active)
            .map(|session| session.current().terminal.clone());
        let tab_bar = self.tab_bar(cx);
        let mut terminal = ShellColumn::new("terminal")
            .title(self.title().to_string())
            .child(tab_bar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(div().absolute().size_full().children(shown)),
            );
        if settings.show_sessions {
            let home = dirs::home_dir();
            let rows = self.sessions.iter().enumerate().map(|(index, session)| {
                let active = index == self.active;
                sessions::row(&workspace, index, session, active, home.as_deref(), now, cx)
            });
            let list = List::new().children(rows);
            let new = IconButton::new("new-session", IconName::Plus)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip("New session")
                .on_click(move |_, window, cx| {
                    workspace.update(cx, |this, cx| this.new_terminal(true, window, cx))
                });
            shell = shell.column(
                self.side_column(Side::Sessions, cx)
                    .leading(left)
                    .action(new)
                    .child(
                        div()
                            .id("sessions")
                            .size_full()
                            .overflow_y_scroll()
                            .p_2()
                            .child(list),
                    ),
            );
        } else {
            terminal = terminal.leading(left);
        }
        if settings.show_panel {
            let tab = settings.panel;
            let chosen = match tab {
                PanelTab::Git => "git",
                PanelTab::Files => "files",
            };
            let switch = SegmentedControl::new("panel-tab", chosen)
                .segment("git", "Git", None)
                .segment("files", "Files", None)
                .size(ControlSize::Sm)
                .on_change(|value, _, cx| show_tab(value, cx));
            let quiet = self
                .sessions
                .get(self.active)
                .is_some_and(|session| !session.current().reports);
            let caption = || Caption::new("This shell reports no folder.").px_3().pt_2();
            // Hidden, not dropped, so each keeps its state.
            let body = |on: bool| {
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .when(!on, |body| body.hidden())
            };
            // Git's list draws every row, so terminal output must not redraw it.
            let fill = || StyleRefinement::default().size_full();
            shell = shell.column(terminal).column(
                self.side_column(Side::Panel, cx)
                    .action(end)
                    // Below the title bar, clear of Windows' caption buttons.
                    .child(div().flex().justify_center().px_2().pt_2().child(switch))
                    .children(quiet.then(caption))
                    .child(body(tab == PanelTab::Git).child(self.git.clone().cached(fill())))
                    .child(body(tab == PanelTab::Files).child(self.files.clone())),
            );
        } else {
            shell = shell.column(terminal.action(end));
        }
        // The context sits above the focus root, so its keys work wherever focus is.
        div()
            .size_full()
            .font_family(cx.theme().font_family.clone())
            .child(self.pointer.listener(crate::pet::gaze::Source::Main))
            .key_context(CONTEXT)
            .map(menus::window_actions)
            .on_action(cx.listener(|this, _: &ToggleSessions, _, cx| {
                this.toggle(
                    |settings| settings.show_sessions = !settings.show_sessions,
                    cx,
                )
            }))
            .on_action(cx.listener(|this, _: &TogglePanel, _, cx| {
                this.toggle(|settings| settings.show_panel = !settings.show_panel, cx)
            }))
            .on_action(
                cx.listener(|this, _: &NewTab, window, cx| this.new_terminal(false, window, cx)),
            )
            .on_action(cx.listener(|this, _: &CloseTab, window, cx| {
                let tab = this.sessions[this.active].active;
                this.close_tab(this.active, tab, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextTab, window, cx| this.step_tab(true, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousTab, window, cx| this.step_tab(false, window, cx)),
            )
            .on_action(cx.listener(|this, _: &LastTab, window, cx| {
                let last = this.sessions[this.active].tabs.len() - 1;
                this.select_tab(last, window, cx);
            }))
            .on_action(cx.listener(|this, select: &SelectTab, window, cx| {
                if select.0 < this.sessions[this.active].tabs.len() {
                    this.select_tab(select.0, window, cx);
                } else {
                    cx.propagate();
                }
            }))
            .on_action(
                cx.listener(|this, _: &NewSession, window, cx| this.new_terminal(true, window, cx)),
            )
            .on_action(cx.listener(|this, _: &CloseSession, window, cx| {
                this.request_close(this.active, window, cx)
            }))
            .on_action(cx.listener(|this, _: &NextSession, window, cx| this.step(true, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousSession, window, cx| this.step(false, window, cx)),
            )
            .child(
                ResizeBorder::new()
                    .when(self.background.see_through(), ResizeBorder::glass)
                    .child(FocusScope::new(&self.focus).root().size_full().child(shell)),
            )
            .children(self.close_dialog(cx))
    }
}
