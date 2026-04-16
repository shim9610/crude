// src/browser/lib.rs
pub mod virtual_browser;
pub mod driver_updater;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug,Serialize, Deserialize)]
pub enum DeviceType {
    Mobile,
    Desktop,
}

#[derive(Debug, Clone,PartialEq)]
pub enum ResultType {
    Place,
    PowerLink,
    Organic,
}

/// Selenium WebDriver connection URL
pub const SELENIUM_URL: &str = "http://localhost:9515";

/// Whether to use headless mode (true: background execution, false: show browser window)
pub const HEADLESS: bool = false;

// Mobile User-Agent (iOS 18 / Chrome 146 for iOS — April 2026 baseline)
pub const MOBILE_USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/146.0.7680.40 Mobile/15E148 Safari/604.1";

// Desktop User-Agent (Chrome 146 — frozen UA scheme)
pub const DESKTOP_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";

pub const EXTERNAL_IP : &str = "0.0.0.0";

pub use virtual_browser::VirtualBrowser;