# screamd

**⚠️ Warning: This service is currently in development and intended for testing purposes only. It is not yet fit for production use. ⚠️**

`screamd` is a small service that implements the [scream-test](https://www.microsoft.com/insidetrack/blog/microsoft-uses-a-scream-test-to-silence-its-unused-servers/). This test helps identify unused servers by systematically warning users and then taking progressively more drastic actions.

## Purpose

The primary purpose of `screamd` is to execute the scream test, which consists of the following phases:

1.  **Warning Phase:** For a configurable duration, users and administrators are notified with a message.
2.  **Reboot Phase:** After the warning period, the machine is rebooted once a day for a configurable number of days.
3.  **Shutdown Phase:** If no one intervenes during the warning or reboot phases, the system is shut down permanently.

## Notifications

`screamd` uses multiple methods to notify users of a pending decommissioning:

*   **Login Banner:** A message is displayed at the login screen (GDM) and for shell logins.
*   **GDM Login Screen:** The warning message is displayed on the GDM login screen, if GDM is in use.

The content of these messages can be customized in the configuration file.

## Configuration

The configuration for `screamd` is located in `/etc/screamd/config.toml`. The following fields can be configured:

*   `warn_message`: The message to be broadcast to all users during the warning phase.
*   `warn_duration_days`: The number of days the warning message will be shown.
*   `reboot_duration_days`: The number of days the system will be rebooted daily after the warning period.
*   `warn_interval_seconds`: The interval in seconds at which the warning message is displayed.
*   `reboot_time`: The time of day (in HH:MM format) when daily reboots are performed.

## Installation

To install the service, use the `install.sh` script.

1.  **Build the application (as a regular user):**
    ```bash
    ./install.sh build
    ```

2.  **Install the service (with sudo):**
    ```bash
    sudo ./install.sh install
    ```

The `install.sh` script performs the following actions:

*   Builds the release binary.
*   Creates a `screamd` user and group.
*   Creates the configuration directory `/etc/screamd` and the state directory `/var/lib/screamd`.
*   Copies the binary to `/usr/local/bin/screamd` and the configuration file to `/etc/screamd/config.toml`.
*   Sets appropriate permissions for the directories and files.
*   Installs a sudoers file to allow the `screamd` user to execute shutdown and reboot commands.
*   Installs and enables a systemd service to run `screamd` automatically.