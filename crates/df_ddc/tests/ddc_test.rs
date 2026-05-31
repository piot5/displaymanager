use df_ddc::ddc_trait::{DisplayDevice, DdcControl};
use df_ddc::ddc_types::{PowerState, InputSource, VcpCode};
use df_ddc::ddc_backends::ddc_debug::DebugBackend;
use df_ddc::error::DdcError;
use df_ddc::list_monitors;
use std::sync::Arc;
use std::thread;

/// 1. Extended positive test (consolidate existing logic)
#[test]
fn test_exhaustive_ddc_success_path() {
    let device = DisplayDevice {
        info: "Full Feature Monitor".to_string(),
        inner: Box::new(DebugBackend::new()),
    };

    // Validate trait helper functions
    assert!(device.inner.set_power(PowerState::Off).is_ok());
    assert!(device.inner.set_input(InputSource::Hdmi1).is_ok());

    // RGB matrix check
    let codes = [VcpCode::RedGain, VcpCode::GreenGain, VcpCode::BlueGain];
    for code in codes {
        device.inner.set_vcp_feature(code as u8, 75).unwrap();
        let (val, _) = device.inner.get_vcp_feature(code as u8).unwrap();
        assert_eq!(val, 75);
    }
    println!("LOG: Positive path for all core VCP functions succeeded.");
}

/// 2. Error-mapping test (simulates hardware failures)
struct FailBackend;
impl DdcControl for FailBackend {
    fn get_vcp_feature(&self, _: u8) -> Result<(u32, u32), DdcError> {
        Err(DdcError::CommunicationFailed { reason: "Timeout".into() })
    }
    fn set_vcp_feature(&self, _: u8, _: u32) -> Result<(), DdcError> {
        Err(DdcError::UnsupportedFeature)
    }
}

#[test]
fn test_library_error_propagation() {
    let fail_device = DisplayDevice {
        info: "Broken Monitor".to_string(),
        inner: Box::new(FailBackend),
    };

    // Check that get_capabilities correctly wraps backend errors
    let caps_result = fail_device.inner.get_capabilities();
    match caps_result {
        Err(DdcError::CommunicationFailed { .. }) => println!("LOG: Error mapping (communication) is correct."),
        _ => panic!("Expected CommunicationError was not raised"),
    }
    // Check UnsupportedFeature for set operations
    let set_result = fail_device.inner.set_power(PowerState::On);
    assert!(matches!(set_result, Err(DdcError::UnsupportedFeature)));
}

/// 3. Integrationstest: Monitor Enumeration
#[test]
fn test_system_enumeration() {
    // This test checks whether the lib.rs logic works on the current OS
    let monitors = list_monitors();
    
    println!("LOG: Found {} monitor(s) in the system.", monitors.len());
    for m in &monitors {
        println!(" - Found: {}", m.info);
        // A cautious probe run (may fail if no permissions or no DDC available)
        let _ = m.inner.get_vcp_feature(VcpCode::Brightness as u8);
    }
}

/// 4. Thread-Safety Test (Validierung der Sync/Send Implementierung)
#[test]
fn test_multithreaded_access() {
    let backend = Arc::new(DebugBackend::new());
    let mut handles = vec![];

    for i in 0..5 {
        let b = Arc::clone(&backend);
        handles.push(thread::spawn(move || {
            let _ = b.set_vcp_feature(VcpCode::Brightness as u8, i * 10);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
    
    let (final_val, _) = backend.get_vcp_feature(VcpCode::Brightness as u8).unwrap();
    println!("LOG: Thread-safe access succeeded. Last value: {}", final_val);
}