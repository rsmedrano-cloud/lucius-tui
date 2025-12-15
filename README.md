# Lucius CLI (Rust)

A blazing fast, lightweight (sub-20MB RAM) TUI for local LLMs, written in Rust.

## Features



-   **Lightweight & Fast**: Built with Rust and `ratatui` for minimal resource usage and a responsive feel.

-   **Context Engine (`LUCIUS.md`)**: Automatically finds and uses a `LUCIUS.md` file in your project's directory hierarchy to provide persistent system-level context to the LLM.

-   **Distributed Homelab Management (MCP)**: Lucius acts as a central control plane. It enables the LLM to interact with remote worker agents (built using `mcp-worker`) deployed across your homelab.

    -   **Tool-use Loop**: The LLM generates command tasks, which Lucius pushes to a Redis queue. Remote `mcp-worker` agents pick up these tasks, execute them, and report results back to Redis.

-   **Clipboard Integration**: Easily copy the last response from Lucius to the system clipboard using `Ctrl+Y`.

-   **Model Management**: Switch between different local models, see connection status, and refresh the model list from within the UI.

-   **Persistent Configuration**: Remembers your Ollama URL and selected model between sessions.

-   **Modern UI**: A clean interface with rounded borders, padded text, and dynamic information display.

## Keybindings

| Key                 | Action                                       |
| ------------------- | -------------------------------------------- |
| `Ctrl+H`            | Toggle the help screen.                      |
| `Ctrl+S`            | Switch to the Settings screen.               |
| `Ctrl+Q`            | Quit the application.                        |
| `Ctrl+L`            | Clear the chat history.                      |
| `Ctrl+Y`            | Yank (copy) the last response to the clipboard. |
| `Ctrl+T`            | In Chat mode, list available MCP tools in the status bar. |
| `Esc`               | Exit modal screens (Help/Settings) or interrupt a streaming response. |
| `Enter`             | Send the message in the input box.           |
| `Tab`               | In Settings, switch focus between inputs.    |
| Mouse Scroll        | Scroll the conversation history.             |
| `Shift` + Mouse Drag | Select text using the terminal's native selection. |


## UI/UX Enhancements

*   **Dynamic Status Line**: A status line is displayed between the conversation and input box. It shows whether a `LUCIUS.md` file is in use, provides feedback for actions like copying to the clipboard, and indicates the status of the MCP server.
*   **Bottom Bar Information**: The bottom of the TUI dynamically displays the current working directory and the active LLM model.
*   **Improved Rendering**: The conversation and input boxes use rounded borders and internal padding for a cleaner look and to improve the native mouse selection experience.

## Changelog

All notable changes to this project are documented in `CHANGELOG.md`. This project adheres to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format and [Semantic Versioning](https://semver.org/spec/v2.0.0.html). For a detailed history of changes, including new features, bug fixes, and improvements, please refer to the `CHANGELOG.md` file in the project's root directory.

## Requirements

*   **Rust Toolchain**: This project requires a Rust toolchain version `1.70.0` or newer. It is recommended to use `rustup` to manage your Rust installations:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    rustup update
    ```
    You can check your Rust version with `rustc --version`.

## Running the Executables

Lucius is comprised of two main components:
1.  The `lucius` TUI application (your primary interface).
2.  The `lucius-mcp-worker` (a background agent that runs on remote machines, managed as a Git Submodule).

### Two-Command Install & Build

To get started with both the `lucius` client and the `lucius-mcp-worker` agent, use the following two commands from your terminal:

1.  **Clone the Main Repository Recursively:**
    ```bash
    git clone --recursive https://github.com/rsmedrano-cloud/lucius-tui.git
    cd lucius-tui
    ```
    *Note: The `--recursive` flag is crucial as it also clones the `lucius-mcp-worker` submodule. If you cloned without it, run `git submodule update --init --recursive` from the `lucius-tui` directory.*

2.  **Build Both Components (Workspace Build):**
    All subsequent commands should be run from the root of the `lucius-tui` directory.
    ```bash
    cargo build --release --workspace
    ```
    *This command builds both the `lucius` TUI client and the `lucius-mcp-worker` agent. The executables will be placed in the `target/release` directory at the root of the project.*

### Running the `lucius` TUI Application

After building, run the main `lucius` application from the project root:
```bash
./target/release/lucius
```
Alternatively, you can use `cargo run`:
```bash
cargo run --release --bin lucius
```

### Deploying `lucius-mcp-worker` on Docker Swarm

To set up a robust distributed Homelab Management Control Plane, you can deploy the `lucius-mcp-worker` agents on your Docker Swarm.

**Prerequisites:**
*   A running Docker Swarm.
*   Passwordless SSH access to your Swarm master node (e.g., `ssh user@100.73.97.118`).
*   Docker CLI installed on your local machine (where Lucius TUI runs) to build and push images.

**1. Deploy Redis on Docker Swarm:**
First, deploy a Redis service on your Swarm. It's recommended to constrain it to a manager node for stability.
```bash
docker service create \
  --name redis \
  --publish published=6379,target=6379 \
  --constraint 'node.role == manager' \
  redis:7.2-alpine
```

**2. Build and Push `lucius-mcp-worker` Docker Image:**
From your local machine, navigate to the `lucius/lucius-mcp-worker` directory within your cloned `lucius-tui` project and build the Docker image. You'll need to push this image to a Docker registry accessible by your Swarm nodes (e.g., Docker Hub, or a private registry).

```bash
# Navigate to the worker's Dockerfile directory
cd lucius/lucius-mcp-worker

# Build the Docker image
docker build -t <your_docker_registry>/lucius-mcp-worker:latest .

# Push the image to your registry
docker push <your_docker_registry>/lucius-mcp-worker:latest
```
Remember to replace `<your_docker_registry>` with your actual Docker Hub username or private registry address.

**3. Deploy `lucius-mcp-worker` as a Global Service on Docker Swarm:**
SSH into your Swarm master node (e.g., `ssh user@100.73.97.118`) and deploy the `mcp-worker` as a global service. This ensures one worker agent runs on every node in your Swarm, each with access to the Docker socket for executing commands.

```bash
docker service create \
  --name mcp-worker \
  --mount type=bind,source=/var/run/docker.sock,destination=/var/run/docker.sock \
  --env REDIS_HOST=redis \
  --mode global \
  <your_docker_registry>/lucius-mcp-worker:latest
```
*   `--mount type=bind,source=/var/run/docker.sock,destination=/var/run/docker.sock`: Allows the worker to interact with the Docker daemon on the host.
*   `--env REDIS_HOST=redis`: Configures the worker to connect to the Redis service named `redis` within the Swarm's overlay network.
*   `--mode global`: Deploys one instance of the worker on every node in your Swarm.

**4. Configuring Lucius TUI for Remote MCP:**
Once the Redis and `mcp-worker` services are running on your Swarm, configure your local Lucius TUI client to connect to the remote Redis instance.
In the Lucius TUI application, press `Ctrl+S` to navigate to the `Settings` screen.
Enter the **IP address or hostname of your Swarm master node** (e.g., `100.73.97.118`) into the "MCP Redis Host" field. This is the address Lucius will use to connect to the Redis service deployed on your Swarm.

This setup enables your Lucius TUI to send tasks to the central Redis, which are then picked up and executed by the distributed `mcp-worker` agents across your homelab.



### Making `lucius` Globally Accessible

To run `lucius` from any directory, you can copy the executable to a directory in your system's `PATH`.

```bash
# Ensure you are in the lucius-tui project root
sudo cp target/release/lucius /usr/local/bin/
```
Now, you should be able to simply type `lucius` in your terminal to start the application.

