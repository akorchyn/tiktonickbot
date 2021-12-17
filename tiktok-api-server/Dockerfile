FROM mcr.microsoft.com/playwright:v1.17.0-focal
WORKDIR /app
COPY requirements.txt requirements.txt
RUN pip3 install -r requirements.txt
COPY . .
CMD python3 -m flask run --no-debugger --no-reload --without-threads --port=$PORT --host='0.0.0.0'