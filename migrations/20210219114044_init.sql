CREATE TABLE "user" (
    username TEXT UNIQUE NOT NULL,
    pw_hash TEXT NOT NULL,
    CONSTRAINT pk_name
        PRIMARY KEY (username)
);

CREATE TABLE redirect (
    path TEXT NOT NULL,
    "user" TEXT NOT NULL,
    created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    url TEXT NOT NULL,
    CONSTRAINT pk_path
        PRIMARY KEY (path),
    CONSTRAINT fk_user
        FOREIGN KEY ("user")
        REFERENCES "user" (username)
        ON DELETE CASCADE
);