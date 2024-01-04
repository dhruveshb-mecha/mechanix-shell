use relm4::Sender;
use std::time::Duration;
use tokio::{sync::oneshot, time};

use tracing::{error, info};

use crate::Message;

use super::service::DeviceInfoService;

#[derive(Debug)]
pub enum ServiceMessage {
    Start { respond_to: oneshot::Sender<u32> },
    Stop { respond_to: oneshot::Sender<u32> },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ServiceStatus {
    INACTIVE = 0,
    STARTED = 1,
    STOPPED = -1,
}

pub struct DeviceInfoServiceHandle {
    status: ServiceStatus,
}

impl DeviceInfoServiceHandle {
    pub fn new() -> Self {
        Self {
            status: ServiceStatus::INACTIVE,
        }
    }

    pub async fn run(&mut self, sender: Sender<Message>) {
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            match DeviceInfoService::get_cpu_usage().await {
                Ok(cpu_usage) => {
                    info!("cpu usage {}", cpu_usage);
                    let _ = sender.send(Message::CpuUsgaeStatusChanged(cpu_usage));
                }
                Err(e) => {
                    error!("error while getting cpu status {}", e);
                    // let _ = sender.send(Message::BatteryStatusUpdate(BatteryState::NotFound));
                }
            };

            match DeviceInfoService::get_memory_usage().await {
                Ok(memory_usage) => {
                    info!("memory usage {}", memory_usage);
                    let _ = sender.send(Message::MemoryUsageStatusChanged(memory_usage));
                }
                Err(e) => {
                    error!("error while getting memory status {}", e);
                    // let _ = sender.send(Message::BatteryStatusUpdate(BatteryState::NotFound));
                }
            };
        }
    }

    pub fn stop(&mut self) {
        self.status = ServiceStatus::STOPPED;
    }

    pub fn start(&mut self) {
        self.status = ServiceStatus::STARTED;
    }
}
