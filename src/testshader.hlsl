// Define the buffer structure
struct Point {
    float x;
    float y;
};

// Define the output structure
struct CurveParams {
    float a;
    float b;
};

// Buffers
StructuredBuffer<Point> inputData : register(t0);
RWStructuredBuffer<CurveParams> outputData : register(u0);

cbuffer Constants : register(b0) {
    uint n;  // Number of data points in the inputData buffer
};

[numthreads(1, 1, 1)]
void main(uint3 dtID : SV_DispatchThreadID) {
    // Set hardcoded values
    CurveParams params;
    params.a = 1.23;
    params.b = 4.56;

    // Write to output buffer
    outputData[0] = params;
}
