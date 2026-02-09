// src/bin/browser_runner.rs
//! Browser execution server - Communicates with UI via TCP
//! 
//! cargo run --features dev-tools --bin browser_runner [port]
//! Or run automatically as a subprocess from work_flow_ui

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command as ProcessCommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

use collector::workflow::{Sequence, ExecContext};
use browser::virtual_browser::VirtualBrowser;
use browser::DeviceType;
use browser::driver_updater::ChromeDriver;

const DEFAULT_PORT: u16 = 19876;
const CHROMEDRIVER_PORT: u16 = 9515;

/// Check if port is in use
fn is_port_in_use(port: u16) -> bool {
    std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
}

/// Execute chromedriver (if necessary) - returns logs
fn ensure_chromedriver(logs: &mut Vec<String>) -> Option<Child> {
    if !is_port_in_use(CHROMEDRIVER_PORT) {

        logs.push("🚗 Starting chromedriver...".to_string());

        let driver_path = match ChromeDriver::install() {
            Ok(path) => {
                logs.push(format!("✅ ChromeDriver installed/verified: {:?}", path));
                path
            },
            Err(e) => {
                logs.push(format!("❌ ChromeDriver update failed (trying PATH): {}", e));
                std::path::PathBuf::from("chromedriver")
            }
        };

        match ProcessCommand::new(driver_path)
            .arg(format!("--port={}", CHROMEDRIVER_PORT))
            .spawn()
        {
            Ok(child) => {
                logs.push(format!("✅ chromedriver process started (pid: {})", child.id()));
                
                for i in 0..20 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if is_port_in_use(CHROMEDRIVER_PORT) {
                        logs.push(format!("✅ chromedriver ready! ({}ms)", (i + 1) * 500));
                        return Some(child);
                    }
                }
                logs.push("⚠️ chromedriver start timeout".to_string());
                Some(child)
            }
            Err(e) => {
                logs.push(format!("❌ chromedriver execution failed: {}", e));
                logs.push("   Please check if chromedriver is in PATH or installed".to_string());
                None
            }
        }
    } else {
        logs.push(format!("✅ chromedriver is already running! (port: {})", CHROMEDRIVER_PORT));
        None
    }
}

// Parse directly with serde_json::Value (without derive)
fn parse_command(json: &serde_json::Value) -> Option<Command> {
    let cmd = json.get("cmd")?.as_str()?;
    match cmd {
        "open" => Some(Command::Open {
            device: json.get("device")?.as_str()?.to_string(),
            headless: json.get("headless").and_then(|v| v.as_bool()),
        }),
        "run" => Some(Command::Run {
            sequence_json: json.get("sequence_json")?.as_str()?.to_string(),
            keyword: json.get("keyword").and_then(|v| v.as_str()).map(|s| s.to_string()),
        }),
        "run_sequence" => Some(Command::RunSequence {
            sequence_json: json.get("sequence_json")?.as_str()?.to_string(),
            start_url: json.get("start_url")?.as_str()?.to_string(),
            device: json.get("device").and_then(|v| v.as_str()).unwrap_or("desktop").to_string(),
            headless: json.get("headless").and_then(|v| v.as_bool()).unwrap_or(false),
            inputs: json.get("inputs")
                .and_then(|v| v.as_object())
                .map(|obj| obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect())
                .unwrap_or_default(),
        }),
        "close" => Some(Command::Close),
        "ping" => Some(Command::Ping),
        "stop" => Some(Command::Stop),
        "shutdown" => Some(Command::Shutdown),
        _ => None,
    }
}

#[derive(Debug)]
enum Command {
    Open { device: String, headless: Option<bool> },
    Run { sequence_json: String, keyword: Option<String> },
    RunSequence { 
        sequence_json: String, 
        start_url: String, 
        device: String, 
        headless: bool,
        inputs: HashMap<String, String>,
    },
    Close,
    Ping,
    Stop,
    Shutdown,
}

fn make_response(status: &str, message: &str) -> String {
    serde_json::json!({
        "status": status,
        "message": message
    }).to_string()
}

fn make_response_with_logs(status: &str, message: &str, logs: Vec<String>) -> String {
    serde_json::json!({
        "status": status,
        "message": message,
        "logs": logs
    }).to_string()
}

fn make_response_ok(message: &str) -> String {
    make_response("ok", message)
}

fn make_response_ok_with_logs(message: &str, logs: Vec<String>) -> String {
    make_response_with_logs("ok", message, logs)
}

fn make_response_error(message: &str) -> String {
    make_response("error", message)
}

fn make_response_error_with_logs(message: &str, logs: Vec<String>) -> String {
    make_response_with_logs("error", message, logs)
}

struct BrowserState {
    browser: Option<VirtualBrowser>,
    runtime: Runtime,
    shutdown_flag: Arc<AtomicBool>,
    chromedriver_child: Option<Child>,
}

impl BrowserState {
    fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        Self {
            browser: None,
            runtime,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            chromedriver_child: None,
        }
    }
    
    fn is_open(&self) -> bool {
        self.browser.is_some()
    }
    
    
    fn cleanup(&mut self) {
        // Close browser
        if let Some(browser) = self.browser.take() {
            let _ = self.runtime.block_on(browser.close());
        }
        
        // Close chromedriver
        if let Some(ref mut child) = self.chromedriver_child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn handle_command(cmd: Command, state: &mut BrowserState) -> String {
    let mut logs: Vec<String> = Vec::new(); 
    match cmd {
        Command::Ping => {
            make_response_ok("pong")
        }
        
        Command::Open { device, headless } => {
            if state.is_open() {
                return make_response_error("Browser is already open");
            }
            
            // Re-check chromedriver status
            if !is_port_in_use(CHROMEDRIVER_PORT) {
                logs.push("⚠️ chromedriver is dead, restarting...".to_string());
                if let Some(child) = ensure_chromedriver(&mut logs) {
                    state.chromedriver_child = Some(child);
                } else {
                    return make_response_error_with_logs("Failed to start chromedriver", logs);
                }
            }
            
            let headless = headless.unwrap_or(false);
            let device_type = match device.to_lowercase().as_str() {
                "mobile" => DeviceType::Mobile,
                _ => DeviceType::Desktop,
            };
            
            match state.runtime.block_on(VirtualBrowser::new(device_type, Some(headless))) {
                Ok(browser) => {
                    let init_result = state.runtime.block_on(async {
                        browser.driver.goto("https://www.google.com").await
                    });
                    
                    if let Err(e) = init_result {
                        logs.push(format!("⚠️ Initial page load failed: {}", e));
                    }
                    
                    state.browser = Some(browser);
                    state.shutdown_flag.store(false, Ordering::SeqCst);
                    logs.push(format!("✅ {} browser opened (headless={})", device, headless));
                    make_response_ok_with_logs(&format!("{} browser opened (headless={})", device, headless), logs)
                }
                Err(e) => {
                    logs.push(format!("❌ Failed to open browser: {}", e));
                    make_response_error_with_logs(&format!("Failed to open browser: {}", e), logs)
                }
            }
        }
        
        Command::Run { sequence_json, keyword } => {
            if !state.is_open() {
                return make_response_error("Browser is not open");
            }
            
            let sequence: Sequence = match serde_json::from_str(&sequence_json) {
                Ok(seq) => seq,
                Err(e) => return make_response_error(&format!("Sequence parsing error: {}", e)),
            };
            
            let keyword = keyword.unwrap_or_else(|| "test".to_string());
            let seq_name = sequence.sequence_name.clone();
            let handler_count = sequence.step_sequence.len();
            
            logs.push(format!("▶️ Starting execution of sequence '{}' (keyword: {}, {} handlers)", seq_name, keyword, handler_count));
            
            let browser = state.browser.as_mut().unwrap();
            let shutdown_flag = state.shutdown_flag.clone();
            
            match state.runtime.block_on(async {
                let result = sequence.run(&browser.driver, shutdown_flag, None).await?;
                Ok::<_, anyhow::Error>(result)
            }) {
                Ok(result) => {
                    logs.push(format!("✅ Sequence '{}' completed ({} results)", seq_name, result.len()));
                    make_response_ok_with_logs(&format!("Sequence '{}' execution completed ({} results)", seq_name, result.len()), logs)
                }
                Err(e) => {
                    logs.push(format!("❌ Sequence '{}' failed: {}", seq_name, e));
                    make_response_error_with_logs(&format!("Execution error: {}", e), logs)
                }
            }
        }
        
        // Execute sequence in separate browser (starts at start_url, supports inputs)
        Command::RunSequence { sequence_json, start_url, device, headless, inputs } => {
            // Re-check chromedriver status
            if !is_port_in_use(CHROMEDRIVER_PORT) {
                logs.push("⚠️ chromedriver is dead, restarting...".to_string());
                if let Some(child) = ensure_chromedriver(&mut logs) {
                    state.chromedriver_child = Some(child);
                } else {
                    return make_response_error_with_logs("Failed to start chromedriver", logs);
                }
            }
            
            let sequence: Sequence = match serde_json::from_str(&sequence_json) {
                Ok(seq) => seq,
                Err(e) => return make_response_error(&format!("Sequence parsing error: {}", e)),
            };
            
            let device_type = match device.to_lowercase().as_str() {
                "mobile" => DeviceType::Mobile,
                _ => DeviceType::Desktop,
            };
            
            let seq_name = sequence.sequence_name.clone();
            let handler_count = sequence.step_sequence.len();
            
            logs.push(format!("🚀 Starting execution of sequence '{}'", seq_name));
            logs.push(format!("   📍 URL: {}", start_url));
            logs.push(format!("   📱 Device: {} (headless={})", device, headless));
            logs.push(format!("   📝 Handlers: {}", handler_count));
            
            // inputs logs
            if !inputs.is_empty() {
                logs.push(format!("   🔑 Inputs: {:?}", inputs.keys().collect::<Vec<_>>()));
            }
            
            // Create separate browser
            let browser_result = state.runtime.block_on(VirtualBrowser::new(device_type, Some(headless)));
            
            let temp_browser = match browser_result {
                Ok(b) => b,
                Err(e) => {
                    logs.push(format!("❌ Failed to open browser: {}", e));
                    return make_response_error_with_logs(&format!("Failed to open browser: {}", e), logs);
                }
            };
            
            // Configure ExecContext (inject inputs)
            let mut ctx = ExecContext::default();
            for (k, v) in &inputs {
                ctx.bindings.insert(k.clone(), v.clone());
            }
            
            // Run sequence
            let shutdown_flag = state.shutdown_flag.clone();
            shutdown_flag.store(false, Ordering::SeqCst);
            
            let result = state.runtime.block_on(async {
                // Navigate to start_url
                temp_browser.driver.goto(&start_url).await?;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                
                // Run sequence (pass ctx)
                let result = sequence.run(&temp_browser.driver, shutdown_flag, Some(&mut ctx)).await?;
                Ok::<_, anyhow::Error>(result)
            });
            
            // Close browser
            let _ = state.runtime.block_on(temp_browser.close());
            
            match result {
                Ok(data) => {
                    logs.push(format!("✅ Sequence '{}' completed ({} results)", seq_name, data.len()));
                    
                    // 필드별 통계
                    if !data.is_empty() {
                        let mut field_stats: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
                        for row in &data {
                            for (k, v) in row {
                                if !v.is_empty() {
                                    *field_stats.entry(k.clone()).or_insert(0) += 1;
                                }
                            }
                        }
                        logs.push(format!("   📊 Fields extracted:"));
                        for (field, count) in &field_stats {
                            logs.push(format!("      - {}: {} values", field, count));
                        }
                    }
                    
                    let response = serde_json::json!({
                        "status": "ok",
                        "message": format!("Sequence '{}' execution completed ({} results)", seq_name, data.len()),
                        "data": data,
                        "logs": logs
                    });
                    response.to_string()
                }
                Err(e) => {
                    logs.push(format!("❌ Sequence '{}' failed: {}", seq_name, e));
                    make_response_error_with_logs(&format!("Execution error: {}", e), logs)
                }
            }
        }
        
        Command::Stop => {
            state.shutdown_flag.store(true, Ordering::SeqCst);
            logs.push("⏹️ Stop execution requested".to_string());
            make_response_ok_with_logs("Stop execution requested", logs)
        }
        
        Command::Shutdown => {
            logs.push("🔴 Shutdown command received, cleaning up...".to_string());
            
            if let Some(browser) = state.browser.take() {
                let _ = state.runtime.block_on(browser.close());
                logs.push("   Browser closed".to_string());
            }
            
            if let Some(ref mut child) = state.chromedriver_child {
                let _ = child.kill();
                let _ = child.wait();
                logs.push("   chromedriver closed".to_string());
            }
            
            make_response_ok_with_logs("shutdown complete", logs)
        }
        
        Command::Close => {
            if !state.is_open() {
                return make_response_ok("Browser is already closed");
            }
            
            if let Some(browser) = state.browser.take() {
                let _ = state.runtime.block_on(browser.close());
            }
            
            logs.push("🔴 Browser closed".to_string());
            make_response_ok_with_logs("Browser closed", logs)
        }
    }
}

fn handle_client(mut stream: TcpStream, state: &mut BrowserState) -> bool {
    let _peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    
    let reader = BufReader::new(stream.try_clone().unwrap());
    let mut should_shutdown = false;
    
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        
        if line.is_empty() {
            continue;
        }
        
        let (response_json, shutdown) = match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(json) => {
                match parse_command(&json) {
                    Some(cmd) => {
                        let shutdown = matches!(cmd, Command::Shutdown);
                        (handle_command(cmd, state), shutdown)
                    }
                    None => (make_response_error("Unknown command"), false)
                }
            }
            Err(e) => (make_response_error(&format!("JSON parsing error: {}", e)), false),
        };
        
        if let Err(_) = writeln!(stream, "{}", response_json) {
            break;
        }
        
        if shutdown {
            should_shutdown = true;
            break;
        }
    }
    
    should_shutdown
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let port = args.get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    
    // Parent PID (UI process)
    let parent_pid: Option<u32> = args.get(2)
        .and_then(|s| s.parse().ok());
    
    let addr = format!("127.0.0.1:{}", port);
    eprintln!("🚀 Starting browser_runner (port: {})", port);
    if let Some(pid) = parent_pid {
        eprintln!("   Parent PID: {} (terminates together when closed)", pid);
    }
    
    let mut state = BrowserState::new();
    
    // Check/Run chromedriver
    let mut startup_logs: Vec<String> = Vec::new();
    state.chromedriver_child = ensure_chromedriver(&mut startup_logs);
    for log in &startup_logs {
        eprintln!("   {}", log);
    }
    
    let listener = TcpListener::bind(&addr).expect("Failed to bind port");
    
    // Check timeout with non-blocking
    listener.set_nonblocking(true).expect("Failed to set non-blocking");
    
    loop {
        // Check parent PID (exit if dead)
        if let Some(pid) = parent_pid {
            if !is_process_alive(pid) {
                eprintln!("💀 Parent process ({}) terminated, exiting together...", pid);
                break;
            }
        }
        
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).ok();
                let should_shutdown = handle_client(stream, &mut state);
                if should_shutdown {
                    eprintln!("🔴 Exiting due to shutdown command");
                    break;
                }
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
    
    // Cleanup before exit
    eprintln!("🧹 Cleaning up...");
    state.cleanup();
    eprintln!("👋 browser_runner exiting");
}

/// Check if process is alive
#[cfg(target_os = "windows")]
fn is_process_alive(pid: u32) -> bool {
    use std::process::Command;
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_process_alive(pid: u32) -> bool {
    use std::path::Path;
    Path::new(&format!("/proc/{}", pid)).exists()
}