use actix_web::{web, App, HttpServer, HttpResponse, HttpRequest, Responder};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use bcrypt::{hash, verify};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::env;
use serde_json::json;

#[derive(Serialize, Deserialize, Clone)]
struct Item {
    id: usize,
    name: String,
    price: f64,
}

#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: usize,
    email: String,
    password: String,
    token: Option<String>,
}

#[derive(Serialize)]
struct Token {
    token: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

struct AppState {
    items: Arc<RwLock<Vec<Item>>>,
    users: Arc<RwLock<Vec<User>>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let items: Arc<RwLock<Vec<Item>>> = Arc::new(RwLock::new(load_items().await.unwrap_or_default()));
    let users: Arc<RwLock<Vec<User>>> = Arc::new(RwLock::new(vec![]));

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                items: items.clone(),
                users: users.clone(),
            }))
            .route("/items", web::post().to(create_item))
            .route("/items", web::get().to(get_items))
            .route("/items/{id}", web::get().to(get_item))
            .route("/updateitems", web::put().to(update_items))
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}

async fn load_items() -> Result<Vec<Item>, std::io::Error> {
    let mut file = match File::open("items.json").await {
        Ok(file) => file,
        Err(_) => return Ok(vec![]),
    };

    let mut data = String::new();
    file.read_to_string(&mut data).await?;
    let items: Vec<Item> = serde_json::from_str(&data)?;
    Ok(items)
}

async fn save_items(items: &Vec<Item>) -> std::io::Result<()> {
    let data = serde_json::to_string(items)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("items.json")
        .await?;
    file.write_all(data.as_bytes()).await?;
    Ok(())
}

async fn create_item(data: web::Data<AppState>, req: HttpRequest, item: web::Json<Item>) -> impl Responder {
    let token = match extract_token(&req) {
        Some(token) => token,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "missing authorization header".into() }),
    };

    let users = data.users.read().unwrap();
    if !users.iter().any(|u| u.token.as_ref() == Some(&token)) {
        return HttpResponse::Unauthorized().json(ErrorResponse { error: "Invalid token".into() });
    }

    let mut items = data.items.write().unwrap();

    let mut new_item = item.into_inner();
    new_item.id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    items.push(new_item.clone());

    if let Err(_) = save_items(&items).await {
        return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to save items".into() });
    }

    HttpResponse::Created().json(new_item)
}

async fn get_items(data: web::Data<AppState>) -> impl Responder {
    let items = data.items.read().unwrap();
    HttpResponse::Ok().json(&*items)
}

async fn get_item(data: web::Data<AppState>, path: web::Path<usize>) -> impl Responder {
    let id = path.into_inner();
    let items = data.items.read().unwrap();
    match items.iter().find(|i| i.id == id) {
        Some(item) => HttpResponse::Ok().json(item),
        None => HttpResponse::NotFound().json(ErrorResponse { error: format!("no item with id: {}", id) }),
    }
}

async fn update_items(data: web::Data<AppState>, req: HttpRequest, update: web::Json<UpdateRequest>) -> impl Responder {
    let token = match extract_token(&req) {
        Some(token) => token,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "missing authorization header".into() }),
    };

    let users = data.users.read().unwrap();
    if !users.iter().any(|u| u.token.as_ref() == Some(&token)) {
        return HttpResponse::Unauthorized().json(ErrorResponse { error: "Invalid token".into() });
    }

    let mut items = data.items.write().unwrap();
    let mut updated_ids = vec![];

    for id in &update.ids {
        if let Some(item) = items.iter_mut().find(|i| i.id == *id) {
            if let Some(name) = &update.item.name {
                item.name = name.clone();
            }
            if let Some(price) = update.item.price {
                item.price = price;
            }
            updated_ids.push(*id);
        }
    }

    if let Err(_) = save_items(&items).await {
        return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to save items".into() });
    }

    HttpResponse::Ok().json(updated_ids)
}

#[derive(Deserialize)]
struct UpdateRequest {
    ids: Vec<usize>,
    item: UpdateItem,
}

#[derive(Deserialize)]
struct UpdateItem {
    name: Option<String>,
    price: Option<f64>,
}

async fn register(data: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let mut users = data.users.write().unwrap();
    let mut new_user = user.into_inner();

    new_user.id = users.len() + 1;
    new_user.token = Some(format!("token{}", new_user.id));
    new_user.password = hash(&new_user.password, bcrypt::DEFAULT_COST).unwrap();

    users.push(new_user.clone());

    HttpResponse::Ok().json(json!({
        "id": new_user.id,
        "email": new_user.email,
    }))
}

async fn login(data: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let users = data.users.read().unwrap();
    for u in users.iter() {
        if u.email == user.email {
            if verify(&user.password, &u.password).unwrap() {
                return HttpResponse::Ok().json(Token { token: u.token.clone().unwrap() });
            } else {
                return HttpResponse::Unauthorized().json(ErrorResponse { error: "Invalid password".into() });
            }
        }
    }

    HttpResponse::NotFound().json(ErrorResponse { error: "User not found".into() })
}

fn extract_token(req: &HttpRequest) -> Option<String> {
    req.headers().get("Authorization")?.to_str().ok()?.strip_prefix("Bearer ").map(String::from)
}