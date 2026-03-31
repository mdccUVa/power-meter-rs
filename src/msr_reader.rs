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

use std::{fmt::Display, fs::File, os::unix::fs::FileExt};

pub(super) enum MSRErrorKind {
    Privileges,
    Read,
}

pub(super) struct MSRError {
    kind: MSRErrorKind,
    message: String,
}

impl Display for MSRError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[MSR Reader] ERROR: {}", self.message)
    }
}

fn open_msr(core: u32) -> Result<File, MSRError> {
    let path = format!("/dev/cpu/{}/msr", core);
    let alt_path = format!("/dev/cpu/{}/msr_safe", core);
    let msr_file = match File::open(&path) {
        Ok(file) => Ok(file),
        Err(_) => File::open(&alt_path),
    };

    msr_file.map_err(|e| MSRError {
        kind: MSRErrorKind::Privileges,
        message: format!(
            "Could not open MSR file {} (needs root access) nor MSR safe file {} (not set up) {}",
            path, alt_path, e
        ),
    })
}

fn read_msr(msr_file: File, address: u64) -> Result<u64, MSRError> {
    // Read 64 bits from the file at the specified address:
    let mut buffer = [0u8; 8];
    if let Err(e) = msr_file.read_exact_at(&mut buffer, address as u64) {
        return Err(MSRError {
            kind: MSRErrorKind::Read,
            message: format!("Failed to read MSR at address {}: {}", address, e),
        });
    };

    Ok(u64::from_le_bytes(buffer))
}

pub(super) fn read_msr_fields(
    core: u32,
    msr_address: u64,
    msr_numfields: usize,
    msr_offsets: &[usize],
    msr_sizes: &[usize],
    msr_values: &mut [u64],
) -> Result<(), MSRError> {
    let msr_file = open_msr(core)?;
    let data = read_msr(msr_file, msr_address)?;

    // Parse the fields and store their values:
    for i in 0..msr_numfields {
        let mut field = data >> msr_offsets[i];
        field &= (1 << msr_sizes[i]) - 1;
        msr_values[i] = field;
    }

    Ok(())
}
