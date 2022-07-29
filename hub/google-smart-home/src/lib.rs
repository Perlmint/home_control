use std::collections::HashMap;

#[cfg(feature = "generate")]
mod gen {
    #![allow(non_camel_case_types)]
    include! {concat!(env!("OUT_DIR"), "/smart-home.rs")}
}
#[cfg(not(feature = "generate"))]
mod gen;
pub use gen::*;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub request_id: String,
    pub inputs: Vec<Intent>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum Response {
    EmptyResponse,
    ResponseWithPayload(ResponseWithPayload),
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseWithPayload {
    pub request_id: String,
    pub payload: ResponsePayload,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ResponsePayload {
    Sync(SyncResponse),
    Query(QueryResponse),
    Execute(ExecuteResponse),
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub agent_user_id: String,
    pub devices: Vec<DeviceWithDetail>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse {
    pub devices: HashMap<String, StateOrError>,
}

#[derive(Debug)]
pub enum StateOrError {
    State(States),
    Error(Error),
}

impl serde::Serialize for StateOrError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            StateOrError::State(state) => state.serialize(serializer),
            StateOrError::Error(error) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("status", &Status::Error)?;
                map.serialize_entry("errorCode", error)?;
                map.end()
            }
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteResponse {
    pub commands: Vec<StatusReport>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub custom_data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceWithDetail {
    #[serde(flatten)]
    pub basic: Device,
    pub r#type: Type,
    pub traits: Vec<Trait>,
    pub attributes: Attributes,
    pub name: DeviceName,
    pub will_report_state: bool,
    pub room_hint: Option<String>,
    pub device_info: Option<()>,
    pub other_device_ids: Vec<()>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceName {
    pub default_names: Vec<String>,
    pub name: String,
    pub nicknames: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub devices: Vec<Device>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandsForDevices {
    pub devices: Vec<Device>,
    pub execution: Vec<Command>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteRequest {
    pub commands: Vec<CommandsForDevices>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Success,
    Offline,
    Exceptions,
    Error,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    pub ids: Vec<String>,
    pub status: StateOrError,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "intent", content = "payload")]
pub enum Intent {
    #[serde(rename = "action.devices.SYNC")]
    Sync,
    #[serde(rename = "action.devices.QUERY")]
    Query(QueryRequest),
    #[serde(rename = "action.devices.DISCONNECT")]
    Disconnect,
    #[serde(rename = "action.devices.EXECUTE")]
    Execute(ExecuteRequest),
}

mod serialize_helper {
    use serde::{
        ser::{Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant},
        Serialize, Serializer,
    };

    struct SeqAsMapSerializer<S>(pub S);

    macro_rules! NA {
        () => {
            panic!()
        };
    }

    impl<S> serde::Serializer for SeqAsMapSerializer<S>
    where
        S: serde::ser::SerializeMap,
    {
        type Ok = S::Ok;
        type Error = S::Error;

        type SerializeSeq = SerializeSeqElement<S>;
        type SerializeTuple = Impossible<S::Ok, S::Error>;
        type SerializeTupleStruct = Impossible<S::Ok, S::Error>;
        type SerializeTupleVariant = Impossible<S::Ok, S::Error>;
        type SerializeMap = Impossible<S::Ok, S::Error>;
        type SerializeStruct = Impossible<S::Ok, S::Error>;
        type SerializeStructVariant = Impossible<S::Ok, S::Error>;

        fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
            NA!()
        }
        fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
            NA!()
        }
        fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
            NA!()
        }
        fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
            NA!()
        }
        fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
            NA!()
        }
        fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize,
        {
            NA!()
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_newtype_struct<T: ?Sized>(
            self,
            _name: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize,
        {
            NA!()
        }

        fn serialize_newtype_variant<T: ?Sized>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: serde::Serialize,
        {
            NA!()
        }

        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(SerializeSeqElement {
                is_human_readable: self.is_human_readable(),
                delegate: self.0,
            })
        }

        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            NA!()
        }

        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            NA!()
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            NA!()
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            NA!()
        }

        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            NA!()
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            NA!()
        }
    }

    struct SerializeSeqElement<S> {
        delegate: S,
        is_human_readable: bool,
    }

    impl<M> SerializeSeq for SerializeSeqElement<M>
    where
        M: SerializeMap,
    {
        type Ok = M::Ok;
        type Error = M::Error;

        fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            value.serialize(EnumAsMapElementSerializer {
                delegate: &mut self.delegate,
                is_human_readable: self.is_human_readable,
            })?;
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            self.delegate.end()
        }
    }

    struct EnumAsMapElementSerializer<'a, M> {
        delegate: &'a mut M,
        is_human_readable: bool,
    }

    impl<'a, M> Serializer for EnumAsMapElementSerializer<'a, M>
    where
        M: SerializeMap,
    {
        type Ok = ();
        type Error = M::Error;

        type SerializeSeq = Impossible<Self::Ok, Self::Error>;
        type SerializeTuple = Impossible<Self::Ok, Self::Error>;
        type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
        type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
        type SerializeMap = SerializeVariant<'a, M>;
        type SerializeStruct = Impossible<Self::Ok, Self::Error>;
        type SerializeStructVariant = SerializeVariant<'a, M>;

        fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            NA!()
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            NA!()
        }

        fn serialize_newtype_struct<T: ?Sized>(
            self,
            _name: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            NA!()
        }

        fn serialize_newtype_variant<T: ?Sized>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            value.serialize(self)
        }

        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            NA!()
        }

        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            NA!()
        }

        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            NA!()
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            NA!()
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(SerializeVariant {
                delegate: self.delegate,
                is_human_readable: self.is_human_readable,
            })
        }

        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            NA!()
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(SerializeVariant {
                delegate: self.delegate,
                is_human_readable: self.is_human_readable,
            })
        }
    }

    struct SerializeVariant<'a, M> {
        delegate: &'a mut M,
        is_human_readable: bool,
    }

    impl<'a, M> SerializeMap for SerializeVariant<'a, M>
    where
        M: SerializeMap,
    {
        type Ok = ();

        type Error = M::Error;

        fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            self.delegate.serialize_key(key)?;
            Ok(())
        }

        fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            self.delegate.serialize_value(value)?;
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(())
        }
    }

    impl<'a, M> SerializeStruct for SerializeVariant<'a, M>
    where
        M: SerializeMap,
    {
        type Ok = ();
        type Error = M::Error;

        fn serialize_field<T: ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            self.delegate.serialize_entry(key, value)?;
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(())
        }
    }

    impl<'a, M> SerializeStructVariant for SerializeVariant<'a, M>
    where
        M: SerializeMap,
    {
        type Ok = ();
        type Error = M::Error;

        fn serialize_field<T: ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error>
        where
            T: Serialize,
        {
            self.delegate.serialize_entry(key, value)?;
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(())
        }

        fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
            let _ = key;
            Ok(())
        }
    }

    pub fn serialize_as_merged_struct<T: serde::Serialize, S: serde::Serializer>(
        serializer: S,
        items: &Vec<T>,
    ) -> Result<S::Ok, S::Error> {
        let map_serializer = serializer.serialize_map(None)?;
        items.serialize(SeqAsMapSerializer(map_serializer))
    }
}

impl serde::Serialize for States {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_helper::serialize_as_merged_struct(serializer, &self.0)
    }
}

impl serde::Serialize for Attributes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_helper::serialize_as_merged_struct(serializer, &self.0)
    }
}

#[test]
fn parse_sync_request() {
    let req: Request = serde_json::from_str(
        r#"{
        "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
        "inputs": [{
          "intent": "action.devices.SYNC"
        }]
    }"#,
    )
    .expect("Failed to parse sync request");

    assert!(matches!(req.inputs[0], Intent::Sync));
}

#[test]
fn parse_query_request() {
    let req: Request = serde_json::from_str(
        r#"{
            "inputs":[{
                "intent":"action.devices.QUERY",
                "payload":{
                    "devices":[{
                        "id":"b016f12a-df14-448f-8a5e-5ccfcc43af23"
                    }]
                }
            }],
            "requestId":"4125394601981923570"
        }"#,
    )
    .unwrap();
}

#[test]
fn parse_execute_request() {
    let req: Request = serde_json::from_str(
        r#"{
            "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
            "inputs": [{
              "intent": "action.devices.EXECUTE",
              "payload": {
                "commands": [{
                  "devices": [{
                    "id": "123",
                    "customData": {
                      "fooValue": 74,
                      "barValue": true,
                      "bazValue": "sheepdip"
                    }
                  }, {
                    "id": "456",
                    "customData": {
                      "fooValue": 36,
                      "barValue": false,
                      "bazValue": "moarsheep"
                    }
                  }],
                  "execution": [{
                    "command": "action.devices.commands.OnOff",
                    "params": {
                      "on": true
                    }
                  }]
                }]
              }
            }]
        }"#,
    )
    .unwrap();
}

#[test]
fn serialize_attributes() {
    assert_eq!(serde_json::to_string(&Attributes(vec![])).unwrap(), "{}");
}

#[test]
fn serialize_states() {
    assert_eq!(serde_json::to_string(&States(vec![])).unwrap(), "{}");
    assert_eq!(
        serde_json::to_value(&States(vec![
            State::OnOff { on: Some(true) },
            State::Brightness {
                brightness: Some(80)
            },
            State::TemperatureSetting {
                active_thermostat_mode: Some(ThermostatMode::Off),
                target_temp_reached_estimate_unix_timestamp_sec: None,
                thermostat_humidity_ambient: None,
                _details: TemperatureSettingDetail::SingleTemperaturSetting {
                    thermostat_mode: ThermostatMode::Off,
                    thermostat_temperature_ambient: 27.0,
                    thermostat_temperature_setpoint: 18.0
                }
            }
        ]))
        .unwrap(),
        serde_json::json!({
            "on": true,
            "brightness": 80,
            "activeThermostatMode": "off",
            "thermostatMode": "off",
            "thermostatTemperatureAmbient": 27.0,
            "thermostatTemperatureSetpoint": 18.0,
        })
    );
}
