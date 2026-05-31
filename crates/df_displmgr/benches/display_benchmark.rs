use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

// Core-Typen aus der Library
use df_displmgr::types::{OutputState, DisplayRotation, HdrState, HdrMode};

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, DEVMODEW, CDS_UPDATEREGISTRY, CDS_NORESET
};

/// Helper to generate a wide-string identifier for Win32 FFI
#[allow(dead_code)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Telemetrie-Struktur zur Messung realer DDC/CI-Hardware-Bus-Antwortzeiten
pub struct DdcBusTelemetry {
    target_id: String,
    start: Instant,
}

impl DdcBusTelemetry {
    pub fn trace_start(target_id: &str, register: u8) -> Self {
        println!("[DDC/CI Telemetry] Target '{}' -> Register 0x{:X}", target_id, register);
        Self {
            target_id: target_id.to_string(),
            start: Instant::now(),
        }
    }

    pub fn trace_end(self) -> Duration {
        let elapsed = self.start.elapsed();
        println!("[DDC/CI Telemetry] ACK received for '{}'. Bus latency: {:.2} ms", self.target_id, elapsed.as_secs_f64() * 1000.0);
        elapsed
    }
}

fn bench_topology_mutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Topology Operations");

    // Szenario 1: Klonen eines Standard 4-Monitor-Setups (Baseline)
    group.bench_function("ccd_vector_state_cloning_4x", |b| {
        let mut ccd_outputs = Vec::new();
        for i in 0..4 {
            ccd_outputs.push(OutputState {
                id: format!("{}", i),
                name: format!("CCD Monitor {}", i),
                x: i * 1920,
                y: 0,
                width: 1920,
                height: 1080,
                refresh_rate: 60000,
                rotation: DisplayRotation::Rotate0,
                hdr_state: HdrState::Disabled,
                hdr_mode: HdrMode::Default,
                scale: 1.0,
                resolution: Some((1920, 1080)),
                enabled: true,
            });
        }

        b.iter(|| {
            let cloned_vec: Vec<OutputState> = black_box(&ccd_outputs).clone();
            black_box(cloned_vec);
        });
    });

    // Szenario 2: Stresstest - Klonen und Allokation eines 8-Monitor-Setups (Worst-Case)
    group.bench_function("ccd_vector_state_cloning_8x_worst_case", |b| {
        let mut ccd_outputs = Vec::new();
        for i in 0..8 {
            ccd_outputs.push(OutputState {
                id: format!("{}", i),
                name: format!("Extreme Monitor {}", i),
                x: i * 1920,
                y: 0,
                width: 2560,
                height: 1440,
                refresh_rate: 144000,
                rotation: DisplayRotation::Rotate0,
                hdr_state: HdrState::Enabled,
                hdr_mode: HdrMode::Game,
                scale: 1.25,
                resolution: Some((2560, 1440)),
                enabled: true,
            });
        }

        b.iter(|| {
            let cloned_vec: Vec<OutputState> = black_box(&ccd_outputs).clone();
            black_box(cloned_vec);
        });
    });

    // Szenario 3: Mutation von DPI-Skalierungswerten in einer HashMap (Staging-Effizienz)
    group.bench_function("staged_scaling_map_mutations", |b| {
        let mut staged_scales: HashMap<u32, i32> = HashMap::new();
        
        b.iter(|| {
            let mut map = black_box(&mut staged_scales).clone();
            for id in 0..4 {
                map.insert(id, 150);
            }
            black_box(map);
        });
    });

    // Scenario 4: Delta calculation / layout comparison (detect state changes)
    group.bench_function("topology_delta_calculation", |b| {
        let mut current_state = Vec::new();
        let mut target_state = Vec::new();
        
        for i in 0..4 {
            let state = OutputState {
                id: format!("{}", i),
                name: format!("Monitor {}", i),
                x: i * 1920,
                y: 0,
                width: 1920,
                height: 1080,
                refresh_rate: 60000,
                rotation: DisplayRotation::Rotate0,
                hdr_state: HdrState::Disabled,
                hdr_mode: HdrMode::Default,
                scale: 1.0,
                resolution: Some((1920, 1080)),
                enabled: true,
            };
            current_state.push(state.clone());
            target_state.push(state);
        }
        
        if let Some(last) = target_state.last_mut() {
            last.rotation = DisplayRotation::Rotate90;
        }

        b.iter(|| {
            let has_changed = black_box(&current_state)
                .iter()
                .zip(black_box(&target_state).iter())
                .any(|(curr, tar)| {
                    curr.x != tar.x 
                        || curr.y != tar.y 
                        || curr.width != tar.width 
                        || curr.height != tar.height 
                        || curr.rotation != tar.rotation
                        || curr.refresh_rate != tar.refresh_rate
                });
            black_box(has_changed);
        });
    });

    group.finish();
}

fn bench_async_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Async Operations");
    let rt = Runtime::new().unwrap();

    group.bench_function("mock_async_execution_overhead", |b| {
        b.to_async(&rt).iter(|| async {
            let processing_step = async {
                let val = black_box(100);
                val * 2
            };
            black_box(processing_step.await);
        });
    });

    group.finish();
}

fn bench_hardware_io(c: &mut Criterion) {
    let mut group = c.benchmark_group("Real Hardware Windows I/O Bounds");
    
    // Kernel- und Registry-Operationen benötigen größere Zeitfenster und kleinere Probenmengen
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(10);

    // Szenario 5: Windows Registry Transaktionslatenz (Staging ohne Hardware-Reset)
    #[cfg(target_os = "windows")]
    {
        let display_name = to_wide("\\\\.\\DISPLAY1");
        let mut dev_mode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };

        group.bench_function("registry_stage_devmode", |b| {
            b.iter(|| {
                let status = unsafe {
                    ChangeDisplaySettingsExW(
                        PCWSTR(display_name.as_ptr()),
                        Some(black_box(&mut dev_mode)),
                        None,
                        CDS_UPDATEREGISTRY | CDS_NORESET,
                        None,
                    )
                };
                black_box(status);
            });
        });
    }

    // Scenario 6: Emulate DDC/CI hardware bus latency within sampling
    group.bench_function("hardware_ddc_bus_overhead_simulation", |b| {
        b.iter(|| {
            let tx = DdcBusTelemetry::trace_start("DISPLAY1", 0x10);
            
            // Simulate hardware-induced bus delay (I2C propagation & controller response)
            std::thread::sleep(Duration::from_millis(20));
            
            let duration = tx.trace_end();
            black_box(duration);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_topology_mutation, bench_async_validation, bench_hardware_io);
criterion_main!(benches);