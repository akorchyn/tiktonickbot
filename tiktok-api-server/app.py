from multiprocessing import AuthenticationError
from flask import Flask, request, abort
from TikTokApi import TikTokApi
import json
import os
from functools import wraps

import resource
resource.setrlimit(resource.RLIMIT_NOFILE, (430, 430))

app = Flask(__name__)
api = TikTokApi.get_instance(use_test_endgpoints=True, proxy="89.191.131.243:8080")
API_KEY = os.environ.get('SECRET_KEY', 'blahblah')
custom_cookie=os.environ.get('COOKIE', None)

def checkAppKey(view_function):
    @wraps(view_function)
    def decorated_function(*args, **kwargs):
        if request.args.get('key') and request.args.get('key') == API_KEY:
            return view_function(*args, **kwargs)
        else:
            abort(401)
    return decorated_function

@app.route("/api/user_info/", methods=['GET'])
@checkAppKey
def user_info():
    username = request.args.get('username', type=str)
    if username is None:
        return ""
    return json.dumps(api.get_user_object(username, custom_verifyFp=custom_cookie))

@app.route("/api/user_videos/", methods=['GET'])
@checkAppKey
def user_videos():
    username = request.args.get('username', type=str)
    count = request.args.get('count', default=5, type=int)
    if username is None:
        return ""
    return json.dumps(api.by_username(username, count, custom_verifyFp=custom_cookie))

@app.route("/api/user_likes/", methods=['GET'])
@checkAppKey
def user_likes():
    username = request.args.get('username', type=str)
    count = request.args.get('count', default=5, type=int)
    if username is None:
        return ""
    return json.dumps(api.user_liked_by_username(username, count, custom_verifyFp=custom_cookie))

@app.route("/api/status/", methods=['GET'])
@checkAppKey
def status():
    try:
        api.user_liked_by_username("wolf49xxx", 1, custom_verifyFp=custom_cookie)
        return ""
    except:
        abort(500)

@app.route("/api/new_cookie", methods=['POST'])
@checkAppKey
def new_cookie():
    global custom_cookie
    custom_cookie = request.form.get('cookie', default=custom_cookie)
    print("new cookie is {}".format(custom_cookie))
    return ""