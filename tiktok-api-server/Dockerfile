FROM arm64v8/ubuntu:20.04
WORKDIR /app
COPY requirements.txt requirements.txt
RUN apt-get update
RUN apt-get install -y python3 python3-pip
RUN pip3 install -r requirements.txt
RUN python3 -m playwright install
RUN DEBIAN_FRONTEND=noninteractive python3 -m playwright install-deps
COPY . .
CMD gunicorn wsji:app --bind=0.0.0.0:$PORT 
