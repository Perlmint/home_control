#[cfg(test)]
use serde_json::json;

pub const BASE_URL: once_cell::sync::Lazy<reqwest::Url> = once_cell::sync::Lazy::new(|| {
    reqwest::Url::parse("https://api.smartthings.com/v1/devices/").unwrap()
});

#[derive(serde::Deserialize)]
pub struct AirconditionerStatus {}

#[derive(serde::Deserialize)]
pub struct DeviceDescriptor {
    pub label: String,
    pub components: Vec<ComponentDescriptor>,
}

#[derive(serde::Deserialize)]
pub struct ComponentDescriptor {
    pub id: String,
    pub label: String,
    pub capabilities: Vec<CapabilityWithVersion>,
}

#[derive(serde::Deserialize)]
pub struct CapabilityWithVersion {
    pub id: Capability,
    pub version: u8,
}

#[derive(Clone, Copy, PartialEq, Hash, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename = "camelCase")]
pub enum Capability {
    Switch,
    AirConditionerMode,
    AirConditionerFanMode,
    FanOscillationMode,
    TemperatureMeasurement,
    ThermostatCoolingSetpoint,
    RelativeHumidityMeasurement,
    AirQualitySensor,
    OdorSensor,
    DustSensor,
    VeryFineDustSensor,
    #[serde(other)]
    Other,
}

pub struct EnumMapIgnoreUnknown;
static END_OF_MAP_IDENTIFIER: &'static str = "__PRIVATE_MARKER__";

impl<'de, T> serde_with::DeserializeAs<'de, Vec<T>> for EnumMapIgnoreUnknown
where
    T: serde::Deserialize<'de>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{
            Deserialize, DeserializeSeed, Deserializer, EnumAccess, Error, MapAccess, SeqAccess,
            VariantAccess, Visitor,
        };
        struct EnumMapVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T> Visitor<'de> for EnumMapVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = Vec<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "a map or enum values")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                struct SeqDeserializer<M>(M);

                impl<'de, M> Deserializer<'de> for SeqDeserializer<M>
                where
                    M: MapAccess<'de>,
                {
                    type Error = M::Error;

                    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                    where
                        V: Visitor<'de>,
                    {
                        visitor.visit_seq(self)
                    }

                    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                    where
                        V: Visitor<'de>,
                    {
                        self.deserialize_seq(visitor)
                    }

                    serde::forward_to_deserialize_any! {
                        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                        bytes byte_buf option unit unit_struct newtype_struct tuple
                        tuple_struct map struct enum identifier ignored_any
                    }
                }
                impl<'de, M> SeqAccess<'de> for SeqDeserializer<M>
                where
                    M: MapAccess<'de>,
                {
                    type Error = M::Error;

                    fn next_element_seed<T>(
                        &mut self,
                        seed: T,
                    ) -> Result<Option<T::Value>, Self::Error>
                    where
                        T: DeserializeSeed<'de>,
                    {
                        struct EnumDeserializer<M>(M);

                        impl<'de, M> Deserializer<'de> for EnumDeserializer<M>
                        where
                            M: MapAccess<'de>,
                        {
                            type Error = M::Error;

                            fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                            where
                                V: Visitor<'de>,
                            {
                                self.deserialize_enum("", &[], visitor)
                            }

                            fn deserialize_enum<V>(
                                self,
                                _name: &'static str,
                                _variants: &'static [&'static str],
                                visitor: V,
                            ) -> Result<V::Value, Self::Error>
                            where
                                V: Visitor<'de>,
                            {
                                visitor.visit_enum(self) //
                            }

                            serde::forward_to_deserialize_any! {
                                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                                bytes byte_buf option unit unit_struct newtype_struct seq tuple
                                tuple_struct map struct identifier ignored_any
                            }
                        }

                        impl<'de, M> EnumAccess<'de> for EnumDeserializer<M>
                        where
                            M: MapAccess<'de>,
                        {
                            type Error = M::Error;
                            type Variant = Self;

                            fn variant_seed<T>(
                                mut self,
                                seed: T,
                            ) -> Result<(T::Value, Self::Variant), Self::Error>
                            where
                                T: DeserializeSeed<'de>,
                            {
                                match self.0.next_key_seed(seed).unwrap() {
                                    Some(key) => Ok((key, self)),

                                    // Unfortunately we loose the optional aspect of MapAccess, so we need to special case an error value to mark the end of the map.
                                    None => Err(Error::custom(END_OF_MAP_IDENTIFIER)),
                                }
                            }
                        }

                        impl<'de, M> VariantAccess<'de> for EnumDeserializer<M>
                        where
                            M: MapAccess<'de>,
                        {
                            type Error = M::Error;

                            fn unit_variant(mut self) -> Result<(), Self::Error> {
                                let _: serde_json::Value = self.0.next_value()?;
                                Ok(())
                            }

                            fn newtype_variant_seed<T>(
                                mut self,
                                seed: T,
                            ) -> Result<T::Value, Self::Error>
                            where
                                T: DeserializeSeed<'de>,
                            {
                                self.0.next_value_seed(seed)
                            }

                            fn tuple_variant<V>(
                                mut self,
                                len: usize,
                                visitor: V,
                            ) -> Result<V::Value, Self::Error>
                            where
                                V: Visitor<'de>,
                            {
                                self.0.next_value_seed(SeedTupleVariant { len, visitor })
                            }

                            fn struct_variant<V>(
                                mut self,
                                _fields: &'static [&'static str],
                                visitor: V,
                            ) -> Result<V::Value, Self::Error>
                            where
                                V: Visitor<'de>,
                            {
                                self.0.next_value_seed(SeedStructVariant { visitor })
                            }
                        }

                        struct SeedTupleVariant<V> {
                            len: usize,
                            visitor: V,
                        }

                        impl<'de, V> DeserializeSeed<'de> for SeedTupleVariant<V>
                        where
                            V: Visitor<'de>,
                        {
                            type Value = V::Value;

                            fn deserialize<D>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error>
                            where
                                D: Deserializer<'de>,
                            {
                                deserializer.deserialize_tuple(self.len, self.visitor)
                            }
                        }

                        struct SeedStructVariant<V> {
                            visitor: V,
                        }

                        impl<'de, V> DeserializeSeed<'de> for SeedStructVariant<V>
                        where
                            V: Visitor<'de>,
                        {
                            type Value = V::Value;

                            fn deserialize<D>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error>
                            where
                                D: Deserializer<'de>,
                            {
                                deserializer.deserialize_map(self.visitor)
                            }
                        }

                        match seed.deserialize(EnumDeserializer(&mut self.0)) {
                            Ok(value) => Ok(Some(value)),
                            Err(err) => {
                                // Unfortunately we loose the optional aspect of MapAccess, so we need to special case an error value to mark the end of the map.
                                if err.to_string().contains(END_OF_MAP_IDENTIFIER) {
                                    Ok(None)
                                } else {
                                    Err(err)
                                }
                            }
                        }
                    }

                    fn size_hint(&self) -> Option<usize> {
                        self.0.size_hint()
                    }
                }

                Vec::deserialize(SeqDeserializer(map))
            }
        }

        deserializer.deserialize_map(EnumMapVisitor(std::marker::PhantomData))
    }
}

#[serde_with::serde_as]
#[derive(Clone, serde::Deserialize)]
pub struct ComponentStatus(#[serde_as(as = "EnumMapIgnoreUnknown")] pub Vec<CapabilityStatus>);

#[test]
fn test_component_status_deserialization() {
    let statuses: ComponentStatus =
        serde_json::from_str(include_str!("./components_main_status.json")).unwrap();
    for status in statuses.0 {
        match status {
            CapabilityStatus::RelativeHumidityMeasurement { humidity } => {
                assert_eq!(humidity.value, 55);
                assert_eq!(humidity.unit, enums::HumidityUnit::Percent);
            }
            CapabilityStatus::ThermostatSetpointControl {
                minimum_setpoint,
                maximum_setpoint,
            } => {
                assert_eq!(minimum_setpoint.value, 16);
                assert_eq!(minimum_setpoint.unit, enums::TemperatureUnit::Celsius);
                assert_eq!(maximum_setpoint.value, 30);
                assert_eq!(maximum_setpoint.unit, enums::TemperatureUnit::Celsius);
            }
            CapabilityStatus::AirConditionerMode {
                supported_ac_modes,
                air_conditioner_mode,
            } => {
                assert_eq!(
                    supported_ac_modes.value,
                    [
                        enums::AirConditionerMode::Cool,
                        enums::AirConditionerMode::Dry,
                        enums::AirConditionerMode::Wind,
                        enums::AirConditionerMode::Auto,
                        enums::AirConditionerMode::AIComfort,
                    ]
                    .into()
                );
                assert_eq!(air_conditioner_mode.value, enums::AirConditionerMode::Wind);
            }
            _ => {}
        }
    }
}

pub mod enums {
    use std::hash::Hash;

    #[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum OnOff {
        On,
        Off,
    }

    impl From<bool> for OnOff {
        fn from(v: bool) -> Self {
            match v {
                true => Self::On,
                false => Self::Off,
            }
        }
    }

    impl From<&bool> for OnOff {
        fn from(v: &bool) -> Self {
            (*v).into()
        }
    }

    impl From<OnOff> for bool {
        fn from(v: OnOff) -> Self {
            v == OnOff::On
        }
    }

    impl From<&OnOff> for bool {
        fn from(v: &OnOff) -> Self {
            (*v).into()
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum AcOptionalMode {
        Off,
        WindFree,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum AirConditionerMode {
        AIComfort,
        Cool,
        Dry,
        Wind,
        Auto,
    }

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    pub enum TemperatureUnit {
        #[serde(rename = "C")]
        Celsius,
        #[serde(rename = "F")]
        Farenheit,
    }

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    pub enum HumidityUnit {
        #[serde(rename = "%")]
        Percent,
    }

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    pub enum DustLevelUnit {
        #[serde(rename = "μg/m^3")]
        MicroGramPerSquareMeter,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    pub enum AirConditionerFanMode {
        Auto,
        Low,
        Medium,
        High,
        Turbo,
    }
}

pub mod status {
    use super::enums;
    use std::{collections::HashSet, hash::Hash};

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    pub struct Switch {
        pub value: enums::OnOff,
    }

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AcMode {
        pub value: enums::AirConditionerMode,
    }

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AcOptionalMode {
        pub value: enums::AcOptionalMode,
    }

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FanMode {
        pub value: enums::AirConditionerFanMode,
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SupportedModes<M: Clone + PartialEq + Eq + Hash> {
        pub value: HashSet<M>,
    }

    pub type SupportedAcFanModes = SupportedModes<enums::AirConditionerFanMode>;
    pub type SupportedAcModes = SupportedModes<enums::AirConditionerMode>;
    pub type SupportedAcOptionalModes = SupportedModes<enums::AcOptionalMode>;

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ValueWithUnit<V: Clone + PartialEq, U: Clone + PartialEq> {
        pub value: V,
        pub unit: U,
    }

    pub type Temperature = ValueWithUnit<i16, enums::TemperatureUnit>;
    pub type DustLevel = ValueWithUnit<u16, enums::DustLevelUnit>;
    pub type Humidity = ValueWithUnit<u8, enums::HumidityUnit>;
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityStatus {
    #[serde(rename_all = "camelCase")]
    Switch { switch: status::Switch },
    #[serde(rename_all = "camelCase")]
    AirConditionerMode {
        supported_ac_modes: status::SupportedAcModes,
        air_conditioner_mode: status::AcMode,
    },
    #[serde(rename = "custom.thermostatSetpointControl")]
    #[serde(rename_all = "camelCase")]
    ThermostatSetpointControl {
        minimum_setpoint: status::Temperature,
        maximum_setpoint: status::Temperature,
    },
    #[serde(rename_all = "camelCase")]
    TemperatureMeasurement { temperature: status::Temperature },
    #[serde(rename_all = "camelCase")]
    ThermostatCoolingSetpoint {
        cooling_setpoint: status::Temperature,
    },
    #[serde(rename_all = "camelCase")]
    DustSensor {
        dust_level: status::DustLevel,
        fine_dust_level: status::DustLevel,
    },
    #[serde(rename_all = "camelCase")]
    RelativeHumidityMeasurement { humidity: status::Humidity },
    #[serde(rename_all = "camelCase")]
    VeryFineDustSensor {
        very_fine_dust_level: status::DustLevel,
    },
    #[serde(other)]
    Unknown,
}

pub mod command {
    use crate::enums;

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct CommandWithoutArguments<T: serde::Serialize + std::fmt::Debug + Clone> {
        command: T,
    }

    impl<T: serde::Serialize + std::fmt::Debug + Clone> CommandWithoutArguments<T> {
        pub fn new<I: Into<T>>(command: I) -> Self {
            Self {
                command: command.into(),
            }
        }
    }

    pub type Switch = CommandWithoutArguments<enums::OnOff>;

    struct AsArguments<T>(std::marker::PhantomData<T>);

    impl<T: serde::Serialize> serde_with::SerializeAs<T> for AsArguments<T> {
        fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut strct = serializer.serialize_struct("", 1)?;
            strct.serialize_field("arguments", &(source,))?;
            strct.end()
        }
    }

    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    #[serde(tag = "command")]
    pub enum AirConditionerMode {
        #[serde(with = "serde_with::As::<AsArguments::<_>>")]
        SetAirConditionerMode(enums::AirConditionerMode),
    }

    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    #[serde(tag = "command")]
    pub enum ThermostatCoolingSetpoint {
        #[serde(with = "serde_with::As::<AsArguments::<_>>")]
        SetCoolingSetpoint(i16),
    }
}

macro_rules! enum_of_types {
    ($(#[$attrs:meta])*
    $visability:vis enum $name:ident {
        $($item_name:ident($ty:path),)*
    }) => {
        $(#[$attrs])*
        $visability enum $name {
            $($item_name($ty),)*
        }

        $(impl From<$ty> for $name {
            fn from(v: $ty) -> Self {
                Self::$item_name(v)
            }
        })*
    };
}

enum_of_types! {
    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    #[serde(tag = "capability")]
    pub enum CapabilityCommand {
        Switch(command::Switch),
        AirConditionerMode(command::AirConditionerMode),
        ThermostatCoolingSetpoint(command::ThermostatCoolingSetpoint),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityCommandRequest {
    commands: Vec<CapabilityCommand>,
}

#[test]
fn test_command_serialization() {
    assert_eq!(
        serde_json::to_value(&CapabilityCommandRequest {
            commands: vec![command::Switch::new(true).into()],
        })
        .unwrap(),
        json!({
            "commands": [
                {
                    "capability": "switch",
                    "command": "on"
                }
            ]
        })
    );

    assert_eq!(
        serde_json::to_value(&CapabilityCommandRequest {
            commands: vec![command::ThermostatCoolingSetpoint::SetCoolingSetpoint(18).into()],
        })
        .unwrap(),
        json!({
            "commands": [
                {
                    "capability": "thermostatCoolingSetpoint",
                    "command": "setCoolingSetpoint",
                    "arguments": [18]
                }
            ]
        })
    );
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed while prepare request URL")]
    UrlError(#[from] url::ParseError),
    #[error("Failed while send request")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed while parsing response")]
    ParseError(#[from] serde_json::Error),
}

#[derive(Debug, serde::Deserialize)]
pub struct CommandResult {
    pub id: String,
    pub status: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CommandResponse {
    pub results: Vec<CommandResult>,
}

pub struct ApiClient {
    pub token: String,
    http_client: reqwest::Client,
}

impl ApiClient {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn descriptor(&self, device_id: &str) -> Result<DeviceDescriptor, Error> {
        Ok(self
            .http_client
            .get(BASE_URL.join(device_id)?)
            .bearer_auth(&self.token)
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn component_status(
        &self,
        device_id: &str,
        component_name: &str,
    ) -> Result<ComponentStatus, Error> {
        Ok(self
            .http_client
            .get(BASE_URL.join(&format!(
                "{}/components/{}/status",
                device_id, component_name
            ))?)
            .bearer_auth(&self.token)
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn capability_status(
        &self,
        device_id: &str,
        component_name: &str,
        capability: Capability,
    ) -> Result<CapabilityStatus, Error> {
        Ok(self
            .http_client
            .get(BASE_URL.join(&format!(
                "{}/components/{}/capabilities/{}/status",
                device_id,
                component_name,
                serde_qs::to_string(&capability).unwrap(),
            ))?)
            .bearer_auth(&self.token)
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn command<T: Into<CapabilityCommand>>(
        &self,
        device_id: &str,
        command: T,
    ) -> Result<(), Error> {
        let body = CapabilityCommandRequest {
            commands: vec![command.into()],
        };
        let ret: CommandResponse = self
            .http_client
            .post(BASE_URL.join(&format!("{}/commands", device_id))?)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        eprintln!("{:?}", ret);
        Ok(())
    }
}
