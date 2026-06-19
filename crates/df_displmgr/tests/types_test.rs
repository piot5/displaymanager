use df_displmgr::types::{
    ActivationPlan, AdapterId, ConnectorId, DisplayId, DisplayIdentity, DisplayRotation, Extent2D,
    HdrMode, HdrState, OutputState, Point2D, Rect, VideoMode,
};

/// Test DisplayId creation and display
#[test]
fn test_display_id_creation() {
    let id = DisplayId("HDMI-1".to_string());
    assert_eq!(id.to_string(), "HDMI-1");
    assert_eq!(id.0, "HDMI-1");
}

/// Test DisplayId comparison and ordering
#[test]
fn test_display_id_comparison() {
    let id1 = DisplayId("HDMI-1".to_string());
    let id2 = DisplayId("HDMI-2".to_string());
    let id3 = DisplayId("HDMI-1".to_string());

    assert!(id1 < id2);
    assert!(id2 > id1);
    assert_eq!(id1, id3);
    assert_ne!(id1, id2);
}

/// Test DisplayId hashing
#[test]
fn test_display_id_hash() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert(DisplayId("HDMI-1".to_string()), 42);
    map.insert(DisplayId("DP-1".to_string()), 100);

    assert_eq!(map.get(&DisplayId("HDMI-1".to_string())), Some(&42));
    assert_eq!(map.get(&DisplayId("DP-1".to_string())), Some(&100));
}

/// Test DisplayId serialization
#[test]
fn test_display_id_serialization_roundtrip() {
    let id = DisplayId("HDMI-1".to_string());
    let json = serde_json::to_string(&id).unwrap();
    let parsed: DisplayId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, parsed);
}

/// Test ConnectorId
#[test]
fn test_connector_id() {
    let conn = ConnectorId("DP-1".to_string());
    assert_eq!(conn.to_string(), "DP-1");
    assert_eq!(conn.0, "DP-1");
}

/// Test AdapterId
#[test]
fn test_adapter_id() {
    let adapter = AdapterId("card0".to_string());
    assert_eq!(adapter.to_string(), "card0");
    assert_eq!(adapter.0, "card0");
}

/// Test DisplayIdentity
#[test]
fn test_display_identity_serialization() {
    let identity = DisplayIdentity {
        id: DisplayId("HDMI-1".to_string()),
        connector_id: ConnectorId("HDMI-A-1".to_string()),
        adapter_id: AdapterId("card0".to_string()),
        hardware_uuid: Some("uuid-1234".to_string()),
        monitor_name: "Test Monitor".to_string(),
    };

    let json = serde_json::to_string(&identity).unwrap();
    let parsed: DisplayIdentity = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, identity.id);
    assert_eq!(parsed.connector_id, identity.connector_id);
    assert_eq!(parsed.adapter_id, identity.adapter_id);
    assert_eq!(parsed.hardware_uuid, identity.hardware_uuid);
    assert_eq!(parsed.monitor_name, identity.monitor_name);
}

/// Test DisplayRotation variants
#[test]
fn test_display_rotation_values() {
    assert_eq!(DisplayRotation::Rotate0 as u32, 0);
    assert_eq!(DisplayRotation::Rotate90 as u32, 1);
    assert_eq!(DisplayRotation::Rotate180 as u32, 2);
    assert_eq!(DisplayRotation::Rotate270 as u32, 3);
}

/// Test DisplayRotation default
#[test]
fn test_display_rotation_default() {
    let rot = DisplayRotation::default();
    assert_eq!(rot, DisplayRotation::Rotate0);
}

/// Test HdrState variants
#[test]
fn test_hdr_state_variants() {
    assert_eq!(HdrState::Enabled, HdrState::Enabled);
    assert_eq!(HdrState::Disabled, HdrState::Disabled);
    assert_ne!(HdrState::Enabled, HdrState::Disabled);
}

/// Test HdrState default
#[test]
fn test_hdr_state_default() {
    let state = HdrState::default();
    assert_eq!(state, HdrState::Disabled);
}

/// Test HdrMode variants
#[test]
fn test_hdr_mode_variants() {
    assert_eq!(HdrMode::Default, HdrMode::Default);
    assert_eq!(HdrMode::Cinema, HdrMode::Cinema);
    assert_eq!(HdrMode::Game, HdrMode::Game);
}

/// Test HdrMode default
#[test]
fn test_hdr_mode_default() {
    let mode = HdrMode::default();
    assert_eq!(mode, HdrMode::Default);
}

/// Test Point2D
#[test]
fn test_point2d_default() {
    let point = Point2D::default();
    assert_eq!(point.x, 0);
    assert_eq!(point.y, 0);
}

/// Test Point2D creation
#[test]
fn test_point2d_creation() {
    let point = Point2D { x: 100, y: 200 };
    assert_eq!(point.x, 100);
    assert_eq!(point.y, 200);
}

/// Test Extent2D
#[test]
fn test_extent2d_default() {
    let extent = Extent2D::default();
    assert_eq!(extent.width, 0);
    assert_eq!(extent.height, 0);
}

/// Test Extent2D creation
#[test]
fn test_extent2d_creation() {
    let extent = Extent2D {
        width: 1920,
        height: 1080,
    };
    assert_eq!(extent.width, 1920);
    assert_eq!(extent.height, 1080);
}

/// Test Rect
#[test]
fn test_rect_default() {
    let rect = Rect::default();
    assert_eq!(rect.origin.x, 0);
    assert_eq!(rect.origin.y, 0);
    assert_eq!(rect.size.width, 0);
    assert_eq!(rect.size.height, 0);
}

/// Test Rect creation
#[test]
fn test_rect_creation() {
    let rect = Rect {
        origin: Point2D { x: 0, y: 0 },
        size: Extent2D {
            width: 1920,
            height: 1080,
        },
    };
    assert_eq!(rect.origin.x, 0);
    assert_eq!(rect.size.width, 1920);
}

/// Test VideoMode
#[test]
fn test_video_mode_default() {
    let mode = VideoMode::default();
    assert_eq!(mode.resolution.width, 0);
    assert_eq!(mode.resolution.height, 0);
    assert_eq!(mode.refresh_rate, 0);
}

/// Test VideoMode creation
#[test]
fn test_video_mode_creation() {
    let mode = VideoMode {
        resolution: Extent2D {
            width: 2560,
            height: 1440,
        },
        refresh_rate: 144000,
    };
    assert_eq!(mode.resolution.width, 2560);
    assert_eq!(mode.refresh_rate, 144000);
}

/// Test OutputState default
#[test]
fn test_output_state_default() {
    let state = OutputState::default();
    assert!(!state.enabled);
    assert!(!state.is_primary);
    assert_eq!(state.scale, 1.0);
    assert_eq!(state.refresh_rate, 0);
}

/// Test OutputState is_landscape
#[test]
fn test_output_state_is_landscape() {
    let mut state = OutputState::default();
    state.geometry.size = Extent2D {
        width: 1920,
        height: 1080,
    };

    // Rotate0: width >= height = landscape
    state.rotation = DisplayRotation::Rotate0;
    assert!(state.is_landscape());

    // Rotate90: width < height (after rotation) = not landscape
    state.rotation = DisplayRotation::Rotate90;
    assert!(!state.is_landscape());

    // Rotate180: width >= height = landscape
    state.rotation = DisplayRotation::Rotate180;
    assert!(state.is_landscape());

    // Rotate270: width < height (after rotation) = not landscape
    state.rotation = DisplayRotation::Rotate270;
    assert!(!state.is_landscape());
}

/// Test OutputState is_landscape with portrait
#[test]
fn test_output_state_is_portrait() {
    let mut state = OutputState::default();
    state.geometry.size = Extent2D {
        width: 1080,
        height: 1920,
    };

    // Rotate0: width < height = not landscape (portrait)
    state.rotation = DisplayRotation::Rotate0;
    assert!(!state.is_landscape());

    // Rotate90: width >= height (after rotation) = landscape
    state.rotation = DisplayRotation::Rotate90;
    assert!(state.is_landscape());
}

/// Test OutputState refresh_rate_hz
#[test]
fn test_output_state_refresh_rate_hz() {
    let mut state = OutputState::default();
    state.refresh_rate = 144000; // 144 Hz in mHz

    assert_eq!(state.refresh_rate_hz(), 144.0);
}

/// Test OutputState serialization
#[test]
fn test_output_state_serialization() {
    let state = OutputState {
        identity: DisplayIdentity {
            id: DisplayId("HDMI-1".to_string()),
            connector_id: ConnectorId("HDMI-A-1".to_string()),
            adapter_id: AdapterId("card0".to_string()),
            hardware_uuid: None,
            monitor_name: "Test".to_string(),
        },
        geometry: Rect {
            origin: Point2D { x: 0, y: 0 },
            size: Extent2D {
                width: 1920,
                height: 1080,
            },
        },
        refresh_rate: 60000,
        rotation: DisplayRotation::Rotate0,
        hdr_state: HdrState::Disabled,
        hdr_mode: HdrMode::Default,
        scale: 1.0,
        native_resolution: None,
        supported_modes: vec![],
        enabled: true,
        is_primary: true,
    };

    let json = serde_json::to_string(&state).unwrap();
    let parsed: OutputState = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.identity.id, state.identity.id);
    assert_eq!(parsed.enabled, state.enabled);
    assert_eq!(parsed.is_primary, state.is_primary);
}

/// Test ActivationPlan default
#[test]
fn test_activation_plan_default() {
    let plan = ActivationPlan::default();
    assert!(plan.position.is_none());
    assert!(plan.resolution.is_none());
    assert!(plan.rotation.is_none());
}

/// Test ActivationPlan with values
#[test]
fn test_activation_plan_with_values() {
    let plan = ActivationPlan {
        position: Some(Point2D { x: 1920, y: 0 }),
        resolution: Some(Extent2D {
            width: 1920,
            height: 1080,
        }),
        rotation: Some(DisplayRotation::Rotate90),
    };

    assert!(plan.position.is_some());
    assert_eq!(plan.position.unwrap().x, 1920);
    assert!(plan.resolution.is_some());
    assert_eq!(plan.resolution.unwrap().width, 1920);
    assert!(plan.rotation.is_some());
    assert_eq!(plan.rotation.unwrap(), DisplayRotation::Rotate90);
}

/// Test video mode fields
#[test]
fn test_monitor_mode_fields() {
    let mode = VideoMode {
        resolution: Extent2D {
            width: 3840,
            height: 2160,
        },
        refresh_rate: 120000,
    };

    assert_eq!(mode.resolution.width, 3840);
    assert_eq!(mode.resolution.height, 2160);
    assert_eq!(mode.refresh_rate, 120000);
}
