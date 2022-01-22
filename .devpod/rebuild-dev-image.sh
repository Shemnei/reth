#!/usr/bin/env sh
podman-compose build

### This is equivalent:
# docker build -t build-env_dev - < Dockerfile

### Stop all containers with:
#docker stop $(docker ps -q)

### Remove unused images with:
# docker image prune --all

### Remove all dangling data, i.e. containers stopped, volumes excluding
### containers and images with no containers:
# docker system prune --all
