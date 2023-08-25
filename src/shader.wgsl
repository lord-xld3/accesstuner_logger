// Data structures

struct DataBuffer {
    values: array<f32>;
};

// Bind groups
@block
struct XData {
    data: DataBuffer;
};
@group(0) @binding(0)
var<storage, read> x_data: XData;

@block
struct YData {
    data: DataBuffer;
};
@group(0) @binding(1)
var<storage, read> y_data: YData;

@block
struct IntermediateData {
    sum_a: f32;
    sum_b: f32;
    error: f32;
};
@group(0) @binding(2)
var<storage, read_write> intermediate_data: IntermediateData;

@block
struct ResultData {
    a: f32;
    b: f32;
};
@group(0) @binding(3)
var<storage, write> result_data: ResultData;

@compute
fn aggregation_pass([[builtin(global_invocation_id)]] id: u32) {
    let x = x_data.data.values[id];
    let y = y_data.data.values[id];
    
    let prediction = result_data.a * exp(result_data.b * x);
    let error = prediction - y;
    
    // Gradients with respect to a and b
    let da = 2.0 * error * exp(result_data.b * x);
    let db = 2.0 * error * result_data.a * x * exp(result_data.b * x);

    // Aggregate the gradients
    atomicAdd(intermediate_data.sum_a, da);
    atomicAdd(intermediate_data.sum_b, db);
    atomicAdd(intermediate_data.error, error * error);
}

@compute
fn calculation_pass([[builtin(global_invocation_id)]] id: u32) {
    if (id == 0u) {
        // Update a and b using the aggregated gradients
        let learning_rate = 0.001;  // A small learning rate for updates
        result_data.a -= learning_rate * intermediate_data.sum_a;
        result_data.b -= learning_rate * intermediate_data.sum_b;

        // Reset intermediate data for next iteration
        intermediate_data.sum_a = 0.0;
        intermediate_data.sum_b = 0.0;
    }
}
