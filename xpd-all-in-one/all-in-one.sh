#!/bin/sh

set -e

echo "Setting bot commands"
xpd-setcommands
echo "Starting bot"
xpd-gateway
echo "Bot shut down"