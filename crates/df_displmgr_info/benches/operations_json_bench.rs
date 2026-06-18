// benches/operations_json_bench.rs
use criterion::{criterion_group, criterion_main, Criterion, black_box, Throughput};
use std::time::Duration;
use df_displmgr_info::{
    MonitorDetails, 
    edid_types::{
        MonitorTopology, EdidData, DeepDdcStats, MonitorCapabilities, 
        InputSource, PowerState, AudioMuteState, ChromaticityCoordinates,
        HdrMetadata, AudioCapabilities, VideoInterfaceInfo
    }
};

/// Helper function to generate populated (complex) hardware matrices
fn generate_populated_details(count: usize) -> Vec<MonitorDetails> {
    let mock_edid = EdidData {
        model_name: "Dell U2723QE".to_string(),
        manufacturer_id: "DEL".to_string(),
        product_code: 0x42C2,
        serial_number_binary: 123456789,
        serial_number_ascii: Some("DISPLAYFLOW-SER".to_string()),
        week_of_manufacture: 10,
        year_of_manufacture: 2024,
        video_interface: VideoInterfaceInfo::Unknown, 
        chromaticity: Some(ChromaticityCoordinates {
            red_x: 0.640, red_y: 0.330, green_x: 0.300, green_y: 0.600,
            blue_x: 0.150, blue_y: 0.060, white_x: 0.3127, white_y: 0.3290,
        }),
        extension_blocks: 1,
        modes: vec![],
        hdr_caps: HdrMetadata {
            supports_sdr_eotf: true, supports_hdr_traditional: true,
            supports_smpte_st2084: true, supports_hlg: true,
            max_luminance_cd_m2: Some(400.0), max_frame_average_luminance_cd_m2: Some(350.0),
            min_luminance_cd_m2: Some(0.001),
        },
        audio_caps: AudioCapabilities {
            extra_audio_descriptors_count: 0, short_audio_descriptors: vec![],
        },
    };

    let mock_ddc = DeepDdcStats {
        core_caps: MonitorCapabilities { brightness: 75, brightness_max: 100, contrast: 50, contrast_max: 100 },
        input_source: InputSource::DisplayPort1,
        power_state: PowerState::On,
        volume: Some((30, 100)),
        audio_mute: AudioMuteState::Unmuted,
        color_gains: Some((98, 95, 100)),
        horizontal_freq_hz: Some(135000),
        vertical_freq_centihz: Some(6000), 
        operating_hours: Some(1420),
        osd_language_code: Some(2),
        panel_type_code: Some(1),
    };

    (0..count).map(|i| MonitorDetails {
        target_id: i as u32,
        friendly_name: format!("Dell UltraSharp U2723QE ({})", i),
        is_active: true,
        output_tech: "DisplayPort".to_string(),
        gdi_name: format!("\\\\.\\DISPLAY{}", i + 1),
        device_path: "\\\\?\\DISPLAY#DEL42C2#7&21a11b&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}".to_string(),
        topology: Some(MonitorTopology { x: (i as i32) * 3840, y: 0, width: 3840, height: 2160, rotation: "Identity".to_string() }),
        edid: Some(mock_edid.clone()),
        ddc_stats: Some(mock_ddc.clone()),
    }).collect()
}

/// Helper function to generate minimal hardware matrices (all optional substructures set to None)
fn generate_empty_details(count: usize) -> Vec<MonitorDetails> {
    (0..count).map(|i| MonitorDetails {
        target_id: i as u32,
        friendly_name: format!("Generic Display ({})", i),
        is_active: false,
        output_tech: "Unknown".to_string(),
        gdi_name: format!("\\\\.\\DISPLAY{}", i + 1),
        device_path: String::new(),
        topology: None,
        edid: None,
        ddc_stats: None,
    }).collect()
}

fn bench_extended_serialization(c: &mut Criterion) {
    // -----------------------------------------------------------------
    // FOCUS 1: Data density comparison & throughput profiles (scaling)
    // -----------------------------------------------------------------
    let populated_1 = generate_populated_details(1);
    let populated_10 = generate_populated_details(10);
    let empty_10 = generate_empty_details(10);

    let mut group_scale = c.benchmark_group("JSON_Density_and_Scale");
    
    // Measure actual bytes-per-second throughput based on the final string size
    if let Ok(s) = serde_json::to_string(&populated_1) {
        group_scale.throughput(Throughput::Bytes(s.len() as u64));
    }
    group_scale.bench_function("Populated_Single_Monitor", |b| {
        b.iter(|| {
            let json_str = serde_json::to_string(black_box(&populated_1)).unwrap();
            black_box(json_str);
        })
    });

    if let Ok(s) = serde_json::to_string(&populated_10) {
        group_scale.throughput(Throughput::Bytes(s.len() as u64));
    }
    group_scale.bench_function("Populated_Multi_Monitor_10x", |b| {
        b.iter(|| {
            let json_str = serde_json::to_string(black_box(&populated_10)).unwrap();
            black_box(json_str);
        })
    });

    // Direct comparison: How much faster does the loop run when Serde can skip fields (skip_serializing_if)?
    if let Ok(s) = serde_json::to_string(&empty_10) {
        group_scale.throughput(Throughput::Bytes(s.len() as u64));
    }
    group_scale.bench_function("Stripped_Empty_Monitor_10x", |b| {
        b.iter(|| {
            let json_str = serde_json::to_string(black_box(&empty_10)).unwrap();
            black_box(json_str);
        })
    });
    group_scale.finish();

    // -----------------------------------------------------------------
    // FOCUS 2: Substructure granularity analysis (where is the overhead?)
    // -----------------------------------------------------------------
    let base_dataset = generate_populated_details(1);
    
    // Isolate slices (copy inner substructures)
    let topology_slice = base_dataset[0].topology.clone().unwrap();
    let edid_slice = base_dataset[0].edid.clone().unwrap();
    let ddc_slice = base_dataset[0].ddc_stats.clone().unwrap();

    let mut group_granularity = c.benchmark_group("JSON_Substructure_Overhead");
    
    group_granularity.bench_function("Component_Isolation_Topology", |b| {
        b.iter(|| {
            let json_str = serde_json::to_string(black_box(&topology_slice)).unwrap();
            black_box(json_str);
        })
    });

    group_granularity.bench_function("Component_Isolation_EdidData", |b| {
        b.iter(|| {
            let json_str = serde_json::to_string(black_box(&edid_slice)).unwrap();
            black_box(json_str);
        })
    });

    group_granularity.bench_function("Component_Isolation_DeepDdcStats", |b| {
        b.iter(|| {
            let json_str = serde_json::to_string(black_box(&ddc_slice)).unwrap();
            black_box(json_str);
        })
    });
    group_granularity.finish();
}

// Framework configuration for more stable analysis values (increased warm-up time to reduce outliers)
criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(4));
    targets = bench_extended_serialization
}
criterion_main!(benches);