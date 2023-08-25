use std::{
    io,
    ops::Range,
};
use wgpu::{
    Buffer,
    BufferAddress,
    util::DeviceExt,
};

pub async fn run(x_data: &[f32], y_data: &[f32]) -> io::Result<Vec<f32>> {
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
        .request_device(&Default::default(), None)
        .await
        .unwrap();
    let x_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("X Data Buffer"),
        contents: bytemuck::cast_slice(x_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let y_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Y Data Buffer"),
        contents: bytemuck::cast_slice(y_data),  // assuming y_data has the same type & structure as x_data
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let intermediate_values = [0.0f32, 0.0f32, 0.0f32]; // sum_a, sum_b, and error
    let intermediate_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Intermediate Data Buffer"),
        contents: bytemuck::cast_slice(&intermediate_values),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
    });
    let initial_values = [1.0f32, 1.0f32];
    let result_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Result Data Buffer"),
        contents: bytemuck::cast_slice(&initial_values),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(4 * x_data.len() as u64),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(4 * y_data.len() as u64),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(4 * 3),  // 3 f32 values (a, b, c)
                },
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: x_data_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: y_data_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: result_data_buffer.as_entire_binding(),
            },
        ],
    });

    let u32_size = std::mem::size_of::<u32>() as u32;
    let output_buffer_size = (u32_size * 256 * 256) as u64; 
    let output_buffer_desc = wgpu::BufferDescriptor {
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST
            // this tells wpgu that we want to read this buffer from the cpu
            | wgpu::BufferUsages::MAP_READ,
        label: None,
        mapped_at_creation: false,
    };
    let output_buffer = device.create_buffer(&output_buffer_desc);

    let shader_src = include_str!("shader.wgsl");
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Exponential Fit Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });
    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let aggregation_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Aggregation Pass Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &shader_module,
        entry_point: "aggregation_pass",
    });
    let calculation_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Calculation Pass Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &shader_module,
        entry_point: "calculation_pass",
    });
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &shader_module,
        entry_point: "main",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("Compute Pass"),
    });
    const MAX_ITERATIONS: usize = 1000;
    const ERROR_THRESHOLD: f32 = 0.01;
    let mut fitted_y_data: Vec<f32> = Vec::new();
    let mut mapped_buffer: wgpu::Buffer;
    for _ in 0..MAX_ITERATIONS {
        // Run the aggregation pass
        compute_pass.set_pipeline(&aggregation_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(x_data.len() as u32, 1, 1);
    
        // Run the calculation pass
        compute_pass.set_pipeline(&calculation_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(1, 1, 1);
    
        // Unmap the output buffer and read intermediate error to check for convergence
        {
            let buffer_slice = output_buffer.slice(..);
            let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            device.poll(wgpu::Maintain::Wait);
            let mapped_buffer = rx.receive().await.unwrap().unwrap();
            let data = mapped_buffer.get_mapped_range();
            
            // Extract the result data
            let result_data = data; // Use the same variable to retrieve the mapped range
            let (a, b): (f32, f32) = (*bytemuck::from_bytes(&result_data[0..4]), *bytemuck::from_bytes(&result_data[4..8]));
            
            let (a, b): (f32, f32) = (*bytemuck::from_bytes(&result_data[0..4]), *bytemuck::from_bytes(&result_data[4..8]));

            // Calculate the error and check for convergence
            let error_range: Range<BufferAddress> = (2 * std::mem::size_of::<f32>() as u64)..(3 * std::mem::size_of::<f32>() as u64);
            let error_slice = intermediate_data_buffer.slice(error_range);
            let error_data = error_slice.get_mapped_range();
            let error: f32 = *bytemuck::from_bytes(&error_data[..]);

            if error < ERROR_THRESHOLD {
                break;
            }

            // Calculate fitted y data
            let fitted_y_data: Vec<f32> = x_data.iter().map(|&x| a * (b * x).exp()).collect();
            println!("Optimized parameters: a = {}, b = {}", a, b);

            // Return the result here if converged
            if error < ERROR_THRESHOLD {
                return Ok(fitted_y_data);
            }
        }
        output_buffer.unmap();


    }
    // Return the result after the loop
    Ok(fitted_y_data)
}