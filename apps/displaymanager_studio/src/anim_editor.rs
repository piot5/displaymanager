// Allow unused/dead code during active development to keep logical structure intact
#![allow(dead_code)]
use anyhow::Context;
use raw_window_handle::{
    HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use std::collections::HashMap;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use eframe::egui;

// Integration of the central project types
use crate::loader::{FlowPackage, AppState};

// --- GPU CORE & TYPES ---

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub offset: [f32; 2],
    pub scale: f32,
    pub time: f32,
    pub logic_params: [f32; 4],
    pub feature_flags: [f32; 4],
}

pub struct GpuCore {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub pipelines: HashMap<String, wgpu::RenderPipeline>,
}

impl GpuCore {
    pub async fn new(instance: &wgpu::Instance, shader_src: &str, target_shader: &str) -> anyhow::Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .context("No GPU adapter found")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FlowShader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout, &uniform_layout],
            push_constant_ranges: &[],
        });

        let mut pipelines = HashMap::new();
        for ep in ["fs_default", target_shader] {
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(ep),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: ep,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });
            pipelines.insert(ep.to_string(), pipeline);
        }

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            device,
            queue,
            bind_group_layout,
            uniform_layout,
            sampler,
            pipelines,
        })
    }

    pub unsafe fn fetch_worker_w() -> HWND {
        let progman = FindWindowW(w!("Progman"), None);
        let _ = SendMessageTimeoutW(progman, 0x052C, WPARAM(0), LPARAM(0), SMTO_NORMAL, 1000, None);
        let mut workerw = HWND(0);

        unsafe extern "system" fn enum_proc(h: HWND, l: LPARAM) -> BOOL {
            if FindWindowExW(h, None, w!("SHELLDLL_DefView"), None).0 != 0 {
                let out_ptr = l.0 as *mut HWND;
                *out_ptr = FindWindowExW(None, h, w!("WorkerW"), None);
            }
            true.into()
        }

        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut workerw as *mut _ as isize));
        workerw
    }
}

// --- SYSTEM INTEGRATION ---

pub struct WindowWrapper(pub HWND);

impl HasWindowHandle for WindowWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = Win32WindowHandle::new(std::num::NonZeroIsize::new(self.0 .0 as isize).unwrap());
        unsafe { Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(handle))) }
    }
}

impl HasDisplayHandle for WindowWrapper {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, HandleError> {
        unsafe {
            Ok(raw_window_handle::DisplayHandle::borrow_raw(
                raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new()),
            ))
        }
    }
}

pub struct MonitorWindow {
    pub hwnd: HWND,
    pub surface: wgpu::Surface<'static>,
    pub texture_bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

pub unsafe fn init_windows(
    gpu: &GpuCore,
    inst: &wgpu::Instance,
    class: windows::core::PCWSTR,
    hi: HINSTANCE,
    is_wp: bool,
    _flow: &FlowPackage,
) -> Vec<MonitorWindow> {
    let mut rects: Vec<RECT> = Vec::new();

    unsafe extern "system" fn monitor_enum(_: HMONITOR, _: HDC, r: *mut RECT, d: LPARAM) -> BOOL {
        let rects = &mut *(d.0 as *mut Vec<RECT>);
        rects.push(*r);
        true.into()
    }

    let _ = EnumDisplayMonitors(HDC(0), None, Some(monitor_enum), LPARAM(&mut rects as *mut _ as isize));

    let workerw = if is_wp { GpuCore::fetch_worker_w() } else { HWND(0) };
    let mut windows = Vec::new();

    for &r in rects.iter() {
        let (w, h) = ((r.right - r.left) as u32, (r.bottom - r.top) as u32);
        let hwnd = CreateWindowExW(
            if is_wp { WINDOW_EX_STYLE(0) } else { WS_EX_TOPMOST | WS_EX_TOOLWINDOW },
            class,
            w!(""),
            if is_wp { WS_CHILD | WS_VISIBLE } else { WS_POPUP | WS_VISIBLE },
            if is_wp { 0 } else { r.left },
            if is_wp { 0 } else { r.top },
            w as i32,
            h as i32,
            if is_wp { workerw } else { HWND(0) },
            None,
            hi,
            None,
        );

        // Dummy buffer for initialization
        let _buf = vec![0u8; (w * h * 4) as usize];
        
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        if let Ok(surface) = inst.create_surface(WindowWrapper(hwnd)) {
            surface.configure(
                &gpu.device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    width: w,
                    height: h,
                    present_mode: wgpu::PresentMode::Fifo,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                },
            );

            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let t_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &gpu.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&gpu.sampler) },
                ],
            });
            
            let u_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: std::mem::size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            
            let u_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &gpu.uniform_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: u_buf.as_entire_binding() }],
            });

            windows.push(MonitorWindow {
                hwnd,
                surface,
                texture_bind_group: t_bg,
                uniform_buffer: u_buf,
                uniform_bind_group: u_bg,
            });
        }
    }
    windows
}

pub unsafe extern "system" fn wnd_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if m == WM_DESTROY {
        PostQuitMessage(0);
        LRESULT(0)
    } else {
        DefWindowProcW(h, m, w, l)
    }
}

// UI function for the studio editor tab
pub fn render_anim_editor(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Anim Engine Studio");
    ui.separator();
    ui.label("Echtzeit-Shader-Editor:");
    
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut state.editor_content)
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .desired_width(f32::INFINITY)
                .lock_focus(true),
        );
    });
}