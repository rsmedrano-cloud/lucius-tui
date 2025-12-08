#!/bin/bash

# install.sh
# This script builds the Lucius TUI client and optionally the Lucius MCP Worker.
# It detects if the lucius-mcp-worker submodule is present and builds accordingly.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting Lucius installation script...${NC}"

# Check for Rust toolchain
if ! command -v cargo &> /dev/null
then
    echo -e "${YELLOW}Cargo (Rust build tool) not found. Please install Rustup: https://rustup.rs/${NC}"
    exit 1
fi

# Navigate to the lucius client directory (where Cargo.toml for lucius is)
# Assuming this script is run from the root of the lucius-tui repository
CLIENT_DIR="lucius"

if [ ! -d "$CLIENT_DIR" ]; then
    echo -e "${YELLOW}Error: '$CLIENT_DIR' directory not found. Please run this script from the root of the lucius-tui repository.${NC}"
    exit 1
fi

cd "$CLIENT_DIR"

# Detect if lucius-mcp-worker submodule is present
MCP_WORKER_DIR="../lucius-mcp-worker" # Relative to CLIENT_DIR

if [ -d "$MCP_WORKER_DIR" ] && [ -f "$MCP_WORKER_DIR/Cargo.toml" ]; then
    echo -e "${GREEN}Lucius MCP Worker submodule detected. Building both client and worker...${NC}"
    # Go to workspace root to build both
    cd ..
    cargo build --release --workspace
    echo -e "${GREEN}Client (lucius) and Worker (lucius-mcp-worker) built successfully!${NC}"
    echo -e "${YELLOW}Executables are in: ./lucius/target/release/lucius and ./lucius-mcp-worker/target/release/lucius-mcp-worker${NC}"
else
    echo -e "${YELLOW}Lucius MCP Worker submodule not found. Building client only...${NC}"
    cargo build --release --bin lucius
    echo -e "${GREEN}Client (lucius) built successfully!${NC}"
    echo -e "${YELLOW}Executable is in: ./lucius/target/release/lucius${NC}"
fi

# --- Systemd Service Stub for lucius-mcp-worker ---
echo -e "\n${YELLOW}----------------------------------------------------${NC}"
echo -e "${YELLOW}To install lucius-mcp-worker as a systemd service (requires root privileges):${NC}"
echo -e "${YELLOW}1. Create a file '/etc/systemd/system/lucius-mcp-worker.service' with content like this:${NC}"
echo -e "${YELLOW}----------------------------------------------------${NC}"
cat << EOF
[Unit]
Description=Lucius MCP Worker
After=network.target

[Service]
ExecStart=/path/to/your/lucius-mcp-worker/target/release/lucius-mcp-worker
Environment="REDIS_HOST=127.0.0.1" # !!! CHANGE THIS TO YOUR REDIS SERVER IP !!!
WorkingDirectory=/path/to/your/lucius-mcp-worker # Or a suitable data directory
User=lucius-worker # Create a dedicated user for this service
Group=lucius-worker # Create a dedicated group for this service
Restart=always
RestartSec=5s

[Install]
WantedBy=multi-user.target
EOF
echo -e "${YELLOW}----------------------------------------------------${NC}"
echo -e "${YELLOW}2. Reload systemd, enable and start the service:${NC}"
echo -e "${YELLOW}   sudo systemctl daemon-reload${NC}"
echo -e "${YELLOW}   sudo systemctl enable lucius-mcp-worker.service${NC}"
echo -e "${YELLOW}   sudo systemctl start lucius-mcp-worker.service${NC}"
echo -e "${YELLOW}----------------------------------------------------${NC}"

echo -e "\n${GREEN}Installation script finished. Don't forget to configure your PATH for the client if desired!${NC}"
