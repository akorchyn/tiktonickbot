import string
from flask import Flask, request, abort
from flask_httpauth import HTTPTokenAuth
from TikTokApi import TikTokApi
from TikTokApi.exceptions import TikTokNotFoundError
import json
import os
from functools import wraps
from stem import Signal
from stem.control import Controller
import requests

app = Flask(__name__)
api = TikTokApi.get_instance(use_test_endgpoints=True, proxy="socks5://localhost:9050")
API_KEY = os.environ.get('SECRET_KEY', 'blahblah')
auth = HTTPTokenAuth(scheme='Bearer')
custom_cookie=None

@auth.verify_token
def verify_token(token):
    if token == API_KEY:
        return "Ok"

@app.route("/api/user_info/", methods=['GET'])
@auth.login_required
def user_info():
    username = request.args.get('username', type=str)
    if username is None:
        return ""
    try:
        return json.dumps(api.get_user_object(username, custom_verifyFp=custom_cookie))
    except TikTokNotFoundError:
        abort(404)
    except KeyError:
        abort(404)


@app.route("/api/user_videos/", methods=['GET'])
@auth.login_required
def user_videos():
    username = request.args.get('username', type=str)
    count = request.args.get('count', default=5, type=int)
    if username is None:
        return ""
    return json.dumps(api.by_username(username, count, custom_verifyFp=custom_cookie))

@app.route("/api/user_likes/", methods=['GET'])
@auth.login_required
def user_likes():
    username = request.args.get('username', type=str)
    count = request.args.get('count', default=5, type=int)
    if username is None:
        return ""
    return json.dumps(api.user_liked_by_username(username, count, custom_verifyFp=custom_cookie))

@app.route("/api/video_by_id/", methods=['GET'])
@auth.login_required
def video_by_id():
    video_id = request.args.get('video_id', type=int)
    if video_id is None:
        return ""
    return json.dumps([api.get_tiktok_by_id(id=video_id, custom_verifyFp=custom_cookie).get('itemInfo').get('itemStruct')])

@app.route("/api/status/", methods=['GET'])
@auth.login_required
def status():
    try:
        api.user_liked_by_username("wolf49xxx", 1, custom_verifyFp=custom_cookie)
        return ""
    except:
        abort(500)

@app.route("/api/change_proxy/", methods=['POST'])
@auth.login_required
def changeProxy():
    with Controller.from_port(address="127.0.0.1", port=9051) as c:
        c.authenticate()
        c.signal(Signal.NEWNYM)
    return requests.get('https://api.ipify.org', proxies={"http": "socks5://localhost:9050",
                                                          "https": "socks5://localhost:9050"}).text

@app.route("/api/new_cookie", methods=['POST'])
@auth.login_required
def new_cookie():
    global custom_cookie
    custom_cookie = request.form.get('cookie', default=custom_cookie)
    print("new cookie is {}".format(custom_cookie))
    return ""
