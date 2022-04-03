from stem import Signal
from stem.control import Controller

class ProxyFailure(Exception):
    pass

def change_proxy():
    with Controller.from_port(address="127.0.0.1", port=9051) as c:
        c.authenticate()
        c.signal(Signal.NEWNYM)