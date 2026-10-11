use super::{PetWindow, QuotaRow, SettingsWindow, StatusPanelWindow};
use pet_desktop::{
    platform::{
        codex_usage::{self, QueryResult},
        desktop,
    },
    ui_state::{self, PanelDismissal},
    usage::{UsageError, UsageState},
};
use slint::{
    ComponentHandle, PhysicalPosition, VecModel,
    winit_030::{
        WinitWindowAccessor,
        winit::raw_window_handle::{HasWindowHandle, RawWindowHandle},
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use windows::Win32::Foundation::HWND;

pub struct UsagePanel {
    window: StatusPanelWindow,
    settings: slint::Weak<SettingsWindow>,
    state: RefCell<UsageState>,
    dismissal: RefCell<PanelDismissal>,
    native_configured: Cell<bool>,
    generation: Arc<AtomicU64>,
    requests: SyncSender<()>,
    results: Receiver<QueryResult>,
}

impl UsagePanel {
    pub fn new(
        settings: &SettingsWindow,
        generation: Arc<AtomicU64>,
        requests: SyncSender<()>,
        results: Receiver<QueryResult>,
    ) -> Result<Rc<Self>, slint::PlatformError> {
        let this = Rc::new(Self {
            window: StatusPanelWindow::new()?,
            settings: settings.as_weak(),
            state: RefCell::new(UsageState::default()),
            dismissal: RefCell::new(PanelDismissal::default()),
            native_configured: Cell::new(false),
            generation,
            requests,
            results,
        });
        let weak = Rc::downgrade(&this);
        this.window.on_dismiss(move || {
            if let Some(panel) = weak.upgrade() {
                panel.close();
            }
        });
        let weak = Rc::downgrade(&this);
        this.window.on_refresh(move || {
            if let Some(panel) = weak.upgrade() {
                panel.refresh();
            }
        });
        let weak = Rc::downgrade(&this);
        settings.on_refresh_quota(move || {
            if let Some(panel) = weak.upgrade() {
                panel.refresh();
            }
        });
        Ok(this)
    }

    pub fn refresh(&self) {
        let Some(id) = self.state.borrow_mut().request(Instant::now()) else {
            return;
        };
        self.generation.store(id, Ordering::Release);
        // 通道只承载唤醒信号，最新代次保存在原子变量中；满队列直接合并。
        if let Err(TrySendError::Disconnected(())) = self.requests.try_send(()) {
            self.state
                .borrow_mut()
                .complete(id, Err(UsageError::Process), SystemTime::now());
            self.generation.store(0, Ordering::Release);
        }
        self.publish();
    }

    pub fn toggle(&self, pet: &PetWindow) {
        if self.is_visible() {
            self.close();
            return;
        }
        *self.dismissal.borrow_mut() = PanelDismissal::opened(desktop::escape_pressed());
        self.place(pet);
        if self.window.show().is_err() {
            if let Some(settings) = self.settings.upgrade() {
                settings.set_quota_status("无法打开状态面板，请重试。".into());
            }
            return;
        }
        self.configure_native();
        self.refresh();
    }

    pub fn close(&self) {
        if !self.is_visible() && self.state.borrow().in_flight.is_none() {
            return;
        }
        let _ = self.window.hide();
        self.native_configured.set(false);
        self.generation.store(0, Ordering::Release);
        self.state.borrow_mut().cancel();
        self.publish();
    }

    pub fn is_visible(&self) -> bool {
        self.window.window().is_visible()
    }

    pub fn metrics(&self, cpu: Option<f64>, memory: Option<f64>) {
        self.window.set_cpu_text(ui_state::metric_text(cpu).into());
        self.window
            .set_memory_text(ui_state::metric_text(memory).into());
    }

    fn configure_native(&self) {
        if self.native_configured.get() {
            return;
        }
        let configured = self
            .window
            .window()
            .with_winit_window(|window| {
                if let Ok(handle) = window.window_handle()
                    && let RawWindowHandle::Win32(raw) = handle.as_raw()
                {
                    desktop::configure_pet(HWND(raw.hwnd.get() as *mut _));
                    return true;
                }
                false
            })
            .unwrap_or(false);
        self.native_configured.set(configured);
    }

    fn place(&self, pet: &PetWindow) {
        let position = pet.window().position();
        let pet_size = pet.window().size();
        let panel_size = self.window.window().size();
        let dpi = pet.window().scale_factor();
        let width = if panel_size.width > 0 {
            panel_size.width
        } else {
            (320. * dpi) as u32
        };
        let height = if panel_size.height > 0 {
            panel_size.height
        } else {
            (240. * dpi) as u32
        };
        let (x, y) = desktop::panel_position(
            (
                position.x,
                position.y,
                pet_size.width as i32,
                pet_size.height as i32,
            ),
            width as i32,
            height as i32,
        );
        if self.window.window().position() != PhysicalPosition::new(x, y) {
            self.window
                .window()
                .set_position(PhysicalPosition::new(x, y));
        }
    }

    pub fn tick(&self, pet: &PetWindow) {
        let mut changed = false;
        while let Ok(event) = self.results.try_recv() {
            if self
                .state
                .borrow_mut()
                .complete(event.id, event.result, SystemTime::now())
            {
                self.generation.store(0, Ordering::Release);
                changed = true;
            }
        }
        if changed {
            self.publish();
        }
        let visible = self.is_visible();
        if visible {
            self.configure_native();
            self.place(pet);
            let (pressed, point) =
                desktop::pointer_press().unwrap_or((false, (i32::MIN, i32::MIN)));
            let inside = |window: &slint::Window| {
                let position = window.position();
                let size = window.size();
                point.0 >= position.x
                    && point.1 >= position.y
                    && i64::from(point.0) < i64::from(position.x) + i64::from(size.width)
                    && i64::from(point.1) < i64::from(position.y) + i64::from(size.height)
            };
            let dismiss = self.dismissal.borrow_mut().observe(
                pressed,
                inside(self.window.window()),
                inside(pet.window()),
                desktop::escape_pressed(),
            );
            if dismiss {
                self.close();
                return;
            }
        }
        let refresh = self.state.borrow().refresh_due(visible, Instant::now());
        if refresh {
            self.refresh();
        }
    }

    fn publish(&self) {
        let state = self.state.borrow();
        let rows: Vec<QuotaRow> = state
            .snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .windows
                    .iter()
                    .map(|window| QuotaRow {
                        title: window.title.clone().into(),
                        remaining: ui_state::metric_text(window.remaining).into(),
                        reset: window
                            .resets_at
                            .and_then(codex_usage::local_time)
                            .map(|time| format!("重置于 {time}"))
                            .unwrap_or_else(|| "重置时间未知".into())
                            .into(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let updated = state
            .updated_at
            .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| i64::try_from(duration.as_secs()).ok())
            .and_then(codex_usage::local_time)
            .map(|time| format!("更新于 {time}（本地时间）"))
            .unwrap_or_default();
        if let Some(settings) = self.settings.upgrade() {
            let detail = rows
                .iter()
                .map(|row| format!("{}：剩余 {}，{}", row.title, row.remaining, row.reset))
                .collect::<Vec<_>>()
                .join("\n");
            settings.set_quota_status(if detail.is_empty() {
                state.status().into()
            } else {
                format!("{}\n{}\n{}", state.status(), detail, updated).into()
            });
            settings.set_quota_busy(state.in_flight.is_some());
        }
        self.window.set_rows(Rc::new(VecModel::from(rows)).into());
        self.window.set_status(state.status().into());
        self.window.set_updated(updated.into());
        self.window.set_busy(state.in_flight.is_some());
    }
}
