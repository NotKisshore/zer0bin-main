mod config;
mod models;
mod routes;

use std::{io, path::PathBuf};

use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{
    web::{self, Data},
    App, HttpServer,
};
use config::Config;

use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::routes::{get_paste, get_stats, new_paste};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: PgPool,
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    let config = config::load(PathBuf::from("config.json"));
    let pool = PgPoolOptions::new()
        .max_connections(100)
        .connect(&config.databases.postgres_uri)
        .await
        .expect("Failed to connect to database");

    let address = format!(
        "{}:{}",
        config.server.backend_host, config.server.backend_port
    );

    let paste_governor = GovernorConfigBuilder::default()
        .per_second(config.ratelimits.seconds_in_between_pastes)
        .burst_size(config.ratelimits.allowed_pastes_before_ratelimit)
        .finish()
        .unwrap();

    let state = AppState { config, pool };

    println!("🚀 zer0bin is running on {address}");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_header()
            .allow_any_method()
            .allow_any_origin()
            .send_wildcard()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(Data::new(state.clone()))
            .service(get_stats)
            .service(
                web::scope("/p")
                    .wrap(Governor::new(&paste_governor))
                    .service(get_paste)
                    .service(new_paste)
                    // .service(get_raw_paste),
            )
    })
    .bind(address)?
    .run()
    .await
}
