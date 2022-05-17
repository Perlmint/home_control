use std::{collections::HashMap, ops::Deref, sync::Arc};

use axum::{http::StatusCode, routing::post, Extension, Json, Router};

use device::{PlantLedConfig, HomeDevice};
use fallible_iterator::FallibleIterator;
use local_home::{
    ExecuteRequest, ExecuteResponse, Intent, QueryRequest, QueryResponse, Response,
    ResponsePayload, ResponseWithPayload, StatusReport, SyncResponse,
};

use crate::local_home::Status;

mod device;
mod local_home;

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
    Ok(SyncResponse {
        agent_user_id: "".to_string(),
        devices: devices.iter().map(|(id, device)| device.sync(id)).collect(),
    })
}

async fn handle_query(
    devices: Arc<HashMap<String, Arc<Box<dyn HomeDevice + Send + Sync>>>>,
    query: QueryRequest,
) -> Result<QueryResponse, Error> {
    Ok(QueryResponse {
        devices: fallible_iterator::convert(
            futures::future::join_all(query.devices.into_iter().map(|device| {
                let home_device = devices.get(&device.id).unwrap().clone();
                tokio::spawn({
                    Box::pin(async move {
                        home_device
                            .query()
                            .await
                            .map(|state| (device.id.clone(), state))
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
        .collect()?,
    })
}

async fn handle_execute(
    devices: Arc<HashMap<String, Arc<Box<dyn HomeDevice + Send + Sync>>>>,
    execute: ExecuteRequest,
) -> Result<ExecuteResponse, Error> {
    Ok(ExecuteResponse {
        commands: fallible_iterator::convert(
            futures::future::join_all(execute.commands.into_iter().flat_map(|mut command| {
                let execution = Arc::new(command.execution.drain(0..).collect::<Vec<_>>());
                command.devices.into_iter().map({
                    let execution = execution.clone();
                    let devices = devices.clone();
                    move |device| {
                        let home_device = devices.get(&device.id).unwrap().clone();
                        tokio::spawn({
                            let execution = execution.clone();
                            Box::pin(async move {
                                home_device.execute(execution.deref()).await.map(|states| {
                                    StatusReport {
                                        ids: vec![device.id],
                                        status: Status::Success,
                                        states,
                                    }
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
        .collect()?,
    })
}

async fn handle_fulfillment(
    Extension(devices): Extension<Arc<HashMap<String, Arc<Box<dyn HomeDevice + Send + Sync>>>>>,
    Json(mut request): Json<local_home::Request>,
) -> Result<Json<local_home::Response>, Error> {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let app = Router::new()
        .route("/fulfillment", post(handle_fulfillment))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(Extension(Arc::new(HashMap::from([(
            "example".to_string(),
            Arc::new({
                let boxed: Box<dyn HomeDevice + Send + Sync> = Box::new(device::PlantLed::new(PlantLedConfig {
                    host: "192.168.1.1".to_string(),
                    internal_id: 0,
                }));
                boxed
            }),
        )]))));

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
