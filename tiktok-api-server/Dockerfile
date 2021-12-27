FROM arm64v8/ubuntu:20.04
WORKDIR /app
RUN apt-get update && apt-get install -y python3 python3-pip tor
RUN echo "playwright==1.17.2" > /app/requirements.txt && pip3 install -r /app/requirements.txt && python3 -m playwright install && DEBIAN_FRONTEND=noninteractive python3 -m playwright install-deps
COPY requirements.txt requirements.txt
RUN pip3 install -r requirements.txt
COPY . /app
COPY torrc /etc/tor/
CMD /app/start.sh
