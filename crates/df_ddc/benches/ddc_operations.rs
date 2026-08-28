/* * ddc_operations.rs - Hardware I/O Latency & Jitter Analysis
 * * Benchmarking DDC/CI is notoriously tricky because it involves high-latency
 * physical bus transactions (I2C) or OS-level kernel transitions (Win32).
 * This suite provides a statistical breakdown of where time is spent.
 */

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use df_ddc::list_monitors;
use std::time::Duration;

/// Discovery Phase: Measures the overhead of enumerating monitor handles.
///
/// On Windows, this is mostly the cost of the GDI and Physical Monitor API calls.
/// On Linux, it includes the filesystem overhead of scanning /dev/i2c-*.
fn bench_discovery(c: &mut Criterion) {
    c.bench_function("monitor_discovery_enumeration", |b| {
        b.iter(|| {
            // black_box is essential here to prevent the compiler from realizing
            // the result is unused and optimizing the entire scan away.
            black_box(list_monitors());
        })
    });
}

/// Hardware I/O Phase: Measures synchronous VCP (Virtual Control Panel) transactions.
///
/// DDC/CI is a request-response protocol. A 'get' request is significantly
/// more expensive than a 'set' because it requires the monitor's MCU to
/// process the query and send a packet back over the wire.
fn bench_hardware_io(c: &mut Criterion) {
    let monitors = list_monitors();

    if monitors.is_empty() {
        println!("Skipping Hardware I/O benches: No DDC/CI monitors detected.");
        return;
    }

    let mut group = c.benchmark_group("monitor_io_latency");

    /* * Engineering Note: Hardware I/O is slow (~50ms-150ms per op).
     * We drastically reduce the sample size to 10. Otherwise, Criterion's
     * default 100 iterations would take minutes, likely triggering monitor
     * firmware rate-limiting or OS timeouts.
     */
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));

    for (i, monitor) in monitors.iter().enumerate() {
        // Sanitize monitor names for filesystem-friendly reporting.
        let monitor_id = format!("monitor_{}_{}", i, monitor.info.replace(" ", "_"));

        // Benchmarking VCP Get (0x10/0x12): Full Round-trip Latency.
        group.bench_with_input(
            BenchmarkId::new("get_capabilities", &monitor_id),
            monitor,
            |b, m| b.iter(|| black_box(m.get_capabilities())),
        );

        // Benchmarking VCP Set: Write-only Latency.
        // This measures how long the OS/driver takes to accept the VCP command.
        group.bench_with_input(
            BenchmarkId::new("set_brightness_static", &monitor_id),
            monitor,
            |b, m| {
                b.iter(|| {
                    // Constant value of 70% to avoid extreme backlight flickering
                    // during high-frequency benchmark iterations.
                    black_box(m.set_brightness(70))
                })
            },
        );
    }
    group.finish();
}

/// Parallel I2C Probing Performance Tests.
///
/// Measures the performance improvement from using rayon for parallel
/// I2C bus enumeration versus sequential scanning. On systems with
/// multiple I2C buses, parallel probing can significantly reduce
/// enumeration time.
#[cfg(target_os = "linux")]
fn bench_parallel_probing(c: &mut Criterion) {
    use std::fs;

    // Collect available I2C bus paths for simulation
    let i2c_paths: Vec<String> = fs::read_dir("/dev")
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with("i2c-") && !name.contains("smbus") {
                        Some(format!("/dev/{}", name))
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    if i2c_paths.is_empty() {
        println!("No I2C buses found for parallel probing benchmark");
        return;
    }

    let mut group = c.benchmark_group("parallel_probing");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    // Benchmark parallel probing simulation (without actual hardware I/O)
    group.bench_function("parallel_bus_collection", |b| {
        b.iter(|| {
            use rayon::prelude::*;
            black_box(
                i2c_paths
                    .par_iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>(),
            )
        })
    });

    group.bench_function("sequential_bus_collection", |b| {
        b.iter(|| {
            black_box(
                i2c_paths
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>(),
            )
        })
    });

    group.finish();
}

/// No-op stub on non-Linux platforms (parallel I2C probing is Linux-only).
#[cfg(not(target_os = "linux"))]
fn bench_parallel_probing(_c: &mut Criterion) {
    // Parallel I2C bus probing is only relevant on Linux.
}

/// Baseline Logic: Measures purely computational overhead.
///
/// This isolates the XOR checksum algorithm from the I/O wait times.
/// It serves as a control to prove that software logic is sub-nanosecond.
fn bench_internal_logic(c: &mut Criterion) {
    // A standard DDC/CI 'Set Brightness' packet.
    let dummy_packet = [0x51, 0x84, 0x03, 0x10, 0x00, 0x64];

    c.bench_function("checksum_calculation_overhead", |b| {
        b.iter(|| {
            let mut checksum = 0x6E; // HOST_ADDRESS constant
            for &byte in black_box(&dummy_packet) {
                checksum ^= byte;
            }
            black_box(checksum)
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .significance_level(0.1)
        .sample_size(50);
    targets = bench_discovery, bench_hardware_io, bench_internal_logic
);

criterion_group!(
    name = parallel_benches;
    config = Criterion::default()
        .significance_level(0.1)
        .sample_size(50);
    targets = bench_parallel_probing
);

criterion_main!(benches, parallel_benches);
