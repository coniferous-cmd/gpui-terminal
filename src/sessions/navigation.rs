/// Selection after a removal; none means its owning group must close.
pub fn after_close(active: usize, closed: usize, left: usize) -> Option<usize> {
    if left == 0 {
        None
    } else if closed < active || active == left {
        Some(active - 1)
    } else {
        Some(active)
    }
}

/// A moved tab keeps its identity, including when another moves past it.
pub fn after_move(active: usize, from: usize, to: usize) -> usize {
    if active == from {
        to
    } else if from < active && to >= active {
        active - 1
    } else if from > active && to <= active {
        active + 1
    } else {
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removing_any_tab_preserves_the_selected_survivor() {
        for count in 1..12 {
            for active in 0..count {
                for closed in 0..count {
                    let mut tabs: Vec<_> = (0..count).collect();
                    tabs.remove(closed);
                    let next = after_close(active, closed, tabs.len());
                    assert_eq!(next.is_none(), tabs.is_empty());
                    if let Some(next) = next {
                        assert!(next < tabs.len());
                        if active != closed {
                            assert_eq!(tabs[next], active);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn reordering_any_tab_keeps_the_selected_terminal() {
        for count in 1..12 {
            for active in 0..count {
                for from in 0..count {
                    for to in 0..count {
                        let mut tabs: Vec<_> = (0..count).collect();
                        let moved = tabs.remove(from);
                        tabs.insert(to, moved);
                        assert_eq!(tabs[after_move(active, from, to)], active);
                    }
                }
            }
        }
    }

    #[test]
    fn the_last_tab_and_last_session_leave_no_selection() {
        let mut tab_counts = vec![2, 1];
        let mut session = 0;
        tab_counts[session] -= 1;
        assert_eq!(after_close(0, 0, tab_counts[session]), Some(0));
        tab_counts[session] -= 1;
        assert_eq!(after_close(0, 0, tab_counts[session]), None);
        tab_counts.remove(session);
        session = after_close(session, 0, tab_counts.len()).unwrap();
        assert_eq!(tab_counts[session], 1, "the other session survives");
        tab_counts[session] -= 1;
        assert_eq!(after_close(0, 0, tab_counts[session]), None);
        tab_counts.remove(session);
        assert_eq!(after_close(session, session, tab_counts.len()), None);
    }
}
