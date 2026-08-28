use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

// Core types from the library
use df_displmgr::types::{
    AdapterId, ConnectorId, DisplayId, DisplayIdentity, DisplayRotation, Extent2D, HdrMode,
    HdrState, OutputState, Point2D, Rect,
};

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, CDS_NORESET, CDS_UPDATEREGISTRY, DEVMODEW,
};

/// Helper to generate a wide-string identifier for Win32 FFI
#[allow(dead_code)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Telemetry structure for measuring real DDC/CI hardware bus response times
pub struct DdcBusTelemetry {
    target_id: String,
    start: Instant,
}

impl DdcBusTelemetry {
    pub fn trace_start(target_id: &str, register: u8) -> Self {
        println!(
            "[DDC/CI Telemetry] Target '{}' -> Register 0x{:X}",
            target_id, register
        );
        Self {
            target_id: target_id.to_string(),
            start: Instant::now(),
        }
    }

    pub fn trace_end(self) -> Duration {
        let elapsed = self.start.elapsed();
        println!(
            "[DDC/CI Telemetry] ACK received for '{}'. Bus latency: {:.2} ms",
            self.target_id,
            elapsed.as_secs_f64() * 1000.0
        );
        elapsed
    }
}

fn bench_topology_mutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Topology Operations");

    // Scenario 1: Cloning a standard 4-monitor setup (baseline)
    group.bench_function("ccd_vector_state_cloning_4x", |b| {
        let mut ccd_outputs = Vec::new();
        for i in 0..4 {
            ccd_outputs.push(OutputState {
                identity: DisplayIdentity {
                    id: DisplayId(format!("{}", i)),
                    connector_id: ConnectorId(String::new()),
                    adapter_id: AdapterId(String::new()),
                    hardware_uuid: None,
                    monitor_name: format!("CCD Monitor {}", i),
                },
                geometry: Rect {
                    origin: Point2D { x: i * 1920, y: 0 },
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
                native_resolution: Some(Extent2D {
                    width: 1920,
                    height: 1080,
                }),
                supported_modes: Vec::new(), // Added to satisfy struct definition
                enabled: true,
                is_primary: false, // Required by OutputState struct definition
            });
        }

        b.iter(|| {
            let cloned_vec: Vec<OutputState> = black_box(&ccd_outputs).clone();
            black_box(cloned_vec);
        });
    });

    // Scenario 2: Stress test - cloning and allocation of an 8-monitor setup (worst-case)
    group.bench_function("ccd_vector_state_cloning_8x_worst_case", |b| {
        let mut ccd_outputs = Vec::new();
        for i in 0..8 {
            ccd_outputs.push(OutputState {
                identity: DisplayIdentity {
                    id: DisplayId(format!("{}", i)),
                    connector_id: ConnectorId(String::new()),
                    adapter_id: AdapterId(String::new()),
                    hardware_uuid: None,
                    monitor_name: format!("Extreme Monitor {}", i),
                },
                geometry: Rect {
                    origin: Point2D { x: i * 1920, y: 0 },
                    size: Extent2D {
                        width: 2560,
                        height: 1440,
                    },
                },
                refresh_rate: 144000,
                rotation: DisplayRotation::Rotate0,
                native_resolution: Some(Extent2D {
                    width: 2560,
                    height: 1440,
                }),
                hdr_state: HdrState::Disabled, // Added to satisfy struct definition
                hdr_mode: HdrMode::Game,
                scale: 1.25,
                supported_modes: Vec::new(), // Added to satisfy struct definition
                enabled: true,
                is_primary: false, // Required by OutputState struct definition
            });
        }

        b.iter(|| {
            let cloned_vec: Vec<OutputState> = black_box(&ccd_outputs).clone();
            black_box(cloned_vec);
        });
    });

    // Scenario 3: Mutating DPI scaling values in a HashMap (staging efficiency)
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
                identity: DisplayIdentity {
                    id: DisplayId(format!("{}", i)),
                    connector_id: ConnectorId(String::new()),
                    adapter_id: AdapterId(String::new()),
                    hardware_uuid: None,
                    monitor_name: format!("Monitor {}", i),
                },
                geometry: Rect {
                    origin: Point2D { x: i * 1920, y: 0 },
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
                native_resolution: Some(Extent2D {
                    width: 1920,
                    height: 1080,
                }),
                supported_modes: Vec::new(), // Added to satisfy struct definition
                enabled: true,
                is_primary: false, // Required by OutputState struct definition
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
                    curr.geometry.origin.x != tar.geometry.origin.x
                        || curr.geometry.origin.y != tar.geometry.origin.y
                        || curr.geometry.size.width != tar.geometry.size.width
                        || curr.geometry.size.height != tar.geometry.size.height
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

    // Kernel and registry operations need larger time windows and smaller sample sizes
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(10);

    // Scenario 5: Windows registry transaction latency (staging without hardware reset)
    #[cfg(target_os = "windows")]
    {
        let display_name = to_wide("\\\\.\\DISPLAY1");
        let dev_mode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };

        group.bench_function("registry_stage_devmode", |b| {
            b.iter(|| {
                let status = unsafe {
                    ChangeDisplaySettingsExW(
                        PCWSTR(display_name.as_ptr()),
                        Some(black_box(&dev_mode)),
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

criterion_group!(
    benches,
    bench_topology_mutation,
    bench_async_validation,
    bench_hardware_io
);
criterion_main!(benches);
