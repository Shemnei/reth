#!/usr/bin/env sh
podman-compose run --rm dev

### This is equivalent to:
# cd ..
# docker run -it --env USER=dockeruser -v $(pwd):/home/dockeruser/project build-env_dev /bin/bash
