use std::{borrow::BorrowMut, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use tokio::spawn;

use crate::{
    auth::{validate_authentication_data, Auth},
    podcast::PodcastQuery,
    App,
};

use super::send_events::{Event, HelloEvent};

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

    let podcast_data = app.with_podcast(query.id, |podcast| podcast.data.clone());

    match podcast_data {
        Some(podcast_data) => {
            if podcast_data.host != auth.client_id && podcast_data.active_since.is_none() {
                let err_msg = "Can't connect to an inactive podcast";
                return (StatusCode::BAD_REQUEST, err_msg).into_response();
            }
            ws.on_upgrade(move |socket| connected(socket, app, auth, podcast_data.id))
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn connected(socket: WebSocket, app: Arc<App>, auth: Auth, podcast_id: u32) {
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
