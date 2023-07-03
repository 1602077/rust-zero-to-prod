#!/bin/bash

set -eoa pipefail

RUNNING_CONTAINER=$(docker ps --filter 'name=redis' --format '{{.ID}}')

[[ "$RUNNING_CONTAINER" ]] && (echo "killing $RUNNING_CONTAINER" && docker kill $RUNNING_CONTAINER)

docker run \
	-p "6379:6379" \
	-d \
	--name "redis_$(date '+%s')" \
	redis:7

echo "redis is running" >&2
