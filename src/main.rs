use clap::clap_app;
use dotenv::dotenv;
use log::error;
use shadow_rs::shadow;
use sqlx::{Any, Pool};
use std::{env, str::FromStr};
use warp::Filter;

use error::{Error, Result};

shadow!(build);

mod error;
mod server;

#[inline]
async fn run() -> Result<()> {
    let matches = clap_app!(links =>
        (version: build::PKG_VERSION)
        (author: "Tobias Moldan <contact@tobiasmoldan.com>")
        (about: "Simple path based url redirection service")
        (@arg PORT: -p --port +takes_value "Sets the http port to listen to (default: 5000)")
        (@arg CONN: -c --connection +takes_value "The postgresql connection string")
    )
    .get_matches();

    let port = matches
        .value_of("PORT")
        .map(String::from)
        .or_else(|| env::var("LNKS_PORT").ok())
        .map(|p| u16::from_str(&p).ok())
        .flatten()
        .unwrap_or(5000);

    let conn_str = matches
        .value_of("CONN")
        .map(String::from)
        .or_else(|| env::var("LNKS_CONN").ok())
        .ok_or(Error::NoConnectionString)?;

    let pool = Pool::<Any>::connect(&conn_str).await.map_err(Error::from)?;

    sqlx::migrate!().run(&pool).await.map_err(Error::from)?;

    let filter = server::filter(pool.clone());
    let log = warp::log("links::api");
    let filter = filter.with(log);

    let (_, server) =
        warp::serve(filter).bind_with_graceful_shutdown(([0, 0, 0, 0], port), async {
            tokio::signal::ctrl_c().await.ok();
        });

    server.await;

    pool.close().await;

    Ok(())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    dotenv().ok();

    if shadow_rs::is_debug() {
        pretty_env_logger::init();
    } else {
        env_logger::init();
    }

    if let Err(e) = run().await {
        error!("{}", e);
        std::process::exit(1);
    }
}
