use clap::{clap_app, ArgMatches};
use dotenv::dotenv;
use log::error;
use shadow_rs::shadow;
use std::process::exit;
use std::{env, str::FromStr};

use error::{Error, Result};

shadow!(build);

mod command;
mod config;
mod error;
mod server;

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
            (@arg CONNECTION: -c --connection +takes_value "database connection string")
            (@subcommand user =>
                (@arg NAME: +required)
                (@arg CONNECTION: -c --connection +takes_value "database connection string")
            )
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
    let mut config = config::ServerConfig::default();

    if let Some(c) = parse(matches, "CONNECTION") {
        config.db_conn = c;
    }

    if let Some(p) = parse(matches, "PORT") {
        config.port = p;
    }

    if let Some(t) = parse(matches, "ASYNC_THREADS") {
        config.async_threads = t;
    }

    if let Some(t) = parse(matches, "AUTH_THREADS") {
        config.auth_threads = t;
    }

    if let Some(t) = parse(matches, "SYNC_THREADS") {
        config.blocking_threads = t;
    }

    command::run(&config)
}

fn run_add(matches: Option<&ArgMatches>) -> Result<()> {
    let conn = parse(matches, "CONNECTION");

    if let Some(matches) = matches {
        match matches.subcommand() {
            ("user", matches) => run_add_user(conn, matches),
            _ => Err(Error::InvalidCommand),
        }
    } else {
        Err(Error::InvalidCommand)
    }
}

fn run_add_user(conn: Option<String>, matches: Option<&ArgMatches>) -> Result<()> {
    let conn = conn
        .or_else(|| parse(matches, "CONNECTION"))
        .ok_or(Error::NoConnectionString)?;

    let user = matches
        .map(|matches| matches.value_of("NAME"))
        .flatten()
        .map(|s| s.to_string())
        .ok_or(Error::InvalidCommand)?;

    println!("Enter password below:");
    let mut password = String::new();
    std::io::stdin()
        .read_line(&mut password)
        .map_err(|_| Error::Custom("failed to read password"))?;

    password = password.trim().to_string();

    if password.len() == 0 {
        return Err(Error::Custom("password too short"));
    }

    let config = config::AddConfig::User {
        db_url: conn,
        username: user,
        password,
    };

    command::add_user(&config)
}

fn parse<T>(matches: Option<&ArgMatches>, name: &str) -> Option<T>
where
    T: FromStr + Sized,
{
    let project_name = String::from(build::PROJECT_NAME);
    matches
        .map(|m| m.value_of(name))
        .flatten()
        .map(|s| T::from_str(s).ok())
        .flatten()
        .or_else(|| {
            env::var(&format!("{}_{}", project_name.to_ascii_uppercase(), name))
                .ok()
                .map(|s| T::from_str(&s).ok())
                .flatten()
        })
}
