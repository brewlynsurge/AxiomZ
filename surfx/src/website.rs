use include_dir::{include_dir, Dir};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_files::Files;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::engine::SearchEngine;

const STATIC_DIR: Dir = include_dir!("surfx/static");

pub struct WebServer {
    seach_engine: Arc<Mutex<SearchEngine>>
}

impl WebServer {
    pub fn new(seach_engine: SearchEngine) -> Self {
        Self {
            seach_engine: Arc::new(Mutex::new(seach_engine))
        }
    }

    pub async fn start(&self) -> std::io::Result<()> {
        println!("Starting server at http://127.0.0.1:8080");

        let search_engine_clone = self.seach_engine.clone();
        HttpServer::new(move || {
            App::new()
                .route("/", web::get().to(Self::index_html))
                .route("/search", web::get().to({
                    let search_engine_clone = search_engine_clone.clone();
                    move |s| Self::search(s, search_engine_clone.clone())
                }))
                .service(Files::new("/static", "./static").show_files_listing())
        })
        .bind("127.0.0.1:8080")?
        .run()
        .await
    }

    pub async fn index_html() -> impl Responder {
        if let Some(file) = STATIC_DIR.get_file("index.html") {
            let html_content = file.contents_utf8().unwrap_or("Failed to read embedded HTML");
            HttpResponse::Ok()
                .content_type("text/html")
                .body(html_content)
        } else { HttpResponse::InternalServerError().body("index.html not found in embedded files") }
    }

    
    // Handler for search API
    async fn search(query: web::Query<SearchQuery>, seach_engine: Arc<Mutex<SearchEngine>>) -> impl Responder {
        let seach_engine = seach_engine.lock().await;

        let search_results = {
            let mut results = Vec::new();
            
            let search_data = seach_engine.search(&query.q).await;
            for result in search_data {
                results.push(SearchResult {
                    title: result.1,
                    link: result.0,
                    description: result.2.unwrap_or("No description".into()),
                })
            }
            results
        };
        
        HttpResponse::Ok()
            .content_type("application/json")
            .json(search_results)
    }
}



#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Serialize, Deserialize)]
struct SearchResult {
    title: String,
    link: String,
    description: String,
}