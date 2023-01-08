use super::send_events::{Event, HelloEvent};
use crate::{
    auth::{validate_authentication_data, Auth},
    podcast::PodcastQuery,
    App,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::spawn;

pub struct PodcastWsSession {
    pub client_id: u32,
    pub socket: WebSocket,
}

impl PodcastWsSession {
    async fn send<T: Event>(&mut self, event: T) {
        let msg = Message::Text(event.serialize_event());
        println!("sending message");
        let _ = self.socket.send(msg).await;
    }
}

const PODCAST_INACTIVE: (StatusCode, &str) = (
    StatusCode::BAD_REQUEST,
    "Cannot connect to an inactive podcast",
);

pub async fn websocket(
    State(app): State<Arc<App>>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    Query(query): Query<PodcastQuery>,
) -> impl IntoResponse {
    let auth = match validate_authentication_data(app.clone(), &headers) {
        Ok(auth) => auth,
        Err(err) => return err.into_response(),
    };

    app.with_podcast(query.id, |podcast| {
        // Non-host wants to connect to a non-active podcast
        if podcast.data.host != auth.client_id && podcast.data.active_since.is_none() {
            return PODCAST_INACTIVE.into_response();
        }
        ws.on_upgrade(|socket| client_connect(socket, app, auth, podcast.data.id))
    });

    StatusCode::NOT_FOUND.into_response()
}

async fn client_connect(socket: WebSocket, app: Arc<App>, auth: Auth, podcast_id: u32) {
    // Order of operations isn't good...
    println!("connected");

    let mut session = PodcastWsSession {
        client_id: auth.client_id,
        socket,
    };

    let port = app
        .with_podcast(podcast_id, |podcast| podcast.audio_server.port)
        .unwrap();
    let hello_event = HelloEvent { port };
    session.send(hello_event).await;

    app.with_podcast(podcast_id, |p| p.ws_sessions.push(session));

    spawn(async { listen(podcast_id, auth.client_id, app) });
}

async fn listen(podcast_id: u32, client_id: u32, app: Arc<App>) {
    app.on_podcast(podcast_id, async |podcast| {
        let session = podcast.get_client_session(client_id).unwrap();
        let recv = session.socket.recv().await;
    });
}
