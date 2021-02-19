use base64::decode;
use bcrypt::verify;
use sqlx::AnyPool;
use std::sync::Arc;
use tokio::sync::oneshot;
use warp::{Filter, Rejection};

use crate::error::{Error, Result};

pub fn filter(
    db_pool: AnyPool,
    th_pool: Arc<rayon::ThreadPool>,
) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    warp::any()
        .and(basic_auth_filter(db_pool, th_pool))
        .map(|username| format!("Hello {}!", username))
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
                .map(|s| {
                    println!("{}", s);
                    s
                })
        })
        .and_then(move |header: Option<String>| {
            let db_pool = db_pool.clone();
            let th_pool = th_pool.clone();
            async move {
                match basic_auth(db_pool, th_pool, header).await {
                    Ok(username) => Ok(username),
                    Err(e) => Err(e.into_rejection()),
                }
            }
        })
}

async fn basic_auth(
    pool: AnyPool,
    thread_pool: Arc<rayon::ThreadPool>,
    header: Option<String>,
) -> Result<String> {
    let s = header.ok_or(Error::HeaderDecode)?;

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
                Err(e) => Err(Error::DbError(e)),
                Ok(None) => Err(Error::Unauthorized),
                Ok(Some(user)) => {
                    let user: User = user;
                    let password = password.to_string();
                    let (tx, rx) = oneshot::channel();

                    thread_pool.spawn(move || check_password(password, user.pw_hash, tx));

                    match rx.await {
                        Ok(true) => Ok(username.to_string()),
                        Ok(false) => Err(Error::Unauthorized),
                        Err(_) => Err(Error::Unknown),
                    }
                }
            }
        }
        _ => Err(Error::HeaderDecode),
    }
}

fn check_password(password: String, pw_hash: String, tx: oneshot::Sender<bool>) {
    if !tx.is_closed() {
        tx.send(verify(password, &pw_hash).unwrap_or(false)).ok();
    }
}

#[cfg(test)]
mod test {
    use super::basic_auth;
    use crate::error::Error;
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

        if let Err(Error::Unauthorized) = res {
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

        if let Err(Error::Unauthorized) = res {
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

        if let Err(Error::HeaderDecode) = res {
            assert!(true);
        } else {
            assert!(false);
        }
    }
}
