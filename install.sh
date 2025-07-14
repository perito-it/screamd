#!/bin/bash

# Function to build the release binary
build() {
    echo "Building the release binary..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "Failed to build the project." >&2
        exit 1
    fi
}

# Function to install the service
install() {
    # This part of the script must be run as root
    if [ "$(id -u)" -ne 0 ]; then
        echo "Installation requires root privileges. Please run with sudo." >&2
        exit 1
    fi

    # Create configuration directory
    CONFIG_DIR="/etc/screamd"
    echo "Creating configuration directory at $CONFIG_DIR..."
    mkdir -p "$CONFIG_DIR"

    # Copy the binary and configuration
    echo "Installing screamd binary to /usr/local/bin/ and config to $CONFIG_DIR..."
    cp target/release/screamd /usr/local/bin/screamd
    cp config/config.toml "$CONFIG_DIR/config.toml"

    # Install the systemd service
    SERVICE_FILE="screamd.service"
    SYSTEMD_DIR="/etc/systemd/system"
    echo "Installing systemd service file to $SYSTEMD_DIR..."
    cp "$SERVICE_FILE" "$SYSTEMD_DIR/$SERVICE_FILE"

    # Reload systemd, enable and start the service
    echo "Reloading systemd, enabling and starting screamd service..."
    systemctl daemon-reload
    systemctl enable "$SERVICE_FILE"
    systemctl start "$SERVICE_FILE"

    echo "Installation complete. The screamd service is now running."
}

# Main script logic
case "$1" in
    build)
        build
        ;;
    install)
        install
        ;;
    *)
        echo "Usage: $0 {build|install}"
        echo "First, run '$0 build' as a regular user."
        echo "Then, run 'sudo $0 install' to install the service."
        exit 1
        ;;
esac
