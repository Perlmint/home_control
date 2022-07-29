use std::{collections::HashMap, ops::Deref, sync::Arc};

use anyhow::Context;
use axum::{http::StatusCode, routing::post, Extension, Json, Router};

use device::HomeDevice;
use fallible_iterator::FallibleIterator;
use google_smart_home::{
    ExecuteRequest, ExecuteResponse, Intent, QueryRequest, QueryResponse, Response,
    ResponsePayload, ResponseWithPayload, StateOrError, StatusReport, SyncResponse,
};

mod device;

pub enum Error {
    ClientError(anyhow::Error),
    ServerError(anyhow::Error),
}

impl Error {
    pub fn client_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self::ClientError(anyhow::Error::new(e))
    }

    pub fn server_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self::ServerError(anyhow::Error::new(e))
    }
}

trait ErrorWrap {
    type O;
    fn client_error(self) -> Self::O;
    fn server_error(self) -> Self::O;
}

impl ErrorWrap for anyhow::Error {
    type O = Error;
    fn client_error(self) -> Error {
        Error::ClientError(self)
    }

    fn server_error(self) -> Error {
        Error::ServerError(self)
    }
}

impl ErrorWrap for reqwest::Error {
    type O = Error;
    fn client_error(self) -> Error {
        Error::ClientError(anyhow::Error::new(self))
    }

    fn server_error(self) -> Error {
        Error::ServerError(anyhow::Error::new(self))
    }
}

impl<T, E: ErrorWrap<O = Error>> ErrorWrap for Result<T, E> {
    type O = Result<T, Error>;
    fn client_error(self) -> Self::O {
        self.map_err(|e| e.client_error())
    }

    fn server_error(self) -> Self::O {
        self.map_err(|e| e.server_error())
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let (code, e) = match self {
            Error::ClientError(e) => (StatusCode::BAD_REQUEST, e),
            Error::ServerError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        };
        log::error!("Error thrown - {:?}", e);

        (code, "Error").into_response()
    }
}

async fn handle_sync(
    devices: Arc<HashMap<String, Arc<Box<dyn HomeDevice + Send + Sync>>>>,
) -> Result<SyncResponse, Error> {
    log::trace!("handle sync begin");
    let devices = devices.iter().map(|(id, device)| device.sync(id)).collect();
    log::trace!(
        "handle sync end - {}",
        &serde_json::to_string(&devices).unwrap()
    );
    Ok(SyncResponse {
        agent_user_id: "perlmint_home".to_string(),
        devices,
    })
}

async fn handle_query(
    devices: Arc<HashMap<String, Arc<Box<dyn HomeDevice + Send + Sync>>>>,
    query: QueryRequest,
) -> Result<QueryResponse, Error> {
    log::trace!("handle query begin");
    let devices = fallible_iterator::convert(
        futures::future::join_all(query.devices.into_iter().map(|device| {
            let home_device = devices.get(&device.id).cloned();
            let device_id = device.id.clone();
            tokio::spawn({
                Box::pin(async move {
                    let state = if let Some(home_device) = home_device {
                        StateOrError::State(home_device.query().await?)
                    } else {
                        log::warn!("device {} is not found", &device_id);
                        StateOrError::Error(google_smart_home::Error::DeviceNotFound)
                    };

                    Ok((device_id, state))
                })
            })
        }))
        .await
        .into_iter()
        .map(|o| match o {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(Error::server_error(e)),
        }),
    )
    .collect()?;
    log::trace!(
        "handle query end - {}",
        &serde_json::to_string(&devices).unwrap()
    );

    Ok(QueryResponse { devices })
}

async fn handle_execute(
    devices: Arc<HashMap<String, Arc<Box<dyn HomeDevice + Send + Sync>>>>,
    execute: ExecuteRequest,
) -> Result<ExecuteResponse, Error> {
    log::trace!("handle execute begin");
    let commands = fallible_iterator::convert(
        futures::future::join_all(execute.commands.into_iter().flat_map(|mut command| {
            let execution = Arc::new(command.execution.drain(0..).collect::<Vec<_>>());
            command.devices.into_iter().map({
                let execution = execution.clone();
                let devices = devices.clone();
                move |device| {
                    let home_device = devices.get(&device.id).cloned();
                    let device_id = device.id.clone();
                    tokio::spawn({
                        let execution = execution.clone();
                        Box::pin(async move {
                            let state = if let Some(home_device) = home_device {
                                StateOrError::State(home_device.execute(execution.deref()).await?)
                            } else {
                                log::warn!("device {} is not found", &device_id);
                                StateOrError::Error(google_smart_home::Error::DeviceNotFound)
                            };

                            Ok(StatusReport {
                                ids: vec![device_id],
                                status: state,
                            })
                        })
                    })
                }
            })
        }))
        .await
        .into_iter()
        .map(|o| match o {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(Error::server_error(e)),
        }),
    )
    .collect()?;
    log::trace!(
        "handle execute end - {}",
        &serde_json::to_string(&commands).unwrap()
    );

    Ok(ExecuteResponse { commands })
}

async fn handle_fulfillment(
    Extension(devices): Extension<Arc<HashMap<String, Arc<Box<dyn HomeDevice + Send + Sync>>>>>,
    Json(mut request): Json<google_smart_home::Request>,
) -> Result<Json<google_smart_home::Response>, Error> {
    log::trace!("{:?}", request);
    let payload: ResponsePayload = {
        let input = request.inputs.remove(0);

        match input {
            Intent::Sync => ResponsePayload::Sync(handle_sync(devices.clone()).await?),
            Intent::Disconnect => {
                return Ok(Json(Response::EmptyResponse));
            }
            Intent::Query(query) => {
                ResponsePayload::Query(handle_query(devices.clone(), query).await?)
            }
            Intent::Execute(execute) => {
                ResponsePayload::Execute(handle_execute(devices.clone(), execute).await?)
            }
        }
    };

    Ok(Json(Response::ResponseWithPayload(ResponseWithPayload {
        request_id: request.request_id,
        payload,
    })))
}

#[derive(serde::Deserialize)]
#[serde(transparent)]
pub struct HubConfig(pub HashMap<String, device::DeviceConfigs>);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let devices: HashMap<_, _> = {
        let path = std::env::var("HUB_CONFIG").expect("HUB_CONFIG env is mandatory");
        let raw_config = std::fs::read(path).context("Failed to read specified config file")?;
        let config: HubConfig =
            toml::from_slice(&raw_config).context("Failed to parse config file")?;

        fallible_iterator::convert(
            futures::future::join_all(config.0.into_iter().map(|(key, config)| async {
                config
                    .create_device()
                    .await
                    .map(|device| (key, Arc::new(device)))
            }))
            .await
            .into_iter(),
        )
        .collect()?
    };

    let app = Router::new()
        .route("/fulfillment", post(handle_fulfillment))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(Extension(Arc::new(devices)));

    let signal = {
        #[cfg(target_os = "linux")]
        {
            async {
                let mut signal =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .unwrap();
                signal.recv().await;
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            tokio::signal::ctrl_c()
        }
    };

    axum::Server::bind(&"0.0.0.0:8088".parse()?)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async move {
            let _ = signal.await;
        })
        .await?;

    Ok(())
}
