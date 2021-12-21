#!/bin/bash

RELEASE=""
CLEAN=0
DEPLOY=0
RUN=0
DEPLOY_DIR=inky-gotchi-deploy
GDB_SERVER=0
GDB_PORT=1234

while [[ $# -gt 0 ]]; do
    key="$1"
    case $key in
        -r|--release)
        RELEASE="--release"
        shift # past argument
        ;;
        -d|--deploy)
        DEPLOY=1
        shift # past argument
        ;;
        -c|--clean)
        CLEAN=1
        shift # past argument
        ;;
        -u|--run)
        RUN=1
        shift # past argument
        ;;
        -g|--gdbserver)
        GDB_SERVER=1
        shift # past argument
        ;;
        *)
        echo "Invalid option: $1"
        exit 1
        ;;
    esac
done

if [ $CLEAN -eq 1 ]; then
    rm -rf target
fi

cross build --target=arm-unknown-linux-gnueabi $RELEASE

if [ $GDB_SERVER -eq 1 ]; then
    DEPLOY=1
fi

if [ $DEPLOY -eq 1 ]; then
    PID=$(ssh pizw pidof gdbserver)
    if [[ -n $PID ]]; then
        echo "Killing gdb server process: $PID"
        ssh pizw kill -9 $PID
    fi

    rm -rf $DEPLOY_DIR
    mkdir $DEPLOY_DIR
    cp target/arm-unknown-linux-gnueabi/debug/inky-gotchi $DEPLOY_DIR
    scp -r $DEPLOY_DIR pizw:~
fi

if [ $GDB_SERVER -eq 1 ]; then
    ssh pizw gdbserver localhost:$GDB_PORT $DEPLOY_DIR/inky-gotchi </dev/null &>/dev/null &
fi

if [ $RUN -eq 1 ]; then
    ssh pizw /home/pi/$DEPLOY_DIR/inky-gotchi
fi