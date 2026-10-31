#!/usr/bin/env bash
# QloudiCable Sentinel Worker Loop
# Runs locally without any metered API or cloud budget.
# Continuously monitors USB-C port physical stability, detects micro-dropouts,
# and logs health events to a local JSON state ledger.

set -euo pipefail

LOG_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/qloudicable"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/sentinel-events.jsonl"
INTERVAL="${INTERVAL:-5}"

BIN_PATH="$(dirname "$0")/../../target/release/qloudicable"
if [ ! -f "$BIN_PATH" ]; then
    BIN_PATH="$(dirname "$0")/../../target/debug/qloudicable"
fi

if [ ! -f "$BIN_PATH" ]; then
    echo "Error: qloudicable binary not found. Build it with cargo build." >&2
    exit 1
fi

echo "[$(date -u +"%Y-%m-%dT%H:%M:%SZ")] QloudiCable Sentinel Worker started (poll interval: ${INTERVAL}s)..."
echo "[$(date -u +"%Y-%m-%dT%H:%M:%SZ")] Logging events locally to: $LOG_FILE"

last_state="unknown"

while true; do
    now_ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    epoch=$(date +%s)
    
    if report_json=$("$BIN_PATH" --json 2>/dev/null); then
        connected=$(echo "$report_json" | jq -r ".snapshot.ports | map(select(.connected == true)) | length" 2>/dev/null || echo 0)

        if [ "$last_state" = "unknown" ]; then
            last_state="$connected"
            echo "{\"timestamp\":\"$now_ts\",\"epoch\":$epoch,\"event\":\"worker_init\",\"connected_ports\":$connected}" >> "$LOG_FILE"
        elif [ "$connected" -gt "$last_state" ]; then
            echo "[$now_ts] [CONNECT] New port connected (count: $connected)"
            echo "{\"timestamp\":\"$now_ts\",\"epoch\":$epoch,\"event\":\"connect\",\"connected_ports\":$connected}" >> "$LOG_FILE"
            last_state="$connected"
        elif [ "$connected" -lt "$last_state" ]; then
            echo "[$now_ts] [DROPOUT] Port connection dropped (count: $connected)"
            echo "{\"timestamp\":\"$now_ts\",\"epoch\":$epoch,\"event\":\"dropout\",\"connected_ports\":$connected}" >> "$LOG_FILE"
            last_state="$connected"
        fi
    fi

    if [ -f "$LOG_FILE" ] && [ $(wc -l < "$LOG_FILE") -gt 1000 ]; then
        tail -n 800 "$LOG_FILE" > "$LOG_FILE.tmp" && mv "$LOG_FILE.tmp" "$LOG_FILE"
    fi

    sleep "$INTERVAL"
done
