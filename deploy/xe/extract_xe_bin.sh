#!/bin/sh

set -e

START_DIR=${PWD}

mkdir -p /tmp/xapi-xe
curl -fLo /tmp/xapi-xe/xe.rpm https://koji.xcp-ng.org/kojifiles/packages/xapi/23.31.0/1.7.xcpng8.3/x86_64/xapi-xe-23.31.0-1.7.xcpng8.3.x86_64.rpm

# extract rpm
cd /tmp/xapi-xe || exit
7z x ./*.rpm || p7zip -d ./*.rpm
7z x ./*.cpio || p7zip -d ./*.cpio

cp /tmp/xapi-xe/opt/xensource/bin/xe ${START_DIR}/xe
chmod +x ${START_DIR}/xe

# clean up
rm -rf /tmp/xapi-xe
