use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Key {
    pub code: u16,
    pub label: &'static str,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub left: bool,
    pub numpad: bool,
}

pub fn layout(tkl: bool) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut add = |code, label, x, y, width, height, numpad| {
        keys.push(Key {
            code,
            label,
            x,
            y,
            width,
            height,
            left: x < 6.5,
            numpad,
        });
    };
    add(1, "Esc", 0., 0., 1., 1., false);
    for (i, (code, label)) in [
        (59, "F1"),
        (60, "F2"),
        (61, "F3"),
        (62, "F4"),
        (63, "F5"),
        (64, "F6"),
        (65, "F7"),
        (66, "F8"),
        (67, "F9"),
        (68, "F10"),
        (87, "F11"),
        (88, "F12"),
    ]
    .into_iter()
    .enumerate()
    {
        add(
            code,
            label,
            2. + i as f32 + (i / 4) as f32 * 0.5,
            0.,
            1.,
            1.,
            false,
        );
    }
    for (i, (code, label)) in [(0x137, "Prt"), (70, "Scr"), (0x245, "Pau")]
        .into_iter()
        .enumerate()
    {
        add(code, label, 15.5 + i as f32, 0., 1., 1., false);
    }
    for (i, (code, label)) in [
        (41, "`"),
        (2, "1"),
        (3, "2"),
        (4, "3"),
        (5, "4"),
        (6, "5"),
        (7, "6"),
        (8, "7"),
        (9, "8"),
        (10, "9"),
        (11, "0"),
        (12, "-"),
        (13, "="),
    ]
    .into_iter()
    .enumerate()
    {
        add(code, label, i as f32, 1.35, 1., 1., false);
    }
    add(14, "Back", 13., 1.35, 2., 1., false);
    add(15, "Tab", 0., 2.35, 1.5, 1., false);
    for (i, (code, label)) in [
        (16, "Q"),
        (17, "W"),
        (18, "E"),
        (19, "R"),
        (20, "T"),
        (21, "Y"),
        (22, "U"),
        (23, "I"),
        (24, "O"),
        (25, "P"),
        (26, "["),
        (27, "]"),
    ]
    .into_iter()
    .enumerate()
    {
        add(code, label, 1.5 + i as f32, 2.35, 1., 1., false);
    }
    add(43, "\\", 13.5, 2.35, 1.5, 1., false);
    add(58, "Caps", 0., 3.35, 1.75, 1., false);
    for (i, (code, label)) in [
        (30, "A"),
        (31, "S"),
        (32, "D"),
        (33, "F"),
        (34, "G"),
        (35, "H"),
        (36, "J"),
        (37, "K"),
        (38, "L"),
        (39, ";"),
        (40, "'"),
    ]
    .into_iter()
    .enumerate()
    {
        add(code, label, 1.75 + i as f32, 3.35, 1., 1., false);
    }
    add(28, "Enter", 12.75, 3.35, 2.25, 1., false);
    add(42, "Shift", 0., 4.35, 2.25, 1., false);
    for (i, (code, label)) in [
        (44, "Z"),
        (45, "X"),
        (46, "C"),
        (47, "V"),
        (48, "B"),
        (49, "N"),
        (50, "M"),
        (51, ","),
        (52, "."),
        (53, "/"),
    ]
    .into_iter()
    .enumerate()
    {
        add(code, label, 2.25 + i as f32, 4.35, 1., 1., false);
    }
    add(54, "Shift", 12.25, 4.35, 2.75, 1., false);
    for (code, label, x, w) in [
        (29, "Ctrl", 0., 1.25),
        (0x15b, "Win", 1.25, 1.25),
        (56, "Alt", 2.5, 1.25),
        (57, "", 3.75, 6.25),
        (0x138, "Alt", 10., 1.25),
        (0x15c, "Win", 11.25, 1.25),
        (0x15d, "≡", 12.5, 1.25),
        (0x11d, "Ctrl", 13.75, 1.25),
    ] {
        add(code, label, x, 5.35, w, 1., false);
    }
    for (i, (code, label)) in [
        (0x152, "Ins"),
        (0x147, "Hm"),
        (0x149, "Pg↑"),
        (0x153, "Del"),
        (0x14f, "End"),
        (0x151, "Pg↓"),
    ]
    .into_iter()
    .enumerate()
    {
        add(
            code,
            label,
            15.5 + (i % 3) as f32,
            1.35 + (i / 3) as f32,
            1.,
            1.,
            false,
        );
    }
    for (code, label, x, y) in [
        (0x148, "↑", 16.5, 4.35),
        (0x14b, "←", 15.5, 5.35),
        (0x150, "↓", 16.5, 5.35),
        (0x14d, "→", 17.5, 5.35),
    ] {
        add(code, label, x, y, 1., 1., false);
    }
    for (i, (code, label)) in [(69, "Num"), (0x135, "/"), (55, "*"), (74, "-")]
        .into_iter()
        .enumerate()
    {
        add(code, label, 19. + i as f32, 1.35, 1., 1., true);
    }
    for (i, (code, label)) in [
        (71, "7"),
        (72, "8"),
        (73, "9"),
        (75, "4"),
        (76, "5"),
        (77, "6"),
        (79, "1"),
        (80, "2"),
        (81, "3"),
    ]
    .into_iter()
    .enumerate()
    {
        add(
            code,
            label,
            19. + (i % 3) as f32,
            2.35 + (i / 3) as f32,
            1.,
            1.,
            true,
        );
    }
    add(78, "+", 22., 2.35, 1., 2., true);
    add(0x11c, "↵", 22., 4.35, 1., 2., true);
    add(82, "0", 19., 5.35, 2., 1., true);
    add(83, ".", 21., 5.35, 1., 1., true);
    if tkl {
        keys.retain(|key| !key.numpad);
    }
    keys
}

pub fn scan_code(make_code: u16, flags: u16, virtual_key: u16) -> Option<u16> {
    if virtual_key == 255 || make_code == 0xff {
        return None;
    }
    if virtual_key == 0x13 {
        return Some(0x245);
    }
    if virtual_key == 0x2c {
        return Some(0x137);
    }
    if virtual_key == 0x90 {
        return Some(69);
    }
    if make_code == 0 {
        return None;
    }
    Some(make_code | if flags & 2 != 0 { 0x100 } else { 0 })
}

#[derive(Debug, Default)]
pub struct KeyboardState {
    devices: HashMap<usize, HashSet<u16>>,
    hint: Option<KeyHint>,
    pub last_input: Option<Instant>,
    pub left_until: Option<Instant>,
    pub right_until: Option<Instant>,
    pub paused: bool,
}

#[derive(Debug)]
struct KeyHint {
    text: String,
    devices: HashSet<usize>,
    codes: HashSet<u16>,
    fade_at: Option<Instant>,
}

const HINT_HOLD: Duration = Duration::from_millis(800);
const HINT_FADE: Duration = Duration::from_millis(120);

fn modifier(code: u16) -> Option<usize> {
    match code {
        29 | 0x11d => Some(0),
        56 | 0x138 => Some(1),
        42 | 54 => Some(2),
        0x15b | 0x15c => Some(3),
        _ => None,
    }
}

fn key_name(key: &Key) -> String {
    let label = match key.code {
        57 => "Space",
        14 => "Backspace",
        58 => "Caps Lock",
        0x137 => "Print Screen",
        70 => "Scroll Lock",
        0x245 => "Pause",
        0x15d => "Menu",
        0x152 => "Insert",
        0x147 => "Home",
        0x149 => "Page Up",
        0x153 => "Delete",
        0x151 => "Page Down",
        69 => "Num Lock",
        0x11c => "Enter",
        _ => key.label,
    };
    if key.numpad && key.code != 69 {
        format!("Num {label}")
    } else {
        label.into()
    }
}

impl KeyboardState {
    pub fn transition(&mut self, device: usize, code: u16, down: bool, now: Instant) {
        if self.paused {
            return;
        }
        static KEYS: LazyLock<HashMap<u16, Key>> =
            LazyLock::new(|| layout(false).into_iter().map(|k| (k.code, k)).collect());
        let Some(key) = KEYS.get(&code) else {
            return;
        };
        if down {
            if self.devices.entry(device).or_default().insert(code) {
                self.last_input = Some(now);
                let until = Some(now + Duration::from_millis(110));
                if key.left {
                    self.left_until = until;
                } else {
                    self.right_until = until;
                }
                let pressed = self.pressed();
                let mut parts = Vec::new();
                for (index, name) in ["Ctrl", "Alt", "Shift", "Win"].into_iter().enumerate() {
                    if pressed.iter().any(|code| modifier(*code) == Some(index)) {
                        parts.push(name.to_owned());
                    }
                }
                if modifier(code).is_none() {
                    parts.push(key_name(key));
                }
                let codes: HashSet<_> = pressed
                    .into_iter()
                    .filter(|held| *held == code || modifier(*held).is_some())
                    .collect();
                let devices = self
                    .devices
                    .iter()
                    .filter(|(_, held)| held.iter().any(|code| codes.contains(code)))
                    .map(|(device, _)| *device)
                    .collect();
                // 只保留最近一次按下的提示，快速按下松开也不会在两帧之间丢失。
                self.hint = Some(KeyHint {
                    text: parts.join(" + "),
                    devices,
                    codes,
                    fade_at: None,
                });
            }
        } else if let Some(pressed) = self.devices.get_mut(&device) {
            pressed.remove(&code);
            if pressed.is_empty() {
                self.devices.remove(&device);
            }
            if self.devices.is_empty()
                && let Some(hint) = self.hint.as_mut()
                && hint.fade_at.is_none()
            {
                hint.fade_at = Some(now + HINT_HOLD);
            }
        }
    }
    pub fn hint(&mut self, now: Instant) -> (&str, f32) {
        if self
            .hint
            .as_ref()
            .is_some_and(|hint| hint.fade_at.is_some_and(|at| now >= at + HINT_FADE))
        {
            self.hint = None;
        }
        match &self.hint {
            Some(hint) => {
                let opacity = hint.fade_at.map_or(1., |at| {
                    1. - (now.saturating_duration_since(at).as_secs_f32() / HINT_FADE.as_secs_f32())
                        .min(1.)
                });
                (&hint.text, opacity)
            }
            None => ("", 0.),
        }
    }
    pub fn pressed(&self) -> HashSet<u16> {
        self.devices.values().flatten().copied().collect()
    }
    pub fn remove_device(&mut self, device: usize) {
        self.devices.remove(&device);
        if self.devices.is_empty()
            || self
                .hint
                .as_ref()
                .is_some_and(|hint| hint.devices.contains(&device))
        {
            self.clear_feedback();
        }
    }
    pub fn release(&mut self, code: u16) {
        for keys in self.devices.values_mut() {
            keys.remove(&code);
        }
        self.devices.retain(|_, keys| !keys.is_empty());
        if self.devices.is_empty()
            || self
                .hint
                .as_ref()
                .is_some_and(|hint| hint.codes.contains(&code))
        {
            self.clear_feedback();
        }
    }
    pub fn clear(&mut self) {
        self.devices.clear();
        self.clear_feedback();
    }
    fn clear_feedback(&mut self) {
        self.hint = None;
        self.left_until = None;
        self.right_until = None;
    }
    pub fn pause(&mut self, paused: bool) {
        self.paused = paused;
        if paused {
            self.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_unique_layouts() {
        for (tkl, count) in [(false, 104), (true, 87)] {
            let keys = layout(tkl);
            assert_eq!(keys.len(), count);
            assert_eq!(
                keys.iter().map(|k| k.code).collect::<HashSet<_>>().len(),
                count
            );
        }
    }
    #[test]
    fn extended_and_special_keys() {
        assert_eq!(scan_code(28, 2, 13), Some(0x11c));
        assert_eq!(scan_code(28, 0, 13), Some(28));
        assert_eq!(scan_code(69, 4, 19), Some(0x245));
        assert_eq!(scan_code(69, 2, 0x90), Some(69));
        assert_eq!(scan_code(29, 2, 0x11), Some(0x11d));
        assert_eq!(scan_code(56, 0, 0x12), Some(56));
        assert_eq!(scan_code(55, 2, 0x2c), Some(0x137));
        assert_eq!(scan_code(0, 0, 255), None);
    }
    #[test]
    fn repeats_devices_and_pause() {
        let now = Instant::now();
        let mut s = KeyboardState::default();
        s.transition(1, 30, true, now);
        s.transition(1, 30, true, now);
        s.transition(2, 30, true, now);
        s.transition(1, 36, true, now);
        assert_eq!(s.pressed().len(), 2);
        assert!(s.left_until.is_some() && s.right_until.is_some());
        s.transition(1, 30, false, now);
        assert!(s.pressed().contains(&30));
        s.remove_device(2);
        assert!(!s.pressed().contains(&30));
        s.pause(true);
        s.transition(1, 30, true, now);
        assert!(s.pressed().is_empty());
        assert!(s.left_until.is_none() && s.right_until.is_none());
        s.pause(false);
        s.transition(1, 0xffff, true, now);
        assert!(s.pressed().is_empty());
        s.transition(1, 30, true, now);
        s.release(30);
        assert!(s.pressed().is_empty());
    }

    #[test]
    fn hint_names_and_modifier_order() {
        let now = Instant::now();
        for (code, label) in [
            (30, "A"),
            (57, "Space"),
            (14, "Backspace"),
            (71, "Num 7"),
            (0x11c, "Num Enter"),
            (0x148, "↑"),
            (69, "Num Lock"),
        ] {
            let mut s = KeyboardState::default();
            s.transition(1, code, true, now);
            assert_eq!(s.hint(now), (label, 1.));
        }
        let mut s = KeyboardState::default();
        for code in [0x15c, 54, 0x138, 0x11d, 29, 46] {
            s.transition(1, code, true, now);
        }
        assert_eq!(s.hint(now).0, "Ctrl + Alt + Shift + Win + C");
        let mut s = KeyboardState::default();
        s.transition(1, 29, true, now);
        assert_eq!(s.hint(now).0, "Ctrl");
    }

    #[test]
    fn latest_press_replaces_hint_without_repeat_queue() {
        let now = Instant::now();
        let mut s = KeyboardState::default();
        s.transition(1, 30, true, now);
        s.transition(1, 31, true, now);
        s.transition(1, 30, true, now + Duration::from_millis(5));
        assert_eq!(s.hint(now).0, "S");
        s.transition(1, 0xffff, true, now);
        assert_eq!(s.hint(now).0, "S");
        assert_eq!(s.last_input, Some(now));
    }

    #[test]
    fn quick_tap_survives_frame_and_fades_after_release() {
        let now = Instant::now();
        let mut s = KeyboardState::default();
        s.transition(1, 30, true, now);
        let released = now + Duration::from_millis(2);
        s.transition(1, 30, false, released);
        assert!(s.pressed().is_empty());
        assert_eq!(s.hint(now + Duration::from_millis(16)), ("A", 1.));
        assert_eq!(s.hint(released + HINT_HOLD), ("A", 1.));
        assert!((s.hint(released + HINT_HOLD + HINT_FADE / 2).1 - 0.5).abs() < 0.001);
        assert_eq!(s.hint(released + HINT_HOLD + HINT_FADE), ("", 0.));
        assert!(s.hint.is_none());
    }

    #[test]
    fn held_hint_and_multi_device_release() {
        let now = Instant::now();
        let mut s = KeyboardState::default();
        s.transition(1, 29, true, now);
        s.transition(2, 46, true, now);
        s.transition(1, 29, false, now);
        assert_eq!(s.hint(now + Duration::from_secs(60)), ("Ctrl + C", 1.));
        s.transition(2, 46, false, now + Duration::from_secs(61));
        assert_eq!(s.hint(now + Duration::from_secs(62)), ("", 0.));
        s.transition(1, 30, true, now);
        s.transition(2, 30, true, now);
        s.transition(1, 30, false, now);
        assert_eq!(s.hint(now + Duration::from_secs(60)), ("A", 1.));
    }

    #[test]
    fn hint_cleanup_and_fade_interruption() {
        let now = Instant::now();
        let mut s = KeyboardState::default();
        s.transition(1, 30, true, now);
        s.transition(1, 30, false, now);
        let later = now + HINT_HOLD + HINT_FADE / 2;
        s.transition(2, 31, true, later);
        assert_eq!(s.hint(later), ("S", 1.));
        s.remove_device(1);
        assert_eq!(s.hint(later).0, "S");
        s.remove_device(2);
        assert_eq!(s.hint(later), ("", 0.));
        for action in 0..3 {
            s.pause(false);
            s.transition(1, 30, true, now);
            match action {
                0 => s.clear(),
                1 => s.pause(true),
                _ => s.release(30),
            }
            assert_eq!(s.hint(now), ("", 0.));
            assert!(s.left_until.is_none() && s.right_until.is_none());
        }
    }

    #[test]
    fn lost_release_of_older_key_does_not_leave_hint_stuck() {
        let now = Instant::now();
        for disconnect in [false, true] {
            let mut s = KeyboardState::default();
            s.transition(1, 30, true, now);
            s.transition(2, 31, true, now);
            s.transition(2, 31, false, now);
            assert_eq!(s.hint(now).0, "S");
            if disconnect {
                s.remove_device(1);
            } else {
                s.release(30);
            }
            assert!(s.pressed().is_empty());
            assert_eq!(s.hint(now), ("", 0.));
        }
    }
}
