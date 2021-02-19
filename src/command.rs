use bcrypt::{hash, DEFAULT_COST};
use rayon::ThreadPool;
use sqlx::{AnyConnection, AnyPool, Connection};
use std::sync::Arc;
use warp::Filter;

use crate::config::{AddConfig, ServerConfig};
use crate::error::{Error, Result};
use crate::server;

pub fn run(config: &ServerConfig) -> Result<()> {
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

pub fn add_user(config: &AddConfig) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let AddConfig::User {
        db_url,
        username,
        password,
    } = config;

    let password_hash =
        hash(password, DEFAULT_COST).map_err(|_| Error::Custom("failed to hash password"))?;

    async fn run(db_url: &str, username: &str, password_hash: &str) -> Result<()> {
        let mut connection = AnyConnection::connect(db_url).await.map_err(Error::from)?;

        sqlx::query("INSERT INTO \"user\" (username, pw_hash) VALUES ($1,$2)")
            .bind(username)
            .bind(password_hash)
            .execute(&mut connection)
            .await
            .map_err(Error::from)?;

        Ok(())
    }

    rt.block_on(run(db_url, username, &password_hash))
}
