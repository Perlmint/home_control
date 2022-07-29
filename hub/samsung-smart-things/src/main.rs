use samsung_smart_things as samsung;

#[tokio::main]
async fn main() {
    env_logger::init();
    const TOKEN: &str = "";
    const DEVICE_ID: &str = "";

    let client = samsung::ApiClient::new(TOKEN);
    client.command(
        DEVICE_ID,
        samsung::command::Switch::new(false),
        // samsung::command::ThermostatCoolingSetpoint::SetCoolingSetpoint(20)
        // samsung::command::AirConditionerMode::SetAirConditionerMode(samsung::enums::AirConditionerMode::Wind),
    ).await.unwrap();
}