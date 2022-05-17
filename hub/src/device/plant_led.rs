use reqwest::Url;

use anyhow::Context;

use crate::{
    local_home::{
        state::{Brightness, OnOff, State},
        Device, DeviceName, DeviceTrait, DeviceType, DeviceWithDetail, Execution,
    },
    Error, ErrorWrap,
};

pub struct PlantLedConfig {
    pub host: String,
    pub internal_id: u8,
}

pub struct PlantLed {
    api_endpoint: Url,
}

impl PlantLed {
    pub fn new(config: PlantLedConfig) -> Self {
        Self {
            api_endpoint: Url::parse(&format!(
                "http://{}/lights/{}/power",
                config.host, config.internal_id
            ))
            .unwrap(),
        }
    }

    fn parse_plant_led_response(body: String) -> Result<State, Error> {
        let raw_brightness: u16 = body
            .trim()
            .parse()
            .with_context(|| format!("Brightness parse failed - {}", &body))
            .server_error()?;

        let brightness = (raw_brightness as f32 / 255f32 * 100f32) as _;

        log::debug!("plant led brightness: {} -> {}", &body, brightness);

        Ok(State {
            on_off: Some(OnOff {
                on: raw_brightness != 0,
            }),
            brightness: Some(Brightness { brightness }),
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl super::HomeDevice for PlantLed {
    fn sync(&self, global_id: &str) -> DeviceWithDetail {
        DeviceWithDetail {
            basic: Device {
                id: global_id.to_string(),
                custom_data: Default::default(),
            },
            name: DeviceName {
                name: "식물등".to_string(),
                default_names: Default::default(),
                nicknames: Default::default(),
            },
            device_info: None,
            other_device_ids: Default::default(),
            room_hint: None,
            traits: vec![DeviceTrait::OnOff, DeviceTrait::Brightness],
            r#type: DeviceType::Light,
            will_report_state: false,
        }
    }

    async fn query(&self) -> Result<State, Error> {
        let body = reqwest::get(self.api_endpoint.clone())
            .await
            .server_error()?
            .text()
            .await
            .server_error()?;

        Self::parse_plant_led_response(body)
    }

    async fn execute(&self, executions: &Vec<Execution>) -> Result<State, Error> {
        let query = match executions.get(0).unwrap() {
            Execution::OnOff(onoff) => if onoff.on { 128 } else { 0 }.to_string(),
            Execution::BrightnessAbsolute(brightness) => {
                ((brightness.brightness as f32 * 255f32 / 100f32) as u8).to_string()
            }
            _ => {
                unreachable!();
            }
        };
        log::info!("set plant led power {}", &query);

        let client = reqwest::Client::new();
        let request = client
            .put(self.api_endpoint.clone())
            .header(reqwest::header::CONTENT_LENGTH, query.len())
            .body(query)
            .build()
            .server_error()?;
        log::debug!("{:?}\n{:?}", &request, request.body());
        let body = client
            .execute(request)
            .await
            .server_error()?
            .text()
            .await
            .server_error()?;

        Self::parse_plant_led_response(body)
    }
}
