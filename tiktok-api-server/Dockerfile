FROM mcr.microsoft.com/playwright:v1.17.0-focal
WORKDIR /app
COPY requirements.txt requirements.txt
RUN pip3 install -r requirements.txt
COPY . .
CMD gunicorn wsji:app --bind=0.0.0.0:$PORT --max-requests 10