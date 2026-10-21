//! Window-local pointer tracking and a smooth, bounded gaze over her own animation.

use std::{cell::Cell, rc::Rc, time::Instant};

use gpui::{
    DispatchPhase, IntoElement, MouseExitEvent, MouseMoveEvent, Pixels, Point, Styled, canvas,
};

use super::{HEAD, ROOM, WIDTH, left, look::Pose};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Main,
    Pet,
}

type Position = Option<(Source, Point<Pixels>)>;

#[derive(Clone, Default)]
pub struct Pointer(Rc<Cell<Position>>);

impl Pointer {
    pub fn position(&self) -> Option<(Source, Point<Pixels>)> {
        self.0.get()
    }

    fn enter(&self, source: Source, position: Point<Pixels>) {
        if self.0.get().is_none_or(|(old, _)| old != source) {
            log::debug!("pet: Ely watches the pointer in {source:?}");
        }
        self.0.set(Some((source, position)));
    }

    pub(super) fn leave(&self, source: Source) {
        if self.0.get().is_some_and(|(old, _)| old == source) {
            log::debug!("pet: Ely returns to her own gaze");
            self.0.set(None);
        }
    }

    pub fn listener(&self, source: Source) -> impl IntoElement {
        let pointer = self.clone();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let moved = pointer.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _| {
                    if phase == DispatchPhase::Capture {
                        if gpui::Bounds::new(
                            gpui::point(gpui::px(0.), gpui::px(0.)),
                            window.viewport_size(),
                        )
                        .contains(&event.position)
                        {
                            moved.enter(source, event.position);
                        } else {
                            moved.leave(source);
                        }
                    }
                });
                let exited = pointer.clone();
                window.on_mouse_event(move |_: &MouseExitEvent, phase, _, _| {
                    if phase == DispatchPhase::Capture {
                        exited.leave(source);
                    }
                });
            },
        )
        .absolute()
        .size_0()
    }
}

/// A point in the main window, relative to her popup's top left.
pub fn local(
    position: Point<Pixels>,
    pane: gpui::Bounds<Pixels>,
    at: f32,
    bottom: bool,
) -> (f32, f32) {
    let x = f32::from(position.x - pane.origin.x) - left(pane.size.width, at);
    let edge = if bottom {
        f32::from(pane.size.height)
    } else {
        0.0
    };
    let y = f32::from(position.y - pane.origin.y) - edge + HEAD + ROOM;
    (x, y)
}

pub(super) fn position(
    pet: &super::Pet,
    window: &gpui::Window,
    cx: &mut gpui::App,
) -> Option<(f32, f32)> {
    let (source, position) = pet.pointer.position()?;
    match source {
        Source::Pet if hovered(window) => Some((position.x.into(), position.y.into())),
        Source::Pet => {
            pet.pointer.leave(source);
            None
        }
        Source::Main => match pet.main.update(cx, |_, window, _| {
            if !hovered(window) {
                pet.pointer.leave(source);
                return None;
            }
            Some(local(
                position,
                super::pane(window),
                pet.at,
                pet.edge == super::Edge::Bottom,
            ))
        }) {
            Ok(position) => position,
            Err(error) => {
                log::error!("pet: the main window's pointer does not read: {error:#}");
                None
            }
        },
    }
}

// AppKit sends exits; gpui's macOS hover flag means activation, which her popup never takes.
fn hovered(window: &gpui::Window) -> bool {
    cfg!(target_os = "macos") || window.is_window_hovered()
}

#[derive(Default)]
pub struct Gaze {
    last: Option<Instant>,
    look: (f32, f32),
    weight: f32,
}

impl Gaze {
    pub fn apply(&mut self, pose: &mut Pose, pointer: Option<(f32, f32)>, now: Instant) {
        let dt = self.last.replace(now).map_or(0.0, |last| {
            now.saturating_duration_since(last).as_secs_f32()
        });
        self.step(pose, pointer, dt);
    }

    fn step(&mut self, pose: &mut Pose, pointer: Option<(f32, f32)>, dt: f32) {
        let k = 1.0 - (-dt / 0.12).exp();
        if let Some((x, y)) = pointer {
            let unit = WIDTH / 292.0;
            let angle = pose.tilt.to_radians();
            let center = (
                WIDTH / 2.0 + (pose.shake + 44.0 * angle.sin()) * unit,
                ROOM + (230.0 - 44.0 * angle.cos() - pose.lift - pose.bob) * unit,
            );
            let target = target((x - center.0, y - center.1));
            if self.weight == 0.0 {
                self.look = pose.look;
            }
            self.look.0 += (target.0 - self.look.0) * k;
            self.look.1 += (target.1 - self.look.1) * k;
        }
        self.weight += (if pointer.is_some() { 1.0 } else { 0.0 } - self.weight) * k;
        pose.look.0 += (self.look.0 - pose.look.0) * self.weight;
        pose.look.1 += (self.look.1 - pose.look.1) * self.weight;
    }
}

fn target((x, y): (f32, f32)) -> (f32, f32) {
    let (x, y) = (x / 160.0, y / 160.0);
    let length = x.hypot(y).max(1.0);
    (6.0 * x / length, 5.0 * y / length)
}

#[cfg(test)]
mod tests {
    use super::super::look::{Eyes, Mood, pose};
    use super::*;
    use gpui::{Bounds, point, px, size};

    #[test]
    fn gaze_is_bounded_in_every_direction() {
        assert_eq!(target((0.0, 0.0)), (0.0, 0.0));
        assert_eq!(target((1600.0, 0.0)), (6.0, 0.0));
        assert_eq!(target((0.0, -1600.0)), (0.0, -5.0));
        let (x, y) = target((1600.0, 1600.0));
        assert!(((x / 6.0).powi(2) + (y / 5.0).powi(2) - 1.0).abs() < 0.00001);
    }

    #[test]
    fn late_exit_from_one_window_keeps_the_other_windows_pointer() {
        let pointer = Pointer::default();
        let at = point(px(12.), px(30.));
        pointer.enter(Source::Main, at);
        pointer.enter(Source::Pet, at);
        pointer.leave(Source::Main);
        assert_eq!(pointer.position(), Some((Source::Pet, at)));
        pointer.leave(Source::Pet);
        assert_eq!(pointer.position(), None);
    }

    #[test]
    fn popup_coordinates_follow_either_edge_and_narrow_panes() {
        let pane = Bounds::new(point(px(5.), px(5.)), size(px(200.), px(600.)));
        let at = point(px(93.), px(5.));
        assert_eq!(local(at, pane, 140.0, false), (0.0, HEAD + ROOM));
        assert_eq!(local(at, pane, 140.0, true), (0.0, HEAD + ROOM - 600.0));
    }

    #[test]
    fn stationary_pointer_stays_watched_then_leaving_restores_animation() {
        let mut gaze = Gaze::default();
        let base = pose(Mood::Working, 1.0, 10.0, None);
        let mut watched = base.clone();
        for _ in 0..600 {
            watched = base.clone();
            gaze.step(&mut watched, Some((2000.0, 80.0)), 1.0 / 60.0);
        }
        assert!(watched.look.0 > 5.9);
        for _ in 0..120 {
            watched = base.clone();
            gaze.step(&mut watched, None, 1.0 / 60.0);
        }
        assert!((watched.look.0 - base.look.0).abs() < 0.001);
        assert!((watched.look.1 - base.look.1).abs() < 0.001);
    }

    #[test]
    fn closed_eyes_and_other_pose_fields_are_preserved() {
        for (mood, held) in [(Mood::Calm, 301.0), (Mood::Done, 1.0), (Mood::Waiting, 1.0)] {
            let mut pose = pose(mood, held, 10.0, None);
            let before = pose.clone();
            Gaze::default().step(&mut pose, Some((500.0, 500.0)), 0.1);
            assert_eq!(pose.eyes, before.eyes);
            assert_eq!(pose.open, before.open);
            assert_eq!(pose.lift, before.lift);
        }
        assert_eq!(pose(Mood::Calm, 301.0, 10.0, None).eyes, Eyes::Lines);
    }

    #[test]
    fn smoothing_is_independent_of_refresh_rate() {
        let run = |fps| {
            let mut gaze = Gaze::default();
            for _ in 0..fps {
                let mut pose = pose(Mood::Calm, 1.0, 10.0, None);
                gaze.step(&mut pose, Some((500.0, 90.0)), 1.0 / fps as f32);
            }
            (gaze.look, gaze.weight)
        };
        let (a, wa) = run(60);
        let (b, wb) = run(144);
        assert!((a.0 - b.0).abs() < 0.0001);
        assert!((wa - wb).abs() < 0.0001);
    }
}
