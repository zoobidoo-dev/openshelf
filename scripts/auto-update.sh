#!/usr/bin/env bash
set -euo pipefail

APP_DIR="/home/zoobidoo/Learning-Projects/Products by SheerLuck/open-shelf/openshelf"
LOG="/var/log/openshelf-update.log"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG"; }

cd "$APP_DIR"

log "Fetching upstream..."
git fetch upstream

BEFORE=$(git rev-parse HEAD)
UPSTREAM=$(git rev-parse upstream/main)

if [ "$BEFORE" = "$UPSTREAM" ]; then
    log "Already up to date."
    exit 0
fi

log "New upstream commits found. Merging..."
git merge upstream/main --no-edit

git push origin main

log "Rebuilding and restarting container..."
docker compose up -d --build

log "Update complete. Running version: $(git rev-parse --short HEAD)"
