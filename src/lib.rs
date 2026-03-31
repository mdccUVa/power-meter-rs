// libpwrm: Library for monitoring CPU and GPU power usage and energy consumption.
// Copyright (C) 2026  Manuel de Castro <manuel@infor.uva.es>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// and GNU Lesser General Public License along with this program.
// If not, see <https://www.gnu.org/licenses/>.

mod nvml_utils;
mod rapl_utils;

use std::{
    fs::{self, File},
    io::Write,
    mem::swap,
    path::Path,
    sync::{LazyLock, Mutex},
    thread,
    time::Duration,
};

use nvml_utils::{EnergyAux as NVMLAux, EnergyData as NVMLData, GPUMonitor};
use rapl_utils::{CPUMonitor, EnergyAux as RAPLAux, EnergyData as RAPLData};

#[derive(Debug)]
struct MonitoringConfig {
    do_monitoring: bool,
    monitoring_thread: Option<std::thread::JoinHandle<()>>,
    output_dir: String,
    cpu_out_filename: String,
    gpu_out_filename: String,
    cpu_out: Option<File>,
    gpu_out: Option<File>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            do_monitoring: true,
            monitoring_thread: None,
            output_dir: "power_meter_out".to_string(),
            cpu_out_filename: "cpu".to_string(),
            gpu_out_filename: "gpu".to_string(),
            cpu_out: None,
            gpu_out: None,
        }
    }
}

fn monitoring_loop(sampling_interval_ms: u64, rapl: &mut CPUMonitor, nvml: &mut GPUMonitor) {
    log::info!(
        "Starting monitoring with period {} ms",
        sampling_interval_ms
    );

    // Structs used to take measurements from Intel/AMD's RAPL interface:
    let mut cpu_pkg_data = RAPLAux::default();
    let mut current_cpu_pkg_data = RAPLAux::default();
    let mut cpu_pkg_results = RAPLData::default();

    // Structs used to take measurements from Nvidia NVML:
    let mut cuda_data = NVMLAux::default();
    let mut current_cuda_data = NVMLAux::default();
    let mut cuda_results = NVMLData::default();

    // Get the initial energy readings:
    // CPU: Get the current energy measurement for RAPL's package domain
    rapl.update_package_energy(&mut cpu_pkg_data)
        .expect("[POWER METER] Failed to read CPU energy data");
    log::debug!("Read initial CPU energy data");
    // CUDA
    nvml.update_gpu_energy(&mut cuda_data)
        .expect("[POWER METER] Failed to read GPU energy data");
    log::debug!("Read initial GPU energy data");

    // Write the header for the output files:
    {
        let mut config = CONFIG.lock().unwrap();

        let output_header = "Power,Energy,Total energy";
        if let Some(cpu_out) = &mut config.cpu_out {
            writeln!(cpu_out, "{}", output_header)
                .expect("[POWER METER] Failed to write CPU output file header");
        }
        if let Some(gpu_out) = &mut config.gpu_out {
            writeln!(gpu_out, "{}", output_header)
                .expect("[POWER METER] Failed to write GPU output file header");
        }
    }

    let mut do_monitoring = { CONFIG.lock().unwrap().do_monitoring };
    while do_monitoring {
        // Sleep for the specified sampling interval:
        thread::sleep(Duration::from_millis(sampling_interval_ms));

        // CPU: Update energy measurements
        rapl.update_package_energy(&mut current_cpu_pkg_data)
            .expect("[POWER METER] Failed to read CPU energy data");
        // CPU: Compute energy and average power usage for this interval, update total energy
        // consumption
        rapl.update_energy_data(&mut cpu_pkg_results, &cpu_pkg_data, &current_cpu_pkg_data);

        // CUDA: Update energy measurements
        nvml.update_gpu_energy(&mut current_cuda_data)
            .expect("[POWER METER] Failed to read GPU energy data");
        // CUDA: Compute energy and average power usage for this interval, update total energy
        // consumption
        nvml.update_energy_data(&mut cuda_results, &cuda_data, &current_cuda_data);

        // Swap structs for the next iteration:
        swap(&mut cpu_pkg_data, &mut current_cpu_pkg_data);
        swap(&mut cuda_data, &mut current_cuda_data);

        log::debug!("Writing results to file.");
        {
            let mut config = CONFIG.lock().unwrap();

            if let Some(cpu_out) = &mut config.cpu_out {
                writeln!(
                    cpu_out,
                    "{},{},{}",
                    cpu_pkg_results.power, cpu_pkg_results.energy, cpu_pkg_results.total_energy
                )
                .expect("[POWER METER] Failed to write CPU energy data to file");
            }
            if let Some(gpu_out) = &mut config.gpu_out {
                writeln!(
                    gpu_out,
                    "{},{},{}",
                    cuda_results.power, cuda_results.energy, cuda_results.total_energy
                )
                .expect("[POWER METER] Failed to write GPU energy data to file");
            }

            do_monitoring = config.do_monitoring;
        }

        log::debug!("Finished taking a sample.");
    }
}

static CONFIG: LazyLock<Mutex<MonitoringConfig>> =
    LazyLock::new(|| Mutex::new(MonitoringConfig::default()));

#[unsafe(no_mangle)]
pub extern "C" fn pwmr_launch_monitoring_loop(sampling_interval_ms: u64) {
    let _ = env_logger::try_init();

    let mut config = CONFIG.lock().unwrap();

    // Create output files
    fs::create_dir_all(&config.output_dir)
        .expect("[POWER METER] Failed to create output directory");
    let cpu_out_path = Path::new(&config.output_dir).join(&config.cpu_out_filename);
    let gpu_out_path = Path::new(&config.output_dir).join(&config.gpu_out_filename);
    config.cpu_out =
        Some(File::create(cpu_out_path).expect("[POWER METER] Failed to create CPU output file"));
    config.gpu_out =
        Some(File::create(gpu_out_path).expect("[POWER METER] Failed to create GPU output file"));

    // Intel: Initialize internal counters
    let mut cpu_monitor = rapl_utils::CPUMonitor::new()
        .expect("[POWER METER] An error was encountered during initialization");
    // CUDA: Initialize NVML, number of GPUs, and device handles
    let mut gpu_monitor = nvml_utils::GPUMonitor::new();

    // Launch monitoring on a separate thread:
    config.do_monitoring = true;
    config.monitoring_thread = Some(thread::spawn(move || {
        monitoring_loop(sampling_interval_ms, &mut cpu_monitor, &mut gpu_monitor)
    }));

    log::debug!("Spawned monitoring loop");
}

#[unsafe(no_mangle)]
pub extern "C" fn pwmr_stop_monitoring_loop() {
    log::debug!("Requested monitoring loop stop");

    // Signal monitoring thread to stop and get its handle
    let maybe_handle = {
        let mut config = CONFIG.lock().unwrap();

        config.do_monitoring = false;

        if config.monitoring_thread.is_some() {
            config.monitoring_thread.take()
        } else {
            None
        }
    };
    // Stop monitoring thread
    if let Some(handle) = maybe_handle {
        handle
            .join()
            .expect("[POWER METER] Failed to join monitoring thread");
    }

    log::info!("Stopped monitoring loop");
}

#[unsafe(no_mangle)]
pub extern "C" fn pwmr_set_output_dir(path_ptr: *const std::os::raw::c_char) {
    let mut config = CONFIG.lock().unwrap();

    let path_arr = unsafe { std::ffi::CStr::from_ptr(path_ptr) };
    let path = path_arr
        .to_str()
        .expect("[POWER METER] Invalid output directory string");

    config.output_dir = path.to_string();

    log::debug!("Changed output directory to {}", path);
}

#[unsafe(no_mangle)]
pub extern "C" fn pwmr_set_cpu_out_filename(filename_ptr: *const std::os::raw::c_char) {
    let mut config = CONFIG.lock().unwrap();

    let filename_arr = unsafe { std::ffi::CStr::from_ptr(filename_ptr) };
    let filename = filename_arr
        .to_str()
        .expect("[POWER METER] Invalid CPU output filename string");

    config.cpu_out_filename = filename.to_string();

    log::debug!("Changed CPU output file name to {}", filename);
}

#[unsafe(no_mangle)]
pub extern "C" fn pwmr_set_gpu_out_filename(filename_ptr: *const std::os::raw::c_char) {
    let mut config = CONFIG.lock().unwrap();

    let filename_arr = unsafe { std::ffi::CStr::from_ptr(filename_ptr) };
    let filename = filename_arr
        .to_str()
        .expect("[POWER METER] Invalid GPU output filename string");

    config.gpu_out_filename = filename.to_string();

    log::debug!("Changed GPU output file name to {}", filename);
}
