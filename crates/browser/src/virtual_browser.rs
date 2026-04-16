// src/browser/virtual_browser.rs
use thirtyfour::prelude::*;
use anyhow::Result;
use collector::workflow::{Sequence,ExecContext};
use std::{collections::HashMap, sync::Arc};
use std::sync::atomic::AtomicBool;

use crate::{HEADLESS,SELENIUM_URL,DESKTOP_USER_AGENT,MOBILE_USER_AGENT,EXTERNAL_IP,DeviceType};

pub struct VirtualBrowser {
    pub driver: WebDriver,
    _device_type: DeviceType,
}
impl VirtualBrowser {
    pub async fn new(device_type: DeviceType, headless: Option<bool>) -> Result<Self> {
        
        let selenium_url = SELENIUM_URL;
        let is_headless = headless.unwrap_or(HEADLESS);

        let mut caps = DesiredCapabilities::chrome();
        
        let user_agent = match device_type {
            DeviceType::Mobile => {
                MOBILE_USER_AGENT.to_string()
            },
            DeviceType::Desktop => {
                DESKTOP_USER_AGENT.to_string()
            },
        };
        
        caps.add_arg(&format!("--user-agent={}", user_agent))?;

        // Chrome 132+ removed the legacy headless binary; `--headless=new` is required.
        if is_headless {
            caps.add_arg("--headless=new")?;
        }

        caps.add_arg("--log-level=3")?;
        caps.add_arg("--silent")?;

        caps.add_arg("--disable-background-networking")?;
        caps.add_arg("--disable-sync")?;
        caps.add_arg("--disable-default-apps")?;

        caps.add_arg("--disable-breakpad")?;

        caps.add_arg("--no-sandbox")?;
        caps.add_arg("--disable-dev-shm-usage")?;
        caps.add_arg("--disable-notifications")?;
        caps.add_arg("--disable-geolocation")?;
        caps.add_arg("--disable-extensions")?;

        caps.add_arg("--deny-permission-prompts")?;
        caps.add_arg("--disable-prompt-on-repost")?;
        caps.add_arg("--disable-blink-features=AutomationControlled")?;
        caps.add_arg("--window-size=1920,1080")?;
        caps.add_arg("--force-device-scale-factor=1")?;
        caps.add_arg("--start-maximized")?;
        caps.set_disable_web_security()?;

        // Remove the "Chrome is being controlled by automated test software" banner
        // and strip the automation extension so navigator.webdriver stays consistent.
        caps.add_exclude_switch("enable-automation")?;
        caps.add_experimental_option("useAutomationExtension", false)?;

        let external_ip = EXTERNAL_IP;
        if external_ip != "0.0.0.0" {
                caps.add_arg(&format!("--host={}", external_ip))?;
        }


        let driver = WebDriver::new(selenium_url, caps).await?;

    let result = async {
            driver.execute(
                r#"
                Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
                window.chrome = { runtime: {} };
                Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4, 5] });
                Object.defineProperty(navigator, 'languages', { get: () => ['ko-KR', 'ko'] });
                "#,
                vec![]
            ).await?;

            Ok::<_, anyhow::Error>(())
        }.await;
        
        // ✅ Cleanup driver on error
        if let Err(e) = result {
            let _ = driver.quit().await;
            return Err(e);
        }

        Ok(Self { 
            driver, 
            _device_type : device_type,
        })
    }
    
    async fn _dismiss_permission_popups(&self) -> Result<()> {
        let dismiss_selectors = vec![
            // Korean selectors (Do not translate to maintain functionality)
            "button[aria-label*='차단']",
            "button[aria-label*='거부']", 
            "button[aria-label*='허용 안함']",
            "button:contains('차단')",
            "button:contains('거부')",
            "button:contains('닫기')",
            // English selectors
            "[data-testid*='deny']",
            "[data-testid*='block']",
        ];
        
        for selector in dismiss_selectors {
            if let Ok(button) = self.driver.find(By::Css(selector)).await {
                if button.is_displayed().await.unwrap_or(false) {
                    button.click().await.ok();
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
        
        Ok(())
    }
    pub async fn move_to(&self, url: &str) -> Result<String> {
        self.driver.goto(url).await?;
        let title = self.driver.title().await?;
        Ok(title)
    }
    
    pub async fn close(self) -> Result<()> {
        self.driver.quit().await?;
        Ok(())
    }
    pub async fn run_sequence(&self,sequence:Sequence,shutdown_flag:Arc<AtomicBool>,ctx:Option<&mut ExecContext>)->Result<Vec<HashMap<String,String>>>{
        let mut out = vec![];
        let result: Vec<std::collections::HashMap<String, String>> = sequence.run(&self.driver,shutdown_flag,ctx).await?;
        for data in result{
            out.push(data);            
        }
        Ok(out)
    }
}