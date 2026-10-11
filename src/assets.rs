use serde::Deserialize;

pub const BREATH_AMPLITUDE: f32 = 1.3;
const ATLAS: &[u8] = include_bytes!("../assets/firefly/atlas.png");
const GAZE_ATLAS: &[u8] = include_bytes!("../assets/firefly/gaze.png");

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Sprite {
    pub sx: i32,
    pub sy: i32,
    pub sw: i32,
    pub sh: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
#[derive(Deserialize)]
pub struct Character {
    pub version: u8,
    pub atlas_size: [i32; 2],
    pub body: Sprite,
    pub left_hand: Sprite,
    pub right_hand: Sprite,
    pub keyboard: Sprite,
    pub faces: Vec<Sprite>,
    pub expressions: Vec<String>,
    pub anchors: Anchors,
    pub gaze: GazeAssets,
}
#[derive(Clone, Deserialize)]
pub struct GazeAssets {
    pub atlas_size: [i32; 2],
    pub iris_crops: [[i32; 4]; 2],
    pub sclera_crops: [[i32; 4]; 2],
    pub eye_center: [f32; 2],
    pub expressions: Vec<GazeExpression>,
}
#[derive(Clone, Deserialize)]
pub struct GazeExpression {
    pub face: usize,
    pub max_offset: [f32; 2],
    pub eyes: [Eye; 2],
}
#[derive(Clone, Deserialize)]
pub struct Eye {
    pub iris_rect: [f32; 4],
    pub mask: Vec<[f32; 2]>,
}
#[derive(Deserialize)]
pub struct Anchors {
    pub label_y: f32,
    pub left_tap_dy: f32,
    pub right_tap_dy: f32,
}
pub fn firefly() -> Result<Character, String> {
    let c: Character = serde_json::from_str(include_str!("../assets/firefly/character.json"))
        .map_err(|e| e.to_string())?;
    validate(&c, ATLAS)?;
    validate_gaze(&c, GAZE_ATLAS)?;
    Ok(c)
}

fn validate_gaze(c: &Character, atlas: &[u8]) -> Result<(), String> {
    let g = &c.gaze;
    let invalid = || "眼部图集、遮罩或移动范围无效".to_string();
    if atlas.len() < 33
        || &atlas[..8] != b"\x89PNG\r\n\x1a\n"
        || atlas[8..12] != 13_u32.to_be_bytes()
        || &atlas[12..16] != b"IHDR"
        || atlas[24] != 8
        || atlas[25] != 6
    {
        return Err(invalid());
    }
    for (offset, size) in [(16, g.atlas_size[0]), (20, g.atlas_size[1])] {
        if size <= 0
            || size > 4096
            || u32::from_be_bytes(atlas[offset..offset + 4].try_into().unwrap()) != size as u32
        {
            return Err(invalid());
        }
    }
    for [x, y, w, h] in g.iris_crops.iter().chain(&g.sclera_crops).copied() {
        if x < 0
            || y < 0
            || w <= 0
            || h <= 0
            || x.checked_add(w).is_none_or(|v| v > g.atlas_size[0])
            || y.checked_add(h).is_none_or(|v| v > g.atlas_size[1])
        {
            return Err(invalid());
        }
    }
    if !g.eye_center[0].is_finite()
        || !g.eye_center[1].is_finite()
        || !(0.0..=300.).contains(&g.eye_center[0])
        || !(0.0..=320.).contains(&g.eye_center[1])
        || g.expressions.iter().map(|e| e.face).collect::<Vec<_>>() != [0, 2, 4, 5]
    {
        return Err(invalid());
    }
    for e in &g.expressions {
        let face = c.faces.get(e.face).ok_or_else(invalid)?;
        if e.max_offset != [3., if e.face == 4 { 1. } else { 2. }] {
            return Err(invalid());
        }
        for eye in &e.eyes {
            let [x, y, w, h] = eye.iris_rect;
            if !eye.iris_rect.into_iter().all(f32::is_finite)
                || w <= 0.
                || h <= 0.
                || x < 0.
                || y < 0.
                || x + w > face.sw as f32
                || y + h > face.sh as f32
                || !(3..=64).contains(&eye.mask.len())
            {
                return Err(invalid());
            }
            let mut area = 0.;
            for (index, p) in eye.mask.iter().enumerate() {
                if !p.iter().all(|v| v.is_finite())
                    || p[0] < 0.
                    || p[1] < 0.
                    || p[0] > face.sw as f32
                    || p[1] > face.sh as f32
                {
                    return Err(invalid());
                }
                let next = eye.mask[(index + 1) % eye.mask.len()];
                area += p[0] * next[1] - next[0] * p[1];
            }
            if area.abs() < 1. {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

fn validate(c: &Character, atlas: &[u8]) -> Result<(), String> {
    // 读取标准 PNG 头即可核对图集尺寸；像素解码仍由 Slint 完成。
    if atlas.len() < 33
        || &atlas[..8] != b"\x89PNG\r\n\x1a\n"
        || atlas[8..12] != 13_u32.to_be_bytes()
        || &atlas[12..16] != b"IHDR"
        || atlas[25] != 6
    {
        return Err("角色图集必须是带透明通道的 RGBA PNG".into());
    }
    for (offset, declared) in [(16, c.atlas_size[0]), (20, c.atlas_size[1])] {
        let actual = u32::from_be_bytes(atlas[offset..offset + 4].try_into().unwrap());
        if declared <= 0 || actual != declared as u32 {
            return Err("角色图集实际尺寸与清单不一致".into());
        }
    }
    if c.version != 1
        || c.faces.len() != 6
        || c.expressions != ["idle", "blink", "focused", "happy", "sleepy", "surprised"]
    {
        return Err("角色清单版本或表情数量无效".into());
    }
    if ![
        c.anchors.label_y,
        c.anchors.left_tap_dy,
        c.anchors.right_tap_dy,
    ]
    .into_iter()
    .all(|v| v.is_finite() && (0.0..=320.0).contains(&v))
    {
        return Err("角色锚点无效".into());
    }
    if c.anchors.label_y + 24. > 320. {
        return Err("按键气泡超出舞台".into());
    }
    for (s, breath, tap) in [
        (&c.body, BREATH_AMPLITUDE, 0.),
        (&c.left_hand, BREATH_AMPLITUDE, c.anchors.left_tap_dy),
        (&c.right_hand, BREATH_AMPLITUDE, c.anchors.right_tap_dy),
        (&c.keyboard, 0., 0.),
    ]
    .into_iter()
    .chain(c.faces.iter().map(|s| (s, BREATH_AMPLITUDE, 0.)))
    {
        if s.sx < 0
            || s.sy < 0
            || s.sw <= 0
            || s.sh <= 0
            || s.sx
                .checked_add(s.sw)
                .is_none_or(|end| end > c.atlas_size[0])
            || s.sy
                .checked_add(s.sh)
                .is_none_or(|end| end > c.atlas_size[1])
            || ![s.x, s.y, s.width, s.height]
                .into_iter()
                .all(f32::is_finite)
            || s.width <= 0.
            || s.height <= 0.
            || s.x < 0.
            || s.y - breath < 0.
            || s.x + s.width > 300.
            || s.y + s.height + breath + tap > 320.
        {
            return Err("角色图层区域无效".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_is_valid() {
        assert!(super::firefly().is_ok());
    }

    #[test]
    fn atlas_header_and_actual_size_are_checked() {
        let mut c = firefly().unwrap();
        for bytes in [&[][..], &ATLAS[..24], b"not a png".as_slice()] {
            assert!(validate(&c, bytes).is_err());
        }
        let mut rgb = ATLAS.to_vec();
        rgb[25] = 2;
        assert!(validate(&c, &rgb).is_err());
        c.atlas_size[0] += 1;
        assert!(validate(&c, ATLAS).is_err());
    }

    #[test]
    fn source_crop_overflow_and_invalid_geometry_are_rejected() {
        let mut c = firefly().unwrap();
        c.body.sx = i32::MAX;
        assert!(validate(&c, ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.keyboard.sy = c.atlas_size[1];
        assert!(validate(&c, ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.body.width = f32::NAN;
        assert!(validate(&c, ATLAS).is_err());
    }

    #[test]
    fn animation_extents_and_label_stay_inside_stage() {
        let mut c = firefly().unwrap();
        c.body.y = BREATH_AMPLITUDE - 0.1;
        assert!(validate(&c, ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.left_hand.y = 320. - c.left_hand.height - BREATH_AMPLITUDE;
        assert!(validate(&c, ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.anchors.label_y = 297.;
        assert!(validate(&c, ATLAS).is_err());
    }

    #[test]
    fn expression_order_and_count_are_checked() {
        let mut c = firefly().unwrap();
        c.expressions.swap(0, 1);
        assert!(validate(&c, ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.faces.pop();
        assert!(validate(&c, ATLAS).is_err());
    }

    #[test]
    fn bundled_layers_preserve_aspect_and_face_registration() {
        let c = firefly().unwrap();
        for s in [&c.body, &c.left_hand, &c.right_hand, &c.keyboard]
            .into_iter()
            .chain(c.faces.iter())
        {
            assert!((s.width / s.height - s.sw as f32 / s.sh as f32).abs() < 0.001);
        }
        let first = &c.faces[0];
        for face in &c.faces {
            assert_eq!((face.sw, face.sh), (first.sw, first.sh));
            assert_eq!(
                (face.x, face.y, face.width, face.height),
                (first.x, first.y, first.width, first.height)
            );
            assert!(face.y + face.height + BREATH_AMPLITUDE < c.anchors.label_y);
        }
        assert!(c.body.y - BREATH_AMPLITUDE >= 60.);
    }

    #[test]
    fn gaze_atlas_geometry_expression_mapping_and_limits_are_checked() {
        let mut c = firefly().unwrap();
        assert!(validate_gaze(&c, &GAZE_ATLAS[..24]).is_err());
        c.gaze.atlas_size[0] += 1;
        assert!(validate_gaze(&c, GAZE_ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.gaze.iris_crops[0][0] = i32::MAX;
        assert!(validate_gaze(&c, GAZE_ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.gaze.expressions[0].face = 1;
        assert!(validate_gaze(&c, GAZE_ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.gaze.expressions[2].max_offset[1] = 2.;
        assert!(validate_gaze(&c, GAZE_ATLAS).is_err());
        let mut c = firefly().unwrap();
        c.gaze.eye_center[0] = f32::NAN;
        assert!(validate_gaze(&c, GAZE_ATLAS).is_err());
        for mask in [
            vec![],
            vec![[1., 1.]; 3],
            vec![[0., 0.], [301., 0.], [0., 10.]],
            vec![[0., 0.], [f32::NAN, 1.], [0., 10.]],
        ] {
            let mut c = firefly().unwrap();
            c.gaze.expressions[0].eyes[0].mask = mask;
            assert!(validate_gaze(&c, GAZE_ATLAS).is_err());
        }
        for value in [0., -1., f32::INFINITY] {
            let mut c = firefly().unwrap();
            c.gaze.expressions[0].eyes[0].iris_rect[2] = value;
            assert!(validate_gaze(&c, GAZE_ATLAS).is_err());
        }
    }
}
