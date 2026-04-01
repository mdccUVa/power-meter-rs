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

use nvml_utils::{EnergyAux as NVMLAux, EnergyData as NVMLData, GPUMonitor, NVMLUtilsErrorKind};
use rapl_utils::{CPUMonitor, EnergyAux as RAPLAux, EnergyData as RAPLData, RAPLUtilsErrorKind};

#[derive(Debug)]
struct MonitoringConfig {
    do_monitoring: bool,
    monitoring_thread: Option<std::thread::JoinHandle<()>>,
    output_dir: String,
    cpu_monitor: CPUMonitor,
    gpu_monitor: GPUMonitor,
    cpu_out_filename: Option<String>,
    gpu_out_filename: Option<String>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            do_monitoring: true,
            monitoring_thread: None,
            // Intel: Initialize internal counters
            cpu_monitor: CPUMonitor::new()
                .expect("[POWER METER] An error was encountered during initialization"),
            gpu_monitor: GPUMonitor::new(),
            // CUDA: Initialize NVML, number of GPUs, and device handles
            output_dir: "power_meter_out".to_string(),
            cpu_out_filename: Some("cpu".to_string()),
            gpu_out_filename: Some("gpu".to_string()),
        }
    }
}

static CONFIG: LazyLock<Mutex<MonitoringConfig>> =
    LazyLock::new(|| Mutex::new(MonitoringConfig::default()));

fn monitoring_loop(sampling_interval_ms: u64) {
    log::info!(
        "Starting monitoring with period {} ms",
        sampling_interval_ms
    );

    // Structs used to take measurements from Intel/AMD's RAPL interface:
    let mut cpu_pkg_data = RAPLAux::default();
    let mut current_cpu_pkg_data = RAPLAux::default();
    let mut cpu_pkg_results = RAPLData::default();

    // Structs used to take measurements from NVIDIA's NVML library:
    let mut cuda_data = NVMLAux::default();
    let mut current_cuda_data = NVMLAux::default();
    let mut cuda_results = NVMLData::default();

    // Get the initial energy readings:
    {
        let mut config = CONFIG.lock().unwrap();

        // CPU: Get the current energy measurement for RAPL's package domain
        config
            .cpu_monitor
            .update_package_energy(&mut cpu_pkg_data)
            .expect("[POWER METER] Failed to read CPU energy data");
        log::debug!("Read initial CPU energy data");
        // CUDA
        config
            .gpu_monitor
            .update_gpu_energy(&mut cuda_data)
            .expect("[POWER METER] Failed to read GPU energy data");
        log::debug!("Read initial GPU energy data");
    }

    // Create/open output files:
    fs::create_dir_all(&CONFIG.lock().unwrap().output_dir)
        .expect("[POWER METER] Failed to create output directory");

    let mut cpu_out = match CONFIG.lock().unwrap().cpu_out_filename {
        Some(ref filename) => Some(
            File::create(Path::new(&CONFIG.lock().unwrap().output_dir).join(filename))
                .expect("[POWER METER] Failed to create CPU output file"),
        ),
        None => None,
    };
    let mut gpu_out = match CONFIG.lock().unwrap().gpu_out_filename {
        Some(ref filename) => Some(
            File::create(Path::new(&CONFIG.lock().unwrap().output_dir).join(filename))
                .expect("[POWER METER] Failed to create GPU output file"),
        ),
        None => None,
    };

    // Write the header for the output files:
    let output_header = "Power,Energy,Total energy";
    if let Some(cpu_out) = &mut cpu_out {
        writeln!(cpu_out, "{}", output_header)
            .expect("[POWER METER] Failed to write CPU output file header");
    }
    if let Some(gpu_out) = &mut gpu_out {
        writeln!(gpu_out, "{}", output_header)
            .expect("[POWER METER] Failed to write GPU output file header");
    }

    let mut do_monitoring = { CONFIG.lock().unwrap().do_monitoring };
    while do_monitoring {
        // Sleep for the specified sampling interval:
        thread::sleep(Duration::from_millis(sampling_interval_ms));

        {
            let mut config = CONFIG.lock().unwrap();

            // CPU: Update energy measurements
            config
                .cpu_monitor
                .update_package_energy(&mut current_cpu_pkg_data)
                .expect("[POWER METER] Failed to read CPU energy data");
            // CPU: Compute energy and average power usage for this interval, update total energy
            // consumption
            config.cpu_monitor.update_energy_data(
                &mut cpu_pkg_results,
                &cpu_pkg_data,
                &current_cpu_pkg_data,
            );

            // CUDA: Update energy measurements
            config
                .gpu_monitor
                .update_gpu_energy(&mut current_cuda_data)
                .expect("[POWER METER] Failed to read GPU energy data");
            // CUDA: Compute energy and average power usage for this interval, update total energy
            // consumption
            config.gpu_monitor.update_energy_data(
                &mut cuda_results,
                &cuda_data,
                &current_cuda_data,
            );

            // Swap structs for the next iteration:
            swap(&mut cpu_pkg_data, &mut current_cpu_pkg_data);
            swap(&mut cuda_data, &mut current_cuda_data);

            log::debug!("Writing results to file (if enabled).");
            if let Some(cpu_out) = &mut cpu_out {
                writeln!(
                    cpu_out,
                    "{},{},{}",
                    cpu_pkg_results.power, cpu_pkg_results.energy, cpu_pkg_results.total_energy
                )
                .expect("[POWER METER] Failed to write CPU energy data to file");
            }
            if let Some(gpu_out) = &mut gpu_out {
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

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_launch_monitoring_loop(sampling_interval_ms: u64) {
    let _ = env_logger::try_init();

    let mut config = CONFIG.lock().unwrap();

    // Launch monitoring on a separate thread:
    config.do_monitoring = true;
    config.monitoring_thread = Some(thread::spawn(move || monitoring_loop(sampling_interval_ms)));

    log::debug!("Spawned monitoring loop");
}

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_stop_monitoring_loop() -> PwrmResult {
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
        match handle.join() {
            Ok(_) => log::info!("Stopped monitoring loop"),
            Err(e) => {
                log::error!("Failed to stop monitoring loop: {:?}", e);
                return PwrmResult::MonitoringError;
            }
        }
    } else {
        log::debug!("Monitoring loop was not running, nothing to stop");
    }

    PwrmResult::Success
}

#[repr(C)]
pub enum PwrmResult {
    Success = 0,
    PathError = -1,
    MonitoringError = -2,
    NotEnoughData = -3,
}

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_get_avg_cpu_power(avg_power_out: *mut std::os::raw::c_double) -> PwrmResult {
    let config = CONFIG.lock().unwrap();

    let avg_power = config.cpu_monitor.get_average_power();
    match avg_power {
        Ok(power) => {
            unsafe {
                *avg_power_out = power;
            }
            PwrmResult::Success as PwrmResult
        }
        Err(e) => {
            assert!(e.kind == RAPLUtilsErrorKind::NotEnoughData);
            log::error!("Failed to get average CPU power: {}", e);
            unsafe {
                *avg_power_out = 0.0;
            };
            PwrmResult::NotEnoughData as PwrmResult
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_get_avg_gpu_power(avg_power_out: *mut std::os::raw::c_double) -> PwrmResult {
    let config = CONFIG.lock().unwrap();

    let avg_power = config.gpu_monitor.get_average_power();
    match avg_power {
        Ok(power) => {
            unsafe {
                *avg_power_out = power;
            }
            PwrmResult::Success as PwrmResult
        }
        Err(e) => {
            assert!(e.kind == NVMLUtilsErrorKind::NotEnoughData);
            log::error!("Failed to get average GPU power: {}", e);
            unsafe {
                *avg_power_out = 0.0;
            };
            PwrmResult::NotEnoughData as PwrmResult
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_get_total_cpu_energy(
    total_energy_out: *mut std::os::raw::c_double,
) -> PwrmResult {
    let config = CONFIG.lock().unwrap();

    let total_energy = config.cpu_monitor.get_total_energy();
    match total_energy {
        Ok(energy) => {
            unsafe {
                *total_energy_out = energy;
            }
            PwrmResult::Success as PwrmResult
        }
        Err(e) => {
            assert!(e.kind == RAPLUtilsErrorKind::NotEnoughData);
            log::error!("Failed to get total CPU energy: {}", e);
            unsafe {
                *total_energy_out = 0.0;
            };
            PwrmResult::NotEnoughData as PwrmResult
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_get_total_gpu_energy(
    total_energy_out: *mut std::os::raw::c_double,
) -> PwrmResult {
    let config = CONFIG.lock().unwrap();

    let total_energy = config.gpu_monitor.get_total_energy();
    match total_energy {
        Ok(energy) => {
            unsafe {
                *total_energy_out = energy;
            }
            PwrmResult::Success
        }
        Err(e) => {
            assert!(e.kind == NVMLUtilsErrorKind::NotEnoughData);
            log::error!("Failed to get total GPU energy: {}", e);
            unsafe {
                *total_energy_out = 0.0;
            };
            PwrmResult::NotEnoughData
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_set_output_dir(path_ptr: *const std::os::raw::c_char) -> PwrmResult {
    let mut config = CONFIG.lock().unwrap();

    if path_ptr.is_null() {
        log::warn!(
            "[POWER METER] Provided a null output directory path; defaulting to current dir"
        );
        config.output_dir = "./".to_string();

        return PwrmResult::Success;
    }

    let path_arr = unsafe { std::ffi::CStr::from_ptr(path_ptr) };
    let path = match path_arr.to_str() {
        Ok(p) => p,
        Err(e) => {
            log::error!("[POWER METER] Invalid output directory string: {}", e);

            return PwrmResult::PathError;
        }
    };
    config.output_dir = path.to_string();

    log::debug!("Changed output directory to {}", path);

    PwrmResult::Success
}

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_set_cpu_out_filename(
    filename_ptr: *const std::os::raw::c_char,
) -> PwrmResult {
    let mut config = CONFIG.lock().unwrap();

    if filename_ptr.is_null() {
        log::info!("[POWER METER] Provided a null CPU output filename; disabling CPU output file");
        config.cpu_out_filename = None;

        return PwrmResult::Success;
    }

    let filename_arr = unsafe { std::ffi::CStr::from_ptr(filename_ptr) };
    let filename = match filename_arr.to_str() {
        Ok(f) => f,
        Err(e) => {
            log::error!("[POWER METER] Invalid CPU output filename string: {}", e);

            return PwrmResult::PathError;
        }
    };

    if filename.len() == 0 {
        log::info!(
            "[POWER METER] Provided an empty CPU output filename; disabling CPU output file"
        );
        config.cpu_out_filename = None;

        return PwrmResult::Success;
    }

    config.cpu_out_filename = Some(filename.to_string());

    log::debug!("Changed CPU output file name to {}", filename);

    PwrmResult::Success
}

#[unsafe(no_mangle)]
pub extern "C" fn pwrm_set_gpu_out_filename(
    filename_ptr: *const std::os::raw::c_char,
) -> PwrmResult {
    let mut config = CONFIG.lock().unwrap();

    if filename_ptr.is_null() {
        log::info!("[POWER METER] Provided a null GPU output filename; disabling GPU output file");
        config.gpu_out_filename = None;

        return PwrmResult::Success;
    }

    let filename_arr = unsafe { std::ffi::CStr::from_ptr(filename_ptr) };
    let filename = match filename_arr.to_str() {
        Ok(f) => f,
        Err(e) => {
            log::error!("[POWER METER] Invalid GPU output filename string: {}", e);

            return PwrmResult::PathError;
        }
    };

    if filename.len() == 0 {
        log::info!(
            "[POWER METER] Provided an empty GPU output filename; disabling GPU output file"
        );
        config.gpu_out_filename = None;

        return PwrmResult::Success;
    }

    config.gpu_out_filename = Some(filename.to_string());

    log::debug!("Changed GPU output file name to {}", filename);

    PwrmResult::Success
}
