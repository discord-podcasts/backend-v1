use axum::{
    routing::{get, post},
    Router,
};
use futures::Future;
use podcast::{create_podcast, get_podcast, Podcast};
use rand::Rng;
use ws::ws::websocket;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

mod audio_server;
mod auth;
mod podcast;
mod ws;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let router = Router::new()
        .route("/podcast", get(get_podcast))
        .route("/podcast", post(create_podcast))
        .route("/ws", get(websocket))
        .with_state(Arc::new(App::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 5050));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}

pub struct App {
    authentication: HashMap<u32, String>,
    podcasts: Mutex<HashMap<u32, Podcast>>,
}

impl App {
    fn new() -> Self {
        let mut auth = HashMap::new();
        auth.insert(123, "123".to_owned());
        auth.insert(345, "345".to_owned());
        Self {
            authentication: auth,
            podcasts: Mutex::new(HashMap::new()),
        }
    }

    fn generate_id(&self) -> u32 {
        let id: u32 = rand::thread_rng().gen();
        if self.podcasts.lock().unwrap().contains_key(&id) {
            return self.generate_id();
        }
        id
    }

    fn podcasts<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut HashMap<u32, Podcast>) -> R,
    {
        let mut sessions = self.podcasts.lock().unwrap();
        f(&mut sessions)
    }

    fn add_podcast(&self, podcast: Podcast) {
        self.podcasts(|s| s.insert(podcast.data.id, podcast));
    }

    fn remove_podcast(&self, id: u32) {
        self.podcasts(|sessions| sessions.remove(&id));
    }

    fn with_podcast<F, R>(&self, id: u32, f: F) -> Option<R>
    where
        F: FnOnce(&mut Podcast) -> R,
    {
        let mut sessions = self.podcasts.lock().unwrap();
        match sessions.get_mut(&id) {
            Some(session) => Some(f(session)),
            None => None,
        }
    }

    async fn on_podcast<F, Fut>(&self, id: u32, f: F)
    where
        F: FnOnce(&mut Podcast) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut sessions = self.podcasts.lock().unwrap();
        let session = sessions.get_mut(&id);
        if let Some(session) = session {
            f(session).await;
        }
    }
}
