import sqlite3
import os
import click

from flask import current_app, g, Flask
from flask.cli import with_appcontext
from flask_bcrypt import generate_password_hash


def init_db():
    db = get_db()
    with current_app.open_resource('schema.sql') as f:
        db.executescript(f.read().decode('utf-8'))


def get_db():
    if 'db' not in g:
        g.db = sqlite3.connect(
            current_app.config['DATABASE'],
            detect_types=sqlite3.PARSE_DECLTYPES
        )
        g.db.row_factory = sqlite3.Row
    return g.db


def close_db(e=None):
    db = g.pop('db', None)
    if db is not None:
        db.close()


def init_db(app: Flask):
    with app.app_context():
        if not os.path.isfile(current_app.config.get("DATABASE")):
            db = get_db()
            with current_app.open_resource('schema.sql') as f:
                db.executescript(f.read().decode('utf-8'))

    app.teardown_appcontext(close_db)
    app.cli.add_command(add_user)
    app.cli.add_command(del_user)


@click.command('add-user')
@click.argument('username')
@with_appcontext
def add_user(username):
    db = get_db()
    pw = input('password:')
    pw_hash = generate_password_hash(pw)
    db.execute('INSERT INTO user (username, pw_hash) VALUES (?,?)',
               (username, pw_hash,))
    db.commit()


@click.command('del-user')
@click.argument('username')
@with_appcontext
def del_user(username):
    db = get_db()
    db.execute('PRAGMA foreign_keys = 1')
    db.execute('DELETE FROM user WHERE username = ?', (username,))
    db.commit()
