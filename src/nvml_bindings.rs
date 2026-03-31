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

#![allow(nonstandard_style)]

/* Generated with the help of rust-bindgen 0.72.1 */

#[doc = " Return values for NVML API calls."]
pub(crate) type nvmlReturn_enum = ::std::os::raw::c_uint;

#[doc = "!< The operation was successful"]
pub(crate) const NVML_SUCCESS: nvmlReturn_enum = 0;

#[doc = " Return values for NVML API calls."]
pub(crate) use self::nvmlReturn_enum as nvmlReturn_t;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct nvmlDevice_st {
    _unused: [u8; 0],
}
pub(crate) type nvmlDevice_t = *mut nvmlDevice_st;
#[repr(transparent)]
#[derive(Clone, Debug)]
pub(crate) struct NvmlDevice(pub(crate) nvmlDevice_t);
unsafe impl Send for NvmlDevice {} // Making nvmlDevice_t Send is required.

unsafe extern "C" {
    #[doc = " Initialize NVML, but don't initialize any GPUs yet.\n\n \\note nvmlInit_v3 introduces a \"flags\" argument, that allows passing boolean values\n       modifying the behaviour of nvmlInit().\n \\note In NVML 5.319 new nvmlInit_v2 has replaced nvmlInit\"_v1\" (default in NVML 4.304 and older) that\n       did initialize all GPU devices in the system.\n\n This allows NVML to communicate with a GPU\n when other GPUs in the system are unstable or in a bad state.  When using this API, GPUs are\n discovered and initialized in nvmlDeviceGetHandleBy* functions instead.\n\n \\note To contrast nvmlInit_v2 with nvmlInit\"_v1\", NVML 4.304 nvmlInit\"_v1\" will fail when any detected GPU is in\n       a bad or unstable state.\n\n For all products.\n\n This method, should be called once before invoking any other methods in the library.\n A reference count of the number of initializations is maintained.  Shutdown only occurs\n when the reference count reaches zero.\n\n @return\n         - \\ref NVML_SUCCESS                   if NVML has been properly initialized\n         - \\ref NVML_ERROR_DRIVER_NOT_LOADED   if NVIDIA driver is not running\n         - \\ref NVML_ERROR_NO_PERMISSION       if NVML does not have permission to talk to the driver\n         - \\ref NVML_ERROR_UNKNOWN             on any unexpected error"]
    pub(crate) fn nvmlInit_v2() -> nvmlReturn_t;
}

unsafe extern "C" {
    #[doc = " Shut down NVML by releasing all GPU resources previously allocated with \\ref nvmlInit_v2().\n\n For all products.\n\n This method should be called after NVML work is done, once for each call to \\ref nvmlInit_v2()\n A reference count of the number of initializations is maintained.  Shutdown only occurs\n when the reference count reaches zero.  For backwards compatibility, no error is reported if\n nvmlShutdown() is called more times than nvmlInit().\n\n @return\n         - \\ref NVML_SUCCESS                 if NVML has been properly shut down\n         - \\ref NVML_ERROR_UNINITIALIZED     if the library has not been successfully initialized\n         - \\ref NVML_ERROR_UNKNOWN           on any unexpected error"]
    pub(crate) fn nvmlShutdown() -> nvmlReturn_t;
}

unsafe extern "C" {
    #[doc = " Retrieves the number of compute devices in the system. A compute device is a single GPU.\n\n For all products.\n\n Note: New nvmlDeviceGetCount_v2 (default in NVML 5.319) returns count of all devices in the system\n       even if nvmlDeviceGetHandleByIndex_v2 returns NVML_ERROR_NO_PERMISSION for such device.\n       Update your code to handle this error, or use NVML 4.304 or older nvml header file.\n       For backward binary compatibility reasons _v1 version of the API is still present in the shared\n       library.\n       Old _v1 version of nvmlDeviceGetCount doesn't count devices that NVML has no permission to talk to.\n\n @param deviceCount                          Reference in which to return the number of accessible devices\n\n @return\n         - \\ref NVML_SUCCESS                 if \\a deviceCount has been set\n         - \\ref NVML_ERROR_UNINITIALIZED     if the library has not been successfully initialized\n         - \\ref NVML_ERROR_INVALID_ARGUMENT  if \\a deviceCount is NULL\n         - \\ref NVML_ERROR_UNKNOWN           on any unexpected error"]
    pub(crate) fn nvmlDeviceGetCount_v2(deviceCount: *mut ::std::os::raw::c_uint) -> nvmlReturn_t;
}

unsafe extern "C" {
    #[doc = " Acquire the handle for a particular device, based on its index.\n\n For all products.\n\n Valid indices are derived from the \\a accessibleDevices count returned by\n   \\ref nvmlDeviceGetCount_v2(). For example, if \\a accessibleDevices is 2 the valid indices\n   are 0 and 1, corresponding to GPU 0 and GPU 1.\n\n The order in which NVML enumerates devices has no guarantees of consistency between reboots. For that reason it\n   is recommended that devices be looked up by their PCI ids or UUID. See\n   \\ref nvmlDeviceGetHandleByUUID() and \\ref nvmlDeviceGetHandleByPciBusId_v2().\n\n Note: The NVML index may not correlate with other APIs, such as the CUDA device index.\n\n Starting from NVML 5, this API causes NVML to initialize the target GPU\n NVML may initialize additional GPUs if:\n  - The target GPU is an SLI slave\n\n Note: New nvmlDeviceGetCount_v2 (default in NVML 5.319) returns count of all devices in the system\n       even if nvmlDeviceGetHandleByIndex_v2 returns NVML_ERROR_NO_PERMISSION for such device.\n       Update your code to handle this error, or use NVML 4.304 or older nvml header file.\n       For backward binary compatibility reasons _v1 version of the API is still present in the shared\n       library.\n       Old _v1 version of nvmlDeviceGetCount doesn't count devices that NVML has no permission to talk to.\n\n       This means that nvmlDeviceGetHandleByIndex_v2 and _v1 can return different devices for the same index.\n       If you don't touch macros that map old (_v1) versions to _v2 versions at the top of the file you don't\n       need to worry about that.\n\n @param index                                The index of the target GPU, >= 0 and < \\a accessibleDevices\n @param device                               Reference in which to return the device handle\n\n @return\n         - \\ref NVML_SUCCESS                  if \\a device has been set\n         - \\ref NVML_ERROR_UNINITIALIZED      if the library has not been successfully initialized\n         - \\ref NVML_ERROR_INVALID_ARGUMENT   if \\a index is invalid or \\a device is NULL\n         - \\ref NVML_ERROR_INSUFFICIENT_POWER if any attached devices have improperly attached external power cables\n         - \\ref NVML_ERROR_NO_PERMISSION      if the user doesn't have permission to talk to this device\n         - \\ref NVML_ERROR_IRQ_ISSUE          if NVIDIA kernel detected an interrupt issue with the attached GPUs\n         - \\ref NVML_ERROR_GPU_IS_LOST        if the target GPU has fallen off the bus or is otherwise inaccessible\n         - \\ref NVML_ERROR_UNKNOWN            on any unexpected error\n\n @see nvmlDeviceGetIndex\n @see nvmlDeviceGetCount"]
    pub(crate) fn nvmlDeviceGetHandleByIndex_v2(
        index: ::std::os::raw::c_uint,
        device: *mut nvmlDevice_t,
    ) -> nvmlReturn_t;
}

unsafe extern "C" {
    #[doc = " Retrieves total energy consumption for this GPU in millijoules (mJ) since the driver was last reloaded\n\n For Volta &tm; or newer fully supported devices.\n\n @param device                               The identifier of the target device\n @param energy                               Reference in which to return the energy consumption information\n\n @return\n         - \\ref NVML_SUCCESS                 if \\a energy has been populated\n         - \\ref NVML_ERROR_UNINITIALIZED     if the library has not been successfully initialized\n         - \\ref NVML_ERROR_INVALID_ARGUMENT  if \\a device is invalid or \\a energy is NULL\n         - \\ref NVML_ERROR_NOT_SUPPORTED     if the device does not support energy readings\n         - \\ref NVML_ERROR_GPU_IS_LOST       if the target GPU has fallen off the bus or is otherwise inaccessible\n         - \\ref NVML_ERROR_UNKNOWN           on any unexpected error"]
    pub(crate) fn nvmlDeviceGetTotalEnergyConsumption(
        device: nvmlDevice_t,
        energy: *mut ::std::os::raw::c_ulonglong,
    ) -> nvmlReturn_t;
}

unsafe extern "C" {
    #[doc = " Retrieves power usage for this GPU in milliwatts and its associated circuitry (e.g. memory)\n\n For Fermi &tm; or newer fully supported devices.\n\n On Fermi and Kepler GPUs the reading is accurate to within +/- 5% of current power draw.\n\n It is only available if power management mode is supported. See \\ref nvmlDeviceGetPowerManagementMode.\n\n @param device                               The identifier of the target device\n @param power                                Reference in which to return the power usage information\n\n @return\n         - \\ref NVML_SUCCESS                 if \\a power has been populated\n         - \\ref NVML_ERROR_UNINITIALIZED     if the library has not been successfully initialized\n         - \\ref NVML_ERROR_INVALID_ARGUMENT  if \\a device is invalid or \\a power is NULL\n         - \\ref NVML_ERROR_NOT_SUPPORTED     if the device does not support power readings\n         - \\ref NVML_ERROR_GPU_IS_LOST       if the target GPU has fallen off the bus or is otherwise inaccessible\n         - \\ref NVML_ERROR_UNKNOWN           on any unexpected error"]
    pub(crate) fn nvmlDeviceGetPowerUsage(
        device: nvmlDevice_t,
        power: *mut ::std::os::raw::c_uint,
    ) -> nvmlReturn_t;
}
