# Power Meter

This is a Rust reimplementation of [apt-sim's Power Meter library](https://github.com/apt-sim/power_meter), with certain ehancements and improvements.

This library provides a software power usage and energy consumption meter for Intel and AMD CPUs, and NVIDIA GPUs. On CPU it reads from Intel and AMD's RAPL interfaces, while on GPU it uses Nvidia's NVML interface.

## Usage

This project compiles to a dynamic library, `libpwrm.so`, usable in C and C++ projects.

To compile the library, you need [Rust and Cargo](https://rust-lang.org/tools/install/). Then, you can run:

```bash
cargo build --release
```

`libpwrm.so` will be found in `target/release/`.

To use it in your C/C++ project, include `include/power_meter.h` in your relevant source files, and link against `libpwrm.so` when compiling.

`power_meter.h` exposes the following functions:

```c
// Starts monitoring the power usage and energy consumption of the system's CPU and GPU at the specified sampling interval (in milliseconds).
void pwmr_launch_monitoring_loop(uint64_t sampling_interval_ms);
// Stops monitoring the power usage and energy consumption of the system.
void pwmr_stop_monitoring_loop(void);
// Sets the path of the output directory where the measured data will be saved (optional, "power_meter_out" by default).
void pwmr_set_output_dir(const char *path_ptr);
// Sets the name for the output file containing the CPU data (optional, "cpu" by default).
void pwmr_set_cpu_out_filename(const char *filename_ptr);
// Sets the name for the output file containing the GPU data (optional, "gpu" by default).
void pwmr_set_gpu_out_filename(const char *filename_ptr);
```

Therefore, to measure power usage and energy consumption for a specific part of your application, you would do:
```c
#include "power_meter.h"

int main(void) {
    // Set the output directory and file names (optional):
    pwmr_set_output_dir("energy_measurements");
    pwmr_set_cpu_out_filename("cpu_data");
    pwmr_set_gpu_out_filename("gpu_data");

    // Start monitoring with a sampling interval of 500 ms:
    pwmr_launch_monitoring_loop(500);

    // Your application code here:
    // ...

    // Stop monitoring when done:
    pwmr_stop_monitoring_loop();

    return 0;
}
```

## License

This project is licensed under the GNU Lesser General Public License v3.0 (LGPL-3.0). See the [LICENSE.md](LICENSE.md) file for details.
