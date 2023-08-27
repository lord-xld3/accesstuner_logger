use std::io;
use wgpu::util::{DeviceExt, BufferInitDescriptor};
use bytemuck::cast_slice;
use naga;

const PRECISION: u32 = 4096;
const RANGE: f32 = 8.0;

pub async fn run(x_data: &[f32], y_data: &[f32]) -> io::Result<(f32, f32)> {
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
        size: (PRECISION as usize * PRECISION as usize * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create a buffer to read the results from the GPU
    let read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Read Buffer"),
        size: (PRECISION as usize * PRECISION as usize * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let increment_data = [RANGE / PRECISION as f32, PRECISION as f32];
    let increment_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Increment Buffer"),
        contents: bytemuck::cast_slice(&increment_data),
        usage: wgpu::BufferUsages::STORAGE,
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
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<f32>() as _),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                    min_binding_size: wgpu::BufferSize::new((PRECISION as u64 * PRECISION as u64 * std::mem::size_of::<f32>() as u64) as _),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new((std::mem::size_of::<f32>() * 2) as _),
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
                    size: wgpu::BufferSize::new(((PRECISION * PRECISION) as usize * std::mem::size_of::<f32>()) as _),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &increment_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new((std::mem::size_of::<f32>() * 2) as _),
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
            pass.dispatch_workgroups(PRECISION, PRECISION, 1);
        }
        // Copy the result from the result buffer to the read buffer
        encoder.copy_buffer_to_buffer(&results_buffer, 0, &read_buffer, 0, read_buffer.size());
        queue.submit(Some(encoder.finish()));
    }

    // Map the read buffer and read the results
    let result_slice = read_buffer.slice(..);

    // Use a channel to wait for the buffer mapping to complete
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    result_slice.map_async(wgpu::MapMode::Read, move |result| {
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
    let mut min_mse = f32::MAX;
    let mut best_a = 0.0;
    let mut best_n = 0.0;

    let increment = RANGE / PRECISION as f32;

    // Linear search: Find the best_a and best_n with the lowest mse
    for (index, mse) in result_vec.iter().enumerate() {
        let i = index / PRECISION as usize;
        let j = index % PRECISION as usize;
        let a = i as f32 * increment;
        let n = j as f32 * increment;

        if *mse < min_mse {
            min_mse = *mse;
            best_a = a;
            best_n = n;
        }
    }

    println!("Optimized Coefficient (a): {}, Optimized Exponent (n): {}, Minimum Mean Squared Error (MSE): {}", best_a, best_n, min_mse);
    device.stop_capture();
    Ok((best_a, best_n))
}