use base64::decode;
use bcrypt::verify;
use sqlx::AnyPool;
use std::{convert::Infallible, iter::IntoIterator, str::FromStr, sync::Arc};
use tokio::sync::oneshot;
use warp::{http::uri::Uri, hyper::StatusCode, path::FullPath, reply, Filter, Rejection, Reply};

use crate::error::{ApiError, ApiResult};
use crate::model;

pub fn filter(
    db_pool: AnyPool,
    th_pool: Arc<rayon::ThreadPool>,
) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    warp::any()
        .and(new_filter(db_pool.clone(), th_pool.clone()))
        .or(get_own_filter(db_pool.clone(), th_pool))
        .or(get_filter(db_pool))
        .recover(handle_rejection)
}

fn get_own_filter(
    db_pool: AnyPool,
    th_pool: Arc<rayon::ThreadPool>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::any()
        .and(warp::path::end())
        .and(warp::get())
        .and(basic_auth_filter(db_pool.clone(), th_pool))
        .and_then(move |username: String| {
            let db_pool = db_pool.clone();
            async move { get_own(db_pool, username).await.map_err(Rejection::from) }
        })
}

async fn get_own(db_pool: AnyPool, username: String) -> ApiResult<impl Reply> {
    let entries = sqlx::query_as::<_, model::db::Entry>(
        "SELECT created,url, path FROM redirect WHERE \"user\" = $1",
    )
    .bind(username)
    .fetch_all(&db_pool)
    .await?
    .into_iter()
    .map(model::http::EntryResponse::from)
    .collect::<Vec<_>>();

    Ok(warp::reply::json(&entries))
}

fn get_filter(db_pool: AnyPool) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::any()
        .and(warp::get())
        .and(warp::path::full())
        .and_then(move |path: FullPath| {
            let db_pool = db_pool.clone();
            async move { get(db_pool, path).await.map_err(Rejection::from) }
        })
}

async fn get(db_pool: AnyPool, path: FullPath) -> ApiResult<impl Reply> {
    #[derive(sqlx::FromRow)]
    struct UrlContainer {
        url: String,
    }

    let urlc =
        sqlx::query_as::<_, UrlContainer>("SELECT url FROM redirect WHERE path = $1 LIMIT 1")
            .bind(path.as_str().trim_matches('/'))
            .fetch_optional(&db_pool)
            .await?
            .ok_or(ApiError::NotFound)?;

    match Uri::from_str(&urlc.url) {
        Err(_) => Err(ApiError::InvalidUri(urlc.url)),
        Ok(uri) => Ok(warp::redirect(uri)),
    }
}

fn new_filter(
    db_pool: AnyPool,
    th_pool: Arc<rayon::ThreadPool>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::any()
        .and(warp::path::end())
        .and(warp::post())
        .and(basic_auth_filter(db_pool.clone(), th_pool))
        .and(warp::body::json())
        .and_then(move |username, body: model::http::NewEntryRequest| {
            let db_pool = db_pool.clone();
            async move { new(db_pool, username, body).await.map_err(Rejection::from) }
        })
}

async fn new(
    db_pool: AnyPool,
    username: String,
    entry: model::http::NewEntryRequest,
) -> ApiResult<impl Reply> {
    let uri = match Uri::from_str(&entry.url) {
        Err(_) => return Err(ApiError::InvalidUri(entry.url)),
        Ok(uri) => uri,
    };

    let path = entry.path.trim().trim_matches('/');

    let rows = sqlx::query("INSERT INTO redirect (\"user\", url, path) SELECT $1,$2,$3 WHERE NOT EXISTS(SELECT * FROM redirect WHERE path = $2)")
        .bind(username)
        .bind(uri.to_string())
        .bind(path)
        .execute(&db_pool)
        .await
        .map_err(ApiError::from)?
        .rows_affected();

    if rows != 1 {
        Err(ApiError::PathAlreadyExists(entry.path))
    } else {
        Ok(reply::with_status(
            reply::with_header("", "Location", entry.path),
            StatusCode::CREATED,
        ))
    }
}

fn basic_auth_filter(
    db_pool: AnyPool,
    th_pool: Arc<rayon::ThreadPool>,
) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    warp::header::<String>("Authorization")
        .map(|s: String| {
            s.strip_prefix("Basic ")
                .map(|s| decode(s).ok())
                .flatten()
                .map(|vec| String::from_utf8(vec).ok())
                .flatten()
        })
        .and_then(move |header: Option<String>| {
            let db_pool = db_pool.clone();
            let th_pool = th_pool.clone();
            async move {
                match basic_auth(db_pool, th_pool, header).await {
                    Ok(username) => Ok(username),
                    Err(e) => Err(Rejection::from(e)),
                }
            }
        })
}

async fn basic_auth(
    pool: AnyPool,
    thread_pool: Arc<rayon::ThreadPool>,
    header: Option<String>,
) -> ApiResult<String> {
    let s = header.ok_or(ApiError::AuthHeaderDecode)?;

    let mut it = s.splitn(2, ':');

    #[derive(Debug, sqlx::FromRow)]
    struct User {
        pw_hash: String,
    };

    match (it.next(), it.next()) {
        (Some(username), Some(password)) => {
            match sqlx::query_as::<_, User>("SELECT u.pw_hash FROM \"user\" u WHERE username = $1")
                .bind(username.to_string())
                .fetch_optional(&pool)
                .await
            {
                Err(e) => Err(ApiError::from(e)),
                Ok(None) => Err(ApiError::Unauthorized),
                Ok(Some(user)) => {
                    let user: User = user;
                    let password = password.to_string();
                    let (tx, rx) = oneshot::channel();

                    thread_pool.spawn(move || check_password(password, user.pw_hash, tx));

                    match rx.await {
                        Ok(true) => Ok(username.to_string()),
                        Ok(false) => Err(ApiError::Unauthorized),
                        Err(_) => Err(ApiError::Custom("failed to recieve check pw result")),
                    }
                }
            }
        }
        _ => Err(ApiError::AuthHeaderDecode),
    }
}

fn check_password(password: String, pw_hash: String, tx: oneshot::Sender<bool>) {
    if !tx.is_closed() {
        tx.send(verify(password, &pw_hash).unwrap_or(false)).ok();
    }
}

async fn handle_rejection(rejection: Rejection) -> std::result::Result<impl Reply, Infallible> {
    if let Some(error) = rejection.find::<ApiError>() {
        if let ApiError::NotFound = error {
            Ok(warp::reply::with_status("", StatusCode::NOT_FOUND).into_response())
        } else {
            Ok(error.into_response())
        }
    } else {
        Ok(warp::reply::with_status("", StatusCode::NOT_FOUND).into_response())
    }
}

#[cfg(test)]
mod test {
    use super::basic_auth;
    use crate::error::ApiError;
    use std::sync::Arc;

    const TEST_USER: &str = "test";
    const TEST_PW: &str = "test123blub";
    const TEST_PW_HASH: &str = "$2y$12$3lYfycMuf0IGK11QdlEZ6ufujBbJ5IOh4JGw5h9RIcnc1YiQOl5s6";

    async fn init_pools() -> (sqlx::AnyPool, rayon::ThreadPool) {
        let db_pool = sqlx::AnyPool::connect("sqlite::memory:").await.unwrap();

        sqlx::migrate!().run(&db_pool).await.unwrap();

        sqlx::query(&format!(
            "INSERT INTO user (username, pw_hash) VALUES ('{}','{}')",
            TEST_USER, TEST_PW_HASH,
        ))
        .execute(&db_pool)
        .await
        .unwrap();

        let th_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();

        (db_pool, th_pool)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn basic_auth_valid() {
        let (db, th) = init_pools().await;

        let res = basic_auth(db, Arc::new(th), Some(format!("{}:{}", TEST_USER, TEST_PW))).await;

        assert!(res.is_ok());
        assert_eq!("test", &res.unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn basic_auth_invalid_user() {
        let (db, th) = init_pools().await;

        let res = basic_auth(
            db,
            Arc::new(th),
            Some(format!("{}:{}", "not existant", "blub321test")),
        )
        .await;

        if let Err(ApiError::Unauthorized) = res {
            assert!(true);
        } else {
            assert!(false);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn basic_auth_invalid_pw() {
        let (db, th) = init_pools().await;

        let mut old = String::from(TEST_PW);
        let mut new = String::with_capacity(old.capacity());

        while let Some(c) = old.pop() {
            new.push(c);
        }

        let res = basic_auth(db, Arc::new(th), Some(format!("{}:{}", TEST_USER, new))).await;

        if let Err(ApiError::Unauthorized) = res {
            assert!(true);
        } else {
            assert!(false);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn basic_auth_invalid_header() {
        let (db, th) = init_pools().await;

        let res = basic_auth(
            db,
            Arc::new(th),
            Some("something is not quite right here...".to_string()),
        )
        .await;

        if let Err(ApiError::AuthHeaderDecode) = res {
            assert!(true);
        } else {
            assert!(false);
        }
    }
}
