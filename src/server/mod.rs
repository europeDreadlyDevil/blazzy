use std::io;
use std::sync::Arc;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use actix_web::middleware::Logger;
use actix_web::web::{get, scope};
use atomic_refcell::AtomicRefCell;
use env_logger::Env;
use crate::cacher::Cacher;

pub struct Server {
    server: actix_web::dev::Server
}

impl Server {
    pub async fn init(host: (String, u16), cacher: Arc<AtomicRefCell<Cacher>>) -> io::Result<Self> {
        env_logger::init_from_env(Env::default().default_filter_or("info"));
        let cacher = cacher.clone();
        let server = HttpServer::new( move || {
            App::new()
                .wrap(Logger::default())
                .app_data(web::Data::new(cacher.clone()))
                .service(
                    scope("/get_cache")
                        .route("", get().to(Self::get_cache))
                )
        })
            .bind(host)?
            .run();

        Ok(Self{ server })
    }

    pub fn get_server(self) -> actix_web::dev::Server {
        self.server
    }

    async fn get_cache(data: web::Data<Arc<AtomicRefCell<Cacher>>>) -> impl Responder {
        let mut vec = vec![];
        let data = data.borrow_mut();
        for data in data.get().await {
            vec.push(data)
        }

        HttpResponse::Accepted().json(vec)

    }
}