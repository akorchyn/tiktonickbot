from multiprocessing import AuthenticationError
from flask import Flask, request, abort
from TikTokApi import TikTokApi
import json
import os
from functools import wraps

import resource
resource.setrlimit(resource.RLIMIT_NOFILE, (512, 512))

app = Flask(__name__)
api = TikTokApi.get_instance()
API_KEY = os.environ.get('SECRET_KEY', 'blahblah')

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
    return json.dumps(api.get_user_object(username))

@app.route("/api/user_videos/", methods=['GET'])
@checkAppKey
def user_videos():
    username = request.args.get('username', type=str)
    count = request.args.get('count', default=5, type=int)
    if username is None:
        return ""
    return json.dumps(api.by_username(username, count))

@app.route("/api/user_likes/", methods=['GET'])
@checkAppKey
def user_likes():
    username = request.args.get('username', type=str)
    count = request.args.get('count', default=5, type=int)
    if username is None:
        return ""
    return json.dumps(api.user_liked_by_username(username, count))

if __name__ == "__main__":
    app.run(host='0.0.0.0', port=os.environ.get('PORT', 3000), threaded=False)