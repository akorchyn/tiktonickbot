from functools import wraps
from flask import abort

from common.proxy_handling import change_proxy, ProxyFailure

def api_required(apis):
    def decorator(func):
        @wraps(func)
        def wrapper(api_name, *args, **kwargs): 
            api = apis.get(api_name)
            if api is None:
                abort(404)
            return func(api, *args, **kwargs) 
        return wrapper
    return decorator

def abort_404_on_error(func):
    @wraps(func)
    def inner(*args, **kwargs):
        result = func(*args, **kwargs)
        if result is None:
            abort(404)
        return result
    return inner

def abort_503_on_proxy_failure(func):
    @wraps(func)
    def wrapper(*args, **kwargs):
        try:
            return func(*args, **kwargs)
        except ProxyFailure:
            abort(503)
    return wrapper

def change_proxy_on_return_null(func):
    @wraps(func)
    def wrapper(*args, **kwargs):
        result = func(*args, **kwargs)
        if result is None:
            change_proxy()
            raise ProxyFailure()
        return result
    return wrapper