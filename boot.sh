#!/bin/bash

source venv/bin/activate
exec gunicorn  -b :80 --access-logfile - --error-logfile - links:app