use anyhow::{bail, Result};
use mecha_device_info_ctl::DeviceInfoControl;
use mecha_metrics_ctl::DeviceMetricsCtl;

use crate::modules::device_info::errors::{DeviceInfoServiceError, DeviceInfoServiceErrorCodes};

pub struct DeviceInfoService {}

impl DeviceInfoService {
    pub async fn get_cpu_usage() -> Result<f32> {
        let task = "get_cpu_usage";
        let device_matrics = DeviceMetricsCtl::new();
        let cpu_usage = match device_matrics.get_cpu_usage() {
            Ok(v) => v,
            Err(e) => {
                bail!(DeviceInfoServiceError::new(
                    DeviceInfoServiceErrorCodes::GetDeviceInfoError,
                    format!("{}: Failed to get cpu usage: {}", task, e),
                    false
                ))
            }
        };
        Ok(cpu_usage)
    }

    pub async fn get_memory_usage() -> Result<f64> {
        let task = "get_memory_usage";
        let device_matrics = DeviceMetricsCtl::new();
        //use match to get total memory
        let device_info = DeviceInfoControl::new();

        let total_memory = match device_info.get_memory_info() {
            Ok(v) => v.total_memory,
            Err(e) => {
                bail!(DeviceInfoServiceError::new(
                    DeviceInfoServiceErrorCodes::GetDeviceInfoError,
                    format!("{}: Failed to get memory usage: {}", task, e),
                    false
                ))
            }
        };

        let memory_usage = match device_matrics.get_memory_usage() {
            Ok(v) => {
                let memory_usage = (v as f64 / total_memory as f64) * 100.0;
                memory_usage
            }
            Err(e) => {
                bail!(DeviceInfoServiceError::new(
                    DeviceInfoServiceErrorCodes::GetDeviceInfoError,
                    format!("{}: Failed to get memory usage: {}", task, e),
                    false
                ))
            }
        };
        Ok(memory_usage)
    }
}
