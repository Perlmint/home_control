mod plant_led;
pub use plant_led::*;

use crate::{
    local_home::{state::State, DeviceWithDetail, Execution},
    Error,
};

#[async_trait::async_trait]
pub trait HomeDevice {
    fn sync(&self, global_id: &str) -> DeviceWithDetail;
    async fn query(&self) -> Result<State, Error>;
    async fn execute(&self, executions: &Vec<Execution>) -> Result<State, Error>;
}
