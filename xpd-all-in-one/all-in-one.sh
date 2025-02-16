#!/bin/sh

set -e

echo "Setting bot commands"
/usr/bin/xpd-setcommands
echo "Starting bot"
/usr/bin/xpd-gateway
