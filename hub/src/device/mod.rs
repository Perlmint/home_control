mod plant_led;
pub use plant_led::*;
mod samsung_air_conditioner;
pub use samsung_air_conditioner::*;

use crate::Error;
use google_smart_home::{Command, DeviceWithDetail, States};

#[async_trait::async_trait]
pub trait HomeDevice {
    fn sync(&self, global_id: &str) -> DeviceWithDetail;
    async fn query(&self) -> Result<States, Error>;
    async fn execute(&self, executions: &Vec<Command>) -> Result<States, Error>;
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum DeviceConfigs {
    PlantLed(PlantLedConfig),
    SamsungAirConditioner(SamsungAirConditionerConfig),
}

impl DeviceConfigs {
    pub async fn create_device(self) -> anyhow::Result<Box<dyn HomeDevice + Send + Sync>> {
        Ok(match self {
            DeviceConfigs::PlantLed(config) => {
                log::trace!("create PlantLed");
                Box::new(PlantLed::new(config).await?)
            },
            DeviceConfigs::SamsungAirConditioner(config) => {
                log::trace!("create samsung air conditioner");
                Box::new(SamsungAirConditioner::new(config).await?)
            }
        })
    }
}
