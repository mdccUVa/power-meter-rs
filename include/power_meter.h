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

typedef enum PwrmError {
  PWRM_SUCCESS = 0,
  PWRM_PATH_ERROR = -1,
  PWRM_MONITORING_ERROR = -2,
  PWRM_NOT_ENOUGH_DATA = -3,
} pwrmResult_t;

void pwrm_launch_monitoring_loop(uint64_t sampling_interval_ms);
pwrmResult_t pwrm_stop_monitoring_loop(void);
pwrmResult_t pwrm_get_avg_cpu_power(double *avg_power_out);
pwrmResult_t pwrm_get_avg_gpu_power(double *avg_power_out);
pwrmResult_t pwrm_get_total_cpu_energy(double *total_energy_out);
pwrmResult_t pwrm_get_total_gpu_energy(double *total_energy_out);
void pwrm_reset_counters(void);
pwrmResult_t pwrm_set_output_dir(const char *path_ptr);
pwrmResult_t pwrm_set_cpu_out_filename(const char *filename_ptr);
pwrmResult_t pwrm_set_gpu_out_filename(const char *filename_ptr);

#ifdef __cplusplus
}
#endif // __cplusplus
#endif // POWER_METER_H
