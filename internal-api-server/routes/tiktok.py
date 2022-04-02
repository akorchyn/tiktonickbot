from app import app, auth
from TikTokApi import TikTokApi
from TikTokApi.exceptions import TikTokNotFoundError
from flask import request, abort
import json

tiktok = TikTokApi.get_instance(use_test_endgpoints=True, proxy="socks5://localhost:9050")

@app.route("/api/tiktok_user_info/", methods=['GET'])
@auth.login_required
def user_info():
    username = request.args.get('username', type=str)
    if username is None:
        return ""
    try:
        return json.dumps(tiktok.get_user_object(username))
    except TikTokNotFoundError:
        abort(404)
    except KeyError:
        abort(404)


@app.route("/api/tiktok_user_videos/", methods=['GET'])
@auth.login_required
def user_videos():
    username = request.args.get('username', type=str)
    count = request.args.get('count', default=5, type=int)
    if username is None:
        return ""
    return json.dumps(tiktok.by_username(username, count))

@app.route("/api/tiktok_user_likes/", methods=['GET'])
@auth.login_required
def user_likes():
    username = request.args.get('username', type=str)
    count = request.args.get('count', default=5, type=int)
    if username is None:
        return ""
    return json.dumps(tiktok.user_liked_by_username(username, count))

@app.route("/api/tiktok_video_by_id/", methods=['GET'])
@auth.login_required
def video_by_id():
    video_id = request.args.get('video_id', type=int)
    if video_id is None:
        return ""
    return json.dumps([tiktok.get_tiktok_by_id(id=video_id).get('itemInfo').get('itemStruct')])

@app.route("/api/tiktok_status/", methods=['GET'])
@auth.login_required
def status():
    try:
        tiktok.user_liked_by_username("wolf49xxx", 1)
        return ""
    except:
        abort(500)