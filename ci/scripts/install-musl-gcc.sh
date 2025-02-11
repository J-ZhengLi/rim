#!/bin/bash

function hide_output {
  { set +x; } 2>/dev/null
  on_err="
echo ERROR: An error was encountered with the build.
cat /tmp/build.log
exit 1
"
  trap "$on_err" ERR
  bash -c "while true; do sleep 30; echo \$(date) - building ...; done" &
  PING_LOOP_PID=$!
  "$@" &> /tmp/build.log
  trap - ERR
  kill $PING_LOOP_PID
  set -x
}

MUSL_DIR="/usr/local/musl"

mkdir -p $MUSL_DIR

# git clone https://github.com/chase535/aarch64-linux-musl-gcc.git $MUSL_DIR

git clone https://github.com/richfelker/musl-cross-make.git $MUSL_DIR
cd $MUSL_DIR

export TARGET=aarch64-linux-musl
export OUTPUT=$MUSL_DIR

hide_output make
hide_output make install

cd -
