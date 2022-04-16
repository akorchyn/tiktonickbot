from stem import Signal
from stem.control import Controller

PROXY_URL = "socks5://localhost:9050"


class ProxyFailure(Exception):
    pass


def change_proxy():
    with Controller.from_port(address="127.0.0.1", port=9051) as c:
        c.authenticate()
        c.signal(Signal.NEWNYM)
