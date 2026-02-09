// crates/collector/src/test_input_card.rs
//! Input Card Widget for Sequence Testing
//! 
//! Parses binding keys used in the sequence, and
//! provides a UI for inputting values to be used during test execution

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Element, Length, Padding, Theme};
use std::collections::HashMap;

use crate::workflow::{Action, Extraction, Handler, Sequence, Step};

// =========================================================================
// Messages
// =========================================================================

#[derive(Debug, Clone)]
pub enum TestInputMsg {
    /// Change input value
    ValueChanged(String, String),  // (key, value)
    /// Clear all inputs
    ClearAll,
    /// Re-parse keys from sequence
    RefreshKeys,
}

// =========================================================================
// State
// =========================================================================

#[derive(Debug, Clone, Default)]
pub struct TestInputState {
    /// List of found binding keys
    pub binding_keys: Vec<String>,
    /// Test input values (key -> value)
    pub test_values: HashMap<String, String>,
    /// Keys automatically injected by subsequence mapping (Excluded from test input)
    mapped_keys: std::collections::HashSet<String>,
}

impl TestInputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract binding keys from Sequence
    pub fn parse_from_sequence(&mut self, seq: &Sequence) {
        self.binding_keys.clear();
        self.mapped_keys.clear();
        
        for handler in &seq.step_squence {
            self.extract_keys_from_handler(handler);
        }
        
        // Exclude keys injected by subsequence mapping
        self.binding_keys.retain(|k| !self.mapped_keys.contains(k));
        
        // Remove duplicates and sort
        self.binding_keys.sort();
        self.binding_keys.dedup();
        
        // Initialize new keys with empty values (preserve existing values)
        for key in &self.binding_keys {
            self.test_values.entry(key.clone()).or_insert_with(String::new);
        }
        
        // Remove obsolete keys
        let valid_keys: std::collections::HashSet<_> = self.binding_keys.iter().cloned().collect();
        self.test_values.retain(|k, _| valid_keys.contains(k));
    }

    /// Extract binding keys from Workflow (Vec<Handler>)
    pub fn parse_from_workflow(&mut self, handlers: &[Handler]) {
        self.binding_keys.clear();
        self.mapped_keys.clear();
        
        for handler in handlers {
            self.extract_keys_from_handler(handler);
        }
        
        // Exclude keys injected by subsequence mapping
        self.binding_keys.retain(|k| !self.mapped_keys.contains(k));
        
        self.binding_keys.sort();
        self.binding_keys.dedup();
        
        for key in &self.binding_keys {
            self.test_values.entry(key.clone()).or_insert_with(String::new);
        }
        
        let valid_keys: std::collections::HashSet<_> = self.binding_keys.iter().cloned().collect();
        self.test_values.retain(|k, _| valid_keys.contains(k));
    }

    fn extract_keys_from_handler(&mut self, handler: &Handler) {
        match handler {
            Handler::Item(Step::Act(action)) => {
                self.extract_from_action(action);
            }
            Handler::Item(Step::Extract(ext)) => {
                self.extract_from_extraction(ext);
            }
            Handler::Container { selector, steps, .. } => {
                if let Some(k) = &selector.binding {
                    if !k.is_empty() { self.binding_keys.push(k.clone()); }
                }
                for step in steps {
                    if let Step::Extract(ext) = step {
                        self.extract_from_extraction(ext);
                    }
                }
            }
            Handler::SubSequence(inner_seq) => {
                // Extract mapped keys from metadata (These keys are automatically injected from Container)
                for meta in &inner_seq.metadata {
                    if meta.starts_with("mapping:") {
                        // Format: "mapping:binding_key:source_field"
                        let parts: Vec<&str> = meta.split(':').collect();
                        if parts.len() >= 2 {
                            self.mapped_keys.insert(parts[1].to_string());
                        }
                    }
                }
                
                // Also traverse internal handlers of SubSequence
                for h in &inner_seq.step_squence {
                    self.extract_keys_from_handler(h);
                }
            }
        }
    }

    fn extract_from_action(&mut self, action: &Action) {
        match action {
            Action::Click { selector_string } => {
                self.push_binding(&selector_string.binding);
            }
            Action::Navigate { url } => {
                self.push_binding(&url.binding);
            }
            Action::WaitFor { selector_string, time_ms } => {
                self.push_binding(&selector_string.binding);
                self.push_binding(&time_ms.binding);
            }
            Action::ClickByText { selector_string, text } => {
                self.push_binding(&selector_string.binding);
                self.push_binding(&text.binding);
            }
            Action::GetHTML { selector_string, time_ms } => {
                self.push_binding(&selector_string.binding);
                self.push_binding(&time_ms.binding);
            }
            Action::SwitchToFrame { selector_string } => {
                self.push_binding(&selector_string.binding);
            }
            Action::NewTab { url } => {
                self.push_binding(&url.binding);
            }
            Action::SwitchTab { index } => {
                self.push_binding(&index.binding);
            }
            Action::Type { selector, text } => {
                self.push_binding(&selector.binding);
                self.push_binding(&text.binding);
            }
            Action::ClearAndType { selector, text } => {
                self.push_binding(&selector.binding);
                self.push_binding(&text.binding);
            }
            Action::NavigateHref { base, href } => {
                self.push_binding(&base.binding);
                self.push_binding(&href.binding);
            }
            Action::ScrollDown { scroll } => {
                self.push_binding(&scroll.binding);
            }
            Action::ScrollUp { scroll } => {
                self.push_binding(&scroll.binding);
            }
            Action::Wait { time_ms } => {
                self.push_binding(&time_ms.binding);
            }
            _ => {}
        }
    }

    fn extract_from_extraction(&mut self, ext: &Extraction) {
        match ext {
            Extraction::Text { selector_string, field_name } |
            Extraction::Count { selector_string, field_name } |
            Extraction::Exists { selector_string, field_name } |
            Extraction::MultipleText { selector_string, field_name } => {
                self.push_binding(&selector_string.binding);
                self.push_binding(&field_name.binding);
            }
            Extraction::Attribute { selector_string, field_name, attr_str } => {
                self.push_binding(&selector_string.binding);
                self.push_binding(&field_name.binding);
                self.push_binding(&attr_str.binding);
            }
        }
    }

    fn push_binding(&mut self, binding: &Option<String>) {
        if let Some(k) = binding.as_ref() {
            if !k.is_empty() {
                self.binding_keys.push(k.clone());
            }
        }
    }

    /// Update state based on message
    pub fn update(&mut self, msg: TestInputMsg) {
        match msg {
            TestInputMsg::ValueChanged(key, value) => {
                self.test_values.insert(key, value);
            }
            TestInputMsg::ClearAll => {
                for value in self.test_values.values_mut() {
                    value.clear();
                }
            }
            TestInputMsg::RefreshKeys => {
                // External call to parse_from_* required
            }
        }
    }

    /// Generate target_data for test execution
    pub fn get_target_data(&self) -> HashMap<String, String> {
        self.test_values.clone()
    }

    /// Check if there are any required inputs
    pub fn has_required_inputs(&self) -> bool {
        !self.binding_keys.is_empty()
    }

    /// Check if all required inputs are filled
    pub fn all_filled(&self) -> bool {
        self.binding_keys.iter().all(|k| {
            self.test_values.get(k).map(|v| !v.is_empty()).unwrap_or(false)
        })
    }

    /// Count of filled inputs
    pub fn filled_count(&self) -> usize {
        self.binding_keys.iter()
            .filter(|k| self.test_values.get(*k).map(|v| !v.is_empty()).unwrap_or(false))
            .count()
    }

    // =====================================================================
    // View
    // =====================================================================

    pub fn view(&self) -> Element<'_, TestInputMsg, Theme> {
        let header = container(
            row![
                text("🧪 Test Input").size(14),
                Space::new().width(Length::Fill),
                text(format!("{}/{}", self.filled_count(), self.binding_keys.len())).size(11),
                Space::new().width(8),
                button(text("🗑 Clear").size(10))
                    .padding(Padding::from([4, 8]))
                    .on_press(TestInputMsg::ClearAll),
            ]
            .align_y(iced::Alignment::Center)
        )
        .padding(10)
        .width(Length::Fill)
        .style(header_style);

        let content: Element<'_, TestInputMsg, Theme> = if self.binding_keys.is_empty() {
            container(
                text("No binding keys found").size(11)
            )
            .padding(12)
            .width(Length::Fill)
            .style(hint_style)
            .into()
        } else {
            let mut col = column![].spacing(8);
            
            for key in &self.binding_keys {
                let value = self.test_values.get(key).cloned().unwrap_or_default();
                let is_filled = !value.is_empty();
                
                let key_clone = key.clone();
                let input_row = container(
                    row![
                        // Status Icon
                        if is_filled {
                            text("✅").size(12)
                        } else {
                            text("⬜").size(12)
                        },
                        Space::new().width(8),
                        // Key Name
                        container(
                            text(format!("[{}]", key)).size(11)
                        )
                        .width(120)
                        .style(key_badge_style),
                        Space::new().width(8),
                        // Input Field
                        text_input("Enter value...", &value)
                            .on_input(move |v| TestInputMsg::ValueChanged(key_clone.clone(), v))
                            .padding(6)
                            .size(12)
                            .width(Length::Fill),
                    ]
                    .align_y(iced::Alignment::Center)
                )
                .padding(8)
                .width(Length::Fill)
                .style(row_style);
                
                col = col.push(input_row);
            }
            
            scrollable(col.padding(8))
                .height(Length::Fill)
                .into()
        };

        container(
            column![header, content]
        )
        .width(Length::Fill)
        .style(card_style)
        .into()
    }

    /// Compact view (Collapsed state)
    pub fn view_compact(&self) -> Element<'_, TestInputMsg, Theme> {
        let status = if self.binding_keys.is_empty() {
            "No input".to_string()
        } else if self.all_filled() {
            format!("✅ {}/{} filled", self.filled_count(), self.binding_keys.len())
        } else {
            format!("⚠️ {}/{} filled", self.filled_count(), self.binding_keys.len())
        };

        container(
            row![
                text("🧪").size(14),
                Space::new().width(6),
                text(status).size(11),
            ]
            .align_y(iced::Alignment::Center)
        )
        .padding(8)
        .width(Length::Fill)
        .style(card_style)
        .into()
    }
}

// =========================================================================
// Styles
// =========================================================================

fn card_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.12, 0.14, 0.18, 0.95))),
        border: iced::Border {
            color: iced::Color::from_rgba(0.3, 0.35, 0.4, 0.6),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn header_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.15, 0.18, 0.22, 0.9))),
        border: iced::Border {
            color: iced::Color::from_rgba(0.25, 0.3, 0.35, 0.5),
            width: 0.0,
            radius: iced::border::Radius::default().top(8.0),
        },
        ..Default::default()
    }
}

fn hint_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.2, 0.22, 0.25, 0.5))),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn row_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.1, 0.12, 0.15, 0.8))),
        border: iced::Border {
            color: iced::Color::from_rgba(0.25, 0.28, 0.32, 0.5),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn key_badge_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.3, 0.5, 0.7, 0.3))),
        border: iced::Border {
            color: iced::Color::from_rgba(0.4, 0.6, 0.8, 0.5),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}