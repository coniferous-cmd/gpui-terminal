//! Checks the embedded description and every file it names.

use std::{collections::HashSet, fs, str};

use resvg::usvg;
use serde_json::Value;

use super::ART;

fn rig() -> Value {
    serde_json::from_str(super::RIG).expect("the embedded rig parses")
}

fn parse_fails(mutate: impl FnOnce(&mut Value), expected: &str) {
    let mut value = rig();
    mutate(&mut value);
    let text = serde_json::to_string(&value).expect("the mutated rig encodes");
    let error = super::parse(&text).unwrap_err().to_string();
    assert!(error.contains(expected), "{error}");
}

#[test]
fn the_embedded_rig_loads() {
    let assets = super::load().expect("the embedded rig loads");
    assert_eq!(assets.view, [110.0, 6.0, 292.0, 280.0]);
    assert_eq!(assets.layers.len(), 50);
    assert_eq!(assets.layers.first().unwrap().name, "hair-back");
    assert_eq!(assets.layers.last().unwrap().name, "drop-edge");
    for layer in &assets.layers {
        assert_eq!(layer.key, format!("ely/{}.svg", layer.name));
        assert!(
            str::from_utf8(&layer.svg)
                .expect("svg is UTF-8")
                .starts_with("<svg"),
            "{}",
            layer.name
        );
    }
}

#[test]
fn a_top_level_unknown_field_is_refused() {
    parse_fails(
        |value| {
            value["bogus"] = 1.into();
        },
        "unknown field",
    );
}

#[test]
fn a_layer_unknown_field_is_refused() {
    parse_fails(
        |value| {
            value["layers"][0]["bogus"] = 1.into();
        },
        "unknown field",
    );
}

#[test]
fn an_unknown_bone_is_refused() {
    parse_fails(
        |value| {
            value["layers"][0]["binding"]["bone"] = "tail".into();
        },
        "unknown variant",
    );
}

#[test]
fn an_unknown_mouth_is_refused() {
    parse_fails(
        |value| {
            value["layers"][24]["binding"]["mouth"] = "smirk".into();
        },
        "unknown variant",
    );
}

#[test]
fn an_unknown_mark_is_refused() {
    parse_fails(
        |value| {
            value["layers"][41]["binding"]["mark"] = "star".into();
        },
        "unknown variant",
    );
}

#[test]
fn a_side_must_be_zero_or_one() {
    parse_fails(
        |value| {
            value["layers"][8]["binding"]["side"] = 2.into();
        },
        "side must be 0 or 1",
    );
}

#[test]
fn a_name_used_twice_is_refused() {
    parse_fails(
        |value| {
            let first = value["layers"][0]["name"].clone();
            value["layers"][1]["name"] = first;
        },
        "used twice",
    );
}

#[test]
fn a_missing_asset_is_refused() {
    parse_fails(
        |value| {
            value["layers"][0]["asset"] = "layers/nope.svg".into();
        },
        "no embedded layers/nope.svg",
    );
}

#[test]
fn alpha_outside_the_range_is_refused() {
    parse_fails(
        |value| {
            value["layers"][0]["alpha"] = 1.5.into();
        },
        "outside 0 to 1",
    );
}

#[test]
fn a_color_must_be_six_hex_digits() {
    for color in ["GGHHII", "#EC86B5"] {
        parse_fails(
            |value| {
                value["layers"][0]["color"] = color.into();
            },
            "six hex digits",
        );
    }
}

#[test]
fn a_frame_width_must_be_positive() {
    parse_fails(
        |value| {
            value["layers"][0]["frame"][2] = 0.0.into();
        },
        "frame width must be positive",
    );
}

#[test]
fn view_values_must_be_finite() {
    let text = super::RIG.replacen("292.0", "1e100", 1);
    let error = super::parse(&text).unwrap_err().to_string();
    assert!(error.contains("finite"), "{error}");
}

#[test]
fn every_embedded_svg_parses_in_its_frame() {
    let assets = super::load().expect("the embedded rig loads");
    for layer in &assets.layers {
        let svg = str::from_utf8(&layer.svg).expect("svg is UTF-8");
        usvg::Tree::from_str(svg, &usvg::Options::default())
            .unwrap_or_else(|error| panic!("{}: {error}", layer.name));
        let attribute = svg
            .split_once("viewBox=\"")
            .unwrap_or_else(|| panic!("{}: no viewBox", layer.name))
            .1
            .split_once('"')
            .unwrap_or_else(|| panic!("{}: no viewBox", layer.name))
            .0;
        let mut numbers = attribute.split_whitespace().map(|part| {
            part.parse::<f64>()
                .unwrap_or_else(|error| panic!("{}: viewBox `{attribute}`: {error}", layer.name))
        });
        let view = [
            numbers.next().expect("a viewBox x"),
            numbers.next().expect("a viewBox y"),
            numbers.next().expect("a viewBox width"),
            numbers.next().expect("a viewBox height"),
        ];
        assert!(
            numbers.next().is_none(),
            "{}: viewBox has more than four values",
            layer.name
        );
        let [x, y, width, height] = layer.frame;
        assert!(
            (view[0] - x as f64).abs() < 1e-6
                && (view[1] - y as f64).abs() < 1e-6
                && (view[2] - width as f64).abs() < 1e-6
                && (view[3] - height as f64).abs() < 1e-6,
            "{}: viewBox {view:?} is not frame {:?}",
            layer.name,
            layer.frame
        );
    }
}

#[test]
fn the_index_matches_the_description_and_the_disk() {
    let description = rig();
    let embedded: HashSet<&str> = ART.iter().map(|(path, _)| *path).collect();
    assert_eq!(embedded.len(), ART.len(), "ART holds a path twice");
    for layer in description["layers"]
        .as_array()
        .expect("layers is an array")
    {
        let asset = layer["asset"].as_str().expect("asset is a string");
        assert!(embedded.contains(asset), "{asset}");
    }
    let entries = fs::read_dir("assets/pet/ely/layers").expect("the layers folder is read");
    let mut on_disk: Vec<String> = entries
        .map(|entry| {
            let entry = entry.expect("an entry is read");
            format!(
                "layers/{}",
                entry.file_name().to_str().expect("a UTF-8 file name")
            )
        })
        .collect();
    on_disk.sort();
    let mut keys: Vec<&str> = ART.iter().map(|(path, _)| *path).collect();
    keys.sort();
    assert_eq!(on_disk, keys);
}
