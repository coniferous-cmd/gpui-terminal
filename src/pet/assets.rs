//! Ely's built-in rig description and artwork, embedded at compile time and checked once at load.

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use anyhow::{Context as _, Result, bail};
use gpui::SharedString;
use serde::Deserialize;

use super::look::{Mark, Mouth};

/// How a layer moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bone {
    Body,
    Head,
    Lid(usize),
    Iris(usize),
    Arc(usize),
    Lines,
    Brows,
    Blush,
    Mouth(Mouth),
    Lock(usize),
    Ahoge,
    Hand(usize),
    Mark(Mark),
}

/// One checked layer, with its cache key and embedded full SVG.
#[derive(Debug)]
pub struct Layer {
    pub name: String,
    pub color: u32,
    pub alpha: f32,
    pub frame: [f32; 4],
    pub pivot: (f32, f32),
    pub bone: Bone,
    /// gpui's paint_svg cache key: `ely/<name>.svg`.
    pub key: SharedString,
    pub svg: Vec<u8>,
}

/// The whole checked description.
#[derive(Debug)]
pub struct Assets {
    pub view: [f32; 4],
    pub head: (f32, f32),
    pub layers: Vec<Layer>,
}

/// Parses and checks the embedded description; an error names the resource, layer and field.
pub fn load() -> Result<Assets> {
    parse(RIG)
}

const WHERE: &str = "pet: assets/pet/ely/rig.json";

fn parse(text: &str) -> Result<Assets> {
    let mut reader = serde_json::Deserializer::from_str(text);
    let description: Description =
        serde_path_to_error::deserialize(&mut reader).map_err(|error| {
            anyhow::anyhow!(
                "{WHERE} does not parse at `{}`: {}",
                error.path(),
                error.inner()
            )
        })?;
    reader
        .end()
        .with_context(|| format!("{WHERE} holds more than one value"))?;

    for value in description.view {
        if !value.is_finite() {
            bail!("{WHERE}: view must hold finite values");
        }
    }
    if description.view[2] <= 0.0 {
        bail!(
            "{WHERE}: view width must be positive, got {}",
            description.view[2]
        );
    }
    if description.view[3] <= 0.0 {
        bail!(
            "{WHERE}: view height must be positive, got {}",
            description.view[3]
        );
    }
    for value in description.head_pivot {
        if !value.is_finite() {
            bail!("{WHERE}: head_pivot must hold finite values");
        }
    }

    let mut names = HashSet::new();
    let mut layers = Vec::with_capacity(description.layers.len());
    for (i, layer) in description.layers.into_iter().enumerate() {
        let LayerDesc {
            name,
            asset,
            color,
            alpha,
            frame,
            pivot,
            binding,
        } = layer;
        if name.is_empty() {
            bail!("{WHERE}: layer {i}: name must not be empty");
        }
        if !names.insert(name.clone()) {
            bail!("{WHERE}: layer {i}: name `{name}` is used twice");
        }
        for value in frame {
            if !value.is_finite() {
                bail!("{WHERE}: layer {i} (`{name}`): frame must hold finite values");
            }
        }
        if frame[2] <= 0.0 {
            bail!(
                "{WHERE}: layer {i} (`{name}`): frame width must be positive, got {}",
                frame[2]
            );
        }
        if frame[3] <= 0.0 {
            bail!(
                "{WHERE}: layer {i} (`{name}`): frame height must be positive, got {}",
                frame[3]
            );
        }
        for value in pivot {
            if !value.is_finite() {
                bail!("{WHERE}: layer {i} (`{name}`): pivot must hold finite values");
            }
        }
        if !(0.0..=1.0).contains(&alpha) {
            bail!("{WHERE}: layer {i} (`{name}`): alpha {alpha} is outside 0 to 1");
        }
        let color = if color.len() == 6 && color.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            u32::from_str_radix(&color, 16).expect("six hex digits parse")
        } else {
            bail!("{WHERE}: layer {i} (`{name}`): color `{color}` must be six hex digits");
        };
        let (_, bytes) = match ART.iter().find(|(path, _)| *path == asset) {
            Some(entry) => entry,
            None => bail!("{WHERE}: layer {i} (`{name}`): no embedded {asset}"),
        };
        layers.push(Layer {
            name: name.clone(),
            color,
            alpha,
            frame,
            pivot: (pivot[0], pivot[1]),
            bone: bone(binding),
            key: format!("ely/{name}.svg").into(),
            svg: bytes.as_bytes().to_vec(),
        });
    }
    Ok(Assets {
        view: description.view,
        head: (description.head_pivot[0], description.head_pivot[1]),
        layers,
    })
}

fn bone(binding: Binding) -> Bone {
    match binding {
        Binding::Body => Bone::Body,
        Binding::Head => Bone::Head,
        Binding::Lid { side } => Bone::Lid(side.0),
        Binding::Iris { side } => Bone::Iris(side.0),
        Binding::Arc { side } => Bone::Arc(side.0),
        Binding::Lines => Bone::Lines,
        Binding::Brows => Bone::Brows,
        Binding::Blush => Bone::Blush,
        Binding::Mouth { mouth } => Bone::Mouth(match mouth {
            MouthKind::Smile => Mouth::Smile,
            MouthKind::Grin => Mouth::Grin,
            MouthKind::Gasp => Mouth::Gasp,
            MouthKind::Flat => Mouth::Flat,
            MouthKind::Frown => Mouth::Frown,
        }),
        Binding::Lock { side } => Bone::Lock(side.0),
        Binding::Ahoge => Bone::Ahoge,
        Binding::Hand { side } => Bone::Hand(side.0),
        Binding::Mark { mark } => Bone::Mark(match mark {
            MarkKind::Bang => Mark::Bang,
            MarkKind::Z1 => Mark::Z1,
            MarkKind::Z2 => Mark::Z2,
            MarkKind::Heart => Mark::Heart,
            MarkKind::SparkleLeft => Mark::SparkleLeft,
            MarkKind::SparkleRight => Mark::SparkleRight,
            MarkKind::Drop => Mark::Drop,
        }),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Description {
    view: [f32; 4],
    head_pivot: [f32; 2],
    layers: Vec<LayerDesc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayerDesc {
    name: String,
    asset: String,
    color: String,
    alpha: f32,
    frame: [f32; 4],
    pivot: [f32; 2],
    binding: Binding,
}

#[derive(Deserialize)]
#[serde(tag = "bone", rename_all = "snake_case", deny_unknown_fields)]
enum Binding {
    Body,
    Head,
    Lid { side: Side },
    Iris { side: Side },
    Arc { side: Side },
    Lines,
    Brows,
    Blush,
    Mouth { mouth: MouthKind },
    Lock { side: Side },
    Ahoge,
    Hand { side: Side },
    Mark { mark: MarkKind },
}

/// A left or right index: 0 or 1, nothing else.
#[derive(Clone, Copy, Deserialize)]
#[serde(try_from = "u8")]
struct Side(usize);

impl TryFrom<u8> for Side {
    type Error = String;
    fn try_from(side: u8) -> Result<Self, String> {
        if side < 2 {
            Ok(Self(side as usize))
        } else {
            Err(format!("side must be 0 or 1, got {side}"))
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum MouthKind {
    Smile,
    Grin,
    Gasp,
    Flat,
    Frown,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum MarkKind {
    Bang,
    Z1,
    Z2,
    Heart,
    SparkleLeft,
    SparkleRight,
    Drop,
}

/// The description itself, `assets/pet/ely/rig.json`.
const RIG: &str = include_str!("../../assets/pet/ely/rig.json");
/// Every file under `assets/pet/ely/`, its path relative to that folder and its bytes.
const ART: &[(&str, &str)] = &[
    (
        "layers/ahoge.svg",
        include_str!("../../assets/pet/ely/layers/ahoge.svg"),
    ),
    (
        "layers/arc-l.svg",
        include_str!("../../assets/pet/ely/layers/arc-l.svg"),
    ),
    (
        "layers/arc-r.svg",
        include_str!("../../assets/pet/ely/layers/arc-r.svg"),
    ),
    (
        "layers/bang-edge.svg",
        include_str!("../../assets/pet/ely/layers/bang-edge.svg"),
    ),
    (
        "layers/bang.svg",
        include_str!("../../assets/pet/ely/layers/bang.svg"),
    ),
    (
        "layers/bangs.svg",
        include_str!("../../assets/pet/ely/layers/bangs.svg"),
    ),
    (
        "layers/blush.svg",
        include_str!("../../assets/pet/ely/layers/blush.svg"),
    ),
    (
        "layers/bow.svg",
        include_str!("../../assets/pet/ely/layers/bow.svg"),
    ),
    (
        "layers/brows.svg",
        include_str!("../../assets/pet/ely/layers/brows.svg"),
    ),
    (
        "layers/collar.svg",
        include_str!("../../assets/pet/ely/layers/collar.svg"),
    ),
    (
        "layers/drop-edge.svg",
        include_str!("../../assets/pet/ely/layers/drop-edge.svg"),
    ),
    (
        "layers/drop.svg",
        include_str!("../../assets/pet/ely/layers/drop.svg"),
    ),
    (
        "layers/face-shade.svg",
        include_str!("../../assets/pet/ely/layers/face-shade.svg"),
    ),
    (
        "layers/flat.svg",
        include_str!("../../assets/pet/ely/layers/flat.svg"),
    ),
    (
        "layers/frown.svg",
        include_str!("../../assets/pet/ely/layers/frown.svg"),
    ),
    (
        "layers/gasp.svg",
        include_str!("../../assets/pet/ely/layers/gasp.svg"),
    ),
    (
        "layers/gloss.svg",
        include_str!("../../assets/pet/ely/layers/gloss.svg"),
    ),
    (
        "layers/grin.svg",
        include_str!("../../assets/pet/ely/layers/grin.svg"),
    ),
    (
        "layers/hair-back.svg",
        include_str!("../../assets/pet/ely/layers/hair-back.svg"),
    ),
    (
        "layers/hand-l.svg",
        include_str!("../../assets/pet/ely/layers/hand-l.svg"),
    ),
    (
        "layers/hand-r.svg",
        include_str!("../../assets/pet/ely/layers/hand-r.svg"),
    ),
    (
        "layers/heart.svg",
        include_str!("../../assets/pet/ely/layers/heart.svg"),
    ),
    (
        "layers/iris-l.svg",
        include_str!("../../assets/pet/ely/layers/iris-l.svg"),
    ),
    (
        "layers/iris-r.svg",
        include_str!("../../assets/pet/ely/layers/iris-r.svg"),
    ),
    (
        "layers/knot.svg",
        include_str!("../../assets/pet/ely/layers/knot.svg"),
    ),
    (
        "layers/knuckles-l.svg",
        include_str!("../../assets/pet/ely/layers/knuckles-l.svg"),
    ),
    (
        "layers/knuckles-r.svg",
        include_str!("../../assets/pet/ely/layers/knuckles-r.svg"),
    ),
    (
        "layers/lash-l.svg",
        include_str!("../../assets/pet/ely/layers/lash-l.svg"),
    ),
    (
        "layers/lash-r.svg",
        include_str!("../../assets/pet/ely/layers/lash-r.svg"),
    ),
    (
        "layers/lid-l.svg",
        include_str!("../../assets/pet/ely/layers/lid-l.svg"),
    ),
    (
        "layers/lid-r.svg",
        include_str!("../../assets/pet/ely/layers/lid-r.svg"),
    ),
    (
        "layers/lines.svg",
        include_str!("../../assets/pet/ely/layers/lines.svg"),
    ),
    (
        "layers/lock-l.svg",
        include_str!("../../assets/pet/ely/layers/lock-l.svg"),
    ),
    (
        "layers/lock-r.svg",
        include_str!("../../assets/pet/ely/layers/lock-r.svg"),
    ),
    (
        "layers/neck.svg",
        include_str!("../../assets/pet/ely/layers/neck.svg"),
    ),
    (
        "layers/pin.svg",
        include_str!("../../assets/pet/ely/layers/pin.svg"),
    ),
    (
        "layers/pupil-l.svg",
        include_str!("../../assets/pet/ely/layers/pupil-l.svg"),
    ),
    (
        "layers/pupil-r.svg",
        include_str!("../../assets/pet/ely/layers/pupil-r.svg"),
    ),
    (
        "layers/ring-l.svg",
        include_str!("../../assets/pet/ely/layers/ring-l.svg"),
    ),
    (
        "layers/ring-r.svg",
        include_str!("../../assets/pet/ely/layers/ring-r.svg"),
    ),
    (
        "layers/shade-back.svg",
        include_str!("../../assets/pet/ely/layers/shade-back.svg"),
    ),
    (
        "layers/shine-l.svg",
        include_str!("../../assets/pet/ely/layers/shine-l.svg"),
    ),
    (
        "layers/shine-r.svg",
        include_str!("../../assets/pet/ely/layers/shine-r.svg"),
    ),
    (
        "layers/skin.svg",
        include_str!("../../assets/pet/ely/layers/skin.svg"),
    ),
    (
        "layers/smile.svg",
        include_str!("../../assets/pet/ely/layers/smile.svg"),
    ),
    (
        "layers/sparkle-l.svg",
        include_str!("../../assets/pet/ely/layers/sparkle-l.svg"),
    ),
    (
        "layers/sparkle-r.svg",
        include_str!("../../assets/pet/ely/layers/sparkle-r.svg"),
    ),
    (
        "layers/tongue.svg",
        include_str!("../../assets/pet/ely/layers/tongue.svg"),
    ),
    (
        "layers/z1.svg",
        include_str!("../../assets/pet/ely/layers/z1.svg"),
    ),
    (
        "layers/z2.svg",
        include_str!("../../assets/pet/ely/layers/z2.svg"),
    ),
];
