#!/usr/bin/env bash

set -e

export MSYS_NO_PATHCONV=1

script=`cd $(dirname $0) && pwd`/`basename $0`

image=""
dev=0

while [[ $# -gt 0 ]]
do
  case "$1" in
    --dev)
      dev=1
      ;;
    *)
      if [ -n "$image" ]
      then
        echo "expected single argument for the image name"
        exit 1
      fi
      image="$1"
      ;;
  esac
  shift
done

# MacOS reports "arm64" while Linux reports "aarch64". Commonize this.
machine="$(uname -m | sed 's/arm64/aarch64/')"

script_dir="`dirname $script`"
docker_dir="${script_dir}/docker"
ci_dir="`dirname $script_dir`"
root_dir="`dirname $ci_dir`"

if [ -f "$docker_dir/$image/Dockerfile" ]; then
    dockerfile="$docker_dir/$image/Dockerfile"
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
else
    echo Invalid image: $image
    exit 1
fi

args="$args --volume $root_dir:/checkout"
command=(/checkout/ci/run.sh)

docker \
  run \
  --workdir /checkout/obj \
  $args \
  --env CI_JOB_NAME="${CI_JOB_NAME-$IMAGE}" \
  --init \
  --rm \
  rim \
  "${command[@]}"
