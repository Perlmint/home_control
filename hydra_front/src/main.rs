use axum::{
    extract::{Form, Query},
    headers::{self, HeaderName},
    http::{HeaderValue, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Router, TypedHeader,
};
use reqwest::Url;
use std::{collections::HashMap, sync::Arc};
mod auth;

struct HydraConfig {
    admin_url: Url,
    public_url: Url,
    client_id: String,
}

#[derive(serde::Deserialize)]
struct LoginQuery {
    login_challenge: String,
}

#[derive(serde::Serialize)]
#[serde(rename = "snake_case")]
enum PromptType {
    None,
    Login,
    Consent,
}

#[derive(serde::Serialize)]
struct ConnectQuery {
    prompt: Option<PromptType>,
    max_age: Option<u64>,
    id_token_hint: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct HydraLoginChallengeClient {
    client_id: String,
    client_name: String,
    redirect_uris: Vec<String>,
    grant_types: Vec<String>,
    response_types: Vec<String>,
    scope: String,
    audience: Vec<String>,
    owner: String,
    policy_uri: String,
    allowed_cors_origins: Vec<String>,
    tos_uri: String,
    client_uri: String,
    logo_uri: String,
    contacts: Vec<()>,
    client_secret_expires_at: u64,
    subject_type: String,
    token_endpoint_auth_method: String,
    userinfo_signed_response_alg: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    metadata: HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize)]
struct HydraLoginChallenge {
    challenge: String,
    requested_scope: Vec<String>,
    requested_access_token_audience: Vec<String>,
    skip: bool,
    subject: String,
    oidc_context: HashMap<String, String>,
    client: HydraLoginChallengeClient,
    request_url: String,
    session_id: String,
}

#[derive(Debug, serde::Serialize)]
struct HydraLoginAccept {
    subject: String,
    remember: bool,
    remember_for: u64,
    acr: Option<String>,
    context: Option<HashMap<String, String>>,
    force_subject_identifier: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct HydraLoginReject {
    error: String,
    error_description: String,
    error_hint: String,
    error_debug: String,
    status_code: u16,
}

#[derive(Debug, serde::Deserialize)]
struct HydraLoginResult {
    redirect_to: String,
}

async fn login_get(
    Extension(config): Extension<Arc<HydraConfig>>,
    Query(query): Query<LoginQuery>,
) -> axum::response::Html<String> {
    let url = {
        let mut url = config
            .admin_url
            .join("/oauth2/auth/requests/login")
            .unwrap();
        url.query_pairs_mut()
            .append_pair("login_challenge", &query.login_challenge);
        url
    };
    let challenge: HydraLoginChallenge = reqwest::get(url).await.unwrap().json().await.unwrap();

    println!("{:?}", challenge);

    Html(format!(include_str!("./login.html"), query.login_challenge))
}

#[derive(Debug, serde::Deserialize)]
struct LoginRequest {
    challenge: String,
    id: String,
    password: String,
}

async fn login_post(
    Extension(config): Extension<Arc<HydraConfig>>,
    Extension(authenticators): Extension<Arc<Vec<Box<dyn auth::Authenticator + Send + Sync>>>>,
    Form(request): Form<LoginRequest>,
) -> Redirect {
    let resp: HydraLoginResult = if authenticators
        .iter()
        .any(|auth| auth.auth(&request.id, &request.password).unwrap())
    {
        reqwest::Client::new()
            .put({
                let mut url = config
                    .admin_url
                    .join("/oauth2/auth/requests/login/accept")
                    .unwrap();
                url.query_pairs_mut()
                    .append_pair("login_challenge", &request.challenge);
                url
            })
            .json(&HydraLoginAccept {
                subject: request.id,
                remember: true,
                remember_for: 0,
                acr: None,
                context: Default::default(),
                force_subject_identifier: None,
            })
    } else {
        reqwest::Client::new()
            .put({
                let mut url = config
                    .admin_url
                    .join("/oauth2/auth/requests/login/reject")
                    .unwrap();
                url.query_pairs_mut()
                    .append_pair("login_challenge", &request.challenge);
                url
            })
            .json(&HydraLoginReject {
                error: "invalid".to_string(),
                error_debug: "".to_string(),
                error_description: "".to_string(),
                error_hint: "".to_string(),
                status_code: 403,
            })
    }
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();

    Redirect::to(&resp.redirect_to)
}

#[derive(serde::Deserialize)]
struct ConsentQuery {
    consent_challenge: String,
}

#[derive(serde::Serialize)]
struct ConsentAccept {
    grant_scope: Vec<String>,
    grant_access_token_audience: Vec<String>,
    remember: bool,
    remember_for: u64,
    session: HashMap<String, String>,
}

async fn consent(
    Extension(config): Extension<Arc<HydraConfig>>,
    Query(query): Query<ConsentQuery>,
) -> Redirect {
    let resp: HydraLoginResult = reqwest::Client::new()
        .put({
            let mut url = config
                .admin_url
                .join("/oauth2/auth/requests/consent/accept")
                .unwrap();
            url.query_pairs_mut()
                .append_pair("consent_challenge", &query.consent_challenge);
            url
        })
        .json(&ConsentAccept {
            grant_scope: vec!["openid".to_string()],
            grant_access_token_audience: Vec::new(),
            remember: true,
            remember_for: 0,
            session: HashMap::new(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    Redirect::to(&resp.redirect_to)
}

#[derive(serde::Serialize)]
struct IntrospectRequest<'a> {
    scope: Option<&'a str>,
    token: &'a str,
}

#[derive(serde::Deserialize)]
struct IntrospectResponse {
    active: bool,
    aud: Vec<String>,
    client_id: String,
    exp: u64,
}

async fn forward_auth_bearer(
    Extension(config): Extension<Arc<HydraConfig>>,
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
) -> Response {
    let token = authorization.token();
    let resp: IntrospectResponse = reqwest::Client::new()
        .post(config.admin_url.join("oauth2/introspect").unwrap())
        .form(&IntrospectRequest {
            scope: None,
            token: token,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    if resp.active {
        (StatusCode::OK).into_response()
    } else {
        let mut resp = (StatusCode::UNAUTHORIZED).into_response();
        resp.headers_mut().append(
            HeaderName::from_static("WWW-Authenticate"),
            HeaderValue::from_static(r#"Bearer error="invalid_token"""#),
        );
        resp
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let app = Router::new()
        .route("/forward-auth", get(forward_auth_bearer))
        .route("/login", get(login_get))
        .route("/login", post(login_post))
        .route("/consent", get(consent))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(Extension(Arc::new(HydraConfig {
            admin_url: Url::parse(&std::env::var("HYDRA_ADMIN_URL")?)?,
            public_url: Url::parse(&std::env::var("HYDRA_PUBLIC_URL")?)?,
            client_id: "".to_string(),
        })))
        .layer(Extension(Arc::new({
            let authenticators: Vec<Box<dyn auth::Authenticator + Send + Sync>> = vec![
                #[cfg(feature = "auth-pam")]
                {
                    Box::new(auth::pam::PamAuthenticator::new("login"))
                },
                #[cfg(feature = "auth-shadow")]
                {
                    Box::new(auth::shadow::ShadowAuthenticator::new())
                },
            ];
            authenticators
        })));

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
