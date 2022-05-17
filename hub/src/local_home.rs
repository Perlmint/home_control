use std::collections::HashMap;

use self::state::State;

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

pub mod state {
    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OnOff {
        pub on: bool,
    }

    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Brightness {
        pub brightness: u8,
    }

    #[derive(Debug, Default, serde::Serialize)]
    pub struct State {
        #[serde(flatten)]
        pub on_off: Option<OnOff>,
        #[serde(flatten)]
        pub brightness: Option<Brightness>,
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse {
    pub devices: HashMap<String, state::State>,
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
    pub custom_data: HashMap<String, String>,
}

#[derive(Debug, serde::Serialize)]
pub enum DeviceType {
    #[serde(rename = "action.devices.types.OUTLET")]
    Outlet,
    #[serde(rename = "action.devices.types.LIGHT")]
    Light,
}

#[derive(Debug, serde::Serialize)]
pub enum DeviceTrait {
    #[serde(rename = "action.devices.traits.OnOff")]
    OnOff,
    #[serde(rename = "action.devices.traits.Brightness")]
    Brightness,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceWithDetail {
    #[serde(flatten)]
    pub basic: Device,
    pub r#type: DeviceType,
    pub traits: Vec<DeviceTrait>,
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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExecutionOnOff {
    pub on: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BrightnessAbsolute {
    pub brightness: u8,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum BrightnessRelative {
    #[serde(rename_all = "camelCase")]
    ByPercent { brightness_relative_percent: u8 },
    #[serde(rename_all = "camelCase")]
    ByWeight { brightness_relative_weight: i8 },
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum Execution {
    #[serde(rename = "action.devices.commands.OnOff")]
    OnOff(ExecutionOnOff),
    #[serde(rename = "action.devices.commands.BrightnessAbsolute")]
    BrightnessAbsolute(BrightnessAbsolute),
    #[serde(rename = "action.devices.commands.BrightnessRelative")]
    BrightnessRelative(BrightnessRelative),
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub devices: Vec<Device>,
    pub execution: Vec<Execution>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteRequest {
    pub commands: Vec<Command>,
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
    pub status: Status,
    pub states: State,
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
