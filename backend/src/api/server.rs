use axum::{
    Router,
    body::Body,
    extract::ws::Message,
    http::{
        HeaderValue, Method,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
    middleware::{self, Next},
};

use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use dashmap::{DashMap, Entry};
use deadpool_postgres::Pool;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, mpsc};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::api::metrics::{metrics_handler, track_metrics};
use crate::api::{ApiError, v1};
use crate::sfu::room::RoomSfu;
use crate::utils::{AccessClaims, verify_jwt};
use headers::{Authorization, authorization::Bearer};
use http::StatusCode;
use uuid::Uuid;

#[derive(Default)]
struct ApiV1 {}

impl ApiV1 {
    fn routes() -> Router<ServerState> {
        Router::new()
            .nest("/room", v1::room::routes())
            .nest("/user", v1::user::routes())
            .nest("/auth", v1::auth::routes())
            .nest("/ws", v1::ws::routes())
    }
}

#[derive(Clone)]
pub struct ServerState {
    /// room id to rooms
    pub rooms: Arc<DashMap<Uuid, Arc<RoomSfu>>>,
    /// conn id to rooms
    pub conn_to_rooms: Arc<DashMap<Uuid, Arc<RoomSfu>>>,
    pub ws_conn: Arc<DashMap<Uuid, mpsc::Sender<Message>>>,
    pub pool: Pool,
    pub hash_sem: Arc<Semaphore>,
    pub allowed_origins: Arc<Vec<String>>,
    pub waiting_room: Arc<DashMap<Uuid, Vec<mpsc::Sender<Message>>>>
}

impl ServerState {
    pub fn new(pool: Pool, allowed_origins: Vec<String>) -> Self {
        Self {
            rooms: Arc::default(),
            ws_conn: Arc::default(),
            pool,
            hash_sem: Arc::new(Semaphore::new(4)),
            conn_to_rooms: Arc::default(),
            allowed_origins: Arc::new(allowed_origins),
            waiting_room: Arc::default(),
        }
    }

    pub async fn broadcast_to_waiting_room(&self, room: Uuid, m: Message) {
        let senders = self.waiting_room.get(&room)
            .map(|m| m.clone())
            .unwrap_or_default();
        
        for s in senders {
            _ = s.send(m.clone()).await;
        }
    }
    
    pub fn drop_from_waiting_room(&self, room: Uuid, sender: mpsc::Sender<Message>) {
        if let Entry::Occupied(mut e) = self.waiting_room.entry(room) {
            e.get_mut().retain(|tx| !tx.same_channel(&sender));
            if e.get().is_empty() {
                e.remove();
            }
        }
    }
}

pub async fn run_server(state: ServerState, port: u16) -> std::io::Result<()> {
    let cors_origins: Vec<HeaderValue> = state
        .allowed_origins
        .iter()
        .map(|o| HeaderValue::from_str(o).expect("invalid origin"))
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION, ACCEPT])
        .max_age(Duration::from_secs(3600));

    let app = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .nest("/v1", ApiV1::routes())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(track_metrics))
        .layer(cors);

    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!("listen at port {} ...", port);
    axum::serve(listener, app).await
}

pub async fn bearer_middleware(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let Ok(claims) = verify_jwt::<AccessClaims>(auth.token()) else {
        return ApiError::Rejection(StatusCode::UNAUTHORIZED, "invalid bearer token".to_owned())
            .into_response();
    };

    req.extensions_mut().insert(claims);

    next.run(req).await
}

pub async fn optional_bearer_middleware(
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let claims: Option<AccessClaims> = match auth {
        None => None,
        Some(TypedHeader(auth)) => match verify_jwt::<AccessClaims>(auth.token()) {
            Ok(claims) => Some(claims),
            Err(_) => {
                return ApiError::Rejection(
                    StatusCode::UNAUTHORIZED,
                    "invalid bearer token".to_owned(),
                )
                    .into_response();
            }
        },
    };

    req.extensions_mut().insert(claims);

    next.run(req).await
}