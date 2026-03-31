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

#ifndef POWER_METER_H
#define POWER_METER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

void pwmr_launch_monitoring_loop(uint64_t sampling_interval_ms);
void pwmr_stop_monitoring_loop(void);
void pwmr_set_output_dir(const char *path_ptr);
void pwmr_set_cpu_out_filename(const char *filename_ptr);
void pwmr_set_gpu_out_filename(const char *filename_ptr);

#ifdef __cplusplus
}
#endif // __cplusplus
#endif // POWER_METER_H
