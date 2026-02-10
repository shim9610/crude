// src/work_flow_ui.rs
//! Drag-and-drop workflow builder - direct Handler binding

use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, text, text_input, Space,
    Id,
};
use iced::widget::operation::scroll_to;
use iced::widget::text_input as ti;
use iced::widget::scrollable::{AbsoluteOffset, Viewport};
use iced_mouse_layer::mouse_layer;
use iced::{
    mouse, Element, Event, Length, Padding, Point, 
    Subscription, Task, Theme, Size,
};
use std::collections::HashMap;

use collector::handler_editor::{EditorMsg, EditorState, extract_binding_keys_from_sequence};
use collector::styles;
use collector::test_input_card::{TestInputMsg, TestInputState};
use collector::workflow::{
    Action, BindValue, Extraction, Handler, KeyAction, Sequence, Step,
};

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("Workflow Builder")
        .theme(Theme::TokyoNightStorm)
        .window_size(Size::new(1600.0, 850.0))
        .subscription(App::subscription)
        .run()
}

// =========================================================================
// Layout constants - used for scroll calculations, do not modify
// =========================================================================
const PALETTE_WIDTH: f32 = 280.0;
const EDITOR_WIDTH: f32 = 450.0;
const TEST_INPUT_WIDTH: f32 = 280.0;
const TOOLBAR_HEIGHT: f32 = 48.0;
const HEADER_HEIGHT: f32 = 48.0;
const CARD_HEIGHT: f32 = 52.0;
const CARD_SPACING: f32 = 12.0;
const CONTENT_PADDING: f32 = 12.0;
const OUTER_PADDING: f32 = 16.0;

// =========================================================================
// Palette handler types - replaces v1 HandlerKind
// =========================================================================

/// Handler templates that can be created by dragging from the palette
#[derive(Debug, Clone, PartialEq)]
enum PaletteKind {
    // Navigation
    Navigate,
    NavigateHref,  
    NewTab,
    Refresh,
    Backward,
    Forward,
    // Click
    Click,
    ClickByText,
    // Wait
    WaitFor,
    Wait,    
    ScrollAll,
    ScrollDown,   
    ScrollUp,     
    // Input
    Type,
    ClearAndType,
    PressKey,
    // Frame
    SwitchToFrame,
    SwitchToDefaultContent,
    // Tab
    SwitchTab,
    CloseTab,
    SwitchToLastTab,
    // Misc
    GetHTML,
    DismissPermission,
    // Extract
    ExtractText,
    ExtractAttribute,
    ExtractCount,
    ExtractExists,
    ExtractMultipleText,
    // Structure
    Container,
    SubSequence,
}

/// Palette group definition
struct PaletteGroup {
    name: &'static str,
    icon: &'static str,
    kinds: &'static [PaletteKind],
}

const PALETTE_GROUPS: &[PaletteGroup] = &[
    PaletteGroup { name: "Navigation", icon: "🧭", kinds: &[
        PaletteKind::Navigate, PaletteKind::NavigateHref, PaletteKind::NewTab, PaletteKind::Refresh,
        PaletteKind::Backward, PaletteKind::Forward,
    ]},
    PaletteGroup { name: "Click", icon: "🖱️", kinds: &[
        PaletteKind::Click, PaletteKind::ClickByText,
    ]},
    PaletteGroup { name: "Wait", icon: "⏳", kinds: &[
        PaletteKind::WaitFor,PaletteKind::Wait, PaletteKind::ScrollAll,PaletteKind::ScrollDown,PaletteKind::ScrollUp,
    ]},
    PaletteGroup { name: "Input", icon: "⌨️", kinds: &[
        PaletteKind::Type, PaletteKind::ClearAndType, PaletteKind::PressKey,
    ]},
    PaletteGroup { name: "Frame", icon: "🖼️", kinds: &[
        PaletteKind::SwitchToFrame, PaletteKind::SwitchToDefaultContent,
    ]},
    PaletteGroup { name: "Tab", icon: "📑", kinds: &[
        PaletteKind::SwitchTab, PaletteKind::CloseTab, PaletteKind::SwitchToLastTab,
    ]},
    PaletteGroup { name: "Misc", icon: "⚡", kinds: &[
        PaletteKind::GetHTML, PaletteKind::DismissPermission,
    ]},
    PaletteGroup { name: "Extract", icon: "📊", kinds: &[
        PaletteKind::ExtractText, PaletteKind::ExtractAttribute,
        PaletteKind::ExtractCount, PaletteKind::ExtractExists,
        PaletteKind::ExtractMultipleText,
    ]},
    PaletteGroup { name: "Structure", icon: "📦", kinds: &[
        PaletteKind::Container, PaletteKind::SubSequence,
    ]},
];

impl PaletteKind {
    fn icon(&self) -> &'static str {
        match self {
            // Navigation
            PaletteKind::Navigate => "🔗",
            PaletteKind::NavigateHref => "🔗",
            PaletteKind::NewTab => "➕",
            PaletteKind::Refresh => "🔄",
            PaletteKind::Backward => "◀️",
            PaletteKind::Forward => "▶️",
            // Click
            PaletteKind::Click => "👆",
            PaletteKind::ClickByText => "🔤",
            // Wait
            PaletteKind::WaitFor => "⏳",
            PaletteKind::Wait => "⏱️", 
            PaletteKind::ScrollAll => "📜",
            PaletteKind::ScrollDown => "⬇️",
            PaletteKind::ScrollUp => "⬆️",
            // Input
            PaletteKind::Type => "⌨️",
            PaletteKind::ClearAndType => "🔁",
            PaletteKind::PressKey => "⌨️",
            // Frame
            PaletteKind::SwitchToFrame => "🖼️",
            PaletteKind::SwitchToDefaultContent => "🏠",
            // Tab
            PaletteKind::SwitchTab => "🔢",
            PaletteKind::CloseTab => "✖️",
            PaletteKind::SwitchToLastTab => "↩️",
            // Misc
            PaletteKind::GetHTML => "📄",
            PaletteKind::DismissPermission => "🚫",
            // Extract
            PaletteKind::ExtractText => "📝",
            PaletteKind::ExtractMultipleText => "📋",
            PaletteKind::ExtractAttribute => "🏷️",
            PaletteKind::ExtractCount => "🔢",
            PaletteKind::ExtractExists => "❓",
            // Structure
            PaletteKind::Container => "📦",
            PaletteKind::SubSequence => "🔄",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            PaletteKind::Navigate => "Navigate",
            PaletteKind::NewTab => "New Tab",
            PaletteKind::Refresh => "Refresh",
            PaletteKind::Backward => "Back",
            PaletteKind::Forward => "Forward",
            PaletteKind::Click => "Click",
            PaletteKind::ClickByText => "Click Text",
            PaletteKind::WaitFor => "Wait For",
            PaletteKind::Wait => "Wait",
            PaletteKind::ScrollAll => "Scroll All",
            PaletteKind::Type => "Type",
            PaletteKind::ClearAndType => "Clear & Type",
            PaletteKind::PressKey => "Press Key",
            PaletteKind::SwitchToFrame => "Switch Frame",
            PaletteKind::SwitchToDefaultContent => "Default Frame",
            PaletteKind::SwitchTab => "Switch Tab",
            PaletteKind::CloseTab => "Close Tab",
            PaletteKind::SwitchToLastTab => "Last Tab",
            PaletteKind::GetHTML => "Get HTML",
            PaletteKind::DismissPermission => "Dismiss",
            PaletteKind::ExtractText => "Text",
            PaletteKind::ExtractAttribute => "Attribute",
            PaletteKind::ExtractCount => "Count",
            PaletteKind::ExtractExists => "Exists",
            PaletteKind::Container => "Container",
            PaletteKind::SubSequence => "SubSequence",
            PaletteKind::NavigateHref => "Navigate Href",
            PaletteKind::ScrollDown => "Scroll Down",
            PaletteKind::ScrollUp => "Scroll Up",
            PaletteKind::ExtractMultipleText => "Multi Text", 
        }
    }

    fn color(&self) -> (f32, f32, f32) {
        match self {
            // Navigation - blue
            PaletteKind::Navigate | PaletteKind::NavigateHref  | PaletteKind::NewTab | PaletteKind::Refresh |
            PaletteKind::Backward | PaletteKind::Forward => (0.3, 0.6, 0.9),
            // Click - sky blue
            PaletteKind::Click | PaletteKind::ClickByText => (0.4, 0.7, 1.0),
            // Wait - yellow
            PaletteKind::WaitFor | PaletteKind::Wait | PaletteKind::ScrollAll | PaletteKind::ScrollDown | PaletteKind::ScrollUp => (0.9, 0.8, 0.4),
            // Input - cyan
            PaletteKind::Type | PaletteKind::ClearAndType | PaletteKind::PressKey => (0.4, 0.8, 0.8),
            // Frame - orange
            PaletteKind::SwitchToFrame | PaletteKind::SwitchToDefaultContent => (0.9, 0.6, 0.3),
            // Tab - teal
            PaletteKind::SwitchTab | PaletteKind::CloseTab | PaletteKind::SwitchToLastTab => (0.3, 0.7, 0.7),
            // Misc - gray
            PaletteKind::GetHTML | PaletteKind::DismissPermission => (0.6, 0.6, 0.7),
            // Extract - purple
            PaletteKind::ExtractText | PaletteKind::ExtractAttribute |
            PaletteKind::ExtractCount | PaletteKind::ExtractExists => (0.7, 0.5, 0.9),
            // Structure - green
            PaletteKind::Container => (0.5, 0.8, 0.5),
            PaletteKind::SubSequence => (0.9, 0.5, 0.6),
            PaletteKind::ExtractMultipleText => (0.7, 0.5, 0.9),
        }
    }

    /// Creates a default Handler for this palette type
    fn default_handler(&self) -> Handler {
        match self {
            PaletteKind::Navigate => Handler::Item(Step::Act(Action::Navigate {
                url: BindValue::new(String::new()),
            })),
            PaletteKind::NewTab => Handler::Item(Step::Act(Action::NewTab {
                url: BindValue::new(String::new()),
            })),
            PaletteKind::Refresh => Handler::Item(Step::Act(Action::Refresh)),
            PaletteKind::Backward => Handler::Item(Step::Act(Action::Backward)),
            PaletteKind::Forward => Handler::Item(Step::Act(Action::Forward)),
            PaletteKind::Click => Handler::Item(Step::Act(Action::Click {
                selector_string: BindValue::new(String::new()),
            })),
            PaletteKind::ClickByText => Handler::Item(Step::Act(Action::ClickByText {
                selector_string: BindValue::new(String::new()),
                text: BindValue::new(String::new()),
            })),
            PaletteKind::WaitFor => Handler::Item(Step::Act(Action::WaitFor {
                selector_string: BindValue::new(String::new()),
                time_ms: BindValue::new(5000),
            })),
            PaletteKind::ScrollAll => Handler::Item(Step::Act(Action::ScrollAll)),
            PaletteKind::Type => Handler::Item(Step::Act(Action::Type {
                selector: BindValue::new(String::new()),
                text: BindValue::new(String::new()),
            })),
            PaletteKind::ClearAndType => Handler::Item(Step::Act(Action::ClearAndType {
                selector: BindValue::new(String::new()),
                text: BindValue::new(String::new()),
            })),
            PaletteKind::PressKey => Handler::Item(Step::Act(Action::PressKey {
                key: KeyAction::Enter,
            })),
            PaletteKind::SwitchToFrame => Handler::Item(Step::Act(Action::SwitchToFrame {
                selector_string: BindValue::new(String::new()),
            })),
            PaletteKind::SwitchToDefaultContent => Handler::Item(Step::Act(Action::SwitchToDefaultContent)),
            PaletteKind::SwitchTab => Handler::Item(Step::Act(Action::SwitchTab {
                index: BindValue::new(0),
            })),
            PaletteKind::CloseTab => Handler::Item(Step::Act(Action::CloseTab)),
            PaletteKind::SwitchToLastTab => Handler::Item(Step::Act(Action::SwitchToLastTab)),
            PaletteKind::GetHTML => Handler::Item(Step::Act(Action::GetHTML {
                selector_string: BindValue::new(String::new()),
                time_ms: BindValue::new(5000),
            })),
            PaletteKind::DismissPermission => Handler::Item(Step::Act(Action::DismissPermission)),
            PaletteKind::ExtractText => Handler::Item(Step::Extract(Extraction::Text {
                selector_string: BindValue::new(String::new()),
                field_name: BindValue::new(String::new()),
            })),
            PaletteKind::ExtractAttribute => Handler::Item(Step::Extract(Extraction::Attribute {
                selector_string: BindValue::new(String::new()),
                field_name: BindValue::new(String::new()),
                attr_str: BindValue::new(String::new()),
            })),
            PaletteKind::ExtractCount => Handler::Item(Step::Extract(Extraction::Count {
                selector_string: BindValue::new(String::new()),
                field_name: BindValue::new(String::new()),
            })),
            PaletteKind::ExtractExists => Handler::Item(Step::Extract(Extraction::Exists {
                selector_string: BindValue::new(String::new()),
                field_name: BindValue::new(String::new()),
            })),
            PaletteKind::Container => Handler::Container {
                selector: BindValue::new(String::new()),
                steps: Vec::new(),
                dedup: false,
            },
            PaletteKind::SubSequence => Handler::SubSequence(Sequence {
                sequence_name: String::new(),
                step_sequence: Vec::new(),
                target_data: HashMap::new(),
                metadata: Vec::new(),
            }),
            PaletteKind::NavigateHref => Handler::Item(Step::Act(Action::NavigateHref {
                base: BindValue::new(String::new()),
                href: BindValue::new(String::new()),
            })),

            PaletteKind::ScrollDown => Handler::Item(Step::Act(Action::ScrollDown {
                scroll: BindValue::new(500),
            })),

            PaletteKind::ScrollUp => Handler::Item(Step::Act(Action::ScrollUp {
                scroll: BindValue::new(500),
            })),
            PaletteKind::Wait => Handler::Item(Step::Act(Action::Wait {
                time_ms: BindValue::new(1000),
            })),
            PaletteKind::ExtractMultipleText => Handler::Item(Step::Extract(Extraction::MultipleText {
                selector_string: BindValue::new(String::new()),
                field_name: BindValue::new(String::new()),
            })), 
        }
    }
}

// =========================================================================
// Handler to UI metadata helpers
// =========================================================================

fn handler_color(handler: &Handler) -> (f32, f32, f32) {
    match handler {
        Handler::Item(Step::Act(action)) => action_color(action),
        Handler::Item(Step::Extract(_)) => (0.7, 0.5, 0.9),
        Handler::Container { .. } => (0.5, 0.8, 0.5),
        Handler::SubSequence(_) => (0.9, 0.5, 0.6),
    }
}

fn action_color(action: &Action) -> (f32, f32, f32) {
    match action {
        // Navigation - blue
        Action::Navigate { .. } | Action::NewTab { .. } | Action::Refresh |
        Action::Backward | Action::Forward => (0.3, 0.6, 0.9),
        // Click - sky blue
        Action::Click { .. } | Action::ClickByText { .. } => (0.4, 0.7, 1.0),
        // Wait - yellow
        Action::WaitFor { .. } | Action::ScrollAll => (0.9, 0.8, 0.4),
        // Input - cyan
        Action::Type { .. } | Action::ClearAndType { .. } => (0.4, 0.8, 0.8),
        // Frame - orange
        Action::SwitchToFrame { .. } | Action::SwitchToDefaultContent => (0.9, 0.6, 0.3),
        // Others
        _ => (0.5, 0.6, 0.7),
    }
}

fn handler_icon(handler: &Handler) -> &'static str {
    match handler {
        Handler::Item(Step::Act(action)) => action_icon(action),
        Handler::Item(Step::Extract(_)) => "📊",
        Handler::Container { .. } => "📦",
        Handler::SubSequence(_) => "🔄",
    }
}

fn action_icon(action: &Action) -> &'static str {
    match action {
        Action::Navigate { .. } => "🔗",
        Action::NewTab { .. } => "➕",
        Action::Refresh => "🔄",
        Action::Backward => "◀️",
        Action::Forward => "▶️",
        Action::Click { .. } => "👆",
        Action::ClickByText { .. } => "🔤",
        Action::WaitFor { .. } => "⏳",
        Action::ScrollAll => "📜",
        Action::Type { .. } => "⌨️",
        Action::ClearAndType { .. } => "🔁",
        Action::SwitchToFrame { .. } => "🖼️",
        Action::SwitchToDefaultContent => "🏠",
        _ => "⚡",
    }
}

fn handler_type_name(handler: &Handler) -> &'static str {
    match handler {
        Handler::Item(Step::Act(action)) => action_type_name(action),
        Handler::Item(Step::Extract(_)) => "Extract",
        Handler::Container { .. } => "Container",
        Handler::SubSequence(_) => "SubSeq",
    }
}

fn action_type_name(action: &Action) -> &'static str {
    match action {
        Action::Navigate { .. } => "Navigate",
        Action::NewTab { .. } => "NewTab",
        Action::Refresh => "Refresh",
        Action::Backward => "Back",
        Action::Forward => "Forward",
        Action::Click { .. } => "Click",
        Action::ClickByText { .. } => "ClickText",
        Action::WaitFor { .. } => "WaitFor",
        Action::ScrollAll => "ScrollAll",
        Action::Type { .. } => "Type",
        Action::ClearAndType { .. } => "Clear&Type",
        Action::SwitchToFrame { .. } => "Frame",
        Action::SwitchToDefaultContent => "DefFrame",
        Action::GetHTML { .. } => "GetHTML",
        Action::DismissPermission => "Dismiss",
        Action::CloseTab => "CloseTab",
        Action::SwitchTab { .. } => "SwitchTab",
        Action::SwitchToLastTab => "LastTab",
        Action::PressKey { .. } => "PressKey",
        Action::NavigateHref{ .. } => "Navigate Href",
        Action::ScrollDown{ .. } => "Scroll Down",
        Action::ScrollUp{ .. } => "Scroll Up",
        Action::Wait { .. } => "Wait",
    }
}

fn handler_description(handler: &Handler) -> String {
    match handler {
        Handler::Item(Step::Act(action)) => action_short_desc(action),
        Handler::Item(Step::Extract(ext)) => extraction_short_desc(ext),
        Handler::Container { steps, .. } => {
            format!("{} steps", steps.len())
        }
        Handler::SubSequence(seq) => {
            if seq.sequence_name.is_empty() {
                "SubSequence".to_string()
            } else {
                seq.sequence_name.clone()
            }
        }
    }
}

fn action_short_desc(action: &Action) -> String {
    match action {
        Action::Click { selector_string } => format!("Click: {}", bv_short(&selector_string)),
        Action::Navigate { url } => format!("Navigate: {}", bv_short(&url)),
        Action::WaitFor { selector_string, .. } => format!("WaitFor: {}", bv_short(&selector_string)),
        Action::Wait { time_ms } => format!("Wait: {}ms", time_ms.value),
        Action::ScrollAll => "ScrollAll".to_string(),
        Action::ClickByText { text, .. } => format!("ClickByText: {}", bv_short(&text)),
        Action::GetHTML { selector_string, .. } => format!("GetHTML: {}", bv_short(&selector_string)),
        Action::DismissPermission => "DismissPermission".to_string(),
        Action::SwitchToDefaultContent => "SwitchToDefault".to_string(),
        Action::SwitchToFrame { selector_string } => format!("SwitchFrame: {}", bv_short(&selector_string)),
        Action::Refresh => "Refresh".to_string(),
        Action::Forward => "Forward".to_string(),
        Action::Backward => "Backward".to_string(),
        Action::NewTab { url } => format!("NewTab: {}", bv_short(&url)),
        Action::SwitchTab { index } => format!("SwitchTab: {}", index.value),
        Action::CloseTab => "CloseTab".to_string(),
        Action::SwitchToLastTab => "SwitchToLastTab".to_string(),
        Action::Type { selector, .. } => format!("Type: {}", bv_short(&selector)),
        Action::PressKey { key } => format!("PressKey: {:?}", key),
        Action::ClearAndType { selector, .. } => format!("ClearAndType: {}", bv_short(&selector)),
        Action::NavigateHref { base, href } => format!("NavHref: {}+{}", bv_short(&base), bv_short(&href)),
        Action::ScrollDown { scroll } => format!("ScrollDown: {}", scroll.value),
        Action::ScrollUp { scroll } => format!("ScrollUp: {}", scroll.value),
    }
}

fn extraction_short_desc(ext: &Extraction) -> String {
    match ext {
        Extraction::Text { field_name, .. } => format!("Text → {}", bv_short(&field_name)),
        Extraction::Count { field_name, .. } => format!("Count → {}", bv_short(&field_name)),
        Extraction::Attribute { field_name, .. } => format!("Attr → {}", bv_short(&field_name)),
        Extraction::Exists { field_name, .. } => format!("Exists → {}", bv_short(&field_name)),
        Extraction::MultipleText { field_name, .. } => format!("MultiText → {}", bv_short(&field_name)),
    }
}

fn bv_short(bv: &BindValue<String>) -> String {
    if let Some(key) = &bv.binding {
        format!("${{{}}}", key)
    } else {
        let v = &bv.value;
        if v.len() > 30 {
            format!("{}...", &v[..27])
        } else {
            v.clone()
        }
    }
}

fn palette_kind_for(handler: &Handler) -> PaletteKind {
    match handler {
        Handler::Item(Step::Act(action)) => match action {
            Action::Navigate { .. } => PaletteKind::Navigate,
            Action::NewTab { .. } => PaletteKind::NewTab,
            Action::Refresh => PaletteKind::Refresh,
            Action::Backward => PaletteKind::Backward,
            Action::Forward => PaletteKind::Forward,
            Action::Click { .. } => PaletteKind::Click,
            Action::ClickByText { .. } => PaletteKind::ClickByText,
            Action::WaitFor { .. } => PaletteKind::WaitFor,
            Action::ScrollAll => PaletteKind::ScrollAll,
            Action::Type { .. } => PaletteKind::Type,
            Action::ClearAndType { .. } => PaletteKind::ClearAndType,
            Action::PressKey { .. } => PaletteKind::PressKey,
            Action::SwitchToFrame { .. } => PaletteKind::SwitchToFrame,
            Action::SwitchToDefaultContent => PaletteKind::SwitchToDefaultContent,
            Action::SwitchTab { .. } => PaletteKind::SwitchTab,
            Action::CloseTab => PaletteKind::CloseTab,
            Action::SwitchToLastTab => PaletteKind::SwitchToLastTab,
            Action::GetHTML { .. } => PaletteKind::GetHTML,
            Action::DismissPermission => PaletteKind::DismissPermission,
            Action::NavigateHref { .. } => PaletteKind::NavigateHref,
            Action::ScrollDown { .. } => PaletteKind::ScrollDown,
            Action::ScrollUp { .. } => PaletteKind::ScrollUp,
            Action::Wait { .. } => PaletteKind::Wait,
        },

        Handler::Item(Step::Extract(extract)) => match extract {
            Extraction::Text { .. } => PaletteKind::ExtractText,
            Extraction::Attribute { .. } => PaletteKind::ExtractAttribute,
            Extraction::Count { .. } => PaletteKind::ExtractCount,
            Extraction::Exists { .. } => PaletteKind::ExtractExists,
            Extraction::MultipleText { .. } => PaletteKind::ExtractMultipleText,
        },

        Handler::Container { .. } => PaletteKind::Container,
        Handler::SubSequence(_) => PaletteKind::SubSequence,
    }
}

// =========================================================================
// Sequence type (Main vs Sub)
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SequenceType {
    #[default]
    Main,
    Sub,
}

impl std::fmt::Display for SequenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SequenceType::Main => write!(f, "🏠 Main"),
            SequenceType::Sub => write!(f, "🔗 Sub"),
        }
    }
}

// =========================================================================
// Messages
// =========================================================================

#[derive(Debug, Clone)]
enum Message {
    // Drag and drop
    DragStartFromPalette(PaletteKind),
    DragStartFromWorkflow(usize),
    DragMove(Point),
    DragEnd,
    
    // Sequence management
    SequenceSelected(String),
    SequenceNameChanged(String),
    SequenceTypeChanged(SequenceType),
    SendToList,
    NewSequence,
    NewSubSequence,
    DeleteSequence,
    
    // File operations
    LoadFromJson,           // Clear and load
    MergeLoad,              // Partial load (keep current + add new)
    FileLoaded(Result<(String, String, bool), String>),  // (path, content, is_merge)
    SaveAllSequences,       // Save all
    SaveAs,
    ExportSequence,         // Export current sequence only (path required)
    FileSaved(Result<String, String>),
    
    // Save As
    SaveAsNameChanged(String),
    SaveAsTypeChanged(SequenceType),
    ConfirmSaveAs,
    CancelSaveAs,
    
    // Editor
    Editor(EditorMsg),
    EditHandler(usize),
    
    // Browser execution
    BrowserTypeChanged(BrowserType),
    OpenBrowser,
    BrowserOpened(Result<String, String>),
    CloseBrowser,
    BrowserClosed(Result<String, String>),
    
    // Sequence execution
    StartUrlChanged(String),
    RunSequence,
    SequenceFinished(Result<String, String>),
    StopSequence,
    
    // Scroll
    WorkflowScrolled(Viewport),
    PaletteScrolled(Viewport),
    
    // Log
    ClearLogs,
    
    // Test input
    TestInput(TestInputMsg),
    
    // App exit
    RequestExit,
    ExitCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrowserType {
    #[default]
    Desktop,
    Mobile,
}

impl std::fmt::Display for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserType::Desktop => write!(f, "Desktop"),
            BrowserType::Mobile => write!(f, "Mobile"),
        }
    }
}

// =========================================================================
// Drag state
// =========================================================================

#[derive(Debug, Clone)]
enum DragSource {
    Palette(PaletteKind),
    Workflow(usize),
}

struct DragState {
    source: DragSource,
    palette_kind: PaletteKind,
    cursor: Point,
    start_pos: Point,
}

// =========================================================================
// App
// =========================================================================

struct App {
    /// Current workflow being edited - direct Handler usage
    workflow: Vec<Handler>,
    current_sequence_name: Option<String>,
    current_sequence_type: SequenceType,
    
    /// Main sequence list - direct Handler storage
    main_sequences: HashMap<String, Vec<Handler>>,
    main_sequence_names: Vec<String>,
    
    /// Sub sequence list - direct Handler storage
    sub_sequences: HashMap<String, Vec<Handler>>,
    sub_sequence_names: Vec<String>,
    
    dragging: Option<DragState>,
    
    /// v2 editor - direct EditorState usage
    editor: EditorState,
    editing_index: Option<usize>,
    
    show_save_as: bool,
    save_as_name: String,
    save_as_type: SequenceType,
    
    current_file_path: Option<String>,
    
    browser_type: BrowserType,
    browser_running: bool,
    browser_status: String,
    
    start_url: String,
    sequence_running: bool,
    last_result: Option<Vec<HashMap<String, String>>>,
    
    logs: Vec<String>,
    
    /// Test input state
    test_inputs: TestInputState,
    
    // ── Scroll state ──
    workflow_scroll: Id,
    workflow_scroll_offset: AbsoluteOffset,
    palette_scroll: Id,
    palette_scroll_offset: AbsoluteOffset,
}

const BROWSER_RUNNER_PORT: u16 = 19876;

/// Start browser_runner if not running
fn start_browser_runner() -> Result<(), String> {
    let port = BROWSER_RUNNER_PORT;
    
    // Check if already running
    if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
        return Ok(());
    }
    
    let exe_path = std::env::current_exe()
        .map(|p| p.parent().unwrap().join("browser_runner"))
        .unwrap_or_else(|_| std::path::PathBuf::from("./browser_runner"));
    
    // Pass current process PID (terminates with parent)
    let my_pid = std::process::id();
    
    std::process::Command::new(&exe_path)
        .arg(port.to_string())
        .arg(my_pid.to_string())  // Pass PID
        .spawn()
        .map_err(|e| format!("Failed to start browser_runner: {}", e))?;
    
    // Wait for connection (max 15 seconds)
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return Ok(());
        }
    }
    
    Err("browser_runner startup timeout".to_string())
}

/// Send shutdown command to browser_runner
fn shutdown_browser_runner() {
    use std::io::Write;
    use std::net::TcpStream;
    use std::time::Duration;
    
    // Quick timeout connection attempt
    if let Ok(mut stream) = TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", BROWSER_RUNNER_PORT).parse().unwrap(),
        Duration::from_millis(500)
    ) {
        stream.set_write_timeout(Some(Duration::from_millis(500))).ok();
        let _ = writeln!(stream, r#"{{"cmd":"shutdown"}}"#);
    }
}

// Drop impl - cleanup browser_runner on exit
impl Drop for App {
    fn drop(&mut self) {
        shutdown_browser_runner();
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            workflow: Vec::new(),
            current_sequence_name: None,
            current_sequence_type: SequenceType::Main,
            main_sequences: HashMap::new(),
            main_sequence_names: Vec::new(),
            sub_sequences: HashMap::new(),
            sub_sequence_names: Vec::new(),
            dragging: None,
            editor: EditorState::default(),
            editing_index: None,
            show_save_as: false,
            save_as_name: String::new(),
            save_as_type: SequenceType::Main,
            current_file_path: None,
            browser_type: BrowserType::Desktop,
            browser_running: false,
            browser_status: String::new(),
            start_url: "https://www.google.com".to_string(),
            sequence_running: false,
            last_result: None,
            logs: Vec::new(),
            test_inputs: TestInputState::new(),
            workflow_scroll: Id::unique(),
            workflow_scroll_offset: AbsoluteOffset { x: 0.0, y: 0.0 },
            palette_scroll: Id::unique(),
            palette_scroll_offset: AbsoluteOffset { x: 0.0, y: 0.0 },
        }
    }
}


/// Build Sequence from current workflow
fn build_sequence(name: &str, handlers: &[Handler]) -> Sequence {
    Sequence {
        sequence_name: name.to_string(),
        step_sequence: handlers.to_vec(),
        target_data: HashMap::new(),
        metadata: Vec::new(),
    }
}

/// Add numbering on name conflict (e.g.: "seq" → "seq_1" → "seq_2")
fn unique_name(base_name: &str, existing_names: &[String]) -> String {
    if !existing_names.contains(&base_name.to_string()) {
        return base_name.to_string();
    }
    
    let mut counter = 1;
    loop {
        let new_name = format!("{}_{}", base_name, counter);
        if !existing_names.contains(&new_name) {
            return new_name;
        }
        counter += 1;
        if counter > 100 { break; } // Prevent infinite loop
    }
    format!("{}_{}", base_name, counter)
}



/// Find Container before given index in workflow
fn find_previous_container(workflow: &[Handler], before_index: usize) -> Option<usize> {
    for i in (0..before_index).rev() {
        if matches!(workflow.get(i), Some(Handler::Container { .. })) {
            return Some(i);
        }
    }
    None
}

/// Extract output fields from previous Container
fn find_previous_container_fields(
    workflow: &[Handler], 
    before_index: usize
) -> Vec<collector::handler_editor::ContainerField> {
    use collector::handler_editor::ContainerField;
    
    let mut fields = Vec::new();
    
    // _index is always available
    fields.push(ContainerField {
        field_name: "_index".to_string(),
        extraction_type: "Auto".to_string(),
    });
    
    if let Some(container_idx) = find_previous_container(workflow, before_index) {
        if let Some(Handler::Container { steps, .. }) = workflow.get(container_idx) {
            for step in steps {
                if let Step::Extract(ext) = step {
                    let (field_name, ext_type) = match ext {
                        Extraction::Text { field_name, .. } => {
                            (field_name.value.clone(), "Text".to_string())
                        }
                        Extraction::Attribute { field_name, attr_str, .. } => {
                            (field_name.value.clone(), format!("Attr:{}", attr_str.value))
                        }
                        Extraction::Count { field_name, .. } => {
                            (field_name.value.clone(), "Count".to_string())
                        }
                        Extraction::Exists { field_name, .. } => {
                            (field_name.value.clone(), "Exists".to_string())
                        }
                        Extraction::MultipleText { field_name, .. } => {
                            (field_name.value.clone(), "MultiText".to_string())
                        }
                    };
                    
                    if !field_name.is_empty() {
                        fields.push(ContainerField {
                            field_name,
                            extraction_type: ext_type,
                        });
                    }
                }
            }
        }
    }
    
    fields
}

impl App {
    // =====================================================================
    // Scroll position calculation
    // =====================================================================
    fn calc_insert_index(&self, cursor_y: f32) -> usize {
        if self.workflow.is_empty() {
            return 0;
        }

        let content_start_y = TOOLBAR_HEIGHT + OUTER_PADDING + HEADER_HEIGHT + CONTENT_PADDING;
        let relative_y = cursor_y - content_start_y + self.workflow_scroll_offset.y;

        if relative_y < 0.0 {
            return 0;
        }

        let card_total_height = CARD_HEIGHT + CARD_SPACING;
        
        for i in 0..self.workflow.len() {
            let card_top = i as f32 * card_total_height;
            let card_middle = card_top + CARD_HEIGHT / 2.0;
            
            if relative_y < card_middle {
                return i;
            }
        }

        self.workflow.len()
    }
    
    fn is_drop_in_delete_zone(&self, cursor: Point) -> bool {
        cursor.x < PALETTE_WIDTH + OUTER_PADDING
    }

    // =====================================================================
    // update
    // =====================================================================
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
                Message::SaveAs => {
                
                if let Some(name) = &self.current_sequence_name {
                    match self.current_sequence_type {
                        SequenceType::Main => {
                            self.main_sequences.insert(name.clone(), self.workflow.clone());
                        }
                        SequenceType::Sub => {
                            self.sub_sequences.insert(name.clone(), self.workflow.clone());
                        }
                    }
                }

                let mut file = collector::SequenceFile::empty();

                for (name, handlers) in &self.main_sequences {
                    let seq = build_sequence(name, handlers);
                    file.insert_main(seq);
                }
                
                for (name, handlers) in &self.sub_sequences {
                    let seq = build_sequence(name, handlers);
                    file.insert_sub(seq);
                }

                let json_str = file.to_json();

                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_title("Save As...") // 타이틀 변경
                            .set_file_name("sequences_copy.json") // 기본 이름 제안
                            .save_file()
                            .await;
                        
                        if let Some(handle) = file {
                            let path = handle.path().to_string_lossy().to_string();
                            match std::fs::write(handle.path(), &json_str) {
                                Ok(_) => Ok(path),
                                Err(e) => Err(format!("Save failed: {}", e)),
                            }
                        } else {
                            Err("Cancelled".to_string())
                        }
                    },
                    Message::FileSaved 
                );
            }
            Message::DragStartFromPalette(kind) => {
                self.dragging = Some(DragState {
                    source: DragSource::Palette(kind.clone()),
                    palette_kind: kind,
                    cursor: Point::ORIGIN,
                    start_pos: Point::ORIGIN,
                });
            }
            Message::DragStartFromWorkflow(index) => {
                if let Some(handler) = self.workflow.get(index) {
                    let pk = palette_kind_for(handler);
                    self.dragging = Some(DragState {
                        source: DragSource::Workflow(index),
                        palette_kind: pk,
                        cursor: Point::ORIGIN,
                        start_pos: Point::ORIGIN,
                    });
                }
            }
            Message::DragMove(position) => {
                if let Some(ref mut drag) = self.dragging {
                    if drag.start_pos == Point::ORIGIN {
                        drag.start_pos = position;
                    }
                    drag.cursor = position;
                }
            }
            Message::DragEnd => {
                if let Some(drag) = self.dragging.take() {
                    let dx = drag.cursor.x - drag.start_pos.x;
                    let dy = drag.cursor.y - drag.start_pos.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    if let DragSource::Workflow(from_index) = drag.source {
                        if distance < 5.0 {
                            return self.scroll_restore_task();
                        }
                        if self.is_drop_in_delete_zone(drag.cursor) {
                            self.workflow.remove(from_index);
                            self.refresh_test_inputs();
                            return self.scroll_restore_task();
                        }
                    }                  
                    if !self.is_drop_in_delete_zone(drag.cursor) {
                        let insert_index = self.calc_insert_index(drag.cursor.y);
                        
                        match drag.source {
                            DragSource::Palette(kind) => {
                                // SubSequence requires Container and must be placed below it
                                if kind == PaletteKind::SubSequence {
                                    if !self.has_any_container() {
                                        self.logs.push("⚠️ SubSequence requires a Container to be present".to_string());
                                        return self.scroll_restore_task();
                                    }
                                    if !self.is_below_first_container(insert_index) {
                                        self.logs.push("⚠️ SubSequence can only be placed after a Container".to_string());
                                        return self.scroll_restore_task();
                                    }
                                }
                                let new_handler = kind.default_handler();
                                self.workflow.insert(insert_index, new_handler);
                            }
                            DragSource::Workflow(from_index) => {
                                if from_index != insert_index {
                                    let is_subseq = matches!(&self.workflow[from_index], Handler::SubSequence(_));
                                    let target = if from_index < insert_index {
                                        insert_index - 1
                                    } else {
                                        insert_index
                                    };
                                    
                                    // SubSequence cannot be moved above Container
                                    if is_subseq && !self.is_below_first_container(target) {
                                        self.logs.push("⚠️ SubSequence cannot be moved above a Container".to_string());
                                        return self.scroll_restore_task();
                                    }
                                    
                                    let item = self.workflow.remove(from_index);
                                    self.workflow.insert(target.min(self.workflow.len()), item);
                                }
                            }
                        }
                        self.refresh_test_inputs();
                    }
                }
            }
            
            Message::SequenceSelected(name) => {
                // Search in main sequences
                if let Some(handlers) = self.main_sequences.get(&name) {
                    self.workflow = handlers.clone();
                    self.current_sequence_name = Some(name);
                    self.current_sequence_type = SequenceType::Main;
                    self.editor.close();
                    self.editing_index = None;
                    self.refresh_test_inputs();
                } 
                // Search in sub sequences
                else if let Some(handlers) = self.sub_sequences.get(&name) {
                    self.workflow = handlers.clone();
                    self.current_sequence_name = Some(name);
                    self.current_sequence_type = SequenceType::Sub;
                    self.editor.close();
                    self.editing_index = None;
                    self.refresh_test_inputs();
                }
            }
            
            Message::SequenceNameChanged(new_name) => {
                if let Some(old_name) = self.current_sequence_name.clone() {
                    match self.current_sequence_type {
                        SequenceType::Main => {
                            if let Some(handlers) = self.main_sequences.remove(&old_name) {
                                self.main_sequences.insert(new_name.clone(), handlers);
                            }
                            if let Some(pos) = self.main_sequence_names.iter().position(|n| n == &old_name) {
                                self.main_sequence_names[pos] = new_name.clone();
                            }
                        }
                        SequenceType::Sub => {
                            if let Some(handlers) = self.sub_sequences.remove(&old_name) {
                                self.sub_sequences.insert(new_name.clone(), handlers);
                            }
                            if let Some(pos) = self.sub_sequence_names.iter().position(|n| n == &old_name) {
                                self.sub_sequence_names[pos] = new_name.clone();
                            }
                        }
                    }
                }
                self.current_sequence_name = Some(new_name);
            }
            
            Message::SequenceTypeChanged(new_type) => {
                if self.current_sequence_type == new_type {
                    return Task::none();
                }

                if let Some(name) = self.current_sequence_name.clone() {
                    let handlers = self.workflow.clone();
                    
        
                    let mut moved = false;
                    match self.current_sequence_type {
                        SequenceType::Main => {
                            if self.main_sequences.contains_key(&name) {
                                self.main_sequences.remove(&name);
                                self.main_sequence_names.retain(|n| n != &name);
                                moved = true;
                            }
                        }
                        SequenceType::Sub => {
                            if self.sub_sequences.contains_key(&name) {
                                self.sub_sequences.remove(&name);
                                self.sub_sequence_names.retain(|n| n != &name);
                                moved = true;
                            }
                        }
                    }
                    if moved {
                        let new_name = match new_type {
                            SequenceType::Main => unique_name(&name, &self.main_sequence_names),
                            SequenceType::Sub => unique_name(&name, &self.sub_sequence_names),
                        };
                        match new_type {
                            SequenceType::Main => {
                                self.main_sequence_names.push(new_name.clone());
                                self.main_sequences.insert(new_name.clone(), handlers);
                            }
                            SequenceType::Sub => {
                                self.sub_sequence_names.push(new_name.clone());
                                self.sub_sequences.insert(new_name.clone(), handlers);
                            }
                        }
                        if name != new_name {
                            self.logs.push(format!("📋 Renamed (collision): '{}' → '{}'", name, new_name));
                            self.current_sequence_name = Some(new_name);
                        }
                        self.logs.push(format!("📋 Moved: '{}' → {:?}", self.current_sequence_name.as_ref().unwrap(), new_type));
                    }
                }
                self.current_sequence_type = new_type;
            }
            
            Message::NewSequence => {
                self.workflow.clear();
                self.current_sequence_name = None;
                self.current_sequence_type = SequenceType::Main;
                self.editor.close();
                self.editing_index = None;
                self.refresh_test_inputs();
            }
            
            Message::NewSubSequence => {
                self.workflow.clear();
                self.current_sequence_name = None;
                self.current_sequence_type = SequenceType::Sub;
                self.editor.close();
                self.editing_index = None;
                self.refresh_test_inputs();
            }
            
            Message::LoadFromJson => {
                // Clear and load (is_merge = false)
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_title("Load Workflow (clear existing)")
                            .pick_file()
                            .await;
                        
                        if let Some(handle) = file {
                            let path = handle.path().to_string_lossy().to_string();
                            match std::fs::read_to_string(handle.path()) {
                                Ok(content) => Ok((path, content, false)), // is_merge = false
                                Err(e) => Err(format!("Failed to read file: {}", e)),
                            }
                        } else {
                            Err("Cancelled".to_string())
                        }
                    },
                    Message::FileLoaded
                );
            }
            
            Message::MergeLoad => {
                // Partial load - keep current and add (is_merge = true)
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_title("Merge Sequences (keep current)")
                            .pick_file()
                            .await;
                        
                        if let Some(handle) = file {
                            let path = handle.path().to_string_lossy().to_string();
                            match std::fs::read_to_string(handle.path()) {
                                Ok(content) => Ok((path, content, true)), // is_merge = true
                                Err(e) => Err(format!("Failed to read file: {}", e)),
                            }
                        } else {
                            Err("Cancelled".to_string())
                        }
                    },
                    Message::FileLoaded
                );
            }
            
            Message::FileLoaded(result) => {
                match result {
                        Ok((path, content, is_merge)) => {
                            match collector::SequenceFile::new(&content) {
                                Ok(file) => {
                                    let mut added_main = 0usize;
                                    let mut added_sub = 0usize;
                                    if !is_merge {
                                        self.main_sequences.clear();
                                        self.main_sequence_names.clear();
                                        self.sub_sequences.clear();
                                        self.sub_sequence_names.clear();
                                        self.workflow.clear();
                                        self.current_sequence_name = None;
                                    }
                                    for (name, seq) in file.main_sequences {
                                        let final_name = if is_merge {
                                            unique_name(&name, &self.main_sequence_names)
                                        } else {
                                            name
                                        };
                                        
                                        if !self.main_sequence_names.contains(&final_name) {
                                            self.main_sequence_names.push(final_name.clone());
                                            added_main += 1;
                                        }
                                        self.main_sequences.insert(final_name, seq.step_sequence);
                                    }
                                    for (name, seq) in file.sub_sequences {
                                        let final_name = if is_merge {
                                            unique_name(&name, &self.sub_sequence_names)
                                        } else {
                                            name
                                        };

                                        if !self.sub_sequence_names.contains(&final_name) {
                                            self.sub_sequence_names.push(final_name.clone());
                                            added_sub += 1;
                                        }
                                        self.sub_sequences.insert(final_name, seq.step_sequence);
                                    }
                                    if !is_merge && self.current_sequence_name.is_none() {
                                        if let Some(first) = self.main_sequence_names.first().cloned() {
                                            if let Some(handlers) = self.main_sequences.get(&first) {
                                                self.workflow = handlers.clone();
                                                self.current_sequence_name = Some(first);
                                                self.current_sequence_type = SequenceType::Main;
                                            }
                                        }
                                    }
                                    
                                    if !is_merge {
                                        self.current_file_path = Some(path.clone());
                                    }
                                    let action = if is_merge { "Merge" } else { "Load" };
                                    self.logs.push(format!(
                                        "✅ {} complete: {} (main +{}, sub +{})", 
                                        action, path, added_main, added_sub
                                    ));
                                    self.refresh_test_inputs();
                                }
                                Err(e) => {
                                    self.logs.push(format!("❌ JSON parse error (Not a valid SequenceFile): {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            if e != "Cancelled" {
                                self.logs.push(format!("❌ {}", e));
                            }
                        }
                    }
            }
            
            Message::ExportSequence => {
                let name = self.current_sequence_name.clone()
                    .unwrap_or_else(|| "untitled".to_string());
                if self.workflow.is_empty() {
                    self.logs.push("❌ No sequence to export".to_string());
                    return self.scroll_restore_task();
                }
                let seq = build_sequence(&name, &self.workflow);
                let mut file = collector::SequenceFile::empty();
                match self.current_sequence_type {
                    SequenceType::Main => file.insert_main(seq),
                    SequenceType::Sub => file.insert_sub(seq),
                }
                let json_str = file.to_json();
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_title("Export Sequence")
                            .set_file_name(&format!("{}.json", name))
                            .save_file()
                            .await;
                        
                        if let Some(handle) = file {
                            let path = handle.path().to_string_lossy().to_string();
                            match std::fs::write(handle.path(), &json_str) {
                                Ok(_) => Ok(path),
                                Err(e) => Err(format!("Save failed: {}", e)),
                            }
                        } else {
                            Err("Cancelled".to_string())
                        }
                    },
                    Message::FileSaved
                );
            }
            
            Message::SaveAllSequences => {
                // Save all - use existing path if available
                if let Some(name) = &self.current_sequence_name {
                    match self.current_sequence_type {
                        SequenceType::Main => {
                            self.main_sequences.insert(name.clone(), self.workflow.clone());
                        }
                        SequenceType::Sub => {
                            self.sub_sequences.insert(name.clone(), self.workflow.clone());
                        }
                    }
                }
                let mut file = collector::SequenceFile::empty();
                for (name, handlers) in &self.main_sequences {
                    let seq = build_sequence(name, handlers);
                    file.main_sequences.insert(name.clone(), seq);
                }
                for (name, handlers) in &self.sub_sequences {
                    let seq = build_sequence(name, handlers);
                    file.sub_sequences.insert(name.clone(), seq);
                }
                let json_str = file.to_json();
                if let Some(path) = self.current_file_path.as_ref() {
                    let path = path.clone();
                    return Task::perform(
                        async move {
                            match std::fs::write(&path, &json_str) {
                                Ok(_) => Ok(path),
                                Err(e) => Err(format!("Save failed: {}", e)),
                            }
                        },
                        Message::FileSaved
                    );
                }
                // Otherwise show dialog
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_title("Save All Sequences")
                            .set_file_name("sequences.json")
                            .save_file()
                            .await;
                        
                        if let Some(handle) = file {
                            let path = handle.path().to_string_lossy().to_string();
                            match std::fs::write(handle.path(), &json_str) {
                                Ok(_) => Ok(path),
                                Err(e) => Err(format!("Save failed: {}", e)),
                            }
                        } else {
                            Err("Cancelled".to_string())
                        }
                    },
                    Message::FileSaved
                );
            }
            
            Message::FileSaved(result) => {
                match result {
                    Ok(path) => {
                        self.current_file_path = Some(path.clone());
                        self.logs.push(format!("✅ Saved: {}", path));
                    }
                    Err(e) => {
                        if e != "Cancelled" {
                            self.logs.push(format!("❌ {}", e));
                        }
                    }
                }
            }
            
            Message::SaveAsNameChanged(name) => {
                self.save_as_name = name;
            }
            
            Message::SaveAsTypeChanged(t) => {
                self.save_as_type = t;
            }
            
            Message::ConfirmSaveAs => {
                if !self.save_as_name.is_empty() {
                    let name = self.save_as_name.clone();
                    
                    match self.save_as_type {
                        SequenceType::Main => {
                            if !self.main_sequence_names.contains(&name) {
                                self.main_sequence_names.push(name.clone());
                            }
                            self.main_sequences.insert(name.clone(), self.workflow.clone());
                        }
                        SequenceType::Sub => {
                            if !self.sub_sequence_names.contains(&name) {
                                self.sub_sequence_names.push(name.clone());
                            }
                            self.sub_sequences.insert(name.clone(), self.workflow.clone());
                        }
                    }
                    
                    self.current_sequence_name = Some(name.clone());
                    self.current_sequence_type = self.save_as_type;
                    self.show_save_as = false;
                    self.logs.push(format!("💾 {:?} saved as: {}", self.save_as_type, name));
                }
            }
            
            Message::CancelSaveAs => {
                self.show_save_as = false;
            }
            
            Message::SendToList => {
                self.show_save_as = true;
                self.save_as_type = self.current_sequence_type;
                
                // Use current name if exists, otherwise generate default name
                if let Some(name) = &self.current_sequence_name {
                    self.save_as_name = name.clone();
                } else {
                    let count = match self.current_sequence_type {
                        SequenceType::Main => self.main_sequence_names.len(),
                        SequenceType::Sub => self.sub_sequence_names.len(),
                    };
                    let prefix = match self.current_sequence_type {
                        SequenceType::Main => "main",
                        SequenceType::Sub => "sub",
                    };
                    self.save_as_name = format!("{}_{}", prefix, count + 1);
                }
            }
            
            Message::DeleteSequence => {
                if let Some(name) = self.current_sequence_name.clone() {
                    match self.current_sequence_type {
                        SequenceType::Main => {
                            self.main_sequences.remove(&name);
                            self.main_sequence_names.retain(|n| n != &name);
                        }
                        SequenceType::Sub => {
                            self.sub_sequences.remove(&name);
                            self.sub_sequence_names.retain(|n| n != &name);
                        }
                    }
                    self.workflow.clear();
                    self.current_sequence_name = None;
                    self.logs.push(format!("🗑️ Deleted: {}", name));
                }
            }
            
            Message::Editor(editor_msg) => {
                match editor_msg.clone() {
                    EditorMsg::Close => {
                        self.editor.close();
                        self.editing_index = None;
                    }
                    EditorMsg::Save => {
                        // Apply editor's handler to workflow
                        self.editor.apply_mappings_to_handler();
                        if let Some(idx) = self.editing_index {
                            if idx < self.workflow.len() {
                                self.workflow[idx] = self.editor.handler.clone();
                            }
                        }
                        self.editor.close();
                        self.editing_index = None;
                        self.refresh_test_inputs();
                    }
                    EditorMsg::SubSeqSelectFromList(name) => {
                        // Copy handlers when selecting a sub-sequence
                        if let Some(handlers) = self.sub_sequences.get(&name) {
                            if let Handler::SubSequence(ref mut seq) = self.editor.handler {
                                seq.sequence_name = name.clone();
                                seq.step_sequence = handlers.clone();
                                // Re-parse binding keys
                                self.editor.subsequence_binding_keys = extract_binding_keys_from_sequence(seq);
                            }
                        }
                        self.editor.update(editor_msg);
                    }
                    EditorMsg::Scrolled(_) => {
                        // Scroll events are handled directly by editor
                        self.editor.update(editor_msg);
                    }
                    _ => {
                        self.editor.update(editor_msg);
                    }
                }
            }
            
            Message::EditHandler(index) => {
                if let Some(handler) = self.workflow.get(index) {
                    // Pass available sub-sequences to editor
                    self.editor.available_sequences = self.sub_sequence_names.clone();
                    
                    // For SubSequence, pass container fields and binding keys
                    if let Handler::SubSequence(seq) = handler {
                        let container_fields = find_previous_container_fields(&self.workflow, index);
                        self.editor.set_container_context(container_fields);
                        // 바인딩 키 파싱
                        self.editor.subsequence_binding_keys = extract_binding_keys_from_sequence(seq);
                    }
                    
                    self.editor.open(handler.clone());
                    self.editing_index = Some(index);
                }
            }
            
            // ── Browser control ──
            Message::BrowserTypeChanged(browser_type) => {
                if !self.browser_running {
                    self.browser_type = browser_type;
                }
            }
            
            Message::OpenBrowser => {
                if self.browser_running {
                    return self.scroll_restore_task();
                }
                
                self.browser_status = "🚀 Starting browser_runner...".to_string();
                
                let browser_type = self.browser_type;
                
                return Task::perform(
                    async move {
                        // Start browser_runner if not running
                        start_browser_runner()?;
                        
                        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{}", BROWSER_RUNNER_PORT))
                            .map_err(|e| format!("TCP connection failed: {}", e))?;
                        
                        stream.set_read_timeout(Some(std::time::Duration::from_secs(60))).ok();
                        
                        let device = match browser_type {
                            BrowserType::Desktop => "desktop",
                            BrowserType::Mobile => "mobile",
                        };
                        let cmd = serde_json::json!({
                            "cmd": "open",
                            "device": device,
                            "headless": false
                        });
                        
                        use std::io::{BufRead, BufReader, Write};
                        writeln!(stream, "{}", cmd.to_string())
                            .map_err(|e| format!("Command send failed: {}", e))?;
                        
                        let mut reader = BufReader::new(&stream);
                        let mut response = String::new();
                        reader.read_line(&mut response)
                            .map_err(|e| format!("Response read failed: {}", e))?;
                        
                        let resp: serde_json::Value = serde_json::from_str(&response)
                            .map_err(|e| format!("Response parse failed: {}", e))?;
                        
                        if resp["status"] == "ok" {
                            Ok(response)
                        } else {
                            Err(resp["message"].as_str().unwrap_or("Unknown error").to_string())
                        }
                    },
                    Message::BrowserOpened
                );
            }
            
            Message::BrowserOpened(result) => {
                match result {
                    Ok(response_json) => {
                        if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&response_json) {
                            if let Some(logs) = resp["logs"].as_array() {
                                for log in logs {
                                    if let Some(s) = log.as_str() {
                                        self.logs.push(s.to_string());
                                    }
                                }
                            }
                            let msg = resp["message"].as_str().unwrap_or("Browser opened");
                            self.browser_running = true;
                            self.browser_status = format!("✅ {}", msg);
                        } else {
                            self.browser_running = true;
                            self.browser_status = format!("✅ {}", response_json);
                            self.logs.push(format!("✅ {}", response_json));
                        }
                    }
                    Err(e) => {
                        self.browser_running = false;
                        self.browser_status = format!("❌ {}", e);
                        self.logs.push(format!("❌ Failed to open browser: {}", e));
                    }
                }
            }
            
            Message::RunSequence => {
                if self.sequence_running {
                    return self.scroll_restore_task();
                }
                
                if self.workflow.is_empty() {
                    self.browser_status = "⚠️ No sequence to run".to_string();
                    return self.scroll_restore_task();
                }
                
                if self.start_url.is_empty() {
                    self.browser_status = "⚠️ Please enter a start URL".to_string();
                    return self.scroll_restore_task();
                }
                
                self.sequence_running = true;
                self.browser_status = "▶️ Running sequence...".to_string();
                
                let name = self.current_sequence_name.clone()
                    .unwrap_or_else(|| "untitled".to_string());
                self.logs.push(format!("🚀 Starting sequence '{}'", name));
                self.logs.push(format!("   📍 URL: {}", self.start_url));
                self.logs.push(format!("   📱 Device: {:?}", self.browser_type));
                
                let sequence = build_sequence(&name, &self.workflow);
                let sequence_json = serde_json::to_string(&sequence).unwrap_or_default();
                
                let start_url = self.start_url.clone();
                let browser_type = self.browser_type;
                let test_inputs = self.test_inputs.test_values.clone();
                
                return Task::perform(
                    async move {
                        // Start browser_runner if not running
                        start_browser_runner()?;
                        
                        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{}", BROWSER_RUNNER_PORT))
                            .map_err(|e| format!("TCP connection failed: {}", e))?;
                        
                        stream.set_read_timeout(Some(std::time::Duration::from_secs(300))).ok();
                        
                        let device = match browser_type {
                            BrowserType::Desktop => "desktop",
                            BrowserType::Mobile => "mobile",
                        };
                        let cmd = serde_json::json!({
                            "cmd": "run_sequence",
                            "sequence_json": sequence_json,
                            "start_url": start_url,
                            "device": device,
                            "headless": false,
                            "inputs": test_inputs
                        });
                        
                        use std::io::{BufRead, BufReader, Write};
                        writeln!(stream, "{}", cmd.to_string())
                            .map_err(|e| format!("Command send failed: {}", e))?;
                        
                        let mut reader = BufReader::new(&stream);
                        let mut response = String::new();
                        reader.read_line(&mut response)
                            .map_err(|e| format!("Response read failed: {}", e))?;
                        
                        let resp: serde_json::Value = serde_json::from_str(&response)
                            .map_err(|e| format!("Response parse failed: {}", e))?;
                        
                        if resp["status"] == "ok" {
                            Ok(response)
                        } else {
                            Err(resp["message"].as_str().unwrap_or("Unknown error").to_string())
                        }
                    },
                    Message::SequenceFinished
                );
            }
            
            Message::SequenceFinished(result) => {
                self.sequence_running = false;
                match result {
                    Ok(response_json) => {
                        if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&response_json) {
                            if let Some(logs) = resp["logs"].as_array() {
                                for log in logs {
                                    if let Some(s) = log.as_str() {
                                        self.logs.push(s.to_string());
                                    }
                                }
                            }
                            
                            let msg = resp["message"].as_str().unwrap_or("Execution complete");
                            self.browser_status = format!("✅ {}", msg);
                            
                            if let Some(data) = resp["data"].as_array() {
                                self.logs.push(format!("\n📊 Results ({} 건):", data.len()));
                                self.logs.push("─".repeat(60));
                                
                                let mut result_vec = Vec::new();
                                for (i, item) in data.iter().enumerate() {
                                    if let Some(obj) = item.as_object() {
                                        let mut map = HashMap::new();
                                        self.logs.push(format!("[{}]", i + 1));
                                        for (key, value) in obj {
                                            let val_str = value.as_str().unwrap_or("").to_string();
                                            let display_str: String = if val_str.chars().count() > 60 {
                                                format!("{}...", val_str.chars().take(60).collect::<String>())
                                            } else {
                                                val_str.clone()
                                            };
                                            self.logs.push(format!("  {}: {}", key, display_str));
                                            map.insert(key.clone(), val_str);
                                        }
                                        result_vec.push(map);
                                    }
                                }
                                self.logs.push("─".repeat(60));
                                self.last_result = Some(result_vec);
                            }
                        } else {
                            self.browser_status = format!("✅ {}", response_json);
                            self.logs.push(format!("✅ {}", response_json));
                        }
                    }
                    Err(e) => {
                        self.browser_status = format!("❌ {}", e);
                        self.logs.push(format!("❌ Sequence execution failed: {}", e));
                        self.last_result = None;
                    }
                }
            }
            
            Message::StartUrlChanged(url) => {
                self.start_url = url;
            }
            
            Message::StopSequence => {
                if !self.sequence_running {
                    return self.scroll_restore_task();
                }
                
                self.browser_status = "⏹️ Stopping sequence...".to_string();
                
                let port = BROWSER_RUNNER_PORT;
                
                return Task::perform(
                    async move {
                        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{}", port))
                            .map_err(|e| format!("TCP connection failed: {}", e))?;
                        
                        stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).ok();
                        
                        let cmd = serde_json::json!({ "cmd": "stop" });
                        
                        use std::io::{BufRead, BufReader, Write};
                        writeln!(stream, "{}", cmd.to_string())
                            .map_err(|e| format!("Command send failed: {}", e))?;
                        
                        let mut reader = BufReader::new(&stream);
                        let mut response = String::new();
                        reader.read_line(&mut response).ok();
                        
                        Ok::<_, String>("Sequence stopped".to_string())
                    },
                    |result: Result<String, String>| {
                        Message::SequenceFinished(result.map(|s| format!("⏹️ {}", s)))
                    }
                );
            }
            
            Message::CloseBrowser => {
                if !self.browser_running {
                    return self.scroll_restore_task();
                }
                
                self.browser_status = "⏹️ Closing browser...".to_string();
                
                let port = BROWSER_RUNNER_PORT;
                
                return Task::perform(
                    async move {
                        let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{}", port))
                            .map_err(|e| format!("TCP connection failed: {}", e))?;
                        
                        stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).ok();
                        
                        let cmd = serde_json::json!({ "cmd": "close" });
                        
                        use std::io::{BufRead, BufReader, Write};
                        writeln!(stream, "{}", cmd.to_string())
                            .map_err(|e| format!("Command send failed: {}", e))?;
                        
                        let mut reader = BufReader::new(&stream);
                        let mut response = String::new();
                        reader.read_line(&mut response).ok();
                        
                        Ok::<_, String>(response)
                    },
                    |result: Result<String, String>| {
                        match result {
                            Ok(resp) => Message::BrowserClosed(Ok(resp)),
                            Err(e) => Message::BrowserClosed(Err(e)),
                        }
                    }
                );
            }
            
            Message::BrowserClosed(result) => {
                self.browser_running = false;
                match result {
                    Ok(response_json) => {
                        if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&response_json) {
                            if let Some(logs) = resp["logs"].as_array() {
                                for log in logs {
                                    if let Some(s) = log.as_str() {
                                        self.logs.push(s.to_string());
                                    }
                                }
                            }
                            let msg = resp["message"].as_str().unwrap_or("Browser closed");
                            self.browser_status = format!("⏹️ {}", msg);
                        } else {
                            self.browser_status = "⏹️ Browser closed".to_string();
                            self.logs.push("⏹️ Browser closed".to_string());
                        }
                    }
                    Err(e) => {
                        self.browser_status = format!("⚠️ {}", e);
                        self.logs.push(format!("⚠️ Browser close error: {}", e));
                    }
                }
            }
            
            // ── Scroll state ──
            Message::WorkflowScrolled(viewport) => {
                self.workflow_scroll_offset = viewport.absolute_offset();
            }
            
            Message::PaletteScrolled(viewport) => {
                self.palette_scroll_offset = viewport.absolute_offset();
            }
            
            Message::ClearLogs => {
                self.logs.clear();
            }
            
            Message::TestInput(msg) => {
                self.test_inputs.update(msg);
            }
            
            Message::RequestExit => {
                let port = BROWSER_RUNNER_PORT;
                self.logs.push("🔴 Exiting... sending shutdown signal to browser_runner".to_string());
                
                return Task::perform(
                    async move {
                        if let Ok(mut stream) = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)) {
                            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
                            let cmd = serde_json::json!({ "cmd": "shutdown" });
                            use std::io::Write;
                            let _ = writeln!(stream, "{}", cmd.to_string());
                        }
                        Ok::<_, String>("Exit complete".to_string())
                    },
                    |_| Message::ExitCompleted
                );
            }
            
            Message::ExitCompleted => {
                std::process::exit(0);
            }
        }
        
        // ── Restore scroll position ──
        self.scroll_restore_task()
    }

    /// Check if workflow contains at least one Container
    fn has_any_container(&self) -> bool {
        self.workflow.iter().any(|h| matches!(h, Handler::Container { .. }))
    }
    
    /// Check if index is after first Container
    fn is_below_first_container(&self, index: usize) -> bool {
        // Find first Container position
        if let Some(container_idx) = self.workflow.iter().position(|h| matches!(h, Handler::Container { .. })) {
            index > container_idx
        } else {
            false
        }
    }

    /// Refresh test input keys on workflow change
    fn refresh_test_inputs(&mut self) {
        self.test_inputs.parse_from_workflow(&self.workflow);
    }
    
    /// Restore scroll position Task 생성
    fn scroll_restore_task(&self) -> Task<Message> {
        let workflow_task = scroll_to(
            self.workflow_scroll.clone(),
            self.workflow_scroll_offset
        );
        let editor_task = scroll_to(
            self.editor.scroll_id.clone(),
            self.editor.scroll_offset
        );
        let palette_task = scroll_to(
            self.palette_scroll.clone(),
            self.palette_scroll_offset
        );
        Task::batch(vec![workflow_task, editor_task, palette_task])
    }

    fn current_insert_index(&self) -> Option<usize> {
        let drag = self.dragging.as_ref()?;
        if drag.cursor.x < PALETTE_WIDTH + OUTER_PADDING {
            return None;
        }
        Some(self.calc_insert_index(drag.cursor.y))
    }

    // =====================================================================
    // view
    // =====================================================================
    fn view(&self) -> Element<'_, Message> {
        let browser_bar = self.view_browser_bar();
        let toolbar = self.view_toolbar();
        
        let palette = self.view_palette();
        let workflow_list = self.view_workflow();

        // Layout: palette - workflow - editor(if open) - test_input(always right)
        let mut main_row = row![
            container(palette)
                .width(PALETTE_WIDTH)
                .height(Length::Fill)
                .padding(8),
            container(workflow_list)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8),
        ]
        .spacing(8);

        // Add editor panel if open
        if self.editor.is_open {
            let editor_panel = self.editor.view().map(Message::Editor);
            main_row = main_row.push(
                container(editor_panel)
                    .width(EDITOR_WIDTH)
                    .height(Length::Fill)
                    .padding(8)
            );
        }

        // Test inputs always on the right
        let test_input_panel = self.test_inputs.view().map(Message::TestInput);
        main_row = main_row.push(
            container(test_input_panel)
                .width(TEST_INPUT_WIDTH)
                .height(Length::Fill)
                .padding(8)
        );

        let log_panel = self.view_log_panel();

        let content = column![
            browser_bar,
            toolbar,
            main_row,
            log_panel,
        ];

        let mut base: Element<'_, Message> = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .into();

        if self.show_save_as {
            let dialog = self.view_save_as_dialog();
            base = iced::widget::stack![base, dialog].into();
        }

        if let Some(drag) = &self.dragging {
            let (r, g, b) = drag.palette_kind.color();
            let is_delete_zone = (drag.cursor.x < PALETTE_WIDTH + OUTER_PADDING) && drag.cursor!= Point::ORIGIN
                && matches!(drag.source, DragSource::Workflow(_));
            
            let ghost = mouse_layer(
                container(
                    row![
                        text(if is_delete_zone { "🗑️" } else { drag.palette_kind.icon() }).size(14),
                        Space::new().width(6),
                        text(if is_delete_zone { "Delete" } else { drag.palette_kind.display_name() }).size(12),
                    ]
                    .align_y(iced::Alignment::Center)
                )
                .padding(Padding::from([8, 12]))
                .style(move |_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(
                        if is_delete_zone {
                            iced::Color::from_rgba(0.9, 0.3, 0.3, 0.9)
                        } else {
                            iced::Color::from_rgba(r, g, b, 0.9)
                        }
                    )),
                    border: iced::Border {
                        color: if is_delete_zone {
                            iced::Color::from_rgb(0.9, 0.2, 0.2)
                        } else {
                            iced::Color::from_rgb(r, g, b)
                        },
                        width: 2.0,
                        radius: 8.0.into(),
                    },
                    text_color: Some(iced::Color::WHITE),
                    ..Default::default()
                })
            )
            .offset(15.0, 15.0);

            column![base, ghost].into()
        } else {
            base
        }
    }

    // ── Log panel ──
    fn view_log_panel(&self) -> Element<'_, Message> {
        let log_content: Element<'_, Message> = if self.logs.is_empty() {
            text("No logs").size(12).into()
        } else {
            let log_text = self.logs.join("\n");
            text(log_text).size(11).into()
        };
        
        let clear_btn = button(
            row![text("🗑️").size(12), Space::new().width(4), text("Clear").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([4, 8]))
        .style(button::secondary)
        .on_press(Message::ClearLogs);
        
        let exit_btn = button(
            row![text("🚪").size(12), Space::new().width(4), text("Exit").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([4, 8]))
        .style(button::danger)
        .on_press(Message::RequestExit);
        
        let header = row![
            text("📋 Log").size(14),
            Space::new().width(Length::Fill),
            clear_btn,
            Space::new().width(8),
            exit_btn,
        ]
        .align_y(iced::Alignment::Center);
        
        let log_scroll = scrollable(
            container(log_content)
                .width(Length::Fill)
                .padding(8)
        )
        .height(150)
        .anchor_y(scrollable::Anchor::End);
        
        container(
            column![
                header,
                container(log_scroll)
                    .width(Length::Fill)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(
                            iced::Color::from_rgba(0.05, 0.05, 0.08, 0.95)
                        )),
                        border: iced::Border {
                            color: iced::Color::from_rgba(0.3, 0.3, 0.3, 0.5),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
            ]
            .spacing(4)
        )
        .width(Length::Fill)
        .padding(Padding::from([8, 16]))
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgba(0.1, 0.12, 0.15, 0.95)
            )),
            border: iced::Border {
                color: iced::Color::from_rgba(0.2, 0.3, 0.4, 0.5),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    // ── Browser bar ──
    fn view_browser_bar(&self) -> Element<'_, Message> {
        let browser_types = vec![BrowserType::Desktop, BrowserType::Mobile];
        let browser_picker = pick_list(
            browser_types,
            Some(self.browser_type),
            Message::BrowserTypeChanged
        )
        .width(120);

        let open_btn = if self.browser_running {
            button(
                row![text("🌐").size(14), Space::new().width(4), text("Browser Open").size(11)]
                    .align_y(iced::Alignment::Center)
            )
            .padding(Padding::from([6, 12]))
            .style(button::secondary)
        } else {
            button(
                row![text("🌐").size(14), Space::new().width(4), text("Open Browser").size(11)]
                    .align_y(iced::Alignment::Center)
            )
            .padding(Padding::from([6, 12]))
            .style(button::primary)
            .on_press(Message::OpenBrowser)
        };

        let close_btn = if self.browser_running {
            button(
                row![text("✖").size(14), Space::new().width(4), text("Close").size(11)]
                    .align_y(iced::Alignment::Center)
            )
            .padding(Padding::from([6, 12]))
            .style(button::danger)
            .on_press(Message::CloseBrowser)
        } else {
            button(
                row![text("✖").size(14), Space::new().width(4), text("Close").size(11)]
                    .align_y(iced::Alignment::Center)
            )
            .padding(Padding::from([6, 12]))
            .style(button::secondary)
        };

        let url_input = text_input("Start URL...", &self.start_url)
            .on_input(Message::StartUrlChanged)
            .width(350)
            .size(12);

        let run_btn = if self.sequence_running {
            button(
                row![text("⏳").size(14), Space::new().width(4), text("Running...").size(11)]
                    .align_y(iced::Alignment::Center)
            )
            .padding(Padding::from([6, 12]))
            .style(button::secondary)
        } else {
            button(
                row![text("▶️").size(14), Space::new().width(4), text("Run Sequence").size(11)]
                    .align_y(iced::Alignment::Center)
            )
            .padding(Padding::from([6, 12]))
            .style(button::success)
            .on_press(Message::RunSequence)
        };

        let stop_btn = button(
            row![text("⏹️").size(14), Space::new().width(4), text("Stop").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 12]))
        .style(button::danger)
        .on_press(Message::StopSequence);

        let status_text = text(&self.browser_status).size(11);

        container(
            row![
                text("🖥️ Browser").size(14),
                Space::new().width(8),
                browser_picker,
                Space::new().width(6),
                open_btn,
                close_btn,
                Space::new().width(16),
                text("🔗").size(14),
                Space::new().width(4),
                url_input,
                Space::new().width(8),
                run_btn,
                stop_btn,
                Space::new().width(12),
                status_text,
                Space::new().width(Length::Fill),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding(Padding::from([8, 16]))
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgba(0.12, 0.15, 0.18, 0.95)
            )),
            border: iced::Border {
                color: iced::Color::from_rgba(0.2, 0.4, 0.6, 0.5),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    // ── Toolbar ──
    fn view_toolbar(&self) -> Element<'_, Message> {
        // Sequence type picker (Main / Sub)
        let seq_types = vec![SequenceType::Main, SequenceType::Sub];
        let type_picker = pick_list(
            seq_types,
            Some(self.current_sequence_type),
            Message::SequenceTypeChanged
        )
        .width(100);

        // Sequence list - shows all sequences
        let all_names: Vec<String> = match self.current_sequence_type {
            SequenceType::Main => {
                let mut names = self.main_sequence_names.iter()
                    .map(|n| format!("🏠 {}", n))
                    .collect::<Vec<_>>();
                // Also show sub-sequences with different icon
                names.extend(
                    self.sub_sequence_names.iter()
                        .map(|n| format!("🔗 {}", n))
                );
                names
            }
            SequenceType::Sub => {
                let mut names = self.sub_sequence_names.iter()
                    .map(|n| format!("🔗 {}", n))
                    .collect::<Vec<_>>();
                names.extend(
                    self.main_sequence_names.iter()
                        .map(|n| format!("🏠 {}", n))
                );
                names
            }
        };
        
        // Current selection display (with icon)
        let selected_display = self.current_sequence_name.as_ref().map(|n| {
            match self.current_sequence_type {
                SequenceType::Main => format!("🏠 {}", n),
                SequenceType::Sub => format!("🔗 {}", n),
            }
        });

        let sequence_picker = pick_list(
            all_names,
            selected_display,
            |s| {
                // Remove icon prefix to get actual name
                let name = s.chars().skip(2).collect::<String>().trim().to_string();
                Message::SequenceSelected(name)
            }
        )
        .placeholder("Select Sequence...")
        .width(200);

        let new_main_btn = button(
            row![text("🏠").size(14), Space::new().width(4), text("New Main").size(10)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 8]))
        .on_press(Message::NewSequence);

        let new_sub_btn = button(
            row![text("🔗").size(14), Space::new().width(4), text("New Sub").size(10)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 8]))
        .style(button::secondary)
        .on_press(Message::NewSubSequence);

        let load_btn = button(
            row![text("📂").size(14), Space::new().width(4), text("Load").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 10]))
        .on_press(Message::LoadFromJson);

        let merge_btn = button(
            row![text("➕").size(14), Space::new().width(4), text("Merge").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 10]))
        .style(button::secondary)
        .on_press(Message::MergeLoad);

        let save_btn = button(
            row![text("💾").size(14), Space::new().width(4), text("Save").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 10]))
        .style(button::success)
        .on_press(Message::SaveAllSequences);

        let save_as_btn = button(
            row![text("💾").size(14), text("+").size(10), Space::new().width(4), text("Save As").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 10]))
        .style(button::success) 
        .on_press(Message::SaveAs);

        let export_btn = button(
            row![text("📤").size(14), Space::new().width(4), text("Export").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 10]))
        .style(button::primary)
        .on_press(Message::ExportSequence);

        let send_btn = button(
            row![text("📌").size(14), Space::new().width(4), text("Keep").size(11)]
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([6, 10]))
        .on_press(Message::SendToList);

        let delete_btn = if self.current_sequence_name.is_some() {
            button(text("🗑️").size(14))
                .padding(Padding::from([6, 10]))
                .style(button::danger)
                .on_press(Message::DeleteSequence)
        } else {
            button(text("🗑️").size(14))
                .padding(Padding::from([6, 10]))
                .style(button::secondary)
        };

        let file_path_text = if let Some(path) = self.current_file_path.as_ref() {
            let short_path = path.rsplit('/').next()
                .or_else(|| path.rsplit('\\').next())
                .unwrap_or(path);
            text(format!("📁 {}", short_path)).size(10)
        } else {
            text("").size(10)
        };

        // Sequence counter
        let counter_text = text(format!(
            "Main:{} Sub:{}",
            self.main_sequence_names.len(),
            self.sub_sequence_names.len()
        )).size(10);

        container(
            row![
                text("🔧 Workflow Builder").size(16),
                Space::new().width(12),
                type_picker,
                Space::new().width(4),
                sequence_picker,
                Space::new().width(8),
                new_main_btn,
                new_sub_btn,
                Space::new().width(4),
                load_btn,
                merge_btn,
                Space::new().width(4),
                save_btn,
                save_as_btn,
                export_btn,
                Space::new().width(8),
                counter_text,
                Space::new().width(8),
                file_path_text,
                Space::new().width(Length::Fill),
                send_btn,
                delete_btn,
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding(Padding::from([10, 16]))
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgba(0.15, 0.15, 0.2, 0.9)
            )),
            border: iced::Border {
                color: iced::Color::from_rgba(0.3, 0.3, 0.4, 0.5),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    // ── Save As dialog ──
    fn view_save_as_dialog(&self) -> Element<'_, Message> {
        let dialog_content = container(
            column![
                text("💾 Save As").size(18),
                Space::new().height(12),
                // Type selection
                row![
                    text("Save as:").size(12),
                    Space::new().width(8),
                    pick_list(
                        vec![SequenceType::Main, SequenceType::Sub],
                        Some(self.save_as_type),
                        Message::SaveAsTypeChanged,
                    )
                    .width(120),
                ]
                .align_y(iced::Alignment::Center),
                Space::new().height(12),
                text("Sequence Name:").size(12),
                text_input("Enter name...", &self.save_as_name)
                    .on_input(Message::SaveAsNameChanged)
                    .on_submit(Message::ConfirmSaveAs)
                    .padding(10)
                    .width(250),
                Space::new().height(16),
                row![
                    button(text("Cancel").size(12))
                        .padding(Padding::from([8, 16]))
                        .on_press(Message::CancelSaveAs),
                    Space::new().width(Length::Fill),
                    button(text("Save").size(12))
                        .padding(Padding::from([8, 16]))
                        .style(button::success)
                        .on_press(Message::ConfirmSaveAs),
                ]
                .width(250),
            ]
            .spacing(8)
        )
        .padding(24)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgba(0.2, 0.2, 0.25, 0.98)
            )),
            border: iced::Border {
                color: iced::Color::from_rgba(0.4, 0.4, 0.5, 1.0),
                width: 2.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

        container(
            container(dialog_content)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6)
            )),
            ..Default::default()
        })
        .into()
    }

    // ── Palette ──
    fn view_palette(&self) -> Element<'_, Message> {
        let is_delete_target = self.dragging.as_ref()
            .is_some_and(|d| matches!(d.source, DragSource::Workflow(_)));

        let header = container(
            text(if is_delete_target { "🗑️ Drop to Delete" } else { "🎨 Handlers" })
                .size(14)
        )
        .width(Length::Fill)
        .padding(10)
        .style(if is_delete_target {
            |_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(
                    iced::Color::from_rgba(0.5, 0.2, 0.2, 0.8)
                )),
                border: iced::Border {
                    color: iced::Color::from_rgba(0.8, 0.3, 0.3, 1.0),
                    width: 2.0,
                    radius: 10.0.into(),
                },
                text_color: Some(iced::Color::WHITE),
                ..Default::default()
            }
        } else {
            styles::header_style
        });

        // Build cards for each group
        let mut main_column = column![].spacing(6).padding(6);
        
        for group in PALETTE_GROUPS {
            // Group header
            let group_header = container(
                row![
                    text(group.icon).size(11),
                    Space::new().width(4),
                    text(group.name).size(10),
                ].align_y(iced::Alignment::Center)
            )
            .padding(Padding::from([4, 8]))
            .width(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(
                    iced::Color::from_rgba(0.2, 0.22, 0.26, 0.8)
                )),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
            
            main_column = main_column.push(group_header);
            
            // Items in 2-column grid
            let mut items_row = row![].spacing(4);
            for (i, kind) in group.kinds.iter().enumerate() {
                items_row = items_row.push(self.view_palette_card_compact(kind));
                if i % 2 == 1 {
                    main_column = main_column.push(items_row);
                    items_row = row![].spacing(4);
                }
            }
            // Remaining items
            if group.kinds.len() % 2 == 1 {
                items_row = items_row.push(Space::new().width(Length::Fill));
                main_column = main_column.push(items_row);
            }
            
            main_column = main_column.push(Space::new().height(4));
        }

        let scrollable_content = scrollable(main_column)
            .height(Length::Fill)
            .width(Length::Fill)
            .id(self.palette_scroll.clone())
            .on_scroll(Message::PaletteScrolled);

        let content = column![header, scrollable_content];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(if is_delete_target {
                |_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(
                        iced::Color::from_rgba(0.3, 0.15, 0.15, 0.5)
                    )),
                    border: iced::Border {
                        color: iced::Color::from_rgba(0.7, 0.3, 0.3, 0.7),
                        width: 2.0,
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                }
            } else {
                styles::list_container_style
            })
            .into()
    }
    
    /// 컴팩트한 Palette 카드 (2열 그리드용)
    fn view_palette_card_compact(&self, kind: &PaletteKind) -> Element<'_, Message> {
        let (r, g, b) = kind.color();
        let kind_clone = kind.clone();

        // SubSequence is disabled if no Container exists
        let is_disabled = *kind == PaletteKind::SubSequence && !self.has_any_container();

        let is_dragging = self.dragging.as_ref()
            .is_some_and(|d| matches!(&d.source, DragSource::Palette(k) if k == kind));

        let content = container(
            row![
                text(kind.icon()).size(11),
                Space::new().width(3),
                text(kind.display_name()).size(9),
            ]
            .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([5, 6]))
        .width(Length::Fill)
        .style(move |_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                if is_disabled {
                    iced::Color::from_rgba(0.2, 0.2, 0.2, 0.4)
                } else if is_dragging {
                    iced::Color::from_rgba(0.3, 0.3, 0.3, 0.5)
                } else {
                    iced::Color::from_rgba(r * 0.3, g * 0.3, b * 0.3, 0.8)
                }
            )),
            border: iced::Border {
                color: if is_disabled {
                    iced::Color::from_rgba(0.3, 0.3, 0.3, 0.3)
                } else {
                    iced::Color::from_rgba(r, g, b, 0.6)
                },
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        });

        if is_disabled {
            // Disabled state - not clickable, dimmed
            container(content)
                .style(|_: &Theme| container::Style {
                    ..Default::default()
                })
                .into()
        } else {
            mouse_area(content)
                .on_press(Message::DragStartFromPalette(kind_clone))
                .into()
        }
    }

    // ── Workflow list panel ──
    fn view_workflow(&self) -> Element<'_, Message> {
        let name_input = text_input(
            "New Workflow",
            self.current_sequence_name.as_deref().unwrap_or("")
        )
        .on_input(Message::SequenceNameChanged)
        .size(16)
        .width(200)
        .padding(Padding::from([4, 8]))
        .style(|theme: &Theme, status| {
            let palette = theme.palette();
            let is_focused = matches!(status, ti::Status::Focused { .. });
            ti::Style {
                background: iced::Background::Color(iced::Color::TRANSPARENT),
                border: iced::Border {
                    color: if is_focused {
                        palette.primary
                    } else {
                        iced::Color::TRANSPARENT
                    },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                icon: palette.text,
                placeholder: iced::Color::from_rgba(0.5, 0.5, 0.5, 0.7),
                value: palette.text,
                selection: palette.primary,
            }
        });

        let header = container(
            row![
                text("📋").size(16),
                Space::new().width(4),
                name_input,
                Space::new().width(Length::Fill),
                text(format!("{} handlers", self.workflow.len())).size(11)
            ]
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding(12)
        .style(styles::header_style);

        let insert_index = self.current_insert_index();

        let content: Element<'_, Message> = if self.workflow.is_empty() {
            let show_indicator = insert_index.is_some();
            
            container(
                column![
                    if show_indicator {
                        Self::insert_indicator()
                    } else {
                        text("Drag handlers here").size(14).into()
                    }
                ]
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(
                    iced::Color::from_rgba(0.2, 0.2, 0.2, 0.3)
                )),
                border: iced::Border {
                    color: iced::Color::from_rgba(0.4, 0.4, 0.4, 0.5),
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            let mut elements: Vec<Element<'_, Message>> = Vec::new();

            for (i, handler) in self.workflow.iter().enumerate() {
                if insert_index == Some(i) {
                    elements.push(Self::insert_indicator());
                }
                elements.push(self.view_workflow_card(i, handler));
            }

            if insert_index == Some(self.workflow.len()) {
                elements.push(Self::insert_indicator());
            }

            let cards_column = column(elements)
                .spacing(CARD_SPACING)
                .padding(CONTENT_PADDING);

            // ── Scroll state ──
            scrollable(cards_column)
                .id(self.workflow_scroll.clone())
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::default()
                ))
                .on_scroll(Message::WorkflowScrolled)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };

        let layout = column![header, content];

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(if self.dragging.is_some() {
                |_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(
                        iced::Color::from_rgba(0.2, 0.3, 0.2, 0.3)
                    )),
                    border: iced::Border {
                        color: iced::Color::from_rgba(0.4, 0.7, 0.4, 0.7),
                        width: 2.0,
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                }
            } else {
                styles::list_container_style
            })
            .into()
    }

    fn insert_indicator<'a>() -> Element<'a, Message> {
        container(Space::new().height(4))
            .width(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(
                    iced::Color::from_rgba(0.3, 0.8, 0.3, 0.9)
                )),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    /// Workflow card - render Handler directly
    fn view_workflow_card<'a>(&self, index: usize, handler: &'a Handler) -> Element<'a, Message> {
        let (r, g, b) = handler_color(handler);

        let is_dragging = self.dragging.as_ref()
            .is_some_and(|d| matches!(&d.source, DragSource::Workflow(i) if *i == index));

        let is_editing = self.editing_index == Some(index);

        let index_badge = container(
            text(format!("{}", index)).size(10)
        )
        .padding(Padding::from([3, 6]))
        .style(styles::index_badge_style);

        let type_badge = container(
            row![
                text(handler_icon(handler)).size(12),
                Space::new().width(4),
                text(handler_type_name(handler)).size(10),
            ]
            .align_y(iced::Alignment::Center)
        )
        .padding(Padding::from([4, 8]))
        .style(styles::badge_style(r, g, b));

        let desc = handler_description(handler);
        let description = text(desc).size(11);

        let edit_btn = button(text("✏️").size(16))
            .padding(Padding::from([6, 10]))
            .style(button::secondary)
            .on_press(Message::EditHandler(index));

        let content = row![
            index_badge,
            Space::new().width(6),
            type_badge,
            Space::new().width(8),
            description,
            Space::new().width(Length::Fill),
            edit_btn,
        ]
        .align_y(iced::Alignment::Center)
        .padding(10);

        let card = container(content)
            .width(Length::Fill)
            .style(move |_theme: &Theme| {
                if is_dragging {
                    container::Style {
                        background: Some(iced::Background::Color(
                            iced::Color::from_rgba(0.3, 0.3, 0.3, 0.5)
                        )),
                        border: iced::Border {
                            color: iced::Color::from_rgba(0.5, 0.5, 0.5, 0.5),
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }
                } else if is_editing {
                    container::Style {
                        background: Some(iced::Background::Color(
                            iced::Color::from_rgba(0.2, 0.3, 0.4, 0.8)
                        )),
                        border: iced::Border {
                            color: iced::Color::from_rgba(0.4, 0.6, 0.9, 1.0),
                            width: 2.0,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }
                } else {
                    styles::card_style(_theme)
                }
            });

        mouse_area(card)
            .on_press(Message::DragStartFromWorkflow(index))
            .into()
    }

    // ── Global event subscription ──
    fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _status, _id| {
            match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    Some(Message::DragMove(position))
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    Some(Message::DragEnd)
                }
                _ => None,
            }
        })
    }
}