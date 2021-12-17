FROM ubuntu:latest
WORKDIR /app

ENV TZ=Europe/Kiev
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone
RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive apt-get install -y libglib2.0-0\
          libsoup2.4-1\
          libgtk-3-0\
          libcairo2\
          libgl1\
          libegl1\
          libnotify4\
          libgdk-pixbuf2.0-0\
          libopus0\
          libicu66\
          libpango-1.0-0\
          libharfbuzz0b\
          libxml2\
          libxslt1.1\
          libwoff1\
          libfontconfig1\
          libfreetype6\
          libharfbuzz-icu0\
          gstreamer1.0-plugins-base\
          libgstreamer1.0-0\
          libgstreamer-gl1.0-0\
          gstreamer1.0-plugins-bad\
          libjpeg-turbo8\
          libpng16-16\
          libopenjp2-7\
          libwebpdemux2\
          libwebp6\
          libatk1.0-0\
          libenchant1c2a\
          libsecret-1-0\
          libhyphen0\
          libx11-6\
          libxcomposite1\
          libxdamage1\
          libwayland-server0\
          libwayland-egl1\
          libwayland-client0\
          libxkbcommon0\
          libepoxy0\
          libevdev2\
          libatk-bridge2.0-0\
          libgles2\
          gstreamer1.0-libav\
          python3-pip

COPY requirements.txt requirements.txt
RUN pip3 install -r requirements.txt
RUN PLAYWRIGHT_BROWSERS_PATH=/app/.cache/ms-playwright python3 -m playwright install
COPY . .
CMD  python3 app.py