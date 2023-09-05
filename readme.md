This tool is currently a work in progress and is provided "AS IS". It is not intended for public use, and instructions will not be provided until further development.

In a nutshell, this software is designed to calibrate a Mass Airflow Sensor for vehicles by attempting to reduce Fuel Trim values to zero. This should lead to a vehicle that runs more efficiently, accelerates better, and increases longevity of the engine by having the correct amount of airflow for a given amount of fuel.

## How it works

This CLI software is entirely built in Rust using wgpu (a multi-platform, web-capable graphics API). It utilizes a custom compute shader to evaluate log data and return corrected values for a MAF sensor.
It absolutely does NOT need a compute shader for this purpose, but I wanted to learn wgpu anyway and it has provided invaluable experience with Rust, GLSL shaders, and parallel processing using a GPU.

## Compute shader

The compute shader has been built from scratch to be extremely flexible, and utilize up to (4096 * 4096) threads. It uses a genetic algorithm to iteratively converge towards the optimized parameters of "a , n" where "Y = aX ^ n". We can then use these parameters to re-calibrate your MAF sensor.

## Contributions, issues

Please report any issues on this repo, and feel free to fork or open a pull request if you'd like to modify this software.
