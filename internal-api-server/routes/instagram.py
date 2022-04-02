from app import app, auth
from instagrapi import Client
from instagrapi.exceptions import UserNotFound
from flask import request, abort
import os
import json

instagram = Client()
instagram.login(os.environ["INSTAGRAM_LOGIN"],
                os.environ["INSTAGRAM_PASSWORD"])
# instagram.set_proxy("socks5://localhost:9050")


@app.route("/api/instagram_user_info/", methods=['GET'])
@auth.login_required
def instagram_user_info():
    username = request.args.get('username', type=str)
    if username is None:
        return ""
    try:
        return json.dumps(instagram.user_info_by_username(username).dict(), default=str)
    except UserNotFound:
        abort(404)


@app.route("/api/instagram_user_posts/", methods=['GET'])
@auth.login_required
def instagram_posts():
    user_id = request.args.get('user_id', type=int)
    n = request.args.get('count', type=int)
    if user_id is None:
        return ""
    try:
        return json.dumps([x.dict() for x in instagram.user_medias(user_id, n)], default=str)
    except KeyError:
        abort(404)


@app.route("/api/instagram_user_stories/", methods=['GET'])
@auth.login_required
def instagram_stories():
    user_id = request.args.get('user_id', type=int)
    n = request.args.get('count', type=int)
    if user_id is None:
        return ""
    try:
        return json.dumps([x.dict() for x in instagram.user_stories(user_id, n)], default=str)
    except KeyError:
        abort(404)


@app.route("/api/instagram_content_by_id/", methods=['GET'])
@auth.login_required
def instagram_video_by_id():
    content_str = request.args.get('content_str', type=str)
    if content_str is None:
        return ""
    return json.dumps(instagram.media_info(instagram.media_pk_from_code(content_str)).dict(), default=str)
