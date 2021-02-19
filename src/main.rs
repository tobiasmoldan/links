use clap::{clap_app, ArgMatches};
use dotenv::dotenv;
use log::error;
use rayon::ThreadPool;
use shadow_rs::shadow;
use sqlx::{Any, AnyPool, Pool};
use std::{env, str::FromStr};
use std::{process::exit, sync::Arc};
use warp::Filter;

use error::{Error, Result};

shadow!(build);

mod error;
mod server;

struct ServerConfig {
    async_threads: usize,
    blocking_threads: usize,
    auth_threads: usize,
    port: u16,
    db_conn: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            async_threads: 2,
            blocking_threads: 128,
            auth_threads: 4,
            port: 5000,
            db_conn: "sqlite::memory:".to_string(),
        }
    }
}

fn start_server(config: &ServerConfig) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(config.async_threads)
        .max_blocking_threads(config.blocking_threads)
        .build()
        .unwrap();

    let th_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.auth_threads)
        .build()
        .unwrap();

    let th_pool = Arc::new(th_pool);

    async fn run(config: &ServerConfig, th_pool: Arc<ThreadPool>) -> Result<()> {
        let db_pool = AnyPool::connect(&config.db_conn)
            .await
            .map_err(Error::from)?;

        sqlx::migrate!().run(&db_pool).await.map_err(Error::from)?;

        let filter = server::filter(db_pool.clone(), th_pool);
        let log = warp::log("links::api");
        let filter = filter.with(log);

        let (_, server) =
            warp::serve(filter).bind_with_graceful_shutdown(([0, 0, 0, 0], config.port), async {
                tokio::signal::ctrl_c().await.ok();
            });

        server.await;

        db_pool.close().await;

        Ok(())
    }

    rt.block_on(run(config, th_pool.clone()))
}

fn main() {
    dotenv().ok();

    if shadow_rs::is_debug() {
        pretty_env_logger::init();
    } else {
        env_logger::init();
    }

    let matches = clap_app!(links =>
        (version: build::PKG_VERSION)
        (author: "Tobias Moldan <contact@tobiasmoldan.com>")
        (about: "A path based redirection service")
        (@subcommand run =>
            (about: "run the server")
            (@arg PORT: -p --port +takes_value "http port, defaults to 5000")
            (@arg ASYNC_THREADS: --async +takes_value "number of asyncronous worker threads used handling io, defaults to 2")
            (@arg AUTH_THREADS: --auth +takes_value "number of threads used to validate passwords, defaults to 4")
            (@arg SYNC_THREADS: --sync +takes_value "number of max sync worker, defaults to 128")
            (@arg CONNECTION: -c --connection +takes_value "database connection string, defaults to 'sqlite::memory:'")
        )
        (@subcommand add =>
            (about: "add entity to database")
            (@arg USER: -u --user +takes_value +required "add new user")
            (@arg REDIRECT: --url +takes_value +required conflicts_with[USER] "add new redirect")
            (@arg PATH: --path +takes_value +required conflicts_with[USER] "set path")
            (@arg CONNECTION: -c --connection +takes_value "database connection string, defaults to 'sqlite::memory:'")
        )
    )
    .get_matches();

    let result = match matches.subcommand() {
        ("run", matches) => run_server(matches),
        ("add", matches) => run_add(matches),
        _ => Err(Error::InvalidCommand),
    };

    if let Err(e) = result {
        error!("{}", e);
        exit(1);
    }
}

fn run_server(matches: Option<&ArgMatches>) -> Result<()> {
    let mut config = ServerConfig::default();

    if let Some(conn) = env::var("LNKS_CONNECTION").ok() {
        config.db_conn = conn;
    }

    if let Some(conn) = matches
        .map(|m| m.value_of("CONNECTION"))
        .flatten()
        .map(|s| s.to_string())
    {
        config.db_conn = conn;
    }

    if let Some(port) = env::var("LNKS_PORT")
        .ok()
        .map(|s| u16::from_str(&s).ok())
        .flatten()
    {
        config.port = port;
    }

    if let Some(port) = matches
        .map(|m| m.value_of("PORT"))
        .flatten()
        .map(|s| u16::from_str(s).ok())
        .flatten()
    {
        config.port = port;
    }

    if let Some(async_threads) = env::var("LNKS_ASYNC_THREADS")
        .ok()
        .map(|s| usize::from_str(&s).ok())
        .flatten()
    {
        config.async_threads = async_threads;
    }

    if let Some(async_threads) = matches
        .map(|m| m.value_of("ASYNC_THREADS"))
        .flatten()
        .map(|s| usize::from_str(s).ok())
        .flatten()
    {
        config.async_threads = async_threads;
    }

    if let Some(auth_threads) = env::var("LNKS_AUTH_THREADS")
        .ok()
        .map(|s| usize::from_str(&s).ok())
        .flatten()
    {
        config.auth_threads = auth_threads;
    }

    if let Some(auth_threads) = matches
        .map(|m| m.value_of("AUTH_THREADS"))
        .flatten()
        .map(|s| usize::from_str(s).ok())
        .flatten()
    {
        config.auth_threads = auth_threads;
    }

    if let Some(sync_threads) = env::var("LNKS_SYNC_THREADS")
        .ok()
        .map(|s| usize::from_str(&s).ok())
        .flatten()
    {
        config.blocking_threads = sync_threads;
    }

    if let Some(sync_threads) = matches
        .map(|m| m.value_of("SYNC_THREADS"))
        .flatten()
        .map(|s| usize::from_str(s).ok())
        .flatten()
    {
        config.blocking_threads = sync_threads;
    }

    start_server(&config)
}

fn run_add(matches: Option<&ArgMatches>) -> Result<()> {
    todo!()
}
