#!/bin/bash

set -euo pipefail
IFS=$'\n\t'

if [ -n "$CI_JOB_NAME" ]; then
    echo "[CI_JOB_NAME=$CI_JOB_NAME]"
fi

echo Release edition: $EDITION

if [[ "$CI_JOB_NAME" == *windows* ]]; then
    chmod +x ci/scripts/windows-dist.sh
    ci/scripts/windows-dist.sh
else
    chmod +x ci/scripts/linux-dist.sh
    ci/scripts/linux-dist.sh "${CI_JOB_NAME}"
fi
