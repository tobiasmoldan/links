use clap::clap_app;
use log::error;
use shadow_rs::shadow;
use sqlx::PgPool;
use std::str::FromStr;
use warp::Filter;

use error::{Error, Result};

shadow!(build);

mod error;
mod server;

#[inline]
async fn run() -> Result<()> {
    pretty_env_logger::init();

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
        .or_else(|| option_env!("LNKS_PORT"))
        .map(|p| u16::from_str(p).ok())
        .flatten()
        .unwrap_or(5000);

    let conn_str = matches
        .value_of("CONN")
        .or_else(|| option_env!("LNKS_CONN"))
        .ok_or(Error::NoConnectionString)?;

    let pool = PgPool::connect(conn_str)
        .await
        .map_err(|e| error::Error::DbError(e))?;

    let filter = server::filter(pool.clone());
    let log = warp::log("links::api");
    let filter = filter.with(log);

    let (_, server) =
        warp::serve(filter).bind_with_graceful_shutdown(([0, 0, 0, 0], port), async {
            tokio::signal::ctrl_c().await.unwrap();
        });

    server.await;

    pool.close().await;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = run().await {
        error!("{}", e);
        std::process::exit(1);
    }
}
