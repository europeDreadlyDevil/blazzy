use std::io;
use std::sync::Arc;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use actix_web::middleware::Logger;
use actix_web::web::{get};
use env_logger::Env;
use crate::async_cacher::{AsyncCacher, SharedAsyncCacher};

pub struct Server {
    server: actix_web::dev::Server
}

impl Server {
    pub async fn init(host: (String, u16), connection_type: ConnectionType) -> io::Result<Self> {
        env_logger::init_from_env(Env::default().default_filter_or("info"));
        let cacher = SharedAsyncCacher.clone();
        match connection_type {
            ConnectionType::Websocket => {
                let server = HttpServer::new(|| App::new().wrap(Logger::default()).route("/", get().to(crate::websocket::ws_index)))
                    .bind(host)?
                    .run();
                Ok(Self{ server })
            }
            ConnectionType::REST => {
                let server = HttpServer::new( move || {
                    App::new()
                        .wrap(Logger::default())
                        .app_data(web::Data::new(cacher.clone()))
                        .route("/", get().to(Self::get_cache))
                })
                    .bind(host)?
                    .run();

                Ok(Self{ server })
            }
            ConnectionType::Unknown => panic!("Unknown connection type")
        }

    }

    pub fn get_server(self) -> actix_web::dev::Server {
        self.server
    }

    async fn get_cache(data: web::Data<Arc<AsyncCacher>>) -> impl Responder {
        let mut vec = vec![];
        for data in data.get().await {
            vec.push(data)
        }

        HttpResponse::Accepted().json(vec)

    }
}


pub enum ConnectionType {
    Websocket,
    REST,
    Unknown
}