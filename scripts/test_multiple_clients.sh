#!/bin/bash

# Test script to open multiple browser tabs and monitor server performance
# Usage: ./scripts/test_multiple_clients.sh [num_tabs]

NUM_TABS=${1:-10}
URL="http://localhost:8085"

echo "Opening $NUM_TABS browser tabs to test multiple clients..."
echo "Server URL: $URL"
echo ""

# Open browser tabs
for i in $(seq 1 $NUM_TABS); do
    echo "Opening tab $i..."
    xdg-open "$URL" 2>/dev/null &
    sleep 0.5  # Small delay to avoid overwhelming the browser
done

echo ""
echo "All tabs opened. Monitoring server performance..."
echo "Press Ctrl+C to stop monitoring."
echo ""

# Monitor server performance
while true; do
    # Get server PID
    SERVER_PID=$(pgrep -f "target/debug/examples/basic_server")
    
    if [ -z "$SERVER_PID" ]; then
        echo "Server not running!"
        exit 1
    fi
    
    # Get memory and CPU usage
    ps -p $SERVER_PID -o pid,rss,vsz,pcpu,etime --no-headers | \
        awk '{printf "PID: %s | RSS: %d MB | VSZ: %d MB | CPU: %s%% | Runtime: %s\n", 
              $1, $2/1024, $3/1024, $4, $5}'
    
    sleep 5
done

