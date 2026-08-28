//! Unit tests for `df_displmgr` types and traits.
//! These tests run without hardware and validate core data structures.

use df_displmgr::types::*;

#[test]
fn test_display_id_comparison() {
    let a = DisplayId("123".into());
    let b = DisplayId("123".into());
    let c = DisplayId("456".into());
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_display_id_hash() {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(DisplayId("1".into()), "first");
    map.insert(DisplayId("2".into()), "second");
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&DisplayId("1".into())), Some(&"first"));
}

#[test]
fn test_extent2d_default() {
    let e = Extent2D::default();
    assert_eq!(e.width, 0);
    assert_eq!(e.height, 0);
}

#[test]
fn test_point2d_default() {
    let p = Point2D::default();
    assert_eq!(p.x, 0);
    assert_eq!(p.y, 0);
}

#[test]
fn test_rect_default() {
    let r = Rect::default();
    assert_eq!(r.origin, Point2D { x: 0, y: 0 });
    assert_eq!(
        r.size,
        Extent2D {
            width: 0,
            height: 0
        }
    );
}

#[test]
fn test_display_rotation_default() {
    assert_eq!(DisplayRotation::default(), DisplayRotation::Rotate0);
}

#[test]
fn test_display_rotation_values() {
    // Ensure enum variants have correct discriminants for serialization
    assert_ne!(DisplayRotation::Rotate0, DisplayRotation::Rotate90);
    assert_ne!(DisplayRotation::Rotate90, DisplayRotation::Rotate180);
    assert_ne!(DisplayRotation::Rotate180, DisplayRotation::Rotate270);
}

#[test]
fn test_hdr_state_default() {
    assert_eq!(HdrState::default(), HdrState::Disabled);
}

#[test]
fn test_hdr_mode_default() {
    assert_eq!(HdrMode::default(), HdrMode::Default);
}

#[test]
fn test_output_state_default() {
    let o = OutputState::default();
    assert!(!o.enabled);
    assert!(!o.is_primary);
    assert_eq!(o.scale, 1.0);
    assert_eq!(o.refresh_rate, 0);
    assert_eq!(o.rotation, DisplayRotation::Rotate0);
    assert_eq!(o.hdr_state, HdrState::Disabled);
}

#[test]
fn test_output_state_is_landscape() {
    // Landscape (normal)
    let mut o = OutputState::default();
    o.geometry.size = Extent2D {
        width: 1920,
        height: 1080,
    };
    assert!(o.is_landscape());

    // Portrait (rotated 90)
    o.rotation = DisplayRotation::Rotate90;
    assert!(!o.is_landscape());

    // Landscape inverted (180)
    o.rotation = DisplayRotation::Rotate180;
    o.geometry.size = Extent2D {
        width: 1920,
        height: 1080,
    };
    assert!(o.is_landscape());
}

#[test]
fn test_output_state_refresh_rate_hz() {
    let mut o = OutputState {
        refresh_rate: 60000,
        ..Default::default()
    };
    assert_eq!(o.refresh_rate_hz(), 60.0);

    o.refresh_rate = 144000;
    assert!((o.refresh_rate_hz() - 144.0).abs() < f32::EPSILON);

    o.refresh_rate = 239700;
    assert!((o.refresh_rate_hz() - 239.7).abs() < 0.1);
}

#[test]
fn test_video_mode_default() {
    let m = VideoMode::default();
    assert_eq!(m.resolution, Extent2D::default());
    assert_eq!(m.refresh_rate, 0);
}

#[test]
fn test_activation_plan_default() {
    let p = ActivationPlan::default();
    assert!(p.position.is_none());
    assert!(p.resolution.is_none());
    assert!(p.rotation.is_none());
}

#[test]
fn test_activation_plan_with_values() {
    let p = ActivationPlan {
        position: Some(Point2D { x: 100, y: 200 }),
        resolution: Some(Extent2D {
            width: 2560,
            height: 1440,
        }),
        rotation: Some(DisplayRotation::Rotate90),
    };
    assert_eq!(p.position.unwrap().x, 100);
    assert_eq!(p.resolution.unwrap().width, 2560);
    assert_eq!(p.rotation.unwrap(), DisplayRotation::Rotate90);
}

#[test]
fn test_output_state_serialization() {
    let o = OutputState::default();
    let json = serde_json::to_string(&o).unwrap();
    assert!(json.contains("\"enabled\":false"));
    assert!(json.contains("\"is_primary\":false"));

    let deserialized: OutputState = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.scale, 1.0);
}

#[test]
fn test_display_identity_serialization() {
    let id = DisplayIdentity {
        id: DisplayId("123".into()),
        connector_id: ConnectorId("DP-1".into()),
        adapter_id: AdapterId("GPU0".into()),
        hardware_uuid: Some("UUID-123".into()),
        monitor_name: "Test Monitor".into(),
    };
    let json = serde_json::to_string(&id).unwrap();
    assert!(json.contains("Test Monitor"));

    let deserialized: DisplayIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, DisplayId("123".into()));
    assert_eq!(deserialized.hardware_uuid, Some("UUID-123".into()));
}

#[test]
fn test_display_id_serialization_roundtrip() {
    let id = DisplayId("42".into());
    let json = serde_json::to_string(&id).unwrap();
    let deserialized: DisplayId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, deserialized);
}

#[test]
fn test_output_state_supported_modes() {
    let o = OutputState {
        supported_modes: vec![
            VideoMode {
                resolution: Extent2D {
                    width: 1920,
                    height: 1080,
                },
                refresh_rate: 60000,
            },
            VideoMode {
                resolution: Extent2D {
                    width: 2560,
                    height: 1440,
                },
                refresh_rate: 144000,
            },
            VideoMode {
                resolution: Extent2D {
                    width: 3840,
                    height: 2160,
                },
                refresh_rate: 60000,
            },
        ],
        ..Default::default()
    };
    assert_eq!(o.supported_modes.len(), 3);
    assert_eq!(o.supported_modes[1].refresh_rate, 144000);
}

#[test]
fn test_display_id_ord() {
    let a = DisplayId("1".into());
    let b = DisplayId("2".into());
    assert!(a < b);
}
