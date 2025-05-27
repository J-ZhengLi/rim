#!/usr/bin/env bash

set -e

export MSYS_NO_PATHCONV=1

script=`cd $(dirname $0) && pwd`/`basename $0`

# 检查环境变量 IMAGE 是否存在
if [ $# -eq 0 ]; then
  echo "Error: Please specify the IMAGE as the first argument."
  echo "Usage: $0 <IMAGE>"
  exit 1
fi

image="$1"
echo "Current image: $image"

# MacOS reports "arm64" while Linux reports "aarch64". Commonize this.
machine="$(uname -m | sed 's/arm64/aarch64/')"

script_dir="`dirname $script`"
docker_dir="${script_dir}/docker"
ci_dir="`dirname $script_dir`"
root_dir="`dirname $ci_dir`"

if [ -f "$docker_dir/$image/Dockerfile" ]; then
    dockerfile="$docker_dir/$image/Dockerfile"
    context="$script_dir"
    echo "::group::Building docker image for $image"

    # Print docker version
    docker --version

    # Docker build arguments.
    build_args=(
        "build"
        "--rm"
        "-t" "rim"
        "-f" "$dockerfile"
        "$context"
    )

    docker buildx "${build_args[@]}" --output=type=docker
else
    echo Invalid image: $image
    exit 1
fi

# command=(/checkout/ci/scripts/command.sh)
command=(/scripts/command.sh)

docker \
  run \
  --workdir /checkout/rim \
  --env $EDITION \
  --volume $root_dir:/checkout \
  --init \
  --rm \
  rim \
  "${command[@]}"

ls -al
