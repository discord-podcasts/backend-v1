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
        if podcast.data.host != auth.client_id && podcast.data.active_since.is_none() {
            return PODCAST_INACTIVE.into_response();
        }
        ws.on_upgrade(|socket| connected(socket, app, auth, podcast.data.id))
    });

    StatusCode::NOT_FOUND.into_response()
}

async fn connected(socket: WebSocket, app: Arc<App>, auth: Auth, podcast_id: u32) {
    println!("connected");

    let session = PodcastWsSession {
        client_id: auth.client_id,
        socket,
    };
    app.with_podcast(podcast_id, |p| p.ws_sessions.push(session));

    app.with_podcast(podcast_id, |podcast| {
        &podcast.get_client_session(auth.client_id).unwrap().socket
    });

    app.on_podcast(podcast_id, async |podcast| {
        let etest = podcast.data.clone();
        //   let session = &podcast.get_client_session(auth.client_id).unwrap();
        //  let port = podcast.audio_server.port;
        //  let hello_event = HelloEvent { port };
        //  session.send(hello_event).await;
    });

    /*
    spawn(async {
        socket.recv().await;
    });
    */
}
