use std::{
    io,
    env,
};
use wgpu::{
    util::{DeviceExt, BufferInitDescriptor},
};
use bytemuck::{cast_slice, Pod, Zeroable};

// Ensure Point and CurveParams are 'bytemuck' compatible
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Point {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CurveParams {
    a: f32,
    b: f32,
}

pub async fn run(x_data: &[f32], y_data: &[f32]) -> io::Result<Vec<f32>> {
    env::set_var("RUST_BACKTRACE", "1");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: Default::default(),
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    // 1. Load the shader
    // Load the precompiled SPIR-V binary
    let cs_spirv = include_bytes!("shader.spv");

    // Convert the SPIR-V binary for wgpu
    let cs_data = wgpu::util::make_spirv_raw(cs_spirv);

    // Create a shader module
    let cs_module = unsafe {
        device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
            label: Some("Compute Shader"),
            source: cs_data,
        })
    };

    let point_data: Vec<Point> = x_data.iter().zip(y_data).map(|(&x, &y)| Point { x, y }).collect();
    println!("Printing input data before sending to shader:");
    for point in &point_data {
        println!("x: {}, y: {}", point.x, point.y);
    }
    let points_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Points Buffer"),
        contents: cast_slice(&point_data),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // Create a buffer to copy the results to
    let result_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Result Buffer"),
        size: (point_data.len() * std::mem::size_of::<CurveParams>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (point_data.len() * std::mem::size_of::<CurveParams>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create a bind group layout and bind group
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Point>() as _),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<CurveParams>() as _),
                },
                count: None,
            },
        ],
        label: Some("bind_group_layout"),
    });    
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &points_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of_val(&point_data) as _),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &output_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new((point_data.len() * std::mem::size_of::<CurveParams>()) as wgpu::BufferAddress),
                }),
            },
            // ... Add other binding resources as needed ...
        ],
        label: Some("bind_group"),
    });    

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &cs_module,
        entry_point: "main",
    });
    

    // Copy from the GPU output buffer to the result buffer
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(point_data.len() as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &result_buffer, 0, result_buffer.size());
        println!("About to submit compute encoder");
        queue.submit(Some(encoder.finish()));
        println!("Compute encoder submitted");
    }
    
    // Map the result buffer and read the results
    let result_slice = result_buffer.slice(..);

    // Use a channel to wait for the buffer mapping to complete
    println!("Waiting for buffer mapping");
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    result_slice.map_async(wgpu::MapMode::Read, move |result| {
        println!("Inside map_async callback");
        match result {
            Ok(()) => {
                println!("Mapping successful");
                tx.send(result).unwrap();
            },
            Err(e) => {
                eprintln!("Buffer mapping error: {:?}", e);
                tx.send(Err(e)).unwrap();
            }
        }
    });
    device.poll(wgpu::Maintain::Wait);
    rx.receive().await.unwrap().unwrap();

    let mapped_range = result_slice.get_mapped_range();
    println!("Data: {:?}", mapped_range);

    // Process the mapped data
    let result_vec: Vec<CurveParams> = bytemuck::cast_slice(&mapped_range).to_vec();

    // Unmap the buffer
    result_buffer.unmap();

    // Iterate and print the results
    for (index, params) in result_vec.iter().enumerate() {
        println!("Result {}: a = {}, b = {}", index, params.a, params.b);
    }

    Ok(result_vec.iter().map(|params| params.a).collect())
}