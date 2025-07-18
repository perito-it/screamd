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

    # Create a non-root user and group for the service
    if ! id -u screamd >/dev/null 2>&1; then
        echo "Creating screamd user and group..."
        useradd -r -s /bin/false screamd
    fi

    # Create configuration directory
    CONFIG_DIR="/etc/screamd"
    echo "Creating configuration directory at $CONFIG_DIR..."
    mkdir -p "$CONFIG_DIR"

    # Copy the binary and configuration
    echo "Installing screamd binary to /usr/local/bin/ and config to $CONFIG_DIR..."
    cp target/release/screamd /usr/local/bin/screamd
    cp config/config.toml "$CONFIG_DIR/config.toml"

    # Set permissions
    chown -R screamd:screamd "$CONFIG_DIR"
    chmod 700 "$CONFIG_DIR"
    chmod 600 "$CONFIG_DIR/config.toml"
    chown root:root /usr/local/bin/screamd
    chmod 755 /usr/local/bin/screamd

    # Install the sudoers file
    echo "Installing sudoers file..."
    cp config/screamd.sudoers /etc/sudoers.d/screamd
    chmod 440 /etc/sudoers.d/screamd

    # Install the systemd service
    SERVICE_FILE="screamd.service"
    SYSTEMD_DIR="/etc/systemd/system"
    echo "Installing systemd service file to $SYSTEMD_DIR..."
    cp "$SERVICE_FILE" "$SYSTEMD_DIR/$SERVICE_FILE"
    chmod 644 "$SYSTEMD_DIR/$SERVICE_FILE"

    # Reload systemd, enable and start the service
    echo "Reloading systemd, enabling and starting screamd service..."
    systemctl daemon-reload
    systemctl enable "$SERVICE_FILE"
    systemctl start "$SERVICE_FILE"

    echo "Installation complete. The screamd service is now running."
}

# Function to uninstall the service
uninstall() {
    # This part of the script must be run as root
    if [ "$(id -u)" -ne 0 ]; then
        echo "Uninstallation requires root privileges. Please run with sudo." >&2
        exit 1
    fi

    SERVICE_FILE="screamd.service"
    SYSTEMD_DIR="/etc/systemd/system"
    CONFIG_DIR="/etc/screamd"

    echo "Stopping and disabling screamd service..."
    systemctl stop "$SERVICE_FILE" >/dev/null 2>&1
    systemctl disable "$SERVICE_FILE" >/dev/null 2>&1

    echo "Removing systemd service file..."
    rm -f "$SYSTEMD_DIR/$SERVICE_FILE"
    systemctl daemon-reload

    echo "Removing sudoers file..."
    rm -f /etc/sudoers.d/screamd

    echo "Removing login banner..."
    rm -f /etc/profile.d/screamd-banner.sh

    echo "Removing screamd binary..."
    rm -f /usr/local/bin/screamd

    echo "Removing configuration directory..."
    rm -rf "$CONFIG_DIR"

    if id "screamd" &>/dev/null; then
        echo "Removing screamd user..."
        userdel screamd
    fi

    if getent group "screamd" &>/dev/null; then
        echo "Removing screamd group..."
        groupdel screamd
    fi

    echo "Uninstallation complete."
}

# Main script logic
case "$1" in
    build)
        build
        ;;
    install)
        install
        ;;
    uninstall)
        uninstall
        ;;
    *)
        echo "Usage: $0 {build|install|uninstall}"
        echo "First, run '$0 build' as a regular user."
        echo "Then, run 'sudo $0 install' to install the service."
        echo "To uninstall, run 'sudo $0 uninstall'."
        exit 1
        ;;
esac
