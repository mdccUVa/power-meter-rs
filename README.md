# Power Meter

This is a Rust reimplementation of [apt-sim's Power Meter library](https://github.com/apt-sim/power_meter), with certain ehancements and improvements.

This library provides a software power usage and energy consumption meter for Intel and AMD CPUs, and NVIDIA GPUs. On CPU it reads from Intel and AMD's RAPL interfaces, while on GPU it uses NVIDIA's NVML interface.

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
// Starts monitoring the power usage and energy consumption of the system's CPU and GPU
// at the specified sampling interval (in milliseconds).
void pwrm_launch_monitoring_loop(uint64_t sampling_interval_ms);
// Stops monitoring the power usage and energy consumption of the system.
pwrmError_t pwrm_stop_monitoring_loop(void);

// Gets the CPU's average power usage since monitoring started or the last counters reset:
pwrmError_t pwrm_get_avg_cpu_power(double *avg_power_out);
// Gets the GPU's average power usage since monitoring started or the last counters reset:
pwrmError_t pwrm_get_avg_gpu_power(double *avg_power_out);
// Gets the CPU's total energy consumption since monitoring started or the last counters reset:
pwrmError_t pwrm_get_total_cpu_energy(double *total_energy_out);
// Gets the CPU's total energy consumption since monitoring started or the last counters reset:
pwrmError_t pwrm_get_total_gpu_energy(double *total_energy_out);
// Resets the power usage and energy consumption counters to zero,
// discarding all previously collected data:
void pwrm_reset_counters(void);

// Sets the path of the output directory where the measured data will be saved
// (optional, "power_meter_out" by default, NULL uses the current directory).
pwrmError_t pwrm_set_output_dir(const char *path_ptr);
// Sets the name for the output file containing the CPU data
// (optional, "cpu" by default; NULL or "" deactivate it).
pwrmError_t pwrm_set_cpu_out_filename(const char *filename_ptr);
// Sets the name for the output file containing the GPU data
// (optional, "gpu" by default; NULL or "" deactivate it).
pwrmError_t pwrm_set_gpu_out_filename(const char *filename_ptr);
```

Therefore, an example program that uses `libpwrm` to measure power usage and energy consumption could be the following:
```c
#include <stdlib.h>
#include <stdio.h>

#include "power_meter.h"

int main(void) {
	// (Optional) set the output directory and file names:
	pwrm_set_output_dir("energy_measurements"); // No need to check the result if the path is a correct string.
	pwrm_set_cpu_out_filename("cpu_data"); // No need to check the result if the name is a correct string.
	pwrm_set_gpu_out_filename("gpu_data"); // No need to check the result if the name is a correct string.

	// Start monitoring with a sampling interval of 500 ms:
	pwrm_launch_monitoring_loop(500);

	// Your application code goes here:
	// ...

	// Result data will be written to `./energy_measurements/cpu_data` and `./energy_measurements/gpu_data`.

	// Stop monitoring when done:
	if (pwrm_stop_monitoring_loop() != PWRM_SUCCESS) {
		fprintf(stderr, "There was an error with the monitoring thread.\n");
		exit(EXIT_FAILURE);
	}

	// Additionally, print energy consumption and power usage data to stdout:
	double cpu_power, gpu_power, cpu_energy, gpu_energy;
	if (pwrm_get_avg_cpu_power(&cpu_power) != PWRM_SUCCESS) {
		fprintf(stderr, "Not enough measures to compute the average CPU power.\n");
		exit(EXIT_FAILURE);
	}
	if (pwrm_get_avg_gpu_power(&gpu_power) != PWRM_SUCCESS) {
		fprintf(stderr, "Not enough measures to compute the average GPU power.\n");
		exit(EXIT_FAILURE);
	}
	if (pwrm_get_total_cpu_energy(&cpu_energy) != PWRM_SUCCESS) {
		fprintf(stderr, "No measures registered, cannot retrieve the total CPU energy consumption.\n");
		exit(EXIT_FAILURE);
	}
	if (pwrm_get_total_gpu_energy(&gpu_energy) != PWRM_SUCCESS) {
		fprintf(stderr, "No measures registered, cannot retrieve the total GPU energy consumption.\n");
		exit(EXIT_FAILURE);
	}
	printf("Average power usage: CPU %lf W, GPU %lf W\n", cpu_power, gpu_power);
	printf("Total energy consumption: CPU %lf J, GPU %lf J\n", cpu_energy, gpu_energy);

	return 0;
}
```

## License

This project is licensed under the GNU Lesser General Public License v3.0 (LGPL-3.0). See the [LICENSE.md](LICENSE.md) file for details.
