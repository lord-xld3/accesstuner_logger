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
    float sumX = 0;
    float sumY = 0;
    float sumX2 = 0;
    float sumXY = 0;

    // Compute sums needed for linear regression
    for (uint i = 0; i < n; ++i) {
        float x = inputData[i].x;
        float y = log2(inputData[i].y);  // Convert to log

        sumX += x;
        sumY += y;
        sumX2 += x * x;
        sumXY += x * y;
    }

    // Compute linear regression coefficients
    float b = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);
    float lnA = (sumY - b * sumX) / n;

    // Extract curve parameters
    CurveParams params;
    params.a = exp2(lnA);
    params.b = b;

    // Write to output buffer
    outputData[0] = params;
}
