// src/collector/workflow.rs
use serde::{Serialize,Deserialize};
use scraper::{Html, ElementRef, Selector};
use std::collections::HashMap;
use std::vec;
use thirtyfour::prelude::*;
use thirtyfour::error::WebDriverErrorInfo;
use serde_json::Value;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashSet;

#[derive(Debug, Clone,PartialEq, Serialize, Deserialize)]
pub enum KeyAction {
    Enter,
    Tab,
    Escape,
    Space,
    Backspace,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}
#[derive(Default)]
pub struct ExecContext {
    pub bindings: HashMap<String, String>,  // key -> actual selector
    pub data: HashMap<String, String>,      // Collected data
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionResult{
    Done,
    Html(Html),
}
#[derive(Debug, Clone, PartialEq)]
pub enum ExecuteData<'a> {
    HtmlData(Html),
    Results(Vec<HashMap<String,String>>),
    ResultsWithAction(Vec<HashMap<String,String>>, Action),
    ResultsHesh(HashMap<String,String>),
    Container(&'a Html,String),
    Selector(String),
    
}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub struct BindValue<T>{
   pub value :T,
   pub binding :Option<String>,
}
impl<T: Clone> BindValue<T> {
    pub fn new(value: T) -> Self {
        Self { value, binding: None }
    }

    pub fn with_binding(value: T, key: impl Into<String>) -> Self {
        Self { value, binding: Some(key.into()) }
    }
}

impl BindValue<String> {
    pub fn resolve<'a>(&'a self, ctx: Option<&'a ExecContext>) -> &'a str {
        if let (Some(key), Some(ctx)) = (&self.binding, ctx) {
            ctx.bindings.get(key).map(|s| s.as_str()).unwrap_or(&self.value)
        } else {
            &self.value
        }
    }
}
impl BindValue<u64> {
    pub fn resolve(&self, ctx: Option<&ExecContext>) -> u64 {
        if let (Some(key), Some(ctx)) = (&self.binding, ctx) {
            ctx.bindings.get(key)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(self.value)
        } else {
            self.value
        }
    }
}
impl BindValue<i64> {
    pub fn resolve(&self, ctx: Option<&ExecContext>) -> i64 {
        if let (Some(key), Some(ctx)) = (&self.binding, ctx) {
            ctx.bindings.get(key)
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(self.value)
        } else {
            self.value
        }
    }
}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub enum  Action {
    Click { selector_string: BindValue<String> },
    Navigate { url: BindValue<String> },
    NavigateHref {base:BindValue<String>, href: BindValue<String> },
    Wait{time_ms: BindValue<u64>},
    WaitFor { selector_string: BindValue<String>, time_ms: BindValue<u64> },
    ScrollAll,
    ScrollDown{scroll : BindValue<i64>},
    ScrollUp{scroll : BindValue<i64>},
    ClickByText { selector_string: BindValue<String>, text: BindValue<String>},
    GetHTML{ selector_string: BindValue<String>,time_ms: BindValue<u64>},
    DismissPermission,
    SwitchToDefaultContent,
    SwitchToFrame{selector_string: BindValue<String>},
    Refresh,
    Forward,
    Backward,
    NewTab { url: BindValue<String> },
    SwitchTab { index: BindValue<u64> },
    CloseTab,
    SwitchToLastTab,
    Type {selector: BindValue<String>,text: BindValue<String>},
    PressKey { key: KeyAction },
    ClearAndType {selector: BindValue<String>,text: BindValue<String>},
    
}
impl Action {
    async fn execute(&self, driver: & WebDriver,input:& ExecContext) -> Result<ActionResult, WebDriverError> {
        match self {
            Action::Click{selector_string} => {
                let selector=selector_string.resolve(Some(input));
                if let Ok(element) = driver.find(By::Css(selector)).await {
                    if element.is_displayed().await.unwrap_or(false) {
                        element.click().await?;
                    }
                }
            },
            Action::Navigate{url} => {
                let url_unwrap=url.resolve(Some(input));
                driver.goto(url_unwrap).await?;
            },
            Action::Wait{time_ms}=>{
                let time_ms_unwrap = time_ms.resolve(Some(input));
                tokio::time::sleep(tokio::time::Duration::from_millis(time_ms_unwrap)).await;
            }
            Action::WaitFor{selector_string,time_ms} => {
                let selector_unwrap=selector_string.resolve(Some(input));
                let time_ms_unwrap = time_ms.resolve(Some(input));
                // Wait until the element appears
                for _ in 0..30 {
                    if driver.find(By::Css(selector_unwrap)).await.is_ok() {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(time_ms_unwrap)).await;
                }
            },
            Action::ScrollAll => {
                Self::scroll_all_areas(&driver).await?;
            },
            Action::ClickByText {selector_string, text } => {
                let selector_unwrap=selector_string.resolve(Some(input));
                let text_unwrap = text.resolve(Some(input));
                let elements = driver.find_all(By::Css(selector_unwrap)).await.unwrap_or_default();
                for element in elements {
                    if let Ok(element_text) = element.text().await {
                        if element_text.contains(text_unwrap) {
                            element.click().await?;
                            break;
                        }
                    }
                }
            },
            Action::GetHTML{selector_string,time_ms} => {
                let selector_unwrap=selector_string.resolve(Some(input));
                let time_ms_unwrap = time_ms.resolve(Some(input));
                const MAX_RETRIES: usize = 10;
                let mut attempt = 0;

                let selector = Selector::parse(selector_unwrap)
                    .map_err(|_| WebDriverError::UnknownError(WebDriverErrorInfo::new(
                        format!("Invalid CSS selector: {}", selector_unwrap).into()
                    )))?;

                loop {
                    // execute get_html()
                    let html_res = match self.get_html(driver).await {
                        Ok(h) => h,
                        Err(e) => {
                            eprintln!("❌ get_html failed (attempt {}): {:?}", attempt + 1, e);
                            return Err(WebDriverError::UnknownError(WebDriverErrorInfo::new(
                                format!("{:?}", e).into()
                            )));
                        }
                    };

                    // Destructure ActionResult::Data(ExecuteData::HtmlData(html))
                    let found = match &html_res {
                        ActionResult::Html(doc) => {
                            doc.select(&selector).next().is_some()
                        }
                        _ => {
                            eprintln!("❌ HTML data type mismatch: {:?}", html_res);
                            return Err(WebDriverError::UnknownError(WebDriverErrorInfo::new(
                                "HTML data type mismatch".into()
                            )));
                        }
                    };

                    if found {
                        return Ok(html_res);
                    }

                    attempt += 1;
                    if attempt >= MAX_RETRIES {

                        return Err(WebDriverError::UnknownError(WebDriverErrorInfo::new(
                            format!("Selector '{}' not found", selector_unwrap).into()
                        )));
                    }

                    println!(
                        "⚠️ Selector '{}' not found, retrying {} (waiting {}s)",
                        selector_unwrap, attempt,time_ms_unwrap
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(time_ms_unwrap)).await;
                }
            },

            Action::DismissPermission => {
                self.dismiss_permission_popups(driver).await?;
            },
            // Fix: Baskword -> Backward
            Action::Backward => {
                self.handle_backward(driver).await?;
            },
                              
            // Fix: Front -> Forward
            Action::Forward => {
                self.handle_forward(driver).await?;
            },

            Action::Refresh => {
                self.handle_refresh(driver).await?;
            },
  
            Action::SwitchToFrame{selector_string} => {
                let selector_unwrap=selector_string.resolve(Some(input));
                // e.g.: "iframe#searchIframe" or "iframe#entryIframe"
                let iframes = driver.find_all(By::Tag("iframe")).await?;
                let target_frame = driver.query(By::Css(selector_unwrap))
                                                .wait(Duration::from_secs(10), Duration::from_millis(500))
                                                .first()
                                                .await?;
                

                // Get unique identifier of target_frame
                let target_id = target_frame.attr("id").await?;
                let target_name = target_frame.attr("name").await?;
                let target_src = target_frame.attr("src").await?;

                // Find index of desired iframe
                for (index, frame) in iframes.iter().enumerate() {
                    let frame_id = frame.attr("id").await?;
                    let frame_name = frame.attr("name").await?;
                    let frame_src = frame.attr("src").await?;
                    
                    // Switch to the frame if id, name, or src matches
                    if (target_id.is_some() && target_id == frame_id) ||
                    (target_name.is_some() && target_name == frame_name) ||
                    (target_src.is_some() && target_src == frame_src) {
                        driver.enter_frame(index as u16).await?;
                        //println!("Current frame:[{}] id: '{}', name: '{}'", index, frame_id.unwrap_or_else(|| String::from("")), frame_name.unwrap_or_else(|| String::from("")));
                        break;
                    }
                }
            },
            Action::SwitchToDefaultContent => {
                driver.enter_default_frame().await?;
            },
            Action::NewTab { url } => {
                let u = url.resolve(Some(input));
                driver.new_tab().await?;
                let tabs = driver.windows().await?;
                driver.switch_to_window(tabs.last().unwrap().clone()).await?;
                driver.goto(u).await?;
            }

            Action::SwitchTab { index } => {
                let i = index.resolve(Some(input)) as usize;
                let tabs = driver.windows().await?;
                if let Some(tab) = tabs.get(i) {
                    driver.switch_to_window(tab.clone()).await?;
                }
            }

            Action::CloseTab => {
                driver.close_window().await?;
                let tabs = driver.windows().await?;
                if let Some(tab) = tabs.last() {
                    driver.switch_to_window(tab.clone()).await?;
                }
            }

            Action::SwitchToLastTab => {
                let tabs = driver.windows().await?;
                if let Some(tab) = tabs.last() {
                    driver.switch_to_window(tab.clone()).await?;
                }
            }
            Action::Type { selector, text } => {
                let sel = selector.resolve(Some(input));
                let txt = text.resolve(Some(input));
                driver.find(By::Css(sel)).await?.send_keys(txt).await?;
            }

            Action::PressKey { key } => {
                let k = match key {
                    KeyAction::Enter => Key::Enter,
                    KeyAction::Tab => Key::Tab,
                    KeyAction::Escape => Key::Escape,
                    KeyAction::Space => Key::Space,
                    KeyAction::Backspace => Key::Backspace,
                    KeyAction::ArrowUp => Key::Up,
                    KeyAction::ArrowDown => Key::Down,
                    KeyAction::ArrowLeft => Key::Left,
                    KeyAction::ArrowRight => Key::Right,
                };
                driver.action_chain().key_down(k).perform().await?;
            }

            Action::ClearAndType { selector, text } => {
                let sel = selector.resolve(Some(input));
                let txt = text.resolve(Some(input));
                let elem = driver.find(By::Css(sel)).await?;
                elem.clear().await?;
                elem.send_keys(txt).await?;
            }
            Action::NavigateHref { base, href } => {
                let base_unwrap = base.resolve(Some(input));
                let href_unwrap = href.resolve(Some(input));
                if href_unwrap.starts_with("http://") || href_unwrap.starts_with("https://") {
                    driver.goto(href_unwrap).await?;
                } else {
                    let joined = match url::Url::parse(&base_unwrap)
                        .and_then(|b| b.join(&href_unwrap))
                    {
                        Ok(u) => u.to_string(),
                        Err(_) => {
                            if href_unwrap.starts_with('/') {

                                if let Ok(b) = url::Url::parse(&base_unwrap) {
                                    format!("{}{}", b.origin().ascii_serialization(), href_unwrap)
                                } else {
                                    format!("{}{}", base_unwrap.trim_end_matches('/'), href_unwrap)
                                }
                            } else {
                                format!("{}/{}", base_unwrap.trim_end_matches('/'), href_unwrap)
                            }
                        }
                    };

                    driver.goto(joined).await?;
                }
            },

            Action::ScrollDown { scroll } => {
                let delta = scroll.resolve(Some(input)) as i64;
                Self::scroll_delta(driver,delta).await?;           
            },

            Action::ScrollUp { scroll } => {
                let delta = -scroll.resolve(Some(input)) as i64;
                Self::scroll_delta(driver,delta).await?;      
            },
        }
        Ok(ActionResult::Done)
    }
    async fn scroll_delta(driver: &WebDriver, delta: i64) -> Result<(), WebDriverError> {
        if delta == 0 {
            return Ok(());
        }
        
        let mut remaining = delta;
        
        for _ in 0..20 {
            let result = driver.execute(
                r#"
                const delta = arguments[0];
                const beforeY = window.pageYOffset || document.documentElement.scrollTop;
                
                window.scrollBy({ top: delta, behavior: 'instant' });
                
                const afterY = window.pageYOffset || document.documentElement.scrollTop;
                
                return { before: beforeY, after: afterY, moved: afterY - beforeY };
                "#,
                vec![remaining.into()]
            ).await?;
            let moved = result.json()
                .get("moved")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            
            if moved == 0 {
                break;
            }
            remaining -= moved;
            
            if remaining.abs() < 10 {
                break;
            }
            
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        Ok(())
    }
    
    async fn scroll_all_areas(driver: &WebDriver) -> Result<(), WebDriverError> {
        println!("Automatically detecting and loading all scrollable areas...");
        
        driver.execute(
            r#"
            async function scrollAllScrollableAreas() {
                const allElements = document.querySelectorAll('*');
                const scrollableElements = [];
                
                allElements.forEach(el => {
                    const style = window.getComputedStyle(el);
                    const overflowY = style.overflowY;
                    const overflow = style.overflow;
                    
                    const isScrollable = (
                        (overflowY === 'scroll' || overflowY === 'auto' || 
                        overflow === 'scroll' || overflow === 'auto') &&
                        el.scrollHeight > el.clientHeight
                    );
                    
                    if (isScrollable) {
                        scrollableElements.push(el);
                    }
                });
                
                console.log('Scrollable areas found:', scrollableElements.length);
                
                // Process each scrollable area sequentially
                for (let i = 0; i < scrollableElements.length; i++) {
                    const elem = scrollableElements[i];
                    console.log('Scrolling area #' + (i + 1) + '...');
                    
                    let previousHeight = 0;
                    let noChangeCount = 0;
                    const maxScrolls = 100;  // 👈 Increase count as we scroll incrementally
                    
                    for (let scrollNum = 0; scrollNum < maxScrolls; scrollNum++) {
                        const currentHeight = elem.scrollHeight;
                        const currentScroll = elem.scrollTop;
                        
                        // Check if scrolled to the bottom
                        if (currentScroll + elem.clientHeight >= currentHeight - 10) {
                            noChangeCount++;
                            if (noChangeCount >= 3) {
                                console.log('  Area #' + (i + 1) + ' completed');
                                break;
                            }
                        } else {
                            noChangeCount = 0;
                        }
                        
                        // 👇 Scroll incrementally (about one screen height)
                        elem.scrollTop += elem.clientHeight * 0.8;
                        
                        // 👇 Increase wait time
                        await new Promise(resolve => setTimeout(resolve, 800));
                        
                        previousHeight = currentHeight;
                    }
                }
                
                // Scroll body/document as well
                console.log('Scrolling main page...');
                let noChangeCount = 0;
                
                for (let i = 0; i < 50; i++) {
                    const currentHeight = Math.max(
                        document.body.scrollHeight,
                        document.documentElement.scrollHeight
                    );
                    const currentScroll = window.pageYOffset || document.documentElement.scrollTop;
                    
                    // Check if scrolled to the bottom
                    if (currentScroll + window.innerHeight >= currentHeight - 10) {
                        noChangeCount++;
                        if (noChangeCount >= 3) {
                            console.log('Main page completed');
                            break;
                        }
                    } else {
                        noChangeCount = 0;
                    }
                    
                    // 👇 Scroll incrementally
                    window.scrollBy(0, window.innerHeight * 0.8);
                    
                    // 👇 Increase wait time
                    await new Promise(resolve => setTimeout(resolve, 800));
                }
                
                return 'OK';
            }
            
            return scrollAllScrollableAreas();
            "#,
            vec![]
        ).await?;
        
        println!("✅ All scrollable areas loaded!");
        Ok(())
    }
    
    async fn dismiss_permission_popups(&self, driver: &WebDriver) -> Result<ActionResult, WebDriverError> {
        let dismiss_selectors = vec![
            // Korean selectors kept for functionality
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
            if let Ok(button) = driver.find(By::Css(selector)).await {
                if button.is_displayed().await.unwrap_or(false) {
                    button.click().await.ok();
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
        // Fix: ExecuteReslt -> ActionResult
        Ok(ActionResult::Done)
    }

    // Fix: ExecuteReslt -> ActionResult
    async fn get_html(&self, driver: &WebDriver) -> Result<ActionResult, WebDriverError> {
        let dom: String = driver
            .execute("return document.documentElement.outerHTML;", Vec::<Value>::new())
            .await?
            .json()
            .as_str()
            .ok_or_else(|| WebDriverError::UnknownError(WebDriverErrorInfo::new("Failed to convert HTML to string".into())))?
            .to_string();
        let html = Html::parse_document(&dom);
        Ok(ActionResult::Html(html))
    }

    async fn handle_backward(&self, driver: &WebDriver) -> Result<(), WebDriverError> {
        driver.back().await?;
        Ok(())
    }
    
    async fn handle_forward(&self, driver: &WebDriver) -> Result<(), WebDriverError> {
        driver.forward().await?;
        Ok(())
    }
    
    async fn handle_refresh(&self, driver: &WebDriver) -> Result<(), WebDriverError> {
        driver.refresh().await?;
        Ok(())
    }
}
macro_rules! impl_extraction {
    ($self:expr, $target:expr, $ctx:expr) => {{
        let mut data = HashMap::new();
        match $self {
            Extraction::Text { selector_string, field_name } => {
                let selector_str = selector_string.resolve($ctx);
                let selector = match Selector::parse(selector_str) {
                    Ok(s) => s,
                    Err(_) => return Err(WebDriverError::UnknownError(WebDriverErrorInfo::new(format!("Selector parsing failed: {}", selector_str)))),
                };
                let field = field_name.resolve($ctx);
                let out = $target.select(&selector).next()
                    .map(|elem| elem.text().collect::<String>().trim().to_string());
                if let Some(res) = out {
                    data.insert(field.to_string(), res);
                } else {
                    data.insert(field.to_string(), "".to_string());
                }
            },
            Extraction::Count { selector_string, field_name } => {
                let selector_str = selector_string.resolve($ctx);
                let selector = match Selector::parse(selector_str) {
                    Ok(s) => s,
                    Err(_) => return Err(WebDriverError::UnknownError(WebDriverErrorInfo::new(format!("Selector parsing failed: {}", selector_str)))),
                };
                let field = field_name.resolve($ctx);
                let out = $target.select(&selector).count().to_string();
                data.insert(field.to_string(), out);
            },
            Extraction::Attribute { selector_string, field_name, attr_str } => {
                let selector_str = selector_string.resolve($ctx);
                let selector = match Selector::parse(selector_str) {
                    Ok(s) => s,
                    Err(_) => return Err(WebDriverError::UnknownError(WebDriverErrorInfo::new(format!("Selector parsing failed: {}", selector_str)))),
                };
                let field = field_name.resolve($ctx);
                let attr = attr_str.resolve($ctx);
                let out = $target.select(&selector).next()
                    .and_then(|elem| elem.value().attr(&attr));
                if let Some(res) = out {
                    data.insert(field.to_string(), res.to_string());
                } else {
                    data.insert(field.to_string(), "".to_string());
                }
            },
            Extraction::Exists { selector_string, field_name } => {
                let selector_str = selector_string.resolve($ctx);
                let selector = match Selector::parse(selector_str) {
                    Ok(s) => s,
                    Err(_) => return Err(WebDriverError::UnknownError(WebDriverErrorInfo::new(format!("Selector parsing failed: {}", selector_str)))),
                };
                let field = field_name.resolve($ctx);
                let found = $target.select(&selector).next();
                let out = if let Some(_) = found {
                    "true".to_string()
                } else {
                    "false".to_string()
                };
                data.insert(field.to_string(), out);
            },
            Extraction::MultipleText { selector_string, field_name } => {
                let selector_str = selector_string.resolve($ctx);
                let selector = match Selector::parse(selector_str) {
                    Ok(s) => s,
                    Err(_) => return Err(WebDriverError::UnknownError(WebDriverErrorInfo::new(format!("Selector parsing failed: {}", selector_str)))),
                };
                let field = field_name.resolve($ctx);
                let texts: Vec<String> = $target.select(&selector)
                    .map(|elem| elem.text().collect::<String>().trim().to_string())
                    .filter(|text| !text.is_empty())
                    .collect();
                if texts.is_empty() {
                    data.insert(field.to_string(), "".to_string());
                } else {
                    data.insert(field.to_string(), texts.join(", "));
                }
            },
        };
        Ok(data)
    }};
}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub enum  Extraction {
    Text{ selector_string: BindValue<String> ,field_name:BindValue<String> },
    Count{ selector_string: BindValue<String> ,field_name:BindValue<String> },
    Attribute{ selector_string: BindValue<String> ,field_name:BindValue<String>,attr_str:BindValue<String> },
    Exists{ selector_string: BindValue<String> ,field_name:BindValue<String> },
    MultipleText{ selector_string: BindValue<String> ,field_name:BindValue<String> },
}
impl Extraction {
    pub fn execute_html(
        &self, html: &Html, ctx: &ExecContext,
    ) -> Result<HashMap<String, String>, WebDriverError> {
        impl_extraction!(self, html, Some(ctx))
    }

    pub fn execute_element(
        &self, element: &ElementRef<'_>, ctx: &ExecContext,
    ) -> Result<HashMap<String, String>, WebDriverError> {
        impl_extraction!(self, element, Some(ctx))
    }
}
#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Step {
    Act(Action),
    Extract(Extraction),
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Handler{
    Item(Step),
    Container{selector: BindValue<String>,steps:Vec<Step>,dedup:bool},
    SubSequence(Sequence)
}

#[derive(Debug, Clone,Serialize,Deserialize)]
pub struct Sequence {
    pub sequence_name: String,
    pub step_sequence: Vec<Handler>,
    pub target_data: HashMap<String, String>,
    pub metadata:Vec<String>
}

impl Sequence {
    pub async fn run(
        &self,
        driver: &WebDriver,
        shutdown_flag: Arc<AtomicBool>,
        ctx: Option<&mut ExecContext>,
    ) -> Result<Vec<HashMap<String, String>>, WebDriverError> {
        // Create empty context if ctx is None
        let mut default_ctx = ExecContext::default();
        let ctx = ctx.unwrap_or(&mut default_ctx);
        
        let mut results: Vec<HashMap<String, String>> = Vec::new();
        let mut current_html: Option<Html> = None;

        for handler in &self.step_sequence {
            if shutdown_flag.load(Ordering::SeqCst) {
                return Err(WebDriverError::UnknownError(
                    WebDriverErrorInfo::new(format!("{}🛑 Stop signal", self.sequence_name)),
                ));
            }

            match handler {
                Handler::Item(step) => match step {
                    Step::Act(action) => {
                        match action.execute(driver, ctx).await? {
                            ActionResult::Done => {}
                            ActionResult::Html(html) => {
                                current_html = Some(html);
                            }
                        }
                    }
                    Step::Extract(extraction) => {
                        if let Some(ref html) = current_html {
                            let data = extraction.execute_html(html, ctx)?;
                            for (k, v) in &data {
                                ctx.bindings.insert(k.clone(), v.clone());
                            }
                            ctx.data.extend(data);
                        }
                    }
                },

                Handler::Container { selector, steps, dedup } => {
                    let html = current_html.as_ref().ok_or_else(|| {
                        WebDriverError::UnknownError(WebDriverErrorInfo::new(
                            "No HTML available before running Container.".into(),
                        ))
                    })?;

                    let sel_str = selector.resolve(Some(ctx));
                    let css = Selector::parse(sel_str).map_err(|_| {
                        WebDriverError::UnknownError(WebDriverErrorInfo::new(
                            format!("Selector parsing failed: {}", sel_str),
                        ))
                    })?;
                    let mut seen: HashSet<String> = HashSet::new();

                    // Clear results and refill — for SubSequence input
                    results.clear();

                    for (idx, element) in html.select(&css).enumerate() {
                        let mut row = self.target_data.clone();
                        row.insert("_index".to_string(), idx.to_string());
                        for step in steps {
                            match step {
                                Step::Act(_) => {}
                                Step::Extract(extraction) => {
                                    let data = extraction.execute_element(&element, ctx)?;
                                    for (k, v) in data {
                                        row.insert(k, v);
                                    }
                                }
                            }
                        }
                        if *dedup {
                            let dedup_key = row.values().next()
                                .cloned()
                                .unwrap_or_default();
                            if !seen.insert(dedup_key) {
                                continue;
                            }
                        }

                        results.push(row);
                    }
                }

                Handler::SubSequence(sub_seq) => {
                    let container_rows = std::mem::take(&mut results);

                    for row in container_rows {  // & 제거
                        if shutdown_flag.load(Ordering::SeqCst) {
                            return Err(WebDriverError::UnknownError(
                                WebDriverErrorInfo::new(
                                    format!("{}🛑 Stop signal", self.sequence_name),
                                ),
                            ));
                        }
                        
                        let mut sub_ctx = ExecContext {
                            bindings: ctx.bindings.clone(),
                            data: HashMap::new(),
                        };
                        for (k, v) in &row {
                            sub_ctx.bindings.insert(k.clone(), v.clone());
                        }

                        let sub_results = Box::pin(sub_seq.run(
                            driver,
                            Arc::clone(&shutdown_flag),
                            Some(&mut sub_ctx),
                        ))
                        .await?;

                        if sub_results.is_empty() {

                            results.push(row);
                        } else {
                            for mut sub_row in sub_results {
                                for (k, v) in &row {
                                    sub_row.entry(k.clone()).or_insert(v.clone());
                                }
                                results.push(sub_row);
                            }
                        }
                    }
                }
            }
        }
        if !ctx.data.is_empty() {
            results.push(ctx.data.clone());
        }
        Ok(results)
    }

}