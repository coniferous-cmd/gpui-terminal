//! Ely rigged as the films rig her: the icon's paths in layers of one color, each a mask gpui draws with a matrix of its own.

use gpui::{
    App, Bounds, Hsla, Pixels, Point, ScaledPixels, TransformationMatrix, Window, point, px,
    radians, rgb, size,
};

use super::{
    WIDTH,
    assets::{self, Bone},
    look::{Eyes, Pose},
};

/// A point or a shift in the window's device pixels.
type Device = (f32, f32);

/// Ely's loaded layers, with the view box they draw in and the head's pivot.
pub struct Rig {
    view: [f32; 4],
    head: (f32, f32),
    layers: Vec<assets::Layer>,
}

impl Rig {
    /// Loads and checks the embedded assets; an error names the resource and field.
    pub fn load() -> anyhow::Result<Self> {
        let assets = super::assets::load()?;
        Ok(Self {
            view: assets.view,
            head: assets.head,
            layers: assets.layers,
        })
    }

    /// Draws her in `pose`, the head's box from `origin`.
    pub fn paint(&self, origin: Point<Pixels>, pose: &Pose, window: &mut Window, cx: &App) {
        let scale = window.scale_factor();
        let unit = WIDTH / self.view[2];
        let (left, top) = (f32::from(origin.x), f32::from(origin.y));
        let at = |(x, y): (f32, f32)| {
            let x = (left + (x - self.view[0]) * unit) * scale;
            (x, (top + (y - self.view[1]) * unit) * scale)
        };
        let by = |(x, y): (f32, f32)| (x * unit * scale, y * unit * scale);
        let whole = shift(TransformationMatrix::unit(), by((pose.shake, -pose.lift)));
        let breath = shift(whole, by((0.0, -pose.bob)));
        let head = turn(breath, at(self.head), pose.tilt);
        for layer in &self.layers {
            let Some((matrix, alpha)) = moved(layer, pose, [whole, head], &at, &by) else {
                continue;
            };
            let [x, y, w, h] = layer.frame;
            let bounds = Bounds::new(
                origin + point(px((x - self.view[0]) * unit), px((y - self.view[1]) * unit)),
                size(px(w * unit), px(h * unit)),
            );
            let color = Hsla::from(rgb(layer.color)).opacity(layer.alpha * alpha);
            let drawn = window.paint_svg(
                bounds,
                layer.key.clone(),
                Some(layer.svg.as_slice()),
                matrix,
                color,
                cx,
            );
            if let Err(error) = drawn {
                log::error!("pet: Ely's {} does not draw: {error:#}", layer.name);
            }
        }
    }
}

/// Where `layer` stands in `pose`, and its opacity; none while the pose hides it.
fn moved(
    layer: &assets::Layer,
    pose: &Pose,
    [whole, head]: [TransformationMatrix; 2],
    at: &impl Fn((f32, f32)) -> Device,
    by: &impl Fn((f32, f32)) -> Device,
) -> Option<(TransformationMatrix, f32)> {
    let pivot = at(layer.pivot);
    let open = |side: usize| pose.eyes == Eyes::Open || (pose.eyes == Eyes::Wink && side == 1);
    let shut = |side: usize| pose.eyes == Eyes::Arcs || (pose.eyes == Eyes::Wink && side == 0);
    let squash = |side: usize, wide: f32| grow(head, pivot, (wide, wide * pose.open[side]));
    Some(match layer.bone {
        Bone::Body => (whole, 1.0),
        Bone::Head => (head, 1.0),
        Bone::Lid(side) if open(side) => (squash(side, pose.wide), 1.0),
        Bone::Iris(side) if open(side) => (shift(squash(side, pose.wide), by(pose.look)), 1.0),
        Bone::Arc(side) if shut(side) => (squash(side, 1.0), 1.0),
        Bone::Lines if pose.eyes == Eyes::Lines => (head, 1.0),
        Bone::Brows if pose.brows > 0.0 => (head, pose.brows),
        Bone::Blush => (head, 1.0 + 0.6 * pose.flush),
        Bone::Mouth(mouth) if mouth == pose.mouth => (grow(head, pivot, (pose.pop, pose.pop)), 1.0),
        Bone::Lock(side) => {
            let sway = if side == 0 { pose.sway } else { -pose.sway };
            (turn(head, pivot, sway), 1.0)
        }
        Bone::Ahoge => (turn(head, pivot, pose.ahoge), 1.0),
        Bone::Hand(side) => (shift(whole, by((0.0, pose.hands[side]))), 1.0),
        Bone::Mark(mark) => {
            let &(_, motion) = pose.marks.iter().find(|(shown, _)| *shown == mark)?;
            let moved = shift(head, by(motion.at));
            let turned = turn(moved, pivot, motion.turn);
            (
                grow(turned, pivot, (motion.scale, motion.scale)),
                motion.alpha,
            )
        }
        _ => return None,
    })
}

fn shift(matrix: TransformationMatrix, (x, y): Device) -> TransformationMatrix {
    matrix.translate(point(ScaledPixels(x), ScaledPixels(y)))
}

/// `matrix`, after a turn of `degrees` clockwise about `pivot`.
fn turn(matrix: TransformationMatrix, pivot: Device, degrees: f32) -> TransformationMatrix {
    let turned = shift(matrix, pivot).rotate(radians(degrees.to_radians()));
    shift(turned, (-pivot.0, -pivot.1))
}

/// `matrix`, after a growth by `(x, y)` about `pivot`.
fn grow(matrix: TransformationMatrix, pivot: Device, (x, y): (f32, f32)) -> TransformationMatrix {
    shift(shift(matrix, pivot).scale(size(x, y)), (-pivot.0, -pivot.1))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use resvg::usvg;

    use super::*;

    #[test]
    fn every_layer_parses_and_stays_in_its_box() {
        let rig = Rig::load().expect("ely's assets load");
        let mut names = HashSet::new();
        for layer in &rig.layers {
            assert!(names.insert(layer.name.as_str()), "{} twice", layer.name);
            let tree = usvg::Tree::from_str(
                std::str::from_utf8(&layer.svg).expect("svg is utf-8"),
                &usvg::Options::default(),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", layer.name));
            let drawn = tree.root().abs_stroke_bounding_box();
            let [_, _, width, height] = layer.frame;
            assert!(
                drawn.left() >= 0.0
                    && drawn.top() >= 0.0
                    && drawn.right() <= width
                    && drawn.bottom() <= height,
                "{}: {drawn:?} spills out of {width} by {height}",
                layer.name
            );
        }
    }

    #[test]
    fn her_shirt_and_hair_run_below_the_edge_at_her_highest_hop() {
        use super::super::look::{Mood, pose};
        let rig = Rig::load().expect("ely's assets load");
        let moods = [
            Mood::Calm,
            Mood::Done,
            Mood::Working,
            Mood::Error,
            Mood::Waiting,
        ];
        let mut highest = 0.0_f32;
        for mood in moods {
            for held in (0..130).map(|step| f64::from(step) * 0.02) {
                for petted in [None]
                    .into_iter()
                    .chain((0..31).map(|step| Some(f64::from(step) * 0.02)))
                {
                    let pose = pose(mood, held, 10.0, petted);
                    highest = highest.max(pose.lift + pose.bob);
                }
            }
        }
        assert!(
            highest > 40.0,
            "a press inside a hop lifts her {highest} units"
        );
        let edge = rig.view[1] + rig.view[3];
        for name in ["collar", "hair-back"] {
            let layer = rig
                .layers
                .iter()
                .find(|layer| layer.name == name)
                .expect(name);
            let bottom = layer.frame[1] + layer.frame[3] - highest;
            assert!(
                bottom > edge,
                "{name} ends at {bottom}, above the edge at {edge}"
            );
        }
    }

    #[test]
    fn she_is_drawn_from_every_part_the_icon_gives_her() {
        use crate::pet::look::Mark;
        let rig = Rig::load().expect("ely's assets load");
        let named = |name: &str| rig.layers.iter().any(|layer| layer.name == name);
        for name in [
            "hair-back",
            "skin",
            "bangs",
            "ahoge",
            "lock-l",
            "lock-r",
            "hand-l",
            "hand-r",
        ] {
            assert!(named(name), "{name}");
        }
        let marks = [
            Mark::Bang,
            Mark::Z1,
            Mark::Z2,
            Mark::Heart,
            Mark::SparkleLeft,
            Mark::SparkleRight,
            Mark::Drop,
        ];
        for mark in marks {
            assert!(
                rig.layers
                    .iter()
                    .any(|layer| layer.bone == Bone::Mark(mark)),
                "{mark:?}"
            );
        }
    }
}
