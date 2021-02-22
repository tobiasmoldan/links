# LINKS

A rather plain path based redirection service

So, originally I intendet to write this application in python. Mainly to familiarze myself with the [Flask](https://flask.palletsprojects.com/) framework, but after getting a somewhat workable poc I started to dislike python's (or maybe flask's) way of doing things. So long story short I rewrote it in rust to give some crates a try, well sqlx specifically.

## Usage

Links is in essence an HTTP service which allows you to define redirects based on paths (i.e. `/youtube -> https://www.youtube.com`). Leading and trailing slashes will be ignored so `//my-random-route` is interpreted the same way as `/my-random-route/`.
Everything but the user configuration, which is done via the cli, is done with HTTP requests. Users authenticate via basic auth.

### Create new redirect

```bash
curl \
    -X POST \
    --user 'username:password' \
    --header 'Content-Type: application/json' \
    --data '{ "path": "netflix", "url": "https://www.netflix.com/" }' \
    localhost:5000
```

As before leading and trailing `/` will be removed, so you could also put `/netflix` in there.

### Get your redirects back

```bash
curl \
    --user 'username:password' \
    localhost:5000
```

### Delete

Well I hope your SQL skills are up to date.  
Deletes are currently not implemented but planned.

## Configuration

Links will try to load a `.env` file from the current directory.

Options can be either specified via environment variables or parameters (env prefix: `LINKS_`)

```
PORT            http port, defaults to 5000
ASYNC_THREADS   number of asyncronous worker threads used handling io, defaults to 2
AUTH_THREADS    number of threads used to validate passwords, defaults to 4
SYNC_THREADS    number of max sync worker, defaults to 128
CONNECTION      connection string, defaults to 'sqlite::memory:'
```

Manually tested with sqlite and postgres.

There is currently no https implementation so should you decide to run links accessible to everyone use your favourite webserver/reverse proxy/load balancer for https offloading.

## Run your own

1. Set up your SQL database
1. Set environment vars / prepare `.env` file
1. Add a new user with `links add user [USER]`
1. run the server with `links run`

Table creating etc. should be done by itself but the database or file in case of sqlite must already exist.

## Notes

If you decide that you really want to see the python code, please head over to [branch v1](https://github.com/tobiasmoldan/links/tree/v1)...

## License

This project is licensed under the [MIT license](https://github.com/tobiasmoldan/links/blob/main/LICENSE).
