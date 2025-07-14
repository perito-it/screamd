# screamd

screamd is a small service that implements the [scream-test](https://www.microsoft.com/insidetrack/blog/microsoft-uses-a-scream-test-to-silence-its-unused-servers/).

The test constitutes of the following steps:
  * for a certain duration, users and administrators are informed with a message
  * after this duration, the machine is rebooted once a day for some time
  * if no one reacts, the system is shut down

## Installation

To install the service, follow these steps:

1.  **Build the application (as a regular user):**
    ```bash
    ./install.sh build
    ```

2.  **Install the service (with sudo):**
    ```bash
    sudo ./install.sh install
    ```
