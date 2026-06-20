/// Integration tests verifying core DDC/CI abstractions and types.
///
/// These tests confirm that the library API contracts hold without requiring
/// physical monitor hardware. Hardware-dependent tests are deliberately omitted
/// to keep CI portable.

use df_ddc::ddc_trait::Ddc;
use df_ddc::ddc_types::VcpFeature;
use df_ddc::error::DdcError;
use df_ddc::list_monitors;

/// Mock DDC backend for testing the trait interface.
struct MockDdc;

impl Ddc for MockDdc {
    fn get_vcp_feature(&self, _code: u8) -> Result<VcpFeature, DdcError> {
        Ok(VcpFeature {
            code: 0x10,
            current: 50,
            maximum: 100,
            raw: vec![],
        })
    }

    fn set_vcp_feature(&self, _code: u8, _value: u16) -> Result<(), DdcError> {
        Ok(())
    }
}

#[test]
fn test_mock_ddc_backend_get() {
    let mock = MockDdc;
    let feature = mock.get_vcp_feature(0x10).expect("mock get should succeed");
    assert_eq!(feature.current, 50);
    assert_eq!(feature.maximum, 100);
    assert_eq!(feature.code, 0x10);
}

#[test]
fn test_mock_ddc_backend_set() {
    let mock = MockDdc;
    let result = mock.set_vcp_feature(0x10, 75);
    assert!(result.is_ok(), "mock set should succeed");
}

#[test]
fn test_list_monitors_returns_vec() {
    // Verify that list_monitors is callable and returns a Vec without panic.
    let _monitors: Vec<df_ddc::ddc_trait::DisplayDevice> = list_monitors();
}

#[test]
fn test_ddc_error_variants() {
    let access = DdcError::AccessDenied;
    let comm = DdcError::CommunicationFailed {
        reason: "timeout".to_string(),
    };
    let unsupported = DdcError::UnsupportedFeature;
    let invalid = DdcError::InvalidDevice {
        path: "/dev/i2c-0".to_string(),
    };
    let backend = DdcError::BackendNotAvailable {
        details: "no monitors".to_string(),
    };

    assert!(matches!(access, DdcError::AccessDenied));
    assert!(matches!(comm, DdcError::CommunicationFailed { .. }));
    assert!(matches!(unsupported, DdcError::UnsupportedFeature));
    assert!(matches!(invalid, DdcError::InvalidDevice { .. }));
    assert!(matches!(backend, DdcError::BackendNotAvailable { .. }));
}
