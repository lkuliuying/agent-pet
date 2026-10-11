use super::{ExpressionValues, SettingsWindow, sprite};
use pet_desktop::{
    assets::Character,
    expressions::{ExpressionConfig, ExpressionId},
    gaze::Pose,
    gaze_render::GazeRenderer,
};
use slint::VecModel;
use std::rc::Rc;

pub fn values(config: &ExpressionConfig) -> ExpressionValues {
    let optional = |id: Option<ExpressionId>| id.map_or(0, |id| id.index() as i32 + 1);
    ExpressionValues {
        idle: config.idle.index() as i32,
        blink: optional(config.blink),
        input: optional(config.input),
        click: optional(config.click),
        sleepy: optional(config.sleepy),
        drag: optional(config.drag),
        notice: optional(config.notice),
        blink_interval_ms: config.blink_interval_ms as i32,
        blink_duration_ms: config.blink_duration_ms as i32,
        input_duration_ms: config.input_duration_ms as i32,
        click_duration_ms: config.click_duration_ms as i32,
        sleepy_after_ms: config.sleepy_after_ms as i32,
        drag_duration_ms: config.drag_duration_ms as i32,
    }
}

pub fn config(draft: ExpressionValues) -> Result<ExpressionConfig, String> {
    let optional = |value| match value {
        0 => Ok(None),
        1..=6 => ExpressionId::from_index(value - 1).map(Some),
        _ => Err("请选择有效的场景表情".into()),
    };
    let milliseconds = |value| u32::try_from(value).map_err(|_| "表情时间不能为负数".to_owned());
    let config = ExpressionConfig {
        idle: ExpressionId::from_index(draft.idle)?,
        blink: optional(draft.blink)?,
        input: optional(draft.input)?,
        click: optional(draft.click)?,
        sleepy: optional(draft.sleepy)?,
        drag: optional(draft.drag)?,
        notice: optional(draft.notice)?,
        blink_interval_ms: milliseconds(draft.blink_interval_ms)?,
        blink_duration_ms: milliseconds(draft.blink_duration_ms)?,
        input_duration_ms: milliseconds(draft.input_duration_ms)?,
        click_duration_ms: milliseconds(draft.click_duration_ms)?,
        sleepy_after_ms: milliseconds(draft.sleepy_after_ms)?,
        drag_duration_ms: milliseconds(draft.drag_duration_ms)?,
    };
    config.validate()?;
    Ok(config)
}

pub fn bind_preview(
    settings: &SettingsWindow,
    character: &Character,
    renderer: &mut GazeRenderer,
) -> Result<(), String> {
    let images = ExpressionId::ALL
        .into_iter()
        .map(|face| renderer.render(face.index(), Pose::default()))
        .collect::<Result<Vec<_>, _>>()?;
    settings.set_expression_defaults(values(&ExpressionConfig::default()));
    settings.set_preview_body(sprite(character.body));
    settings.set_preview_left_hand(sprite(character.left_hand));
    settings.set_preview_right_hand(sprite(character.right_hand));
    settings.set_preview_keyboard(sprite(character.keyboard));
    settings.set_preview_faces(
        Rc::new(VecModel::from(
            character
                .faces
                .iter()
                .copied()
                .map(sprite)
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
    settings.set_preview_images(Rc::new(VecModel::from(images)).into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_mapping_preserves_all_expressions_and_disabled_scenes() {
        for id in ExpressionId::ALL {
            for scene in [None, Some(id)] {
                let config = ExpressionConfig {
                    idle: id,
                    blink: scene,
                    input: scene,
                    click: scene,
                    sleepy: scene,
                    drag: scene,
                    notice: scene,
                    input_duration_ms: 2300,
                    sleepy_after_ms: 123_000,
                    ..ExpressionConfig::default()
                };
                assert_eq!(super::config(values(&config)).unwrap(), config);
            }
        }
    }

    #[test]
    fn invalid_draft_is_rejected_before_save() {
        let defaults = values(&ExpressionConfig::default());
        for index in [-1, 6] {
            assert!(
                config(ExpressionValues {
                    idle: index,
                    ..defaults.clone()
                })
                .is_err()
            );
        }
        for index in [-1, 7, i32::MIN, i32::MAX] {
            assert!(
                config(ExpressionValues {
                    click: index,
                    ..defaults.clone()
                })
                .is_err()
            );
        }
        assert!(
            config(ExpressionValues {
                click_duration_ms: -100,
                ..defaults.clone()
            })
            .is_err()
        );
        assert!(
            config(ExpressionValues {
                blink_interval_ms: 1000,
                blink_duration_ms: 1000,
                ..defaults
            })
            .is_err()
        );
    }
}
