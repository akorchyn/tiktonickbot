from flask import Flask, abort
from flask_httpauth import HTTPTokenAuth
import os
from stem import Signal
from stem.control import Controller
import requests

from api.twitter import TwitterAPI
from api.social_network_api import SocialNetworkAPI
from common.decorators import api_required, return_404_on_error

app = Flask(__name__)
auth = HTTPTokenAuth(scheme='Bearer')

API_KEY = os.environ.get('SECRET_KEY', 'blahblah')

@auth.verify_token
def verify_token(token):
    if token == API_KEY:
        return "Ok"

@app.route(f"/api/<api_name>/user_info/<username>", methods=['GET'])
@auth.login_required
@api_required
@return_404_on_error
def user_info(api: SocialNetworkAPI, username:str):
    return api.user_info(username)

@app.route(f"/api/<api_name>/<type>/<username>/<int:count>", methods=['GET'])
@auth.login_required
@api_required
@return_404_on_error
def content(api: SocialNetworkAPI, type: str, username: str, count: int):
    if type not in api.content_types():
        abort(404)
    return api.content(username, type, count)

@app.route(f"/api/<api_name>/content_by_id/<content_id>", methods=['GET'])
@auth.login_required
@api_required
@return_404_on_error
def content_by_id(api: SocialNetworkAPI, content_id: str):
    return api.content_by_id(content_id)
        
@app.route(f"/api/<api_name>/status", methods=['GET'])
@auth.login_required
@api_required
def status(api: SocialNetworkAPI):
    if api.status():
        return 200
    else:
        return 503
    
@app.route("/api/change_proxy/", methods=['POST'])
@auth.login_required
def changeProxy():
    with Controller.from_port(address="127.0.0.1", port=9051) as c:
        c.authenticate()
        c.signal(Signal.NEWNYM)
    return requests.get('https://api.ipify.org', proxies={"http": "socks5://localhost:9050",
                                                          "https": "socks5://localhost:9050"}).text
