//! Her animated window, observing the pointer without taking terminal input.

use std::time::Instant;

use gpui::{
    Context, DispatchPhase, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Render, Styled, Window, canvas, div, point, px,
};

use super::{Pet, ROOM, gaze};

impl Render for Pet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        let mut pose = self.pose(now);
        let pointer = gaze::position(self, window, cx);
        self.gaze.apply(&mut pose, pointer, now);
        // gpui gives a hidden window no frames, so she rests while unseen.
        window.request_animation_frame();
        let rig = self.rig.clone();
        let this = cx.entity();
        let drawn = canvas(
            |_, _, _| {},
            move |bounds, _, window, cx| {
                let head = point(bounds.origin.x, bounds.origin.y + px(ROOM));
                rig.paint(head, &pose, window, cx);
                // Her window keeps the pointer while the button is held, inside it or not.
                let held = this.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble && event.dragging() {
                        let scale = window.scale_factor();
                        held.update(cx, |pet, cx| pet.slide(event.position.x, scale, cx));
                    }
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                        this.update(cx, |pet, cx| pet.release(cx));
                    }
                });
            },
        );
        div()
            .size_full()
            .child(self.pointer.listener(gaze::Source::Pet))
            .child(drawn.size_full())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|pet, event: &MouseDownEvent, window, cx| {
                    pet.press(Instant::now());
                    pet.grab(event.position.x, window.scale_factor());
                    cx.notify();
                }),
            )
    }
}
