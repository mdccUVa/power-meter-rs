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

#[path = "msr_reader.rs"]
mod msr_reader;

use std::{
    fmt::Display,
    fs::{self, File},
    io::Read,
    time::Instant,
};

/* CONSTANTS */

const INTEL_MSR_RAPL_POWER_UNIT: u64 = 0x606;
const INTEL_MSR_RAPL_POWER_UNIT_NUMFIELDS: usize = 3;
const INTEL_MSR_RAPL_POWER_UNIT_NAMES: [&str; INTEL_MSR_RAPL_POWER_UNIT_NUMFIELDS] =
    ["Power Units", "Energy Status Units", "Time Units"];
const INTEL_MSR_RAPL_POWER_UNIT_SIZES: [usize; INTEL_MSR_RAPL_POWER_UNIT_NUMFIELDS] = [4, 5, 4];
const INTEL_MSR_RAPL_POWER_UNIT_OFFSETS: [usize; INTEL_MSR_RAPL_POWER_UNIT_NUMFIELDS] = [0, 8, 16];

const AMD_MSR_RAPL_POWER_UNIT: u64 = 0xC0010299;
const AMD_MSR_RAPL_POWER_UNIT_NUMFIELDS: usize = 3;
const AMD_MSR_RAPL_POWER_UNIT_NAMES: [&str; AMD_MSR_RAPL_POWER_UNIT_NUMFIELDS] =
    ["Power Units", "Energy Status Units", "Time Units"];
const AMD_MSR_RAPL_POWER_UNIT_SIZES: [usize; AMD_MSR_RAPL_POWER_UNIT_NUMFIELDS] = [4, 5, 4];
const AMD_MSR_RAPL_POWER_UNIT_OFFSETS: [usize; AMD_MSR_RAPL_POWER_UNIT_NUMFIELDS] = [0, 8, 16];

const INTEL_MSR_PKG_ENERGY_STATUS: u64 = 0x611;
const INTEL_MSR_PKG_ENERGY_STATUS_NUMFIELDS: usize = 1;
const INTEL_MSR_PKG_ENERGY_STATUS_NAMES: [&str; INTEL_MSR_PKG_ENERGY_STATUS_NUMFIELDS] =
    ["Total Energy Consumed"];
const INTEL_MSR_PKG_ENERGY_STATUS_SIZES: [usize; INTEL_MSR_PKG_ENERGY_STATUS_NUMFIELDS] = [32];
const INTEL_MSR_PKG_ENERGY_STATUS_OFFSETS: [usize; INTEL_MSR_PKG_ENERGY_STATUS_NUMFIELDS] = [0];

const AMD_MSR_PKG_ENERGY_STATUS: u64 = 0xC001029B;
const AMD_MSR_PKG_ENERGY_STATUS_NUMFIELDS: usize = 1;
const AMD_MSR_PKG_ENERGY_STATUS_NAMES: [&str; AMD_MSR_PKG_ENERGY_STATUS_NUMFIELDS] =
    ["Total Energy Consumed"];
const AMD_MSR_PKG_ENERGY_STATUS_SIZES: [usize; AMD_MSR_PKG_ENERGY_STATUS_NUMFIELDS] = [32];
const AMD_MSR_PKG_ENERGY_STATUS_OFFSETS: [usize; AMD_MSR_PKG_ENERGY_STATUS_NUMFIELDS] = [0];

const INTEL_MSR_PP0_ENERGY_STATUS: u64 = 0x639;
const INTEL_MSR_PP0_ENERGY_STATUS_NUMFIELDS: usize = 1;
const INTEL_MSR_PP0_ENERGY_STATUS_NAMES: [&str; INTEL_MSR_PP0_ENERGY_STATUS_NUMFIELDS] =
    ["Total Energy Consumed"];
const INTEL_MSR_PP0_ENERGY_STATUS_SIZES: [usize; INTEL_MSR_PP0_ENERGY_STATUS_NUMFIELDS] = [32];
const INTEL_MSR_PP0_ENERGY_STATUS_OFFSETS: [usize; INTEL_MSR_PP0_ENERGY_STATUS_NUMFIELDS] = [0];

const AMD_MSR_CORE_ENERGY_STATUS: u64 = 0xC001029A;
const AMD_MSR_CORE_ENERGY_STATUS_NUMFIELDS: usize = 1;
const AMD_MSR_CORE_ENERGY_STATUS_NAMES: [&str; AMD_MSR_CORE_ENERGY_STATUS_NUMFIELDS] =
    ["Total Energy Consumed"];
const AMD_MSR_CORE_ENERGY_STATUS_SIZES: [usize; AMD_MSR_CORE_ENERGY_STATUS_NUMFIELDS] = [32];
const AMD_MSR_CORE_ENERGY_STATUS_OFFSETS: [usize; AMD_MSR_CORE_ENERGY_STATUS_NUMFIELDS] = [0];

const INTEL_MSR_PKG_POWER_INFO: u64 = 0x614;
const INTEL_MSR_PKG_POWER_INFO_NUMFIELDS: usize = 4;
const INTEL_MSR_PKG_POWER_INFO_NAMES: [&str; INTEL_MSR_PKG_POWER_INFO_NUMFIELDS] = [
    "Thermal Spec Power",
    "Minimum Power",
    "Maximum Power",
    "Maximum Time Window",
];
const INTEL_MSR_PKG_POWER_INFO_SIZES: [usize; INTEL_MSR_PKG_POWER_INFO_NUMFIELDS] = [15, 15, 15, 6];
const INTEL_MSR_PKG_POWER_INFO_OFFSETS: [usize; INTEL_MSR_PKG_POWER_INFO_NUMFIELDS] =
    [0, 16, 32, 48];

/// Limit on the number of NUMA nodes supported by this utility. Arbitrarily set.
const MAX_NUDA_NODES_SUPPORTED: usize = 8;

/* DATA */

#[derive(Clone, Copy, Debug)]
pub(super) enum RAPLDomain {
    Package,
    Core,
    Uncore,
    DRAM,
}

/// Struct containing per-node energy measurements along with the time they were taken.
#[derive(Clone, Debug)]
pub(super) struct EnergyAux {
    /// Timestamp when the struct was last updated.
    time: Instant,
    /// Last measured energy values for each NUMA node.
    energy: [f64; MAX_NUDA_NODES_SUPPORTED],
}

impl Default for EnergyAux {
    fn default() -> Self {
        Self {
            time: Instant::now(),
            energy: [0f64; MAX_NUDA_NODES_SUPPORTED],
        }
    }
}

/// Struct storing the machine's average power draw and energy consumption during the last
/// measurement interval, as well as the total energy consumption.
#[derive(Clone, Default, Debug)]
pub(super) struct EnergyData {
    pub(super) power: f64,
    pub(super) energy: f64,
    pub(super) total_energy: f64,
}

#[derive(Clone, Default, Debug)]
pub(super) struct IntelMeasures {
    power_units: [u64; INTEL_MSR_RAPL_POWER_UNIT_NUMFIELDS],
    pkg_energy_status: [u64; INTEL_MSR_PKG_ENERGY_STATUS_NUMFIELDS],
    pp0_energy_status: [u64; INTEL_MSR_PP0_ENERGY_STATUS_NUMFIELDS],
    pkg_power_info: [u64; INTEL_MSR_PKG_POWER_INFO_NUMFIELDS],
}

#[derive(Clone, Default, Debug)]
pub(super) struct AMDMeasures {
    power_units: [u64; AMD_MSR_RAPL_POWER_UNIT_NUMFIELDS],
    pkg_energy_status: [u64; AMD_MSR_PKG_ENERGY_STATUS_NUMFIELDS],
    core_energy_status: [u64; AMD_MSR_CORE_ENERGY_STATUS_NUMFIELDS],
}

#[derive(Clone, Debug)]
pub(super) enum RAPLMeasures {
    Intel(IntelMeasures),
    AMD(AMDMeasures),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Vendor {
    Intel,
    AMD,
}

#[derive(Clone, Debug)]
pub(super) struct CPUMonitor {
    measures: RAPLMeasures,
    power_increment: f64,
    energy_increment: f64,
    time_increment: f64,
    max_energy_value: f64,
    numa_nodes: usize,
    vendor_id: Vendor,
    first_node_core: Box<Vec<u32>>,
    num_cores: u32,
    history: Vec<EnergyData>,
}

#[derive(PartialEq, Eq, Debug)]
pub(super) enum RAPLUtilsErrorKind {
    UnknownVendor,
    BadSystemFile,
    UnsupportedDomain,
    UnsupportedOp,
    MSRRead,
    NotEnoughData,
}

#[derive(Debug)]
pub(super) struct RAPLUtilsError {
    pub(super) kind: RAPLUtilsErrorKind,
    pub(super) message: String,
}

impl Display for RAPLUtilsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[RAPL Utils] ERROR: {}", self.message)
    }
}

/* UTILITY FUNCTIONS */

impl CPUMonitor {
    const INTEL_VENDOR_ID: u32 = 0x6c65746e; // "ntel" from "GenuineIntel" in little-endian ASCII.
    const AMD_VENDOR_ID: u32 = 0x446d4163; // "cAMD" from "AuthenticAMD" in little-endian ASCII.

    pub(super) fn new() -> Result<Self, RAPLUtilsError> {
        // We need to ID the CPU vendor as the MSR register use differente addresses on Intel/AMD.
        let vendor_id: u32 = core::arch::x86_64::__cpuid(0).ecx;
        let vendor = match vendor_id {
            CPUMonitor::INTEL_VENDOR_ID => Vendor::Intel,
            CPUMonitor::AMD_VENDOR_ID => Vendor::AMD,
            _ => {
                return Err(RAPLUtilsError {
                    kind: RAPLUtilsErrorKind::UnknownVendor,
                    message: format!("CPU Vendor ID not recognised: {:#x}", vendor_id),
                })
            }
        };

        // Get the number of NUMA nodes.
        // /sys/devices/system/node/online contains a list of node IDs separated by "-".
        // The length in characters of the file will be 2 for one node ("0\n"),
        // and increase by 2 for each successive node:
        let Ok(mut nodes_file) = File::open("/sys/devices/system/node/online") else {
            return Err(RAPLUtilsError {
                kind: RAPLUtilsErrorKind::BadSystemFile,
                message: "Could not open /sys/devices/system/node/online".to_string(),
            });
        };
        let mut node_list = String::new();
        let numa_nodes = match nodes_file.read_to_string(&mut node_list) {
            Ok(count) => count / 2,
            Err(e) => {
                return Err(RAPLUtilsError {
                    kind: RAPLUtilsErrorKind::BadSystemFile,
                    message: format!(
                        "Could not read /sys/devices/system/node/online: {}",
                        e.to_string()
                    ),
                })
            }
        };

        let mut first_node_core = Box::new(vec![0; numa_nodes]);
        for (node, first_core) in first_node_core.iter_mut().enumerate() {
            let node_path = format!("/sys/devices/system/node/node{}/cpulist", node);
            let Ok(mut node_file) = File::open(&node_path) else {
                return Err(RAPLUtilsError {
                    kind: RAPLUtilsErrorKind::BadSystemFile,
                    message: format!("Could not open {}", node_path),
                });
            };
            let mut cpu_list = String::new();
            if let Err(e) = node_file.read_to_string(&mut cpu_list) {
                return Err(RAPLUtilsError {
                    kind: RAPLUtilsErrorKind::BadSystemFile,
                    message: format!("Could not read {}: {}", node_path, e.to_string()),
                });
            }
            // Read the file and get the first token, i.e., the ID of the first core in the NUMA
            // node:
            *first_core = match cpu_list.split("-").next() {
                Some(s) => s.parse::<u32>().or_else(|e| {
                    Err(RAPLUtilsError {
                        kind: RAPLUtilsErrorKind::BadSystemFile,
                        message: format!("Could not parse {}: {}", node_path, e.to_string()),
                    })
                })?,
                None => {
                    return Err(RAPLUtilsError {
                        kind: RAPLUtilsErrorKind::BadSystemFile,
                        message: format!("Could not parse {}", node_path),
                    })
                }
            }
        }

        // Get the total number of cores, which is the number of the last online core
        // (should account for all cores in the system) plus one:
        let Ok(mut online_cores) = fs::read_to_string("/sys/devices/system/cpu/online") else {
            return Err(RAPLUtilsError {
                kind: RAPLUtilsErrorKind::BadSystemFile,
                message: "Could not read /sys/devices/system/cpu/online".to_string(),
            });
        };
        online_cores.pop(); // Remove trailing newline.
        let num_cores = match online_cores.split("-").last() {
            Some(s) => {
                s.parse::<u32>().or_else(|e| {
                    Err(RAPLUtilsError {
                        kind: RAPLUtilsErrorKind::BadSystemFile,
                        message: format!(
                            "Could not parse /sys/devices/system/cpu/online: {}",
                            e.to_string()
                        ),
                    })
                })? + 1
            }
            None => {
                return Err(RAPLUtilsError {
                    kind: RAPLUtilsErrorKind::BadSystemFile,
                    message: "Could not parse /sys/devices/system/cpu/online".to_string(),
                })
            }
        };

        let mut measures = match vendor {
            Vendor::Intel => RAPLMeasures::Intel(IntelMeasures::default()),
            Vendor::AMD => RAPLMeasures::AMD(AMDMeasures::default()),
        };

        let (power_increment, energy_increment, time_increment) = match vendor {
            Vendor::Intel => {
                let measures = match &mut measures {
                    RAPLMeasures::Intel(m) => m,
                    _ => unreachable!(),
                };
                read_intel_msr_rapl_power_unit(0, &mut measures.power_units)?;
                (
                    1.0 / (1 << measures.power_units[0]) as f64,
                    1.0 / (1 << measures.power_units[1]) as f64,
                    1.0 / (1 << measures.power_units[2]) as f64,
                )
            }
            Vendor::AMD => {
                let measures = match &mut measures {
                    RAPLMeasures::AMD(m) => m,
                    _ => unreachable!(),
                };
                read_amd_msr_rapl_power_unit(0, &mut measures.power_units)?;
                (
                    1.0 / (1 << measures.power_units[0]) as f64,
                    1.0 / (1 << measures.power_units[1]) as f64,
                    1.0 / (1 << measures.power_units[2]) as f64,
                )
            }
        };

        // The maximum value of the energy counter is 2^32, stored here in Joules:
        let energy_counter_max = (1u64 << 32) as f64 * energy_increment;

        log::info!("Number of NUMA nodes detected: {}", numa_nodes);

        Ok(Self {
            measures,
            power_increment,
            energy_increment,
            time_increment,
            max_energy_value: energy_counter_max,
            numa_nodes: numa_nodes,
            vendor_id: vendor,
            first_node_core,
            num_cores,
            history: Vec::new(),
        })
    }

    pub(super) fn get_node_energy(
        &mut self,
        node: usize,
        domain: RAPLDomain,
    ) -> Result<f64, RAPLUtilsError> {
        match domain {
            RAPLDomain::Package => match self.vendor_id {
                Vendor::Intel => {
                    let measures = match &mut self.measures {
                        RAPLMeasures::Intel(m) => m,
                        _ => unreachable!(),
                    };
                    read_intel_msr_pkg_energy_status(
                        self.first_node_core[node],
                        &mut measures.pkg_energy_status,
                    )?;
                    Ok(measures.pkg_energy_status[0] as f64 * self.energy_increment as f64)
                }
                Vendor::AMD => {
                    let measures = match &mut self.measures {
                        RAPLMeasures::AMD(m) => m,
                        _ => unreachable!(),
                    };
                    read_amd_msr_pkg_energy_status(
                        self.first_node_core[node],
                        &mut measures.pkg_energy_status,
                    )?;
                    Ok(measures.pkg_energy_status[0] as f64 * self.energy_increment as f64)
                }
            },
            RAPLDomain::Core => match self.vendor_id {
                Vendor::Intel => {
                    let measures = match &mut self.measures {
                        RAPLMeasures::Intel(m) => m,
                        _ => unreachable!(),
                    };
                    read_intel_msr_pp0_energy_status(
                        self.first_node_core[node],
                        &mut measures.pp0_energy_status,
                    )?;
                    Ok(measures.pp0_energy_status[0] as f64 * self.energy_increment as f64)
                }
                Vendor::AMD => {
                    let measures = match &mut self.measures {
                        RAPLMeasures::AMD(m) => m,
                        _ => unreachable!(),
                    };
                    read_amd_msr_core_energy_status(
                        self.first_node_core[node],
                        &mut measures.core_energy_status,
                    )?;
                    Ok(measures.core_energy_status[0] as f64 * self.energy_increment as f64)
                }
            },
            RAPLDomain::Uncore => {
                // Uncore energy measurement is not supported on AMD CPUs, and on Intel it is only
                // supported on some models and requires reading from a different MSR register.
                Err(RAPLUtilsError {
                    kind: RAPLUtilsErrorKind::UnsupportedDomain,
                    message: "Reading from RAPL's Uncore domain is not yet implemented".to_string(),
                })
            }
            RAPLDomain::DRAM => {
                // DRAM energy measurement is not supported on AMD CPUs, and on Intel it is only
                // supported on some models and requires reading from a different MSR register.
                Err(RAPLUtilsError {
                    kind: RAPLUtilsErrorKind::UnsupportedDomain,
                    message: "Reading from RAPL's DRAM domain is not yet implemented".to_string(),
                })
            }
        }
    }

    pub(super) fn update_aux_data(
        &mut self,
        data: &mut EnergyAux,
        domain: RAPLDomain,
    ) -> Result<(), RAPLUtilsError> {
        for node in 0..self.numa_nodes {
            data.energy[node] = self.get_node_energy(node, domain)?;
        }
        data.time = Instant::now();

        Ok(())
    }

    pub(super) fn update_package_energy(
        &mut self,
        data: &mut EnergyAux,
    ) -> Result<(), RAPLUtilsError> {
        self.update_aux_data(data, RAPLDomain::Package)
    }

    pub(super) fn update_cores_energy(
        &mut self,
        data: &mut EnergyAux,
    ) -> Result<(), RAPLUtilsError> {
        self.update_aux_data(data, RAPLDomain::Core)
    }

    pub(super) fn get_energy_diff(&self, current_energy: &[f64], previous_energy: &[f64]) -> f64 {
        let mut energy_diff = 0f64;
        for (current, previous) in current_energy.iter().zip(previous_energy.iter()) {
            let node_energy_diff = current - previous
                + if current < previous {
                    // If the energy counter has wrapped around for this node, we need to add the
                    // value before wrapping around (i.e., max possible value) to the diff.
                    // This is 2^32 per Intel's specification.
                    self.max_energy_value
                } else {
                    0f64
                };
            energy_diff += node_energy_diff;
        }

        energy_diff
    }

    pub(super) fn get_power(&self, previous_data: &EnergyAux, current_data: &EnergyAux) -> f64 {
        let time_diff = current_data.time.duration_since(previous_data.time);
        let energy_diff = self.get_energy_diff(&current_data.energy, &previous_data.energy);

        // Power = Energy delta in Joules / Time delta in seconds.
        energy_diff / time_diff.as_secs_f64()
    }

    pub(super) fn update_energy_data(
        &mut self,
        output_data: &mut EnergyData,
        previous_data: &EnergyAux,
        current_data: &EnergyAux,
    ) {
        // Store average power consumption for this interval:
        output_data.power = self.get_power(previous_data, current_data);
        // Get energy delta, taking into account the counter wrap-around:
        let energy_diff = self.get_energy_diff(&current_data.energy, &previous_data.energy);
        // Store energy consumed during this interval:
        output_data.energy = energy_diff;
        // Update the total energy consumed by this node:
        output_data.total_energy += energy_diff;

        // Update the history:
        self.history.push(output_data.clone());
    }

    pub(super) fn get_average_power(&self) -> Result<f64, RAPLUtilsError> {
        if self.history.is_empty() {
            return Err(RAPLUtilsError {
                kind: RAPLUtilsErrorKind::NotEnoughData,
                message: "No measurements have been taken yet".to_string(),
            });
        } else if self.history.len() <= 2 {
            return Err(RAPLUtilsError {
                kind: RAPLUtilsErrorKind::NotEnoughData,
                message: "At least three measurements are required to compute the average power"
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

    pub(super) fn get_total_energy(&self) -> Result<f64, RAPLUtilsError> {
        if self.history.is_empty() {
            return Err(RAPLUtilsError {
                kind: RAPLUtilsErrorKind::NotEnoughData,
                message: "No measurements have been taken yet".to_string(),
            });
        }

        let total_energy = self
            .history
            .last()
            .expect("History is empty, although it was checked to be non-empty")
            .total_energy;

        Ok(total_energy)
    }

    pub(super) fn get_processor_tdp(&mut self) -> Result<f64, RAPLUtilsError> {
        if self.vendor_id == Vendor::AMD {
            return Err(RAPLUtilsError {
                kind: RAPLUtilsErrorKind::UnsupportedOp,
                message: "get_processor_tdp() is only supported by Intel CPUs".to_string(),
            });
        }

        let mut total_tdp = 0.0;
        for node in 0..self.numa_nodes {
            let measures = match &mut self.measures {
                RAPLMeasures::Intel(m) => m,
                _ => unreachable!(),
            };
            read_intel_msr_pkg_power_info(
                self.first_node_core[node],
                &mut measures.pkg_power_info,
            )?;
            total_tdp += measures.pkg_power_info[0] as f64;
        }

        Ok(total_tdp * self.power_increment)
    }
}

fn read_intel_msr_rapl_power_unit(
    core: u32,
    output: &mut [u64; INTEL_MSR_RAPL_POWER_UNIT_NUMFIELDS],
) -> Result<(), RAPLUtilsError> {
    msr_reader::read_msr_fields(
        core,
        INTEL_MSR_RAPL_POWER_UNIT,
        INTEL_MSR_RAPL_POWER_UNIT_NUMFIELDS,
        &INTEL_MSR_RAPL_POWER_UNIT_OFFSETS,
        &INTEL_MSR_RAPL_POWER_UNIT_SIZES,
        output,
    )
    .map_err(|e| RAPLUtilsError {
        kind: RAPLUtilsErrorKind::MSRRead,
        message: format!(
            "Failed to read Intel RAPL Power Unit MSR (0x{:x}): {}",
            INTEL_MSR_RAPL_POWER_UNIT, e
        ),
    })
}

fn read_intel_msr_pkg_energy_status(
    core: u32,
    output: &mut [u64; INTEL_MSR_PKG_ENERGY_STATUS_NUMFIELDS],
) -> Result<(), RAPLUtilsError> {
    msr_reader::read_msr_fields(
        core,
        INTEL_MSR_PKG_ENERGY_STATUS,
        INTEL_MSR_PKG_ENERGY_STATUS_NUMFIELDS,
        &INTEL_MSR_PKG_ENERGY_STATUS_OFFSETS,
        &INTEL_MSR_PKG_ENERGY_STATUS_SIZES,
        output,
    )
    .map_err(|e| RAPLUtilsError {
        kind: RAPLUtilsErrorKind::MSRRead,
        message: format!(
            "Failed to read Intel Package Energy Status MSR (0x{:x}): {}",
            INTEL_MSR_PKG_ENERGY_STATUS, e
        ),
    })
}

fn read_intel_msr_pp0_energy_status(
    core: u32,
    output: &mut [u64; INTEL_MSR_PP0_ENERGY_STATUS_NUMFIELDS],
) -> Result<(), RAPLUtilsError> {
    msr_reader::read_msr_fields(
        core,
        INTEL_MSR_PP0_ENERGY_STATUS,
        INTEL_MSR_PP0_ENERGY_STATUS_NUMFIELDS,
        &INTEL_MSR_PP0_ENERGY_STATUS_OFFSETS,
        &INTEL_MSR_PP0_ENERGY_STATUS_SIZES,
        output,
    )
    .map_err(|e| RAPLUtilsError {
        kind: RAPLUtilsErrorKind::MSRRead,
        message: format!(
            "Failed to read Intel PP0 Energy Status MSR (0x{:x}): {}",
            INTEL_MSR_PP0_ENERGY_STATUS, e
        ),
    })
}

fn read_intel_msr_pkg_power_info(
    core: u32,
    output: &mut [u64; INTEL_MSR_PKG_POWER_INFO_NUMFIELDS],
) -> Result<(), RAPLUtilsError> {
    msr_reader::read_msr_fields(
        core,
        INTEL_MSR_PKG_POWER_INFO,
        INTEL_MSR_PKG_POWER_INFO_NUMFIELDS,
        &INTEL_MSR_PKG_POWER_INFO_OFFSETS,
        &INTEL_MSR_PKG_POWER_INFO_SIZES,
        output,
    )
    .map_err(|e| RAPLUtilsError {
        kind: RAPLUtilsErrorKind::MSRRead,
        message: format!(
            "Failed to read Intel Package Power Info MSR (0x{:x}): {}",
            INTEL_MSR_PKG_POWER_INFO, e
        ),
    })
}

fn read_amd_msr_rapl_power_unit(
    core: u32,
    output: &mut [u64; INTEL_MSR_RAPL_POWER_UNIT_NUMFIELDS],
) -> Result<(), RAPLUtilsError> {
    msr_reader::read_msr_fields(
        core,
        AMD_MSR_RAPL_POWER_UNIT,
        AMD_MSR_RAPL_POWER_UNIT_NUMFIELDS,
        &AMD_MSR_RAPL_POWER_UNIT_OFFSETS,
        &AMD_MSR_RAPL_POWER_UNIT_SIZES,
        output,
    )
    .map_err(|e| RAPLUtilsError {
        kind: RAPLUtilsErrorKind::MSRRead,
        message: format!(
            "Failed to read AMD RAPL Power Unit MSR (0x{:x}): {}",
            AMD_MSR_RAPL_POWER_UNIT, e
        ),
    })
}

fn read_amd_msr_pkg_energy_status(
    core: u32,
    output: &mut [u64; AMD_MSR_PKG_ENERGY_STATUS_NUMFIELDS],
) -> Result<(), RAPLUtilsError> {
    msr_reader::read_msr_fields(
        core,
        AMD_MSR_PKG_ENERGY_STATUS,
        AMD_MSR_PKG_ENERGY_STATUS_NUMFIELDS,
        &AMD_MSR_PKG_ENERGY_STATUS_OFFSETS,
        &AMD_MSR_PKG_ENERGY_STATUS_SIZES,
        output,
    )
    .map_err(|e| RAPLUtilsError {
        kind: RAPLUtilsErrorKind::MSRRead,
        message: format!(
            "Failed to read AMD Package Energy Status MSR (0x{:x}): {}",
            AMD_MSR_PKG_ENERGY_STATUS, e
        ),
    })
}

fn read_amd_msr_core_energy_status(
    core: u32,
    output: &mut [u64; AMD_MSR_CORE_ENERGY_STATUS_NUMFIELDS],
) -> Result<(), RAPLUtilsError> {
    msr_reader::read_msr_fields(
        core,
        AMD_MSR_CORE_ENERGY_STATUS,
        AMD_MSR_CORE_ENERGY_STATUS_NUMFIELDS,
        &AMD_MSR_CORE_ENERGY_STATUS_OFFSETS,
        &AMD_MSR_CORE_ENERGY_STATUS_SIZES,
        output,
    )
    .map_err(|e| RAPLUtilsError {
        kind: RAPLUtilsErrorKind::MSRRead,
        message: format!(
            "Failed to read AMD Core Energy Status MSR (0x{:x}): {}",
            AMD_MSR_CORE_ENERGY_STATUS, e
        ),
    })
}
