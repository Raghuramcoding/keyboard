#!/bin/bash
echo "Starting Keyboard..."
cd "$(dirname "$0")"
[ -d node_modules ] || npm install
npm run build
npm start
