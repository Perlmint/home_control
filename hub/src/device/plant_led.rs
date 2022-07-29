use reqwest::Url;

use anyhow::Context;

use crate::{Error, ErrorWrap};
use google_smart_home::{
    Attributes, Command, Device, DeviceName, DeviceWithDetail, State, States, Trait, Type,
};

#[derive(serde::Deserialize)]
pub struct PlantLedConfig {
    pub host: String,
    pub internal_id: u8,
}

pub struct PlantLed {
    api_endpoint: Url,
}

impl PlantLed {
    pub async fn new(config: PlantLedConfig) -> anyhow::Result<Self> {
        Ok(Self {
            api_endpoint: Url::parse(&format!(
                "http://{}/lights/{}/power",
                config.host, config.internal_id
            ))
            .unwrap(),
        })
    }

    fn parse_plant_led_response(body: String) -> Result<States, Error> {
        let raw_brightness: u16 = body
            .trim()
            .parse()
            .with_context(|| format!("Brightness parse failed - {}", &body))
            .server_error()?;

        let brightness = (raw_brightness as f32 / 255f32 * 100f32) as _;

        log::debug!("plant led brightness: {} -> {}", &body, brightness);

        Ok(States(vec![
            State::OnOff {
                on: Some(raw_brightness != 0),
            },
            State::Brightness {
                brightness: Some(brightness),
            },
        ]))
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
            traits: vec![Trait::OnOff, Trait::Brightness],
            attributes: Attributes(vec![]),
            r#type: Type::Light,
            will_report_state: false,
        }
    }

    async fn query(&self) -> Result<States, Error> {
        let body = reqwest::get(self.api_endpoint.clone())
            .await
            .server_error()?
            .text()
            .await
            .server_error()?;

        Self::parse_plant_led_response(body)
    }

    async fn execute(&self, executions: &Vec<Command>) -> Result<States, Error> {
        let query = match executions.get(0).unwrap() {
            Command::OnOff { on } => if *on { 128 } else { 0 }.to_string(),
            Command::BrightnessAbsolute { brightness } => {
                ((*brightness as f32 * 255f32 / 100f32) as u8).to_string()
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
