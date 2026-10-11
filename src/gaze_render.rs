use crate::{
    assets::{Character, Eye},
    gaze::Pose,
};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

#[derive(Clone)]
struct MaskPixel {
    index: usize,
    x: f32,
    y: f32,
    coverage: f32,
}
struct EyeLayer {
    iris: SharedPixelBuffer<Rgba8Pixel>,
    rect: [f32; 4],
    mask: Vec<MaskPixel>,
}
struct FaceLayer {
    base: SharedPixelBuffer<Rgba8Pixel>,
    eyes: Vec<EyeLayer>,
    source_offset: [f32; 2],
}

pub struct GazeRenderer {
    faces: Vec<FaceLayer>,
    cache: Option<((usize, i16, i16), Image)>,
}

fn crop(
    source: &SharedPixelBuffer<Rgba8Pixel>,
    rect: [i32; 4],
) -> Result<SharedPixelBuffer<Rgba8Pixel>, String> {
    let [x, y, w, h] = rect;
    if x < 0
        || y < 0
        || w <= 0
        || h <= 0
        || x.checked_add(w).is_none_or(|v| v as u32 > source.width())
        || y.checked_add(h).is_none_or(|v| v as u32 > source.height())
    {
        return Err("眼部素材裁剪超出实际图像".into());
    }
    let mut out = SharedPixelBuffer::new(w as u32, h as u32);
    for row in 0..h as usize {
        let start = (y as usize + row) * source.width() as usize + x as usize;
        out.make_mut_slice()[row * w as usize..(row + 1) * w as usize]
            .copy_from_slice(&source.as_slice()[start..start + w as usize]);
    }
    Ok(out)
}

fn inside(p: [f32; 2], polygon: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &next in polygon {
        if (next[1] > p[1]) != (previous[1] > p[1])
            && p[0] < (previous[0] - next[0]) * (p[1] - next[1]) / (previous[1] - next[1]) + next[0]
        {
            inside = !inside;
        }
        previous = next;
    }
    inside
}

fn mask(eye: &Eye, width: u32, height: u32) -> Vec<MaskPixel> {
    let mut result = Vec::new();
    let bounds = eye
        .mask
        .iter()
        .fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ]
        });
    for y in (bounds[1].floor().max(0.) as u32)..(bounds[3].ceil() as u32).min(height) {
        for x in (bounds[0].floor().max(0.) as u32)..(bounds[2].ceil() as u32).min(width) {
            let mut covered = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    covered += u8::from(inside(
                        [
                            x as f32 + (sx as f32 + 0.5) / 4.,
                            y as f32 + (sy as f32 + 0.5) / 4.,
                        ],
                        &eye.mask,
                    ));
                }
            }
            if covered > 0 {
                result.push(MaskPixel {
                    index: (y * width + x) as usize,
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                    coverage: f32::from(covered) / 16.,
                });
            }
        }
    }
    result
}

fn channels(p: Rgba8Pixel) -> [f32; 4] {
    [p.r as f32, p.g as f32, p.b as f32, p.a as f32]
}
fn pixel(channels: [f32; 4]) -> Rgba8Pixel {
    let [r, g, b, a] = channels.map(|v| v.round().clamp(0., 255.) as u8);
    Rgba8Pixel { r, g, b, a }
}
fn sample(image: &SharedPixelBuffer<Rgba8Pixel>, x: f32, y: f32) -> [f32; 4] {
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let mut out = [0.; 4];
    for (dx, wx) in [(0, 1. - fx), (1, fx)] {
        for (dy, wy) in [(0, 1. - fy), (1, fy)] {
            let (px, py) = (x0 + dx, y0 + dy);
            if px >= 0 && py >= 0 && px < image.width() as i32 && py < image.height() as i32 {
                for (out, value) in out.iter_mut().zip(channels(
                    image.as_slice()[(py as u32 * image.width() + px as u32) as usize],
                )) {
                    *out += value * wx * wy;
                }
            }
        }
    }
    out
}

impl GazeRenderer {
    pub fn new(character: &Character, atlas: Image, gaze_atlas: Image) -> Result<Self, String> {
        // 始终使用预乘透明度插值，避免透明像素中隐藏的颜色污染眼睑边缘。
        let source = atlas.to_rgba8_premultiplied().ok_or("无法解码角色图集")?;
        let eyes = gaze_atlas
            .to_rgba8_premultiplied()
            .ok_or("无法解码眼部图集")?;
        if [source.width() as i32, source.height() as i32] != character.atlas_size
            || [eyes.width() as i32, eyes.height() as i32] != character.gaze.atlas_size
        {
            return Err("眼部合成图像与清单尺寸不一致".into());
        }
        let mut faces = Vec::new();
        for (index, spec) in character.faces.iter().enumerate() {
            let mut base = crop(&source, [spec.sx, spec.sy, spec.sw, spec.sh])?;
            let mut layers = Vec::new();
            let mut source_offset = [0.; 2];
            if let Some(expression) = character.gaze.expressions.iter().find(|e| e.face == index) {
                source_offset = [
                    expression.max_offset[0] * spec.sw as f32 / spec.width,
                    expression.max_offset[1] * spec.sh as f32 / spec.height,
                ];
                let mut occupied = vec![false; (spec.sw * spec.sh) as usize];
                for (side, eye) in expression.eyes.iter().enumerate() {
                    let iris = crop(&eyes, character.gaze.iris_crops[side])?;
                    let white = crop(&eyes, character.gaze.sclera_crops[side])?;
                    if !iris.as_slice().iter().any(|p| p.a > 240)
                        || white.as_slice().iter().any(|p| p.a < 250)
                    {
                        return Err("虹膜必须包含有效像素，眼白取样区必须不透明".into());
                    }
                    let mask = mask(eye, base.width(), base.height());
                    if mask.is_empty() {
                        return Err("眼睛遮罩不能为空".into());
                    }
                    let bounds =
                        eye.mask
                            .iter()
                            .fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, p| {
                                [
                                    b[0].min(p[0]),
                                    b[1].min(p[1]),
                                    b[2].max(p[0]),
                                    b[3].max(p[1]),
                                ]
                            });
                    for p in &mask {
                        if occupied[p.index] {
                            return Err("双眼遮罩不得重叠".into());
                        }
                        occupied[p.index] = true;
                        let wx = ((p.x - bounds[0]) / (bounds[2] - bounds[0])
                            * (white.width() - 1) as f32)
                            .clamp(0., (white.width() - 1) as f32);
                        let wy = ((p.y - bounds[1]) / (bounds[3] - bounds[1])
                            * (white.height() - 1) as f32)
                            .clamp(0., (white.height() - 1) as f32);
                        let sampled = sample(&white, wx, wy);
                        let white = [
                            sampled[0] * 255. / sampled[3],
                            sampled[1] * 255. / sampled[3],
                            sampled[2] * 255. / sampled[3],
                            255.,
                        ];
                        let old = channels(base.as_slice()[p.index]);
                        base.make_mut_slice()[p.index] = pixel(std::array::from_fn(|i| {
                            old[i] * (1. - p.coverage) + white[i] * p.coverage
                        }));
                    }
                    layers.push(EyeLayer {
                        iris,
                        rect: eye.iris_rect,
                        mask,
                    });
                }
            }
            faces.push(FaceLayer {
                base,
                eyes: layers,
                source_offset,
            });
        }
        Ok(Self { faces, cache: None })
    }

    pub fn render(&mut self, expression: usize, pose: Pose) -> Result<Image, String> {
        let face = self.faces.get(expression).ok_or("眼部表情索引无效")?;
        if !pose.x.is_finite() || !pose.y.is_finite() {
            return Err("注视偏移必须为有限数值".into());
        }
        let offsets = [
            pose.x.clamp(-1., 1.) * face.source_offset[0],
            pose.y.clamp(-1., 1.) * face.source_offset[1],
        ];
        // 量化到四分之一源像素后缓存最后一帧；缓存大小不会随运行时间增长。
        let quantize = |offset: f32, maximum: f32| {
            // 先限制整数格数，避免取整把极限偏移推到清单范围以外。
            let limit = (maximum * 4.).floor() as i16;
            ((offset * 4.).round() as i16).clamp(-limit, limit)
        };
        let (dx, dy) = (
            quantize(offsets[0], face.source_offset[0]),
            quantize(offsets[1], face.source_offset[1]),
        );
        let key = (expression, dx, dy);
        if let Some((previous, image)) = &self.cache
            && *previous == key
        {
            return Ok(image.clone());
        }
        let mut out = face.base.clone();
        for eye in &face.eyes {
            for p in &eye.mask {
                let x = (p.x - eye.rect[0] - dx as f32 / 4.) / eye.rect[2]
                    * eye.iris.width() as f32
                    - 0.5;
                let y = (p.y - eye.rect[1] - dy as f32 / 4.) / eye.rect[3]
                    * eye.iris.height() as f32
                    - 0.5;
                let iris = sample(&eye.iris, x, y).map(|c| c * p.coverage);
                let old = channels(out.as_slice()[p.index]);
                out.make_mut_slice()[p.index] = pixel(std::array::from_fn(|i| {
                    iris[i] + old[i] * (1. - iris[3] / 255.)
                }));
            }
        }
        let image = Image::from_rgba8_premultiplied(out);
        self.cache = Some((key, image.clone()));
        Ok(image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> (Character, GazeRenderer, Image) {
        let character = crate::assets::firefly().unwrap();
        let atlas =
            Image::load_from_data(include_bytes!("../assets/firefly/atlas.png"), Some("png"))
                .unwrap();
        let gaze = Image::load_from_data(include_bytes!("../assets/firefly/gaze.png"), Some("png"))
            .unwrap();
        let renderer = GazeRenderer::new(&character, atlas.clone(), gaze).unwrap();
        (character, renderer, atlas)
    }

    #[test]
    fn closed_expressions_remain_original_and_open_eyes_only_change_inside_masks() {
        let (c, mut r, atlas) = renderer();
        let source = atlas.to_rgba8_premultiplied().unwrap();
        for (index, s) in c.faces.iter().enumerate() {
            let original = crop(&source, [s.sx, s.sy, s.sw, s.sh]).unwrap();
            let rendered = r
                .render(
                    index,
                    Pose {
                        x: 1.,
                        y: -1.,
                        facing_left: false,
                    },
                )
                .unwrap()
                .to_rgba8_premultiplied()
                .unwrap();
            let mut active = vec![false; (s.sw * s.sh) as usize];
            for e in &r.faces[index].eyes {
                for p in &e.mask {
                    active[p.index] = true;
                }
            }
            for (i, (&a, &b)) in original
                .as_slice()
                .iter()
                .zip(rendered.as_slice())
                .enumerate()
            {
                if !active[i] {
                    assert_eq!(a, b, "遮罩外的眉毛、嘴部和原表情不得改变");
                }
            }
            if [1, 3].contains(&index) {
                assert_eq!(original.as_slice(), rendered.as_slice());
            } else {
                assert_ne!(original.as_slice(), rendered.as_slice());
            }
        }
    }

    #[test]
    fn extremes_move_pupils_and_reject_invalid_state() {
        let (_, mut r, _) = renderer();
        let left = r
            .render(
                0,
                Pose {
                    x: -1.,
                    ..Pose::default()
                },
            )
            .unwrap();
        let right = r
            .render(
                0,
                Pose {
                    x: 1.,
                    ..Pose::default()
                },
            )
            .unwrap();
        assert_ne!(
            left.to_rgba8().unwrap().as_slice(),
            right.to_rgba8().unwrap().as_slice()
        );
        assert_eq!(
            right,
            r.render(
                0,
                Pose {
                    x: 1.,
                    ..Pose::default()
                }
            )
            .unwrap()
        );
        assert!(r.render(6, Pose::default()).is_err());
        assert!(
            r.render(
                0,
                Pose {
                    x: f32::NAN,
                    ..Pose::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn quantized_offsets_never_exceed_expression_limits() {
        let (c, mut r, _) = renderer();
        for expression in &c.gaze.expressions {
            let face = c.faces[expression.face];
            for value in [-4., -1., -0.99, 0., 0.99, 1., 4.] {
                r.render(
                    expression.face,
                    Pose {
                        x: value,
                        y: value,
                        ..Pose::default()
                    },
                )
                .unwrap();
                let ((_, dx, dy), _) = r.cache.as_ref().unwrap();
                let logical = [
                    f32::from(*dx).abs() / 4. * face.width / face.sw as f32,
                    f32::from(*dy).abs() / 4. * face.height / face.sh as f32,
                ];
                assert!(logical[0] <= expression.max_offset[0]);
                assert!(logical[1] <= expression.max_offset[1]);
                if value != 0. {
                    assert_eq!((*dx < 0, *dy < 0), (value < 0., value < 0.));
                }
            }
        }
    }
}
