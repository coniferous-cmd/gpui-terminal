//! Ely rigged as the films rig her: the icon's paths in layers of one color, each a mask gpui draws with a matrix of its own.

use gpui::{
    App, Bounds, Hsla, Pixels, Point, ScaledPixels, SharedString, TransformationMatrix, Window,
    point, px, radians, rgb, size,
};

use super::{
    WIDTH,
    look::{Eyes, Mark, Mouth, Pose},
};

/// A point or a shift in the window's device pixels.
type Device = (f32, f32);

/// The head's box in the icon's units; the edge she holds runs along its bottom.
const VIEW: [f32; 4] = [110.0, 6.0, 292.0, 280.0];
/// Points a unit draws at.
const UNIT: f32 = WIDTH / VIEW[2];

/// How a layer moves.
#[derive(Clone, Copy, PartialEq)]
enum Bone {
    /// Her neck and collar, which stay on the edge as her head bobs.
    Body,
    /// Bobs, and turns with her head.
    Head,
    /// An open eye, which blinks by squashing and looks about as well; left is 0.
    Iris(usize),
    Arc(usize),
    Lines,
    Brows,
    Blush,
    Mouth(Mouth),
    Lock(usize),
    Ahoge,
    /// Holds the edge; taps.
    Hand(usize),
    Mark(Mark),
}

/// A layer: its name, color and opacity, its box in the icon's units, the point it turns and grows about, how it moves, and its shapes.
struct Part {
    name: &'static str,
    color: u32,
    alpha: f32,
    frame: [f32; 4],
    pivot: (f32, f32),
    bone: Bone,
    shapes: &'static str,
}

/// A part with no pivot of its own.
#[rustfmt::skip]
const fn part(name: &'static str, color: u32, frame: [f32; 4], bone: Bone, shapes: &'static str) -> Part {
    Part { name, color, alpha: 1.0, frame, pivot: (256.0, 236.0), bone, shapes }
}

/// Every layer, back to front.
#[rustfmt::skip]
const PARTS: &[Part] = &[
    part("hair-back", 0x2A252B, [112.0, 42.0, 288.0, 300.0], Bone::Head, r#"<path d="M256 44C184 44 130 94 128 170C126 224 122 272 116 320C114 332 116 340 116 340L396 340C396 340 398 332 396 320C390 272 386 224 384 170C382 94 328 44 256 44Z"/>"#),
    part("neck", 0xEFC0B1, [234.0, 224.0, 44.0, 44.0], Bone::Body, r#"<path d="M236 226L276 226L276 260C268 266 244 266 236 260Z"/>"#),
    part("chest", 0xF6CEBE, [188.0, 258.0, 136.0, 86.0], Bone::Body, r#"<path d="M190 342L194 302C204 278 224 264 256 262C288 264 308 278 318 302L322 342Z"/>"#),
    part("collar", 0x1B1A1F, [198.0, 254.0, 116.0, 90.0], Bone::Body, r#"<path d="M226 256L220 342L238 342L242 256ZM270 256L274 342L292 342L286 256ZM200 342L202 300C218 288 238 284 256 284C274 284 294 288 310 300L312 342Z"/>"#),
    part("skin", 0xF9D3C4, [178.0, 94.0, 156.0, 154.0], Bone::Head, r#"<path d="M180 168C180 118 212 96 256 96C300 96 332 118 332 168C332 206 310 232 282 242C272 245 264 246 256 246C248 246 240 245 230 242C202 232 180 206 180 168Z"/>"#),
    Part { name: "iris-l", color: 0x221A20, alpha: 1.0, frame: [197.0, 172.0, 40.0, 46.0], pivot: (220.0, 194.0), bone: Bone::Iris(0), shapes: r#"<ellipse cx="220" cy="194" rx="15" ry="19"/><path d="M208 178L199 172L209 181Z"/>"# },
    Part { name: "shine-l", color: 0xFFFFFF, alpha: 1.0, frame: [205.0, 176.0, 24.0, 32.0], pivot: (220.0, 194.0), bone: Bone::Iris(0), shapes: r#"<circle cx="212" cy="184" r="5.5"/><circle cx="226" cy="204" r="2.5"/>"# },
    Part { name: "iris-r", color: 0x221A20, alpha: 1.0, frame: [275.0, 172.0, 40.0, 46.0], pivot: (292.0, 194.0), bone: Bone::Iris(1), shapes: r#"<ellipse cx="292" cy="194" rx="15" ry="19"/><path d="M296 178L305 172L295 181Z"/>"# },
    Part { name: "shine-r", color: 0xFFFFFF, alpha: 1.0, frame: [277.0, 176.0, 24.0, 32.0], pivot: (292.0, 194.0), bone: Bone::Iris(1), shapes: r#"<circle cx="284" cy="184" r="5.5"/><circle cx="298" cy="204" r="2.5"/>"# },
    Part { name: "arc-l", color: 0x221A20, alpha: 1.0, frame: [198.0, 170.0, 44.0, 34.0], pivot: (220.0, 194.0), bone: Bone::Arc(0), shapes: r#"<path fill="none" stroke="black" stroke-width="6" stroke-linecap="round" d="M204 198Q220 184 236 198"/>"# },
    Part { name: "arc-r", color: 0x221A20, alpha: 1.0, frame: [274.0, 170.0, 44.0, 34.0], pivot: (292.0, 194.0), bone: Bone::Arc(1), shapes: r#"<path fill="none" stroke="black" stroke-width="6" stroke-linecap="round" d="M278 198Q294 184 310 198"/>"# },
    part("lines", 0x221A20, [200.0, 192.0, 116.0, 8.0], Bone::Lines, r#"<path fill="none" stroke="black" stroke-width="5" stroke-linecap="round" d="M204 196H236M280 196H312"/>"#),
    part("brows", 0x2A252B, [202.0, 162.0, 108.0, 16.0], Bone::Brows, r#"<g fill="none" stroke="black" stroke-width="4" stroke-linecap="round"><path d="M206 172Q218 164 232 167"/><path d="M306 172Q294 164 280 167"/></g>"#),
    part("nose", 0xD1988C, [249.0, 202.0, 16.0, 11.0], Bone::Head, r#"<path fill="none" stroke="black" stroke-width="3" stroke-linecap="round" d="M252 206Q257 211 262 205"/>"#),
    Part { name: "blush", color: 0xFF8FA6, alpha: 0.55, frame: [182.0, 204.0, 148.0, 20.0], pivot: (256.0, 214.0), bone: Bone::Blush, shapes: r#"<ellipse cx="200" cy="214" rx="16" ry="8"/><ellipse cx="312" cy="214" rx="16" ry="8"/>"# },
    mouth("smile", 0x8F4E54, [239.0, 221.0, 28.0, 19.0], Mouth::Smile, r#"<path fill="none" stroke="black" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" d="M242 225Q248 231 253 225Q258 232 264 224"/><path d="M252 230L254 238L257 230Z"/>"#),
    mouth("grin", 0x8F4E54, [240.0, 222.0, 32.0, 24.0], Mouth::Grin, r#"<path d="M242 224Q256 244 270 224Z"/>"#),
    mouth("tongue", 0xFF8FA6, [247.0, 228.0, 18.0, 14.0], Mouth::Grin, r#"<path d="M249 233Q256 240 263 233Q256 229 249 233Z"/>"#),
    mouth("gasp", 0x8F4E54, [249.0, 224.0, 14.0, 16.0], Mouth::Gasp, r#"<ellipse cx="256" cy="232" rx="5.5" ry="7"/>"#),
    mouth("flat", 0x8F4E54, [246.0, 227.0, 20.0, 6.0], Mouth::Flat, r#"<path fill="none" stroke="black" stroke-width="3" stroke-linecap="round" d="M248 230H264"/>"#),
    mouth("frown", 0x8F4E54, [244.0, 223.0, 24.0, 14.0], Mouth::Frown, r#"<path fill="none" stroke="black" stroke-width="3" stroke-linecap="round" d="M246 234Q256 226 266 234"/>"#),
    Part { name: "lock-l", color: 0x2A252B, alpha: 1.0, frame: [154.0, 114.0, 62.0, 208.0], pivot: (202.0, 118.0), bone: Bone::Lock(0), shapes: r#"<path d="M202 116C186 148 176 208 174 260C173 284 166 302 156 320C174 312 188 296 196 278C200 240 204 188 214 148C218 128 212 118 202 116Z"/>"# },
    Part { name: "lock-r", color: 0x2A252B, alpha: 1.0, frame: [296.0, 114.0, 62.0, 208.0], pivot: (310.0, 118.0), bone: Bone::Lock(1), shapes: r#"<path d="M310 116C326 148 336 208 338 260C339 284 346 302 356 320C338 312 324 296 316 278C312 240 308 188 298 148C294 128 300 118 310 116Z"/>"# },
    part("bangs", 0x2A252B, [158.0, 54.0, 196.0, 114.0], Bone::Head, r#"<path d="M168 160C160 100 204 56 256 56C308 56 352 100 344 160C336 140 328 128 320 116C316 136 310 148 302 160C296 138 288 122 280 110C278 132 272 148 266 164C262 140 258 124 256 112C254 124 250 140 246 164C240 148 234 132 232 110C224 122 216 138 210 160C202 148 196 136 192 116C184 128 176 140 168 160Z"/>"#),
    part("gloss", 0x4A424E, [195.0, 69.0, 122.0, 26.0], Bone::Head, r#"<path fill="none" stroke="black" stroke-width="3" stroke-linecap="round" d="M198 92Q218 74 244 72M270 72Q296 76 314 92"/>"#),
    part("petals", 0xF2F7FF, [130.0, 54.0, 76.0, 76.0], Bone::Head, r#"<ellipse cx="168" cy="72" rx="9" ry="16"/><ellipse cx="168" cy="72" rx="9" ry="16" transform="rotate(45 168 92)"/><ellipse cx="168" cy="72" rx="9" ry="16" transform="rotate(90 168 92)"/><ellipse cx="168" cy="72" rx="9" ry="16" transform="rotate(135 168 92)"/><ellipse cx="168" cy="72" rx="9" ry="16" transform="rotate(180 168 92)"/><ellipse cx="168" cy="72" rx="9" ry="16" transform="rotate(225 168 92)"/><ellipse cx="168" cy="72" rx="9" ry="16" transform="rotate(270 168 92)"/><ellipse cx="168" cy="72" rx="9" ry="16" transform="rotate(315 168 92)"/>"#),
    part("flower-center", 0x9FD79A, [157.0, 81.0, 22.0, 22.0], Bone::Head, r#"<circle cx="168" cy="92" r="9"/><path fill="none" stroke="black" stroke-width="2" stroke-linecap="round" d="M163 88H173M161 93H175M164 98H172M166 85V99M171 86V98"/>"#),
    Part { name: "ahoge", color: 0x2A252B, alpha: 1.0, frame: [254.0, 18.0, 44.0, 50.0], pivot: (268.0, 64.0), bone: Bone::Ahoge, shapes: r#"<path d="M264 64C256 42 268 20 294 22C278 30 270 46 272 64Z"/>"# },
    part("hand-l", 0xF9D3C4, [152.0, 260.0, 48.0, 84.0], Bone::Hand(0), r#"<path d="M154 342L154 274C154 265 165 265 165 274C165 263 176 263 176 273C176 263 187 263 187 274C187 266 198 266 198 276L198 342Z"/>"#),
    part("hand-r", 0xF9D3C4, [326.0, 110.0, 72.0, 234.0], Bone::Hand(1), r#"<path d="M336 342L334 210C331 192 334 177 344 169L331 127C328 117 338 112 342 122L353 159L361 123C363 113 374 114 375 125L380 173C390 169 396 180 389 188C397 192 395 202 387 204C381 217 375 227 369 233L367 342Z"/>"#),
    mark("bang", 0xFFB020, [383.0, 28.0, 18.0, 75.0], (392.0, 100.0), Mark::Bang, r#"<path d="M385 30L399 30L394 80L390 80Z"/><circle cx="392" cy="94" r="7"/>"#),
    mark("bang-edge", 0x2A252B, [382.0, 27.0, 19.0, 76.0], (392.0, 100.0), Mark::Bang, r#"<g fill="none" stroke="black" stroke-width="3"><path d="M385 30L399 30L394 80L390 80Z"/><circle cx="392" cy="94" r="7"/></g>"#),
    mark("z1", 0xB8BEEA, [352.0, 48.0, 30.0, 32.0], (367.0, 64.0), Mark::Z1, r#"<path fill="none" stroke="black" stroke-width="5" stroke-linecap="round" stroke-linejoin="round" d="M356 52H378L356 76H378"/>"#),
    mark("z2", 0xB8BEEA, [378.0, 22.0, 22.0, 24.0], (389.0, 34.0), Mark::Z2, r#"<path fill="none" stroke="black" stroke-width="5" stroke-linecap="round" stroke-linejoin="round" d="M382 26H396L382 42H396"/>"#),
    mark("heart", 0xFF6B9D, [346.0, 52.0, 52.0, 44.0], (372.0, 78.0), Mark::Heart, r#"<path d="M372 66C364 54 348 62 354 76C358 86 372 94 372 94C372 94 386 86 390 76C396 62 380 54 372 66Z"/>"#),
    mark("sparkle-l", 0xFFE48A, [116.0, 86.0, 24.0, 28.0], (128.0, 100.0), Mark::SparkleLeft, r#"<path d="M128 88Q130 97 138 100Q130 103 128 112Q126 103 118 100Q126 97 128 88Z"/>"#),
    mark("sparkle-r", 0xFFE48A, [384.0, 48.0, 16.0, 20.0], (392.0, 58.0), Mark::SparkleRight, r#"<path d="M392 50Q394 56 398 58Q394 60 392 66Q390 60 386 58Q390 56 392 50Z"/>"#),
    mark("drop", 0x9CD5FF, [309.0, 112.0, 22.0, 33.0], (319.0, 128.0), Mark::Drop, r#"<path d="M319 114Q329 130 323 139Q313 143 311 133Q311 124 319 114Z"/>"#),
    mark("drop-edge", 0x5AA0D8, [308.0, 111.0, 24.0, 35.0], (319.0, 128.0), Mark::Drop, r#"<path fill="none" stroke="black" stroke-width="2" d="M319 114Q329 130 323 139Q313 143 311 133Q311 124 319 114Z"/>"#),
];

#[rustfmt::skip]
const fn mouth(name: &'static str, color: u32, frame: [f32; 4], mouth: Mouth, shapes: &'static str) -> Part {
    Part { name, color, alpha: 1.0, frame, pivot: (256.0, 228.0), bone: Bone::Mouth(mouth), shapes }
}

#[rustfmt::skip]
const fn mark(name: &'static str, color: u32, frame: [f32; 4], pivot: (f32, f32), mark: Mark, shapes: &'static str) -> Part {
    Part { name, color, alpha: 1.0, frame, pivot, bone: Bone::Mark(mark), shapes }
}

/// Every layer's key and SVG, made once.
pub struct Rig {
    layers: Vec<(&'static Part, SharedString, Vec<u8>)>,
}

impl Rig {
    pub fn new() -> Self {
        let layers = (PARTS.iter())
            .map(|part| {
                let key = format!("ely/{}.svg", part.name).into();
                (part, key, svg(part).into_bytes())
            })
            .collect();
        Self { layers }
    }

    /// Draws her in `pose`, the head's box from `origin`.
    pub fn paint(&self, origin: Point<Pixels>, pose: &Pose, window: &mut Window, cx: &App) {
        let scale = window.scale_factor();
        let (left, top) = (f32::from(origin.x), f32::from(origin.y));
        let at = |(x, y): (f32, f32)| {
            let x = (left + (x - VIEW[0]) * UNIT) * scale;
            (x, (top + (y - VIEW[1]) * UNIT) * scale)
        };
        let by = |(x, y): (f32, f32)| (x * UNIT * scale, y * UNIT * scale);
        let whole = shift(TransformationMatrix::unit(), by((pose.shake, -pose.lift)));
        let breath = shift(whole, by((0.0, -pose.bob)));
        let head = turn(breath, at((256.0, 236.0)), pose.tilt);
        for (part, key, svg) in &self.layers {
            let Some((matrix, alpha)) = moved(part, pose, [whole, head], &at, &by) else {
                continue;
            };
            let [x, y, w, h] = part.frame;
            let bounds = Bounds::new(
                origin + point(px((x - VIEW[0]) * UNIT), px((y - VIEW[1]) * UNIT)),
                size(px(w * UNIT), px(h * UNIT)),
            );
            let color = Hsla::from(rgb(part.color)).opacity(part.alpha * alpha);
            let drawn =
                window.paint_svg(bounds, key.clone(), Some(svg.as_slice()), matrix, color, cx);
            if let Err(error) = drawn {
                log::error!("pet: Ely's {} does not draw: {error:#}", part.name);
            }
        }
    }
}

/// Where `part` stands in `pose`, and its opacity; none while the pose hides it.
fn moved(
    part: &Part,
    pose: &Pose,
    [whole, head]: [TransformationMatrix; 2],
    at: &impl Fn((f32, f32)) -> Device,
    by: &impl Fn((f32, f32)) -> Device,
) -> Option<(TransformationMatrix, f32)> {
    let pivot = at(part.pivot);
    let open = |side: usize| pose.eyes == Eyes::Open || (pose.eyes == Eyes::Wink && side == 1);
    let shut = |side: usize| pose.eyes == Eyes::Arcs || (pose.eyes == Eyes::Wink && side == 0);
    let squash = |side: usize, wide: f32| grow(head, pivot, (wide, wide * pose.open[side]));
    Some(match part.bone {
        Bone::Body => (whole, 1.0),
        Bone::Head => (head, 1.0),
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

/// The SVG of `part`: its shapes in its box.
fn svg(part: &Part) -> String {
    let [x, y, w, h] = part.frame;
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{x} {y} {w} {h}" width="{w}" height="{h}">{}</svg>"#,
        part.shapes
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use resvg::usvg;

    use super::*;

    #[test]
    fn every_layer_parses_and_stays_in_its_box() {
        let mut names = HashSet::new();
        for part in PARTS {
            assert!(names.insert(part.name), "{} twice", part.name);
            let tree = usvg::Tree::from_str(&svg(part), &usvg::Options::default())
                .unwrap_or_else(|error| panic!("{}: {error}", part.name));
            let drawn = tree.root().abs_stroke_bounding_box();
            let [_, _, width, height] = part.frame;
            assert!(
                drawn.left() >= 0.0
                    && drawn.top() >= 0.0
                    && drawn.right() <= width
                    && drawn.bottom() <= height,
                "{}: {drawn:?} spills out of {width} by {height}",
                part.name
            );
        }
    }

    #[test]
    fn her_shirt_and_hair_run_below_the_edge_at_her_highest_hop() {
        use super::super::look::{Mood, pose};
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
        let edge = VIEW[1] + VIEW[3];
        for name in ["collar", "hair-back"] {
            let part = PARTS.iter().find(|part| part.name == name).expect(name);
            let bottom = part.frame[1] + part.frame[3] - highest;
            assert!(
                bottom > edge,
                "{name} ends at {bottom}, above the edge at {edge}"
            );
        }
    }

    #[test]
    fn she_is_drawn_from_every_part_the_icon_gives_her() {
        let named = |name: &str| PARTS.iter().any(|part| part.name == name);
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
                PARTS.iter().any(|part| part.bone == Bone::Mark(mark)),
                "{mark:?}"
            );
        }
    }
}
