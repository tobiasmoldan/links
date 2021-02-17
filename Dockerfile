FROM python:3.9-slim-buster

WORKDIR /app

RUN apt update && apt install -y sqlite3

COPY requirements.txt requirements.txt
RUN python -m venv venv
RUN venv/bin/pip install -r requirements.txt
RUN venv/bin/pip install gunicorn

COPY links links
COPY boot.sh boot.sh
RUN chmod +x boot.sh

ENV FLASK_APP=links

EXPOSE 80
ENTRYPOINT ["./boot.sh"]
