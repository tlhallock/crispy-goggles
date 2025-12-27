use actix_web::{App, HttpResponse, HttpServer, Responder, delete, get, post, put, web};
use common::lobby::{Lobby, Player};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
struct AppState {
    players: Arc<RwLock<HashMap<u64, Player>>>,
    next_id: Arc<RwLock<u64>>,
}

#[get("/lobby")]
async fn get_lobby(state: web::Data<AppState>) -> impl Responder {
    let map = state.players.read().unwrap();
    let mut players: Vec<Player> = map.values().cloned().collect();
    players.sort_by_key(|p| p.id);
    HttpResponse::Ok().json(Lobby { players })
}

#[derive(Debug, Deserialize)]
struct CreatePlayerReq {
    name: String,
}

#[post("/players")]
async fn create_player(
    state: web::Data<AppState>,
    body: web::Json<CreatePlayerReq>,
) -> impl Responder {
    let mut id_lock = state.next_id.write().unwrap();
    let id = *id_lock;
    *id_lock += 1;

    let player = Player {
        id,
        name: body.name.clone(),
        ready: false,
    };

    state.players.write().unwrap().insert(id, player.clone());
    HttpResponse::Ok().json(player)
}

#[derive(Debug, Deserialize)]
struct ReadyReq {
    ready: bool,
}

#[put("/players/{id}/ready")]
async fn set_ready(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<ReadyReq>,
) -> impl Responder {
    let id = path.into_inner();
    let mut map = state.players.write().unwrap();
    match map.get_mut(&id) {
        Some(p) => {
            p.ready = body.ready;
            HttpResponse::Ok().json(p)
        }
        None => HttpResponse::NotFound().finish(),
    }
}

#[delete("/players/{id}")]
async fn delete_player(state: web::Data<AppState>, path: web::Path<u64>) -> impl Responder {
    let id = path.into_inner();
    let mut map = state.players.write().unwrap();
    if map.remove(&id).is_some() {
        HttpResponse::NoContent().finish()
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = AppState {
        players: Arc::new(RwLock::new(HashMap::new())),
        next_id: Arc::new(RwLock::new(1)),
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(get_lobby)
            .service(create_player)
            .service(set_ready)
            .service(delete_player)
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
