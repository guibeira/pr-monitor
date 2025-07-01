# PR Monitor

PR Monitor is a desktop utility for developers that lives in your system tray/menu bar. It helps keep your GitHub Pull Requests up-to-date by automatically updating branches that are behind the base branch.

## Features

- **Automatic Branch Updates:** Automatically updates PR branches that are behind their target branch.
- **System Tray Integration:** Runs quietly in your system tray/menu bar for easy access.
- **PR Management:** Add PRs via URL and view lists of open and closed PRs.
- **Status Notifications:** Get notified of PR status changes, such as merge conflicts or blocks.
- **Configurable:** Adjust the refresh interval and toggle notifications to fit your workflow.

We currently only support MacOS, but we plan to add Windows support in the future.

## How to install
To install the application, you will need [Rust](https://www.rust-lang.org/tools/install) and [Node.js](https://nodejs.org/) installed.

Now install the tauri CLI tool, which is required to build and run the application.
```bash
cargo install tauri-cli --version "^2.0.0" --locked
```

Now with all the dependencies installed, you can clone the repository and run the application.

```bash
cargo tauri build
```

It will create a `target/release/bundle` directory containing the application package for your platform.
Just drag and drop the application into your Applications folder to install it.

## Development Guide

### 1. Installation for development

To run the application locally, you will need [Rust](https://www.rust-lang.org/tools/install) and [Node.js](https://nodejs.org/) installed.

1.  **Clone the repository:**
    ```bash
    git clone <YOUR_GIT_REPOSITORY_URL>
    cd pr-monitor
    ```

2.  **Install dependencies:**
    ```bash
    npm install
    ```

3.  **Run in development mode:**
    ```bash
    npm run tauri dev
    ```

### 2. First Run

On the first run, the application will ask for a GitHub Personal Access Token. This token is required to interact with the GitHub API on your behalf.

- You can generate a new token [here](https://github.com/settings/tokens/new).
- The token needs the `repo` scope to access and update your pull requests.

### 3. Adding a Pull Request

1.  Navigate to a pull request on GitHub.
2.  Copy the full URL from your browser's address bar.
3.  Paste the URL into the input field in the PR Monitor app.
4.  Click the `+` button to add it to the monitoring list.

### 4. Managing PRs

- **View Lists:** Use the "Open" and "Closed" tabs to see your monitored pull requests.
- **Delete a PR:** Click the `×` button next to a pull request to remove it from the list.

### 5. Configuration

Navigate to the **Settings** tab to configure the application:

- **Refresh Time:** Set how often (in minutes) the app should check your pull requests for updates.
- **Show Notifications:** Toggle desktop notifications for PR status changes on or off.

