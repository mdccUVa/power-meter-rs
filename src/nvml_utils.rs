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

#[path = "nvml_bindings.rs"]
mod nvml_bindings;

use std::{fmt::Display, iter::zip, time::Instant};

use nvml_bindings::*;

/* CONSTANTS */

/// Limit on the number of CUDA GPUs supported by this utility. Arbitrarily set.
const MAX_GPUS_SUPPORTED: usize = 4;

/* DATA */

/// Struct containing per-GPU energy and power measurements along with the time they were taken.
#[derive(Clone, Debug)]
pub(super) struct EnergyAux {
    /// Timestamp when the struct was last updated.
    time: Instant,
    /// Last measured energy values for each CUDA GPU.
    energy: [f64; MAX_GPUS_SUPPORTED],
    /// Last measured power values for each CUDA GPU.
    power: [f64; MAX_GPUS_SUPPORTED],
}

impl Default for EnergyAux {
    fn default() -> Self {
        Self {
            time: Instant::now(),
            energy: [0.0; MAX_GPUS_SUPPORTED],
            power: [0.0; MAX_GPUS_SUPPORTED],
        }
    }
}

/// Struct storing the machine's average power draw and energy consumption during the last
/// measurement interval, as well as the total energy consumption. Its data represents the sum of
/// all the GPUs in the system.
#[derive(Clone, Default, Debug)]
pub(super) struct EnergyData {
    pub(super) power: f64,
    pub(super) energy: f64,
    pub(super) total_energy: f64,
}

#[derive(Clone, Debug)]
pub(super) struct GPUMonitor {
    num_gpus: usize,
    device_handles: Box<Vec<NvmlDevice>>,
    can_read_energy: Box<Vec<bool>>,
    history: Vec<EnergyData>,
}

#[derive(PartialEq, Eq, Debug)]
pub(super) enum NVMLUtilsErrorKind {
    UninitializedDevice,
    InvalidArgument,
    UnsupportedDevice,
    NVMLOther,
    NotEnoughData,
}

#[derive(Debug)]
pub(super) struct NVMLUtilsError {
    pub(super) kind: NVMLUtilsErrorKind,
    pub(super) message: String,
}

impl Display for NVMLUtilsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[NVML Utils] ERROR: {}", self.message)
    }
}

/* UTILITY FUNCTIONS */

impl GPUMonitor {
    pub(super) fn new() -> Self {
        unsafe { nvmlInit_v2() };
        log::debug!("Called nvmlInit_v2()");

        let mut num_gpus = 0;
        unsafe {
            nvmlDeviceGetCount_v2(&mut num_gpus);
        }
        let num_gpus = num_gpus as usize;
        let mut device_handles = Box::new(Vec::with_capacity(num_gpus));
        let mut can_read_energy = Box::new(Vec::with_capacity(num_gpus));
        for i in 0..num_gpus {
            let mut device_handle = std::ptr::null_mut();
            unsafe {
                nvmlDeviceGetHandleByIndex_v2(i as u32, &mut device_handle);
            }
            device_handles.push(NvmlDevice(device_handle));
            can_read_energy.push(true);
        }

        log::info!("Number of GPUs detected: {}", num_gpus);

        Self {
            num_gpus,
            device_handles,
            can_read_energy,
            history: Vec::new(),
        }
    }

    pub(super) fn update_gpu_energy(&mut self, data: &mut EnergyAux) -> Result<(), NVMLUtilsError> {
        let mut energy = 0;
        let mut power = 0;
        for i in 0..self.num_gpus {
            let mut nvml_error;

            // Read energy:
            if self.can_read_energy[i] {
                unsafe {
                    nvml_error =
                        nvmlDeviceGetTotalEnergyConsumption(self.device_handles[i].0, &mut energy);
                }
                if nvml_error != NVML_SUCCESS {
                    log::warn!(
                        "Failed to read energy for GPU {}; using estimation from power",
                        i,
                    );
                    self.can_read_energy[i] = false;

                    energy = 0;
                }
            } else {
                energy = 0;
            }

            // Value returned by NVML is in millijoules.
            data.energy[i] = energy as f64 / 1000.0;

            // Read power:
            unsafe {
                nvml_error = nvmlDeviceGetPowerUsage(self.device_handles[i].0, &mut power);
            }
            if nvml_error != NVML_SUCCESS {
                return Err(NVMLUtilsError {
                    kind: NVMLUtilsErrorKind::NVMLOther,
                    message: format!(
                        "Failed to read power for GPU {}: error code {}",
                        i, nvml_error
                    ),
                });
            }

            // Value returned by NVML is in milliwats.
            data.power[i] = power as f64 / 1000.0; // Convert from mW to W
        }
        // Update the timestamp:
        data.time = Instant::now();

        Ok(())
    }

    pub(super) fn update_energy_data(
        &mut self,
        output_data: &mut EnergyData,
        previous_data: &EnergyAux,
        current_data: &EnergyAux,
    ) {
        let time_diff = current_data
            .time
            .duration_since(previous_data.time)
            .as_secs_f64();

        let power = current_data.power.iter().sum::<f64>();
        output_data.power = power;

        let energy_diff = zip(current_data.energy, previous_data.energy)
            .take(self.num_gpus)
            .enumerate()
            .map(|(i, (c, p))| {
                if c > 0.0 {
                    c - p
                } else {
                    // If energy reading is unavailable, compute it manually:
                    current_data.power[i] * time_diff
                }
            })
            .sum::<f64>();
        output_data.energy = energy_diff;
        output_data.total_energy += energy_diff;

        // Update the history:
        self.history.push(output_data.clone());
    }

    pub(super) fn get_average_power(&self) -> Result<f64, NVMLUtilsError> {
        if self.history.len() <= 2 {
            return Err(NVMLUtilsError {
                kind: NVMLUtilsErrorKind::NotEnoughData,
                message: "At least three measurements are required to compute average power"
                    .to_string(),
            });
        }
        let mut history = self.history.clone();
        // Remove first (heat-up) and last (cool-down) measurements,
        // as they are likely to be outliers:
        // TODO: Document
        history.remove(0);
        history.pop();

        let average_power = history.iter().map(|d| d.power).sum::<f64>() / history.len() as f64;

        Ok(average_power)
    }

    pub(super) fn get_total_energy(&self) -> Result<f64, NVMLUtilsError> {
        if self.history.is_empty() {
            return Err(NVMLUtilsError {
                kind: NVMLUtilsErrorKind::NotEnoughData,
                message: "No measurements have been taken yet".to_string(),
            });
        }

        let total_energy = self
            .history
            .last()
            .expect("History is empty, althouh it was checked to be non-empty")
            .total_energy;

        Ok(total_energy)
    }
}

impl Drop for GPUMonitor {
    fn drop(&mut self) {
        unsafe { nvmlShutdown() };
        log::debug!("Called nvmlShutdown()");
    }
}
