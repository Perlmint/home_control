use crate::{Error, ErrorWrap};
use google_smart_home as google;
use samsung_smart_things as samsung;

use super::HomeDevice;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SamsungAirConditionerConfig {
    pub token: String, // https://account.smartthings.com/tokens
    pub device_id: String,
}

pub struct SamsungAirConditioner {
    client: samsung::ApiClient,
    device_id: String,
    name: String,
    traits: Vec<google::Trait>,
    attributes: google::Attributes,
}

impl SamsungAirConditioner {
    pub async fn new(config: SamsungAirConditionerConfig) -> anyhow::Result<Self> {
        let client = samsung::ApiClient::new(&config.token);
        let device_id = config.device_id;
        let descriptor = client.descriptor(&device_id).await?;

        let name = descriptor.label;

        let main_status = client.component_status(&device_id, "main").await?;

        let mut traits = Vec::new();
        let mut attributes = google::Attributes(Vec::new());

        let mut available_thermostat_modes = Vec::new();
        let mut thermostat_temperature_range = None;
        let mut thermostat_temperature_unit =
            google::TemperatureSetting_thermostatTemperatureUnit::C;

        for capability in main_status.0 {
            match capability {
                samsung::CapabilityStatus::Switch { .. } => traits.push(google::Trait::OnOff),
                samsung::CapabilityStatus::AirConditionerMode {
                    supported_ac_modes: supperted_ac_modes, ..
                } => {
                    traits.push(google::Trait::TemperatureSetting);
                    for mode in supperted_ac_modes.value {
                        let mode = match mode {
                            samsung::enums::AirConditionerMode::Cool => {
                                Some(google::ThermostatMode::Cool)
                            }
                            samsung::enums::AirConditionerMode::Wind => {
                                Some(google::ThermostatMode::FanOnly)
                            }
                            samsung::enums::AirConditionerMode::Dry => {
                                Some(google::ThermostatMode::Dry)
                            }
                            _ => None,
                        };
                        if let Some(mode) = mode {
                            available_thermostat_modes.push(mode);
                        }
                    }
                }
                samsung::CapabilityStatus::ThermostatSetpointControl {
                    minimum_setpoint,
                    maximum_setpoint,
                } => {
                    assert_eq!(minimum_setpoint.unit, maximum_setpoint.unit);
                    thermostat_temperature_unit = match minimum_setpoint.unit {
                        samsung::enums::TemperatureUnit::Celsius => {
                            google::TemperatureSetting_thermostatTemperatureUnit::C
                        }
                        samsung::enums::TemperatureUnit::Farenheit => {
                            google::TemperatureSetting_thermostatTemperatureUnit::F
                        }
                    };
                    thermostat_temperature_range = Some(google::TemperatureRange {
                        max_threshold_celsius: maximum_setpoint.value as _,
                        min_threshold_celsius: minimum_setpoint.value as _,
                    })
                }
                samsung::CapabilityStatus::DustSensor { .. } => {
                    traits.push(google::Trait::SensorState);
                    attributes.0.push(
                        google::Attribute::SensorState {
                            sensor_states_supported: vec![
                                google::SensorStateSupported::Pm25 {
                                    numeric_capabilities: google::PmSensorStateSupported {
                                        raw_value_unit: Some(google::PmSensorStateSupported_rawValueUnit::MicrogramsPerCubicMeter),
                                    }
                                },
                                google::SensorStateSupported::Pm10 {
                                    numeric_capabilities: google::PmSensorStateSupported {
                                        raw_value_unit: Some(google::PmSensorStateSupported_rawValueUnit::MicrogramsPerCubicMeter),
                                    }
                                }
                            ]
                        }
                    );
                }
                samsung::CapabilityStatus::RelativeHumidityMeasurement { .. } => {
                    traits.push(google::Trait::HumiditySetting);
                    attributes.0.push(google::Attribute::HumiditySetting {
                        command_only_humidity_setting: Some(false),
                        humidity_setpoint_range: None,
                        query_only_humidity_setting: Some(true),
                    })
                }
                _ => {}
            }
        }

        if !available_thermostat_modes.is_empty() {
            attributes.0.push(google::Attribute::TemperatureSetting {
                available_thermostat_modes,
                buffer_range_celsius: None,
                command_only_temperature_setting: None,
                query_only_temperature_setting: None,
                thermostat_temperature_range,
                thermostat_temperature_unit,
            })
        }

        Ok(Self {
            client,
            device_id,
            name,
            traits,
            attributes,
        })
    }

    async fn query_status(&self) -> anyhow::Result<google::States> {
        let main_status = self
            .client
            .component_status(&self.device_id, "main")
            .await?;

        let mut ret = google::States(Vec::new());
        let mut active_thermostat_mode = None;
        let mut thermostat_humidity_ambient = None;
        let mut thermostat_temperature_ambient = 0.0;
        let mut thermostat_temperature_setpoint = 0.0;

        for capability in main_status.0 {
            match capability {
                samsung::CapabilityStatus::Switch { switch } => {
                    ret.0.push(google::State::OnOff {
                        on: Some(switch.value.into()),
                    });
                }
                samsung::CapabilityStatus::AirConditionerMode {
                    air_conditioner_mode,
                    ..
                } => {
                    active_thermostat_mode = match air_conditioner_mode.value {
                        samsung::enums::AirConditionerMode::Cool => {
                            Some(google::ThermostatMode::Cool)
                        }
                        samsung::enums::AirConditionerMode::Wind => {
                            Some(google::ThermostatMode::FanOnly)
                        }
                        samsung::enums::AirConditionerMode::Dry => {
                            Some(google::ThermostatMode::Dry)
                        }
                        _ => None,
                    };
                }
                samsung::CapabilityStatus::ThermostatSetpointControl { .. } => {
                    // ignore
                }
                samsung::CapabilityStatus::TemperatureMeasurement { temperature } => {
                    thermostat_temperature_ambient = temperature.value as _;
                }
                samsung::CapabilityStatus::ThermostatCoolingSetpoint { cooling_setpoint: cooling_set_point } => {
                    thermostat_temperature_setpoint = cooling_set_point.value as _;
                }
                samsung::CapabilityStatus::DustSensor {
                    dust_level,
                    fine_dust_level,
                } => {
                    ret.0.push(google::State::SensorState {
                        current_sensor_state_data: vec![
                            google::SensorState::Pm25 {
                                raw_value: fine_dust_level.value as _,
                            },
                            google::SensorState::Pm10 {
                                raw_value: dust_level.value as _,
                            },
                        ],
                    });
                }
                samsung::CapabilityStatus::RelativeHumidityMeasurement { humidity } => {
                    thermostat_humidity_ambient = Some(humidity.value as _);
                }
                _ => {}
            }
        }

        ret.0.push(google::State::TemperatureSetting {
            active_thermostat_mode: active_thermostat_mode.clone(),
            target_temp_reached_estimate_unix_timestamp_sec: None,
            thermostat_humidity_ambient,
            _details: google::TemperatureSettingDetail::SingleTemperaturSetting {
                thermostat_mode: active_thermostat_mode.unwrap(),
                thermostat_temperature_ambient,
                thermostat_temperature_setpoint,
            },
        });

        Ok(ret)
    }
}

#[async_trait::async_trait]
impl HomeDevice for SamsungAirConditioner {
    fn sync(&self, global_id: &str) -> google::DeviceWithDetail {
        google::DeviceWithDetail {
            basic: google::Device {
                id: global_id.to_string(),
                custom_data: Default::default(),
            },
            r#type: google::Type::AcUnit,
            attributes: self.attributes.clone(),
            traits: self.traits.clone(),
            name: google::DeviceName {
                default_names: vec![],
                name: self.name.clone(),
                nicknames: vec![],
            },
            will_report_state: false,
            room_hint: None,
            device_info: None,
            other_device_ids: vec![],
        }
    }

    async fn query(&self) -> Result<google::States, Error> {
        self.query_status().await.server_error()
    }

    async fn execute(&self, executions: &Vec<google::Command>) -> Result<google::States, Error> {
        let mut states = Vec::new();

        for command in executions {
            match command {
                google::Command::OnOff { on } => {
                    if self.client.command(
                        &self.device_id,
                        samsung::command::Switch::new(on),
                    ).await.is_ok() {
                        states.push(google::State::OnOff {
                            on: Some(*on),
                        })
                    }
                },
                google::Command::ThermostatSetMode { thermostat_mode } => self.client.command(
                    &self.device_id,
                    samsung::command::AirConditionerMode::SetAirConditionerMode(match thermostat_mode {
                        google::ThermostatMode::Auto => samsung::enums::AirConditionerMode::Auto,
                        google::ThermostatMode::Dry => samsung::enums::AirConditionerMode::Dry,
                        google::ThermostatMode::FanOnly => samsung::enums::AirConditionerMode::Wind,
                        google::ThermostatMode::Cool => samsung::enums::AirConditionerMode::Cool,
                        _ => unreachable!("Unknown mode"),
                    }),
                ).await.unwrap(),
                google::Command::ThermostatTemperatureSetpoint {
                    thermostat_temperature_setpoint,
                } => self.client.command(
                    &self.device_id,
                    samsung::command::ThermostatCoolingSetpoint::SetCoolingSetpoint(
                        *thermostat_temperature_setpoint as _,
                    ),
                ).await.unwrap(),
                _ => unreachable!(),
            }
        }

        Ok(google::States(states))
    }
}
