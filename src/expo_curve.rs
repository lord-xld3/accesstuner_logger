use std::{io, env};
use wgpu::util::{DeviceExt, BufferInitDescriptor};
use bytemuck::{cast_slice, Pod, Zeroable};
use naga;

// Ensure Point and CurveParams are 'bytemuck' compatible
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Point {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CurveParams {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

pub async fn run(x_data: &[f32], y_data: &[f32]) -> io::Result<(f32, f32, f32)> {
    env::set_var("RUST_BACKTRACE", "1");
    println!("x_data: {:?}", x_data);
    println!("y_data: {:?}", y_data);
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
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::default(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                limits: wgpu::Limits::default(),
            },
            None, // Trace path
        )
        .await
        .unwrap();

    device.start_capture();
    // 1. Load the shader
    let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Glsl {
            shader: include_str!("shader.comp").into(),
            stage: naga::ShaderStage::Compute,
            defines: naga::FastHashMap::default(),
        },
    });

    // Create buffers for the input data
    let x_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("X Buffer"),
        contents: cast_slice(x_data),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let y_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Y Buffer"),
        contents: cast_slice(y_data),
        usage: wgpu::BufferUsages::STORAGE,
    });
    
    // Create a buffer to hold the results
    let results_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Results Buffer"),
        size: (10000 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create a buffer to read the results from the GPU
    let read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Read Buffer"),
        size: (10000 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create a bind group layout and bind group
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<f32>() as _),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<f32>() as _),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new((10000 * std::mem::size_of::<f32>()) as _),
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
                    buffer: &x_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of_val(x_data) as _),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &y_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of_val(y_data) as _),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &results_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new((10000 * std::mem::size_of::<f32>()) as _),
                }),
            },
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
            pass.dispatch_workgroups(100, 100, 1);
        }
        // Copy the result from the result buffer to the read buffer
        encoder.copy_buffer_to_buffer(&results_buffer, 0, &read_buffer, 0, read_buffer.size());
        println!("About to submit compute encoder");
        queue.submit(Some(encoder.finish()));
        println!("Compute encoder submitted");
    }

    // Map the read buffer and read the results
    let result_slice = read_buffer.slice(..);

    // Use a channel to wait for the buffer mapping to complete
    println!("Waiting for buffer mapping");
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    result_slice.map_async(wgpu::MapMode::Read, move |result| {
        println!("Inside map_async callback");
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.receive().await.unwrap().unwrap();

    // Process the mapped data
    let mapped_range = result_slice.get_mapped_range();
    let result_vec: Vec<f32> = bytemuck::cast_slice(&mapped_range).to_vec();
    // Drop the mapped view explicitly
    drop(mapped_range);

    // Find the minimum MSE and the corresponding a and n
    let mut min_mse = 1e20;
    let mut best_a = 0.0;
    let mut best_n = 0.0;

    // Find the best_a with the lowest MSE
    for i in 0..100 {
        let mse = result_vec[i];
        let a = 1.62 + 0.01 * i as f32;
        if mse < min_mse {
            min_mse = mse;
            best_a = a;
        }
    }

    // Find the best_n with the best_a and lowest MSE
    min_mse = 1e20;
    for i in 0..100 {
        let index = i * 100 + best_a as usize;
        let mse = result_vec[index];
        let n = 3.3 + 0.001 * i as f32;
        if mse < min_mse {
            min_mse = mse;
            best_n = n;
        }
    }

    println!("Optimized Coefficient (a): {}, Optimized Exponent (n): {}, Minimum Mean Squared Error (MSE): {}", best_a, best_n, min_mse);
    device.stop_capture();
    Ok((best_a, best_n, min_mse))
}