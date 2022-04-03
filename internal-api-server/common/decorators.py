from functools import wraps
from flask import abort

from api.twitter import TwitterAPI
from api.instagram import InstagramAPI
from api.tiktok import TikTokAPI

APIs = {
    "twitter": TwitterAPI(),
    "instagram": InstagramAPI(),
    "tiktok": TikTokAPI()
}

def api_required(func):
    @wraps(func)
    def inner(api_name, *args, **kwargs): 
        api = APIs.get(api_name)
        if api is None:
            abort(404)
        return func(api, *args, **kwargs) 
    return inner

def return_404_on_error(func):
    @wraps(func)
    def inner(*args, **kwargs):
        result = func(*args, **kwargs)
        if result is None:
            abort(404)
        return result
    return inner