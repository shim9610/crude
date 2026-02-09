# crude

**No Coding. Just Drag & Drop. Rust-powered High Performance Crawler.**

crude is a visual web scraping workflow builder and execution engine built with **Rust**. It allows you to define complex web interaction and data extraction logic using a simple drag-and-drop interface, without writing a single line of code.

Powered by [Iced](https://github.com/iced-rs/iced) for the GUI and [Thirtyfour](https://github.com/stevepryde/thirtyfour) for browser automation, TaskCrawl offers high performance, type safety, and a seamless developer experience.

---

## ✨ Key Features

- **Visual Workflow Editor**: Construct scraping logic by dragging and dropping handlers.
- **Smart Element Location**: Supports standard CSS selectors for precise targeting.
- **Variable Binding**: Use extracted data (e.g., `{{product_link}}`) dynamically in subsequent steps.
- **High-Performance Engine**: The core runner is a standalone Rust binary optimized for speed and stability.
- **Auto-Managed Driver**: Automatically detects your Chrome version and installs the compatible `chromedriver`.

---

## 🚀 Getting Started

### Prerequisites

- **Rust**: [Install Rust]
- **Google Chrome**: The automation engine requires a Chrome browser installed.

### Installation

```bash
git clone https://github.com/shim9610/crude.git
cd crude

# Install the UI and Runner
cargo install --path . --bin work_flow_ui
cargo install --path . --bin browser_runner
```

### Running the App

```bash
# Start the visual editor
work_flow_ui
```

---

## 📖 Handler Reference

TaskCrawl works by assembling **Handlers** into a sequence. Here is a detailed breakdown of every available handler and how to use it.

### 🧭 Navigation & Browser Control

Handlers for controlling the browser's state and navigation.

| Handler | Description | Parameters |
|---------|-------------|------------|
| **Navigate** | Directs the browser to a specific URL. | `URL`: Target address (supports bindings). |
| **New Tab** | Opens a new tab and navigates to a URL. | `URL`: Address to open. |
| **Switch Tab** | Switches focus to a specific tab index. | `Index`: 0-based tab index. |
| **Close Tab** | Closes the current tab. | None |
| **Last Tab** | Switches to the most recently opened tab. | None |
| **Refresh** | Reloads the current page. | None |
| **Back / Forward** | Simulates browser Back/Forward buttons. | None |
| **Switch Frame** | Switches context to an `<iframe>`. Essential for interacting with embedded content. | `Selector`: CSS selector of the iframe. |
| **Default Frame** | Returns focus to the main page content (exits iframe). | None |

### 🖱️ Interaction (Actions)

Handlers that simulate user actions on the page.

| Handler | Description | Parameters |
|---------|-------------|------------|
| **Click** | Clicks a specific element. | `Selector`: CSS selector of the target. |
| **Click Text** | Finds an element containing specific text and clicks it. Useful when IDs/Classes are dynamic. | `Selector`: Context scope. `Text`: Text to match. |
| **Type** | Types text into an input field (e.g., search box). | `Selector`: Input field. `Text`: String to type. |
| **Clear & Type** | Clears the input field before typing. Recommended for search bars. | `Selector`: Input field. `Text`: String to type. |
| **Press Key** | Simulates special keys (Enter, Esc, Tab, Arrows). | `Key`: Select key from list. |
| **Dismiss Popup** | Automatically attempts to close common permission/cookie popups. | None |
| **Get HTML** | Gets the raw HTML content of an element. | `Selector`: Target element. `Timeout`: Max wait time (ms). |

### ⏳ Wait & Sync

Handlers to ensure the page is ready before proceeding.

| Handler | Description | Parameters |
|---------|-------------|------------|
| **Wait For** | Pauses execution until a specific element appears in the DOM. Prevents "Element Not Found" errors. | `Selector`: Element to wait for. `Timeout`: Max wait time (ms). |
| **Scroll All** | Smart scrolling behavior. Automatically scrolls down to trigger lazy loading until the bottom is reached. | None |

### 📊 Data Extraction

Handlers for scraping data. Extracted data is saved to the result map and **can be used as bindings** in future steps.

| Handler | Description | Output |
|---------|-------------|--------|
| **Extract Text** | Gets the visible text content of an element. | Text string. |
| **Extract Attr** | Gets a specific HTML attribute (e.g., `href`, `src`, `data-id`). | Attribute value. |
| **Count** | Counts how many elements match the selector. | Number (as string). |
| **Exists** | Checks if an element exists. | `"true"` / `"false"`. |
| **Multi Text** | Collects text from *all* matching elements into a comma-separated string. | `"Item 1, Item 2, ..."` |

### 📦 Logic & Structure

Advanced handlers for looping and modularizing logic.

#### Container (The Loop)

Iterates over a list of elements (e.g., search results, product lists).

- **Selector**: Defines the list items (e.g., `div.product-card`).
- **Behavior**: The sequence inside the Container is executed **once for each item** found.
- **Scope**: Inside a Container, all selectors are **relative** to the current item.

#### SubSequence (The Function)

Executes a separate, reusable sequence.

- **Usage**: Ideal for repetitive tasks (e.g., "Login Sequence") or organizing complex logic.
- **Data Mapping**: You can pass data from a Container (e.g., `product_url`) into the SubSequence as input.

---

## 🏗️ Architecture

TaskCrawl uses a decoupled architecture for stability:

```
┌─────────────────┐      TCP      ┌──────────────────┐
│  work_flow_ui   │ ◄───────────► │  browser_runner  │
│    (Frontend)   │               │    (Backend)     │
└─────────────────┘               └──────────────────┘
         │                                 │
         │                                 │
         ▼                                 ▼
┌─────────────────┐               ┌──────────────────┐
│    collector    │               │   chromedriver   │
│  (Core Library) │               │   (WebDriver)    │
└─────────────────┘               └──────────────────┘
```

1. **`work_flow_ui` (Frontend)**
   - Built with `iced` (Rust native GUI).
   - Handles user interaction, file I/O, and visual editing.
   - Communicates with the runner via TCP.

2. **`browser_runner` (Backend)**
   - A background TCP server.
   - Manages the Selenium WebDriver (`chromedriver`).
   - Executes the scraping logic independently to prevent UI freezing.

3. **`collector` (Core Library)**
   - Shared logic defining the `Workflow`, `Action`, and `Extraction` data structures.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.