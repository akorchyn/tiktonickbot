from flask import Flask, request, abort
from flask_httpauth import HTTPTokenAuth
import os
from functools import wraps
from stem import Signal
from stem.control import Controller
import requests

app = Flask(__name__)

API_KEY = os.environ.get('SECRET_KEY', 'blahblah')
auth = HTTPTokenAuth(scheme='Bearer')

@auth.verify_token
def verify_token(token):
    if token == API_KEY:
        return "Ok"

@app.route("/api/change_proxy/", methods=['POST'])
@auth.login_required
def changeProxy():
    with Controller.from_port(address="127.0.0.1", port=9051) as c:
        c.authenticate()
        c.signal(Signal.NEWNYM)
    return requests.get('https://api.ipify.org', proxies={"http": "socks5://localhost:9050",
                                                          "https": "socks5://localhost:9050"}).text

import routes.tiktok
import routes.instagram