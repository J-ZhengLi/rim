#!/usr/bin/env bash

set -e

image=""

# Parse arguments
while [[ $# -gt 0 ]]
do
  case "$1" in
    --image)
      image="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

docker --version
pwd
ls -al

docker_dir="ci/docker"

if [ -f "$docker_dir/$image/Dockerfile" ]; then
    dockerfile="$docker_dir/$image/Dockerfile"
    # build docker image.
    docker buildx build --network host --rm -t rim-ci -f "$dockerfile" .
else
    echo "Invalid docker image: $image"
fi

# 运行 Docker 容器
echo "Running docker with EDITION=$EDITION"
docker run --workdir /checkout/obj \
  -e "EDITION=$EDITION" \
  -v "$PWD:/checkout/obj" \
  --init --rm rim-ci
