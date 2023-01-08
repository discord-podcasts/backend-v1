use std::{sync::Arc, thread, time::Duration};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    audio_server::AudioServer, auth::validate_authentication_data, ws::ws::PodcastWsSession, App,
};

pub struct Podcast {
    pub data: PodcastData,
    pub ws_sessions: Vec<PodcastWsSession>,
    pub audio_server: AudioServer,
}

impl Podcast {
    pub fn get_client_session(&mut self, client_id: u32) -> Option<&mut PodcastWsSession> {
        self.ws_sessions
            .iter_mut()
            .find(|session| session.client_id == client_id)
    }
}

#[derive(Clone, Serialize)]
pub struct PodcastData {
    pub id: u32,
    pub active_since: Option<u128>,
    pub host: u32,
}

#[derive(Deserialize)]
pub struct PodcastQuery {
    pub id: u32,
}

pub async fn get_podcast(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    query: Query<PodcastQuery>,
) -> impl IntoResponse {
    match validate_authentication_data(app.clone(), &headers) {
        Ok(auth) => auth,
        Err(error) => return error.into_response(),
    };

    let id = query.id;
    let podcast_data = app.with_podcast(id, |podcast| podcast.data.clone());
    match podcast_data {
        Some(data) => (StatusCode::OK, Json(data)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn create_podcast(State(app): State<Arc<App>>, headers: HeaderMap) -> impl IntoResponse {
    let auth = match validate_authentication_data(app.clone(), &headers) {
        Ok(auth) => auth,
        Err(error) => return error.into_response(),
    };

    let audio_server = match AudioServer::create(app.clone()) {
        Some(audio_server) => audio_server,
        None => {
            return (StatusCode::BAD_REQUEST, "All possible sockets are in use").into_response()
        }
    };

    let podcast = Podcast {
        data: PodcastData {
            id: app.generate_id(),
            active_since: None,
            host: auth.client_id,
        },
        ws_sessions: Vec::new(),
        audio_server,
    };

    let podcast_data = podcast.data.clone();
    await_host(podcast.data.id, app.clone());
    app.add_podcast(podcast);

    (StatusCode::OK, Json(podcast_data)).into_response()
}

/// Makes sure that the host connects to its podcast in time.
fn await_host(podcast_id: u32, app: Arc<App>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(60));
        let is_connected =
            app.with_podcast(podcast_id, |session| session.data.active_since.is_some());

        match is_connected {
            Some(connected) => {
                if !connected {
                    app.remove_podcast(podcast_id)
                }
            }
            None => return, // Podcast doesn't exist anymore
        };
    });
}
