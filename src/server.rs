use std::convert::Infallible;

use sqlx::{Database, Pool};
use warp::Filter;

pub fn filter<DB>(
    pool: Pool<DB>,
) -> impl Filter<Extract = (&'static str,), Error = Infallible> + Clone
where
    DB: Database,
{
    warp::any()
        .map(move || pool.clone())
        .map(|_p| "Hello World")
}
