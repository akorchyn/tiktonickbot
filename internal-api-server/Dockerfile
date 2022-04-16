FROM ubuntu:20.04
WORKDIR /app
RUN apt-get update && apt-get install -y python3 python3-pip tor git
RUN echo "playwright==1.20.0" > /app/requirements.txt && pip3 install -r /app/requirements.txt && python3 -m playwright install webkit && DEBIAN_FRONTEND=noninteractive python3 -m playwright install-deps
COPY requirements.txt requirements.txt
RUN pip3 install -r requirements.txt
COPY torrc /etc/tor/
COPY . /app
CMD /app/start.sh
