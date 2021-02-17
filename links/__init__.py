from flask import Flask, request, redirect, jsonify
from flask_bcrypt import Bcrypt, check_password_hash
from flask_httpauth import HTTPBasicAuth
from sqlite3 import IntegrityError

from links.db import get_db, init_db

app = Flask(__name__)
app.config.from_mapping(DATABASE='links.sqlite')

init_db(app)

auth = HTTPBasicAuth()
bcrypt = Bcrypt(app)

if __name__ == '__main__':
    app.run()


@app.route('/', methods=["GET"])
@auth.login_required
def get_redirects():
    db = get_db()
    reds = []
    for r in db.execute('SELECT path, url FROM redirect WHERE user_id = ?', (auth.current_user(),)):
        reds.append({'path': r[0], 'url': r[1]})
    return jsonify(reds)


@app.route('/', methods=['POST'])
@auth.login_required
def new_redirect():
    json = request.json
    url = json['url']
    path = json['path']
    if path is None or url is None:
        return 'missing field(s)', 400
    db = get_db()
    try:
        db.execute('INSERT INTO redirect (user_id, path, url) VALUES (?, ?, ?)',
                   (auth.current_user(), path, url, ))
        db.commit()
    except IntegrityError:
        return 'path already exists', 409
    return {'path': path, 'url': url}, 201, {'Location': '/'+path}


@app.route('/<path:path>', methods=['DELETE'])
@auth.login_required
def delete(path):
    db = get_db()
    if db.execute('DELETE FROM redirect WHERE path = ? AND user_id = ?', (path, auth.current_user(),)).rowcount != 1:
        return 'hmmm, something\'s not quite right...', 400
    db.commit()
    return '', 204


@app.route('/<path:path>', methods=['GET'])
def default(path):
    db = get_db()
    url = db.execute('SELECT url FROM redirect WHERE path = ?',
                     (path,)).fetchone()
    if url is None:
        return 'not found', 404
    return redirect(url[0], 307)


@auth.verify_password
def verify_password(username, password):
    c = get_db().execute('SELECT id, pw_hash FROM user WHERE username = ?', (username,))
    user = c.fetchone()
    if user is None or not check_password_hash(user[1], password):
        return None
    return user[0]
