from flask import Flask, abort
from flask_httpauth import HTTPTokenAuth
import os

from api.social_network_api import SocialNetworkAPI
from api.twitter import TwitterAPI
from api.instagram import InstagramAPI
from api.tiktok import TikTokAPI
from common.decorators import abort_500_on_error, api_name_to_api, abort_404_on_error, abort_503_on_proxy_failure

app = Flask(__name__)
auth = HTTPTokenAuth(scheme='Bearer')

API_KEY = os.environ.get('SECRET_KEY', 'blahblah')
APIs = {}
if "TWITTER_OFF" not in os.environ:
    APIs["twitter"] = TwitterAPI()
if "INSTAGRAM_OFF" not in os.environ:
    APIs["instagram"] = InstagramAPI()
if "TIKTOK_OFF" not in os.environ:
    APIs["tiktok"] = TikTokAPI()


@auth.verify_token
def verify_token(token):
    if token == API_KEY:
        return "Ok"


@app.route(f"/api/<api_name>/user_info/<username>", methods=['GET'])
@auth.login_required
@api_name_to_api(APIs)
@abort_500_on_error
@abort_503_on_proxy_failure
@abort_404_on_error
def user_info(api: SocialNetworkAPI, username: str):
    return api.user_info(username)


@app.route(f"/api/<api_name>/<type>/<username>/<int:count>", methods=['GET'])
@auth.login_required
@api_name_to_api(APIs)
@abort_500_on_error
@abort_503_on_proxy_failure
@abort_404_on_error
def content(api: SocialNetworkAPI, type: str, username: str, count: int):
    if type not in api.content_types():
        abort(404)
    return api.content(username, type, count)


@app.route(f"/api/<api_name>/content_by_id/<content_id>", methods=['GET'])
@auth.login_required
@api_name_to_api(APIs)
@abort_500_on_error
@abort_503_on_proxy_failure
@abort_404_on_error
def content_by_id(api: SocialNetworkAPI, content_id: str):
    return api.content_by_id(content_id)


@app.route(f"/api/<api_name>/status", methods=['GET'])
@auth.login_required
@api_name_to_api(APIs)
def status(api: SocialNetworkAPI):
    if api.status():
        return 200
    else:
        return 503
