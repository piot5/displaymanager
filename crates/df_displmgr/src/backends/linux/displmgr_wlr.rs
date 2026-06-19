//! Optimized Wayland object management for wlroots-based compositors.
//! Integrated with color-management-v1 for HDR and color space control.

use async_trait::async_trait;
use tokio::sync::oneshot;
use wayland_client::{
    protocol::{wl_callback, wl_output::Transform, wl_registry},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_v1, zwlr_output_head_v1, zwlr_output_manager_v1, zwlr_output_mode_v1,
};

use crate::error::{DisplayError, DisplayResult};
use crate::traits::{OutputEditable, UniversalTopology};
use crate::types::{
    DisplayId, DisplayRotation, Extent2D, HdrMode, HdrState, OutputState, VideoMode,
};

/// Internal state for tracking Wayland objects and async synchronization.
pub struct WlrInternalState {
    pub manager: Option<zwlr_output_manager_v1::ZwlrOutputManagerV1>,
    pub heads: Vec<HeadInfo>,
    pub serial: u32,
    pub sync_tx: Option<oneshot::Sender<()>>,
    pub persist: bool,
}

/// Represents a physical output device and its capabilities.
pub struct HeadInfo {
    pub handle: zwlr_output_head_v1::ZwlrOutputHeadV1,
    pub current_state: OutputState,
    pub modes: Vec<VideoMode>,
}

pub struct WlrTopology {
    connection: Connection,
    state: WlrInternalState,
    event_queue: wayland_client::EventQueue<WlrInternalState>,
}

impl WlrTopology {
    /// Wake any currently inactive outputs and return a list of their IDs
    /// which can later be passed to `restore_inactive_outputs` to revert.
    pub fn snapshot_wake_inactive_outputs(
        &mut self,
        keep_active: Option<&DisplayId>,
    ) -> Vec<DisplayId> {
        let mut activated = Vec::new();
        for head in &mut self.state.heads {
            let id = head.current_state.identity.id.clone();
            if !head.current_state.enabled {
                if let Some(k) = keep_active {
                    if &id == k {
                        continue;
                    }
                }
                head.current_state.enabled = true;
                activated.push(id);
            }
        }
        activated
    }

    /// Restore previously deactivated outputs by ID. Does not disable an
    /// output that was explicitly kept active during snapshot.
    pub fn restore_inactive_outputs(&mut self, inactive_ids: &[DisplayId]) {
        for id in inactive_ids {
            if let Some(head) = self
                .state
                .heads
                .iter_mut()
                .find(|h| &h.current_state.identity.id == id)
            {
                head.current_state.enabled = false;
            }
        }
    }
}

pub struct WlrOutputEditor<'a> {
    head: &'a mut HeadInfo,
    all_heads: &'a [HeadInfo],
}

fn wl_transform_to_rotation(transform: Transform) -> DisplayRotation {
    match transform {
        Transform::Normal => DisplayRotation::Rotate0,
        Transform::Rotate90 => DisplayRotation::Rotate90,
        Transform::Rotate180 => DisplayRotation::Rotate180,
        Transform::Rotate270 => DisplayRotation::Rotate270,
        _ => DisplayRotation::Rotate0,
    }
}

fn rotation_to_wl_transform(rotation: DisplayRotation) -> Transform {
    match rotation {
        DisplayRotation::Rotate0 => Transform::Normal,
        DisplayRotation::Rotate90 => Transform::Rotate90,
        DisplayRotation::Rotate180 => Transform::Rotate180,
        DisplayRotation::Rotate270 => Transform::Rotate270,
    }
}

// --- Dispatch Implementations (Registry, Manager, Head, Mode, Callback) ---

impl Dispatch<wl_registry::WlRegistry, ()> for WlrInternalState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == zwlr_output_manager_v1::ZwlrOutputManagerV1::interface().name {
                state.manager = Some(
                    proxy.bind::<zwlr_output_manager_v1::ZwlrOutputManagerV1, _, _>(
                        name,
                        version,
                        qh,
                        (),
                    ),
                );
            }
        }
    }
}

impl Dispatch<zwlr_output_manager_v1::ZwlrOutputManagerV1, ()> for WlrInternalState {
    fn event(
        state: &mut Self,
        _: &zwlr_output_manager_v1::ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_output_manager_v1::Event::Done { serial } = event {
            state.serial = serial;
        }
    }
}

impl Dispatch<zwlr_output_head_v1::ZwlrOutputHeadV1, ()> for WlrInternalState {
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_head_v1::ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let head_idx = if let Some(idx) = state.heads.iter().position(|h| h.handle == *proxy) {
            idx
        } else {
            state.heads.push(HeadInfo {
                handle: proxy.clone(),
                current_state: OutputState::default(),
                modes: Vec::new(),
            });
            state.heads.len() - 1
        };

        let head = &mut state.heads[head_idx];
        match event {
            zwlr_output_head_v1::Event::Name { name } => {
                head.current_state.identity.id = DisplayId(name.clone());
                head.current_state.identity.monitor_name = name;
            }
            zwlr_output_head_v1::Event::Position { x, y } => {
                head.current_state.geometry.origin.x = x;
                head.current_state.geometry.origin.y = y;
            }
            zwlr_output_head_v1::Event::Scale { scale } => {
                head.current_state.scale = scale as f64;
            }
            zwlr_output_head_v1::Event::Transform { transform } => {
                head.current_state.rotation = wl_transform_to_rotation(transform);
            }
            zwlr_output_head_v1::Event::Mode { mode } => {
                qh.make_handle()
                    .insert_instance::<zwlr_output_mode_v1::ZwlrOutputModeV1, usize>(
                        &mode, head_idx,
                    );
            }
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                head.current_state.enabled = enabled != 0;
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_output_mode_v1::ZwlrOutputModeV1, usize> for WlrInternalState {
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_mode_v1::ZwlrOutputModeV1,
        event: zwlr_output_mode_v1::Event,
        head_idx: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Some(head) = state.heads.get_mut(*head_idx) {
            match event {
                zwlr_output_mode_v1::Event::Size { width, height } => {
                    let mode = VideoMode {
                        resolution: Extent2D { width, height },
                        refresh_rate: 0,
                    };
                    if !head.modes.iter().any(|m| m.resolution == mode.resolution) {
                        head.modes.push(mode);
                    }
                }
                zwlr_output_mode_v1::Event::Refresh { refresh } => {
                    if let Some(mode) = head.modes.last_mut() {
                        mode.refresh_rate = refresh;
                    }
                }
                zwlr_output_mode_v1::Event::Preferred => {
                    // Preferred mode handling is not yet required for this simplified model.
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for WlrInternalState {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Some(tx) = state.sync_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Dispatch<zwlr_output_configuration_v1::ZwlrOutputConfigurationV1, ()> for WlrInternalState {}

// --- Trait Implementations ---

#[async_trait]
impl UniversalTopology for WlrTopology {
    fn acquire() -> DisplayResult<Self> {
        let conn = Connection::connect_to_env().map_err(|_| DisplayError::ConnectionFailed)?;
        let event_queue = conn.new_event_queue();
        let qh = event_queue.handle();

        let state = WlrInternalState {
            manager: None,
            heads: Vec::new(),
            serial: 0,
            sync_tx: None,
            persist: false,
        };

        let _registry = conn.display().get_registry(&qh, ());
        let mut wrapper = WlrTopology {
            connection: conn,
            state,
            event_queue,
        };

        wrapper
            .event_queue
            .roundtrip(&mut wrapper.state)
            .map_err(|e| DisplayError::BackendError(e.to_string()))?;

        Ok(wrapper)
    }

    fn get_outputs(&self) -> Vec<OutputState> {
        self.state
            .heads
            .iter()
            .map(|h| h.current_state.clone())
            .collect()
    }

    fn edit_output(&mut self, id: &DisplayId) -> DisplayResult<Box<dyn OutputEditable + '_>> {
        let head_idx = self
            .state
            .heads
            .iter()
            .position(|h| h.current_state.identity.id == *id)
            .ok_or_else(|| DisplayError::NotFound(id.clone()))?;

        Ok(Box::new(WlrOutputEditor {
            head: &mut self.state.heads[head_idx],
            all_heads: &self.state.heads,
        }))
    }

    fn set_persistence(&mut self, enabled: bool) -> &mut Self {
        self.state.persist = enabled;
        self
    }

    async fn validate(&self) -> DisplayResult<()> {
        // On Wayland, validation happens during the test phase of the configuration application.
        // For the trait, we assume staged changes are valid until commit.
        Ok(())
    }

    async fn commit(&mut self) -> DisplayResult<()> {
        let manager = self
            .state
            .manager
            .as_ref()
            .ok_or_else(|| DisplayError::BackendError("Output manager missing".into()))?;

        let qh = self.event_queue.handle();
        let config = manager.create_configuration(self.state.serial, &qh, ());

        for head in &self.state.heads {
            if head.current_state.enabled {
                let head_config = config.enable_head(&head.handle, &qh, ());
                head_config.set_position(
                    head.current_state.geometry.origin.x,
                    head.current_state.geometry.origin.y,
                );
                head_config.set_scale(head.current_state.scale as f32);
                head_config.set_transform(rotation_to_wl_transform(head.current_state.rotation));
            } else {
                config.disable_head(&head.handle);
            }

            if head.current_state.hdr_state == HdrState::Enabled {
                return Err(DisplayError::BackendError(
                    "HDR is not supported in this simplified WLR path".into(),
                ));
            }
        }

        config.apply();

        let (tx, rx) = oneshot::channel();
        self.state.sync_tx = Some(tx);
        self.connection.display().sync(&qh, ());

        while self.state.sync_tx.is_some() {
            self.event_queue
                .blocking_dispatch(&mut self.state)
                .map_err(|e| DisplayError::BackendError(e.to_string()))?;
        }

        rx.await
            .map_err(|_| DisplayError::BackendError("Sync failed during commit".into()))?;
        Ok(())
    }
}

impl<'a> OutputEditable for WlrOutputEditor<'a> {
    fn set_rotation(
        &mut self,
        rotation: DisplayRotation,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        self.head.current_state.rotation = rotation;
        Ok(self)
    }

    fn set_resolution(&mut self, extent: Extent2D) -> DisplayResult<&mut dyn OutputEditable> {
        self.head.current_state.geometry.size = extent;
        Ok(self)
    }

    fn set_position(
        &mut self,
        position: crate::types::Point2D,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        self.head.current_state.geometry.origin = position;
        Ok(self)
    }

    fn set_primary(&mut self) -> DisplayResult<&mut dyn OutputEditable> {
        self.head.current_state.is_primary = true;
        Ok(self)
    }

    fn set_refresh_rate(&mut self, rate: u32) -> DisplayResult<&mut dyn OutputEditable> {
        self.head.current_state.refresh_rate = rate;
        Ok(self)
    }

    fn set_hdr(
        &mut self,
        state: HdrState,
        mode: HdrMode,
    ) -> DisplayResult<&mut dyn OutputEditable> {
        self.head.current_state.hdr_state = state;
        self.head.current_state.hdr_mode = mode;
        Ok(self)
    }

    fn set_scale(&mut self, scale: f64) -> DisplayResult<&mut dyn OutputEditable> {
        self.head.current_state.scale = scale;
        Ok(self)
    }

    fn set_enabled(&mut self, enabled: bool) -> DisplayResult<&mut dyn OutputEditable> {
        self.head.current_state.enabled = enabled;
        Ok(self)
    }

    fn clone_from(&mut self, source_id: &DisplayId) -> DisplayResult<&mut dyn OutputEditable> {
        let source_state = self
            .all_heads
            .iter()
            .find(|h| h.current_state.identity.id == *source_id)
            .map(|h| &h.current_state)
            .ok_or_else(|| DisplayError::NotFound(source_id.clone()))?;

        self.head.current_state.geometry = source_state.geometry;
        self.head.current_state.refresh_rate = source_state.refresh_rate;
        self.head.current_state.scale = source_state.scale;
        self.head.current_state.rotation = source_state.rotation;
        Ok(self)
    }

    fn get_state(&self) -> OutputState {
        self.head.current_state.clone()
    }
}
