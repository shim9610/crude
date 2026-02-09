# Crude

**Crude** is a tool for building and executing web crawling logic using a visual interface or as a Rust library.
It is designed for **supervised, fixed-logic crawling** where the target website's structure is known and relatively static.

> **Design Philosophy**: This tool prioritizes **manual monitoring** over automated resilience. It intentionally lacks complex error recovery (like auto-retries or proxy rotation) because it is built for tasks where a failure should immediately stop the process for human inspection.

## 📦 Installation

This tool consists of two parts:

1. **`crude-ui`**: The GUI editor.
2. **`browser_runner`**: The background Selenium/Chrome controller.

### Install via Cargo

To install both the library and the tools (UI & Runner):

```bash
cargo install --git "https://github.com/shim9610/crude" crude --features dev-tools

```

## 🚀 Usage

### 1. GUI Mode (Recommended for prototyping)

Run the UI editor. This will launch the editor and, upon execution, spawn the `browser_runner` process automatically.

```bash
crude-ui

```

### 2. Library Mode (Headless / CLI)

You can use `crude` as a library to execute saved JSON sequences programmatically without the UI.

**`Cargo.toml`**:

```toml
[dependencies]
crude = { git = "https://github.com/shim9610/crude" }
tokio = { version = "1", features = ["full"] }

```

**`src/main.rs`**:

```rust
use crude::browser::virtual_browser::{VirtualBrowser, DeviceType};
use crude::browser::driver_updater::ChromeDriver; // Import Updater
use std::sync::{Arc, atomic::AtomicBool};
use std::process::Command;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // 1. Auto-Update ChromeDriver
    // Checks local Chrome version and downloads matching driver if needed.
    println!("Checking ChromeDriver...");
    let driver_path = ChromeDriver::install().expect("Failed to update ChromeDriver");
    println!("ChromeDriver ready at: {:?}", driver_path);

    // 2. Start ChromeDriver Process
    // VirtualBrowser connects to localhost:9515, so we must spawn the driver first.
    let mut driver_process = Command::new(driver_path)
        .arg("--port=9515")
        .spawn()
        .expect("Failed to start driver process");

    // Give it time to start listening
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 3. Load the Sequence JSON
    let json_content = std::fs::read_to_string("my_sequence.json").unwrap();
    let sequence: crude::collector::workflow::Sequence = serde_json::from_str(&json_content).unwrap();

    // 4. Launch Browser Client
    // DeviceType::Desktop ensures a standard user-agent.
    let browser = VirtualBrowser::new(DeviceType::Desktop, Some(false)) // false = Show GUI
        .await
        .expect("Failed to connect to ChromeDriver");

    // 5. Run
    println!("Running sequence: {}", sequence.sequence_name);
    let shutdown = Arc::new(AtomicBool::new(false));
    
    match sequence.run(&browser.driver, shutdown, None).await {
        Ok(results) => println!("Collected {} items", results.len()),
        Err(e) => eprintln!("Execution failed: {:?}", e),
    }

    // 6. Cleanup
    browser.close().await.ok();     // Close Browser Session
    driver_process.kill().ok();     // Kill Driver Process
}

```

---

## 🛠 Handler Reference

All logic is composed of **Handlers**. Below is the **exact list** of handlers implemented in `workflow.rs`.

### 1. Navigation & Tabs

| Handler | Parameters | Description |
| --- | --- | --- |
| **Navigate** | `url` | Navigates the current tab to the specified URL. |
| **NavigateHref** | `base`, `href` | Joins a base URL with a relative path (often from a binding) and navigates. |
| **NewTab** | `url` | Opens a new tab and navigates to the URL. Focus switches to the new tab. |
| **SwitchTab** | `index` | Switches focus to the tab at the specified index (0-based). |
| **SwitchToLastTab** | - | Switches focus to the last open tab. |
| **CloseTab** | - | Closes the current tab and switches focus to the last remaining tab. |
| **Refresh** | - | Refreshes the current page. |
| **Forward** | - | Navigates forward in browser history. |
| **Backward** | - | Navigates backward in browser history. |

### 2. Interaction (Action)

If an element is not found, these actions generally fail immediately.

| Handler | Parameters | Description | Notes |
| --- | --- | --- | --- |
| **Click** | `selector` | Clicks the element matching the CSS selector. | Must be visible. |
| **ClickByText** | `selector`, `text` | Finds elements by selector, then clicks the one containing the specified `text`. |  |
| **Type** | `selector`, `text` | Types text into the element (e.g., input field). | Appends to existing text. |
| **ClearAndType** | `selector`, `text` | Clears the input field first, then types the text. |  |
| **PressKey** | `key` | Simulates a specific key press. | Supported: `Enter`, `Tab`, `Escape`, `Space`, `Backspace`, `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`. |
| **ScrollDown** | `scroll` (int) | Scrolls the window down by N pixels. |  |
| **ScrollUp** | `scroll` (int) | Scrolls the window up by N pixels. |  |
| **ScrollAll** | - | **Auto-Script**: Attempts to detect all scrollable areas and the main body, and scrolls them to the bottom incrementally. | **Warning**: Can take a long time or loop on infinite-scroll pages. |
| **DismissPermission** | - | **Auto-Script**: Attempts to click common "Deny/Close" buttons for popup permissions (KR/EN). | Hardcoded selectors. |

### 3. Context & Frames

| Handler | Parameters | Description |
| --- | --- | --- |
| **SwitchToFrame** | `selector` | Switches the driver context to an `<iframe>` matching the selector. |
| **SwitchToDefaultContent** | - | Returns the driver context to the main page (exits iframe). |

### 4. Wait & Sync

| Handler | Parameters | Description |
| --- | --- | --- |
| **Wait** | `time_ms` | Unconditional sleep for N milliseconds. |
| **WaitFor** | `selector`, `time_ms` | Polls for the element's existence. **Fails** if not found within `time_ms`. |

### 5. Debug / Misc

| Handler | Parameters | Description |
| --- | --- | --- |
| **GetHTML** | `selector`, `time_ms` | Dumps `outerHTML`. Retries for `time_ms` if the specific selector isn't found in the dump. |

### 6. Extraction (Data)

Extracts data from the current DOM context. Output is stored in the result map and can be bound to variables.

| Handler | Parameters | Description | Output |
| --- | --- | --- | --- |
| **Text** | `selector`, `field_name` | Extracts `innerText` of the **first** matching element. | String |
| **MultipleText** | `selector`, `field_name` | Extracts `innerText` of **all** matching elements. | String (joined by comma `, `) |
| **Attribute** | `selector`, `field_name`, `attr` | Extracts the specified attribute (e.g., `href`) of the **first** match. | String |
| **Count** | `selector`, `field_name` | Counts the number of elements matching the selector. | String (parsed number) |
| **Exists** | `selector`, `field_name` | Checks if at least one element exists. | String ("true" / "false") |

> **Note**: There is NO `MultipleAttribute` handler. You must use `Container` to iterate elements and extract attributes individually if needed.

### 7. Flow Control

| Handler | Parameters | Description |
| --- | --- | --- |
| **Container** | `selector`, `steps`, `dedup` | **Iteration**. Finds all elements matching `selector`. For each element, it runs the inner `steps`. |
| **SubSequence** | `sequence` | Executes another defined sequence structure. |

---

## ⚠️ Known Limitations & Constraints

1. **Strict Error Handling**:
* The tool is "Fail-Fast". If a selector is wrong, a `WaitFor` times out, or an element is obstructed, the sequence aborts with an error.
* There is no conditional branching (If/Else) logic.


2. **Synchronous Execution**:
* All actions happen sequentially. `Container` loops process items one by one. This is by design to mimic human speed and simplify state.


3. **Browser Process**:
* The `browser_runner` is a separate process. If you force-quit the UI/Script, the Chrome window might remain open.


4. **Auto-Scroll Reliability**:
* `ScrollAll` is a heuristic script injected into the page. It may not work on complex virtual-scroller implementations (e.g., React Window) without specific tuning.