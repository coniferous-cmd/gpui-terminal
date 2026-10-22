use std::{path::PathBuf, time::Instant};

use ely_gpui_component::forms::TextInput;
use gpui::{App, Entity, EntityId, Subscription};

use super::{
    look,
    orb::{self, Look},
};
use crate::{term::TerminalView, workspace::TITLE};

/// A group of independent terminals, retaining its selected tab.
pub struct Session {
    pub id: EntityId,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub name: String,
    pub editing: Option<(Entity<TextInput>, Subscription)>,
    pub look: Option<(Look, Instant)>,
}

/// One shell and its reports, owned by exactly one session.
pub struct Tab {
    pub terminal: Entity<TerminalView>,
    pub title: String,
    pub folder: PathBuf,
    pub reports: bool,
    pub bell: bool,
    pub waiting: bool,
    pub _events: Subscription,
}

impl Tab {
    pub fn named(&self) -> &str {
        if self.title.is_empty() {
            TITLE
        } else {
            &self.title
        }
    }

    pub fn looks(&self, cx: &App) -> Option<Look> {
        let view = self.terminal.read(cx);
        look(
            view.records().shown().map(|(_, record)| record),
            view.ended().is_some_and(|code| code != 0),
        )
    }
}

impl Session {
    pub fn current(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn named(&self) -> &str {
        if self.name.is_empty() {
            self.current().named()
        } else {
            &self.name
        }
    }

    pub fn looks(&self, cx: &App) -> Option<Look> {
        self.tabs
            .iter()
            .filter_map(|tab| tab.looks(cx))
            .min_by_key(priority)
    }

    pub fn note(&mut self, now: Instant, cx: &App) -> bool {
        let shows = self.looks(cx);
        if self.look.map(|(shown, _)| shown) != shows {
            self.look = shows.map(|shows| (shows, now));
        }
        self.look
            .is_some_and(|(look, since)| orb::moves(look, now.saturating_duration_since(since)))
    }
}

fn priority(look: &Look) -> u8 {
    match look {
        Look::Waiting => 0,
        Look::Error => 1,
        Look::Working(_) => 2,
        Look::Done => 3,
        Look::Idle => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_status_prioritizes_attention_over_activity() {
        let looks = [
            Look::Idle,
            Look::Working(Some(50)),
            Look::Error,
            Look::Waiting,
            Look::Done,
        ];
        assert_eq!(looks.into_iter().min_by_key(priority), Some(Look::Waiting));
        assert!(priority(&Look::Error) < priority(&Look::Working(None)));
        assert!(priority(&Look::Working(None)) < priority(&Look::Done));
    }
}
