#! /bin/bash

URL=$1
REQUIRED_HEIGHTS=$2

while true
do
    HEIGHTS_COLLECTED=$(curl -s $URL/validate/ipc/count)
    echo "Heights collected $HEIGHTS_COLLECTED/$REQUIRED_HEIGHTS"

    if [ "$HEIGHTS_COLLECTED" -ge "$REQUIRED_HEIGHTS" ]; then
        break
    fi

    sleep 60
done


curl -s $URL/validate/ipc?count=$REQUIRED_HEIGHTS | jq
