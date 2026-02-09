// crates/collector/src/handler_editor.rs
//! Handler Editing UI — Directly edits workflow types

use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input, toggler,
    Space, Id,
};
use iced::widget::scrollable::{AbsoluteOffset, Viewport};
use iced::{Border, Element, Length, Padding, Theme};

use crate::styles;
use crate::workflow::{Action, BindValue, Extraction, Handler, KeyAction, Sequence, Step};

// =========================================================================
// Container output info (for SubSequence input mapping)
// =========================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerField {
    pub field_name: String,
    pub extraction_type: String,
}

// =========================================================================
// Input Mapping — Defines how SubSequence receives Container output
// =========================================================================

#[derive(Debug, Clone, Default)]
pub struct InputMapping {
    /// Binding key to use inside SubSequence (Dropdown selection)
    pub binding_key: String,
    /// Container output field (Dropdown selection)
    pub source_field: Option<String>,
}

// =========================================================================
// Editor Messages
// =========================================================================

#[derive(Debug, Clone)]
pub enum EditorMsg {
    ChangeHandlerKind(String),

    // ── Step Level ──
    ActionVariantChanged(String),
    ExtractionVariantChanged(String),

    // ── BindValue<String> Editing ──
    BindStrValueChanged(FieldId, String),
    BindStrKeyChanged(FieldId, String),
    BindStrToggle(FieldId, bool),

    // ── BindValue<u64> Editing ──
    BindU64ValueChanged(FieldId, String),
    BindU64KeyChanged(FieldId, String),
    BindU64Toggle(FieldId, bool),

    // ── Container Specific ──
    ContainerDedupToggled(bool),
    AddContainerStep,
    RemoveContainerStep(usize),
    ContainerStepExtractionChanged(usize, String),
    ContainerStepBindStr(usize, FieldId, String),
    ContainerStepBindStrKey(usize, FieldId, String),
    ContainerStepBindStrToggle(usize, FieldId, bool),

    // ── SubSequence Specific ──
    SubSeqNameChanged(String),
    SubSeqSelectFromList(String),
    SubSeqJsonPathChanged(String),
    
    // ── Input Mapping (SubSequence) ──
    AddInputMapping,
    RemoveInputMapping(usize),
    MappingBindingKeyChanged(usize, String),  // Dropdown selection
    MappingSourceChanged(usize, String),      // Dropdown selection

    // ── KeyAction pick ──
    KeyActionChanged(String),

    // ── Scroll ──
    Scrolled(Viewport),

    // ── Common ──
    Close,
    Save,
    BindI64ValueChanged(FieldId, String),
    BindI64KeyChanged(FieldId, String),
    BindI64Toggle(FieldId, bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldId {
    Selector,
    Url,
    Text,
    FieldName,
    AttrStr,
    TimeMs,
    Index,
    Base,      
    Href,      
    Scroll,    
}

// =========================================================================
// Editor State
// =========================================================================

#[derive(Debug, Clone)]
pub struct EditorState {
    pub is_open: bool,
    pub handler: Handler,
    pub available_sequences: Vec<String>,
    /// Output fields of the previous Container (for SubSequence input mapping)
    pub container_fields: Vec<ContainerField>,
    /// Binding keys used inside SubSequence (Parsed)
    pub subsequence_binding_keys: Vec<String>,
    /// SubSequence input mapping
    pub input_mappings: Vec<InputMapping>,
    /// Maintain scroll state
    pub scroll_id: Id,
    pub scroll_offset: AbsoluteOffset,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            is_open: false,
            handler: Handler::Item(Step::Act(Action::Click {
                selector_string: BindValue::new(String::new()),
            })),
            available_sequences: Vec::new(),
            container_fields: Vec::new(),
            subsequence_binding_keys: Vec::new(),
            input_mappings: Vec::new(),
            scroll_id: Id::unique(),
            scroll_offset: AbsoluteOffset { x: 0.0, y: 0.0 },
        }
    }
}

impl EditorState {
    pub fn open(&mut self, handler: Handler) {
        self.is_open = true;
        self.scroll_offset = AbsoluteOffset { x: 0.0, y: 0.0 }; // Reset scroll
        // If SubSequence, load existing mappings + parse binding keys
        if let Handler::SubSequence(ref seq) = handler {
            self.input_mappings = parse_mappings_from_metadata(&seq.metadata);
            self.subsequence_binding_keys = extract_binding_keys_from_sequence(seq);
        } else {
            self.input_mappings.clear();
            self.subsequence_binding_keys.clear();
        }
        self.handler = handler;
    }

    /// Set previous Container info when editing SubSequence
    pub fn set_container_context(&mut self, fields: Vec<ContainerField>) {
        self.container_fields = fields;
    }

    pub fn open_new_action(&mut self) {
        self.open(Handler::Item(Step::Act(Action::Click {
            selector_string: BindValue::new(String::new()),
        })));
    }

    pub fn open_new_extraction(&mut self) {
        self.open(Handler::Item(Step::Extract(Extraction::Text {
            selector_string: BindValue::new(String::new()),
            field_name: BindValue::new(String::new()),
        })));
    }

    pub fn open_new_container(&mut self) {
        self.open(Handler::Container {
            selector: BindValue::new(String::new()),
            steps: Vec::new(),
            dedup: false,
        });
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.container_fields.clear();
        self.subsequence_binding_keys.clear();
        self.input_mappings.clear();
    }

    fn kind_name(&self) -> &'static str {
        match &self.handler {
            Handler::Item(Step::Act(_)) => "Action",
            Handler::Item(Step::Extract(_)) => "Extraction",
            Handler::Container { .. } => "Container",
            Handler::SubSequence(_) => "SubSequence",
        }
    }

    /// Save input mappings to Sequence metadata
    pub fn apply_mappings_to_handler(&mut self) {
        if let Handler::SubSequence(ref mut seq) = self.handler {
            seq.metadata.retain(|s| !s.starts_with("mapping:"));
            for m in &self.input_mappings {
                if let Some(src) = &m.source_field {
                    if !m.binding_key.is_empty() {
                        seq.metadata.push(format!("mapping:{}:{}", m.binding_key, src));
                    }
                }
            }
        }
    }

    // =====================================================================
    // update
    // =====================================================================

    pub fn update(&mut self, msg: EditorMsg) {
        match msg {
            EditorMsg::ChangeHandlerKind(kind) => {
                match kind.as_str() {
                    "Action" => self.open_new_action(),
                    "Extraction" => self.open_new_extraction(),
                    "Container" => self.open_new_container(),
                    "SubSequence" => {
                        self.handler = Handler::SubSequence(Sequence {
                            sequence_name: String::new(),
                            step_sequence: Vec::new(),
                            target_data: Default::default(),
                            metadata: Vec::new(),
                        });
                        self.subsequence_binding_keys.clear();
                    }
                    _ => {}
                }
            }

            EditorMsg::ActionVariantChanged(variant) => {
                if let Handler::Item(Step::Act(_)) = &self.handler {
                    self.handler = Handler::Item(Step::Act(default_action_for_variant(&variant)));
                }
            }

            EditorMsg::ExtractionVariantChanged(variant) => {
                if let Handler::Item(Step::Extract(_)) = &self.handler {
                    self.handler = Handler::Item(Step::Extract(default_extraction_for_variant(&variant)));
                }
            }

            EditorMsg::BindStrValueChanged(fid, val) => {
                if let Some(bv) = find_bind_str(&mut self.handler, &fid) {
                    bv.value = val;
                }
            }
            EditorMsg::BindStrKeyChanged(fid, key) => {
                if let Some(bv) = find_bind_str(&mut self.handler, &fid) {
                    bv.binding = if key.is_empty() { None } else { Some(key) };
                }
            }
            EditorMsg::BindStrToggle(fid, enabled) => {
                if let Some(bv) = find_bind_str(&mut self.handler, &fid) {
                    if enabled {
                        if bv.binding.is_none() { bv.binding = Some(String::new()); }
                    } else {
                        bv.binding = None;
                    }
                }
            }

            EditorMsg::BindU64ValueChanged(fid, val) => {
                if let Some(bv) = find_bind_u64(&mut self.handler, &fid) {
                    bv.value = val.parse().unwrap_or(bv.value);
                }
            }
            EditorMsg::BindU64KeyChanged(fid, key) => {
                if let Some(bv) = find_bind_u64(&mut self.handler, &fid) {
                    bv.binding = if key.is_empty() { None } else { Some(key) };
                }
            }
            EditorMsg::BindU64Toggle(fid, enabled) => {
                if let Some(bv) = find_bind_u64(&mut self.handler, &fid) {
                    if enabled {
                        if bv.binding.is_none() { bv.binding = Some(String::new()); }
                    } else {
                        bv.binding = None;
                    }
                }
            }

            EditorMsg::ContainerDedupToggled(v) => {
                if let Handler::Container { dedup, .. } = &mut self.handler {
                    *dedup = v;
                }
            }
            EditorMsg::AddContainerStep => {
                if let Handler::Container { steps, .. } = &mut self.handler {
                    steps.push(Step::Extract(Extraction::Text {
                        selector_string: BindValue::new(String::new()),
                        field_name: BindValue::new(String::new()),
                    }));
                }
            }
            EditorMsg::RemoveContainerStep(idx) => {
                if let Handler::Container { steps, .. } = &mut self.handler {
                    if idx < steps.len() { steps.remove(idx); }
                }
            }
            EditorMsg::ContainerStepExtractionChanged(idx, variant) => {
                if let Handler::Container { steps, .. } = &mut self.handler {
                    if let Some(step) = steps.get_mut(idx) {
                        *step = Step::Extract(default_extraction_for_variant(&variant));
                    }
                }
            }
            EditorMsg::ContainerStepBindStr(idx, fid, val) => {
                if let Handler::Container { steps, .. } = &mut self.handler {
                    if let Some(Step::Extract(ext)) = steps.get_mut(idx) {
                        if let Some(bv) = find_bind_str_in_extraction(ext, &fid) {
                            bv.value = val;
                        }
                    }
                }
            }
            EditorMsg::ContainerStepBindStrKey(idx, fid, key) => {
                if let Handler::Container { steps, .. } = &mut self.handler {
                    if let Some(Step::Extract(ext)) = steps.get_mut(idx) {
                        if let Some(bv) = find_bind_str_in_extraction(ext, &fid) {
                            bv.binding = if key.is_empty() { None } else { Some(key) };
                        }
                    }
                }
            }
            EditorMsg::ContainerStepBindStrToggle(idx, fid, enabled) => {
                if let Handler::Container { steps, .. } = &mut self.handler {
                    if let Some(Step::Extract(ext)) = steps.get_mut(idx) {
                        if let Some(bv) = find_bind_str_in_extraction(ext, &fid) {
                            if enabled {
                                if bv.binding.is_none() { bv.binding = Some(String::new()); }
                            } else {
                                bv.binding = None;
                            }
                        }
                    }
                }
            }

            EditorMsg::SubSeqNameChanged(name) => {
                if let Handler::SubSequence(seq) = &mut self.handler {
                    seq.sequence_name = name;
                }
            }
            EditorMsg::SubSeqSelectFromList(name) => {
                // Binding key re-parsing on subsequence selection is handled in work_flow_ui
                if let Handler::SubSequence(seq) = &mut self.handler {
                    seq.sequence_name = name;
                }
            }
            EditorMsg::SubSeqJsonPathChanged(_path) => {}

            // ── Input Mapping (Both are dropdowns) ──
            EditorMsg::AddInputMapping => {
                self.input_mappings.push(InputMapping::default());
            }
            EditorMsg::RemoveInputMapping(idx) => {
                if idx < self.input_mappings.len() {
                    self.input_mappings.remove(idx);
                }
            }
            EditorMsg::MappingBindingKeyChanged(idx, key) => {
                if let Some(m) = self.input_mappings.get_mut(idx) {
                    m.binding_key = if key == "(none)" { String::new() } else { key };
                }
            }
            EditorMsg::MappingSourceChanged(idx, src) => {
                if let Some(m) = self.input_mappings.get_mut(idx) {
                    m.source_field = if src.is_empty() || src == "(none)" { None } else { Some(src) };
                }
            }

            EditorMsg::KeyActionChanged(k) => {
                if let Handler::Item(Step::Act(Action::PressKey { key })) = &mut self.handler {
                    *key = parse_key_action(&k);
                }
            }

            EditorMsg::Scrolled(viewport) => {
                self.scroll_offset = viewport.absolute_offset();
            }
            EditorMsg::Close => self.close(),
            EditorMsg::Save => {
                self.apply_mappings_to_handler();
                self.close();
            }
            EditorMsg::BindI64ValueChanged(fid, val) => {
                if let Some(bv) = find_bind_i64(&mut self.handler, &fid) {
                    bv.value = val.parse().unwrap_or(bv.value);
                }
            }
            EditorMsg::BindI64KeyChanged(fid, key) => {
                if let Some(bv) = find_bind_i64(&mut self.handler, &fid) {
                    bv.binding = if key.is_empty() { None } else { Some(key) };
                }
            }
            EditorMsg::BindI64Toggle(fid, enabled) => {
                if let Some(bv) = find_bind_i64(&mut self.handler, &fid) {
                    if enabled {
                        if bv.binding.is_none() { bv.binding = Some(String::new()); }
                    } else {
                        bv.binding = None;
                    }
                }
            }
        }
    }

    // =====================================================================
    // view
    // =====================================================================

    pub fn view(&self) -> Element<'_, EditorMsg, Theme> {
        let kind_options = vec![
            "Action".to_string(),
            "Extraction".to_string(),
            "Container".to_string(),
            "SubSequence".to_string(),
        ];

        let header = container(
            row![
                text("📝 Handler Editor").size(18),
                Space::new().width(Length::Fill),
                pick_list(kind_options, Some(self.kind_name().to_string()), EditorMsg::ChangeHandlerKind).width(160),
            ].align_y(iced::Alignment::Center),
        ).padding(12).width(Length::Fill).style(styles::header_style);

        let form: Element<'_, EditorMsg, Theme> = match &self.handler {
            Handler::Item(Step::Act(action)) => self.view_action_form(action),
            Handler::Item(Step::Extract(ext)) => self.view_extraction_form(ext),
            Handler::Container { selector, steps, dedup } => self.view_container_form(selector, steps, *dedup),
            Handler::SubSequence(seq) => self.view_subsequence_form(seq),
        };

        let buttons = container(
            row![
                button(text("Cancel").size(14)).padding(Padding::from([8, 16])).on_press(EditorMsg::Close),
                Space::new().width(Length::Fill),
                button(text("💾 Save").size(14)).padding(Padding::from([8, 16])).style(button::success).on_press(EditorMsg::Save),
            ],
        ).padding(12).width(Length::Fill);

        let content = column![
            header,
            scrollable(container(form).padding(16).width(Length::Fill))
                .height(Length::Fill)
                .id(self.scroll_id.clone())
                .on_scroll(EditorMsg::Scrolled),
            buttons,
        ];

        container(content).width(Length::Fill).height(Length::Fill).style(styles::list_container_style).into()
    }

    // ── Action Editing Form ──
    fn view_action_form<'a>(&self, action: &Action) -> Element<'a, EditorMsg, Theme> {
        // Type display (Immutable - selected from palette)
        let type_badge = container(
            text(format!("📌 {}", action_variant_name(action))).size(12)
        )
        .padding(Padding::from([6, 10]))
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgba(0.25, 0.35, 0.45, 0.8)
            )),
            border: iced::Border {
                color: iced::Color::from_rgba(0.4, 0.5, 0.6, 0.6),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        });

        let mut form = column![type_badge].spacing(12);

        match action {
            Action::Click { selector_string } => {
                form = form.push(bind_str_field("Selector", selector_string, FieldId::Selector));
            }
            Action::Navigate { url } => {
                form = form.push(bind_str_field("URL", url, FieldId::Url));
            }
            Action::WaitFor { selector_string, time_ms } => {
                form = form.push(bind_str_field("Selector", selector_string, FieldId::Selector));
                form = form.push(bind_u64_field("Timeout (ms)", time_ms, FieldId::TimeMs));
            }
            Action::ClickByText { selector_string, text: txt } => {
                form = form.push(bind_str_field("Selector", selector_string, FieldId::Selector));
                form = form.push(bind_str_field("Text", txt, FieldId::Text));
            }
            Action::GetHTML { selector_string, time_ms } => {
                form = form.push(bind_str_field("Selector", selector_string, FieldId::Selector));
                form = form.push(bind_u64_field("Retry (ms)", time_ms, FieldId::TimeMs));
            }
            Action::SwitchToFrame { selector_string } => {
                form = form.push(bind_str_field("Selector", selector_string, FieldId::Selector));
            }
            Action::NewTab { url } => {
                form = form.push(bind_str_field("URL", url, FieldId::Url));
            }
            Action::SwitchTab { index } => {
                form = form.push(bind_u64_field("Tab Index", index, FieldId::Index));
            }
            Action::Type { selector, text: txt } => {
                form = form.push(bind_str_field("Selector", selector, FieldId::Selector));
                form = form.push(bind_str_field("Text", txt, FieldId::Text));
            }
            Action::ClearAndType { selector, text: txt } => {
                form = form.push(bind_str_field("Selector", selector, FieldId::Selector));
                form = form.push(bind_str_field("Text", txt, FieldId::Text));
            }
            Action::PressKey { key } => {
                let key_options: Vec<String> = KEY_ACTION_NAMES.iter().map(|s| s.to_string()).collect();
                form = form.push(field_label_pick("Key", key_options, key_action_name(key), EditorMsg::KeyActionChanged));
            }
            Action::NavigateHref { base, href } => {
                form = form.push(bind_str_field("Base URL", base, FieldId::Base));
                form = form.push(bind_str_field("Href", href, FieldId::Href));
            }
            Action::ScrollDown { scroll } => {
                form = form.push(bind_i64_field("Pixels", scroll, FieldId::Scroll));
            }
            Action::ScrollUp { scroll } => {
                form = form.push(bind_i64_field("Pixels", scroll, FieldId::Scroll));
            }
            Action::Wait { time_ms } => {
                form = form.push(bind_u64_field("Duration (ms)", time_ms, FieldId::TimeMs));
            }
            _ => {
                form = form.push(container(text("This action has no additional parameters.").size(12)).padding(8).style(hint_style));
            }
        }
        form.into()
    }

    // ── Extraction Editing Form ──
    fn view_extraction_form<'a>(&self, ext: &Extraction) -> Element<'a, EditorMsg, Theme> {
        // Type display (Immutable - selected from palette)
        let type_badge = container(
            text(format!("📌 {}", extraction_variant_name(ext))).size(12)
        )
        .padding(Padding::from([6, 10]))
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                iced::Color::from_rgba(0.35, 0.25, 0.45, 0.8)
            )),
            border: iced::Border {
                color: iced::Color::from_rgba(0.5, 0.4, 0.6, 0.6),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        });

        let mut form = column![type_badge].spacing(12);

        match ext {
            Extraction::Text { selector_string, field_name } |
            Extraction::Count { selector_string, field_name } |
            Extraction::Exists { selector_string, field_name } |
            Extraction::MultipleText { selector_string, field_name } => {
                form = form.push(bind_str_field("Selector", selector_string, FieldId::Selector));
                form = form.push(bind_str_field("Field Name", field_name, FieldId::FieldName));
            }
            Extraction::Attribute { selector_string, field_name, attr_str } => {
                form = form.push(bind_str_field("Selector", selector_string, FieldId::Selector));
                form = form.push(bind_str_field("Field Name", field_name, FieldId::FieldName));
                form = form.push(bind_str_field("Attribute", attr_str, FieldId::AttrStr));
            }
        }
        form.into()
    }

    // ── Container Editing Form ──
    fn view_container_form<'a>(&self, selector: &BindValue<String>, steps: &[Step], dedup: bool) -> Element<'a, EditorMsg, Theme> {
        let mut form = column![
            bind_str_field("Container Selector", selector, FieldId::Selector),
            row![text("Dedup").size(12), Space::new().width(8), toggler(dedup).on_toggle(EditorMsg::ContainerDedupToggled)].align_y(iced::Alignment::Center),
        ].spacing(12);

        form = form.push(container(text("Container Steps").size(14)).padding(Padding::from([8, 0])));

        let extraction_options: Vec<String> = EXTRACTION_VARIANTS.iter().map(|s| s.to_string()).collect();

        for (idx, step) in steps.iter().enumerate() {
            let step_card: Element<'a, EditorMsg, Theme> = match step {
                Step::Act(_) => container(text("(Action — Ignored inside Container)").size(11)).padding(8).style(hint_style).into(),
                Step::Extract(ext) => {
                    let variant = extraction_variant_name(ext);
                    let mut step_form = column![
                        row![
                            pick_list(extraction_options.clone(), Some(variant.to_string()), move |v| EditorMsg::ContainerStepExtractionChanged(idx, v)).width(140),
                            Space::new().width(Length::Fill),
                            button(text("🗑").size(12)).padding(6).style(button::danger).on_press(EditorMsg::RemoveContainerStep(idx)),
                        ].align_y(iced::Alignment::Center),
                    ].spacing(8);
                    step_form = push_extraction_fields_indexed(step_form, ext, idx);
                    container(step_form).padding(12).width(Length::Fill).style(styles::card_style).into()
                }
            };
            form = form.push(step_card);
        }

        form = form.push(button(text("➕ Add Extraction Step").size(12)).padding(8).on_press(EditorMsg::AddContainerStep));
        form.into()
    }

    // ── SubSequence Editing Form (Both are dropdowns!) ──
    fn view_subsequence_form<'a>(&self, seq: &Sequence) -> Element<'a, EditorMsg, Theme> {
        let mut form = column![
            field_label_input("Sequence Name", "sequence name", &seq.sequence_name, EditorMsg::SubSeqNameChanged),
        ].spacing(12);

        if !self.available_sequences.is_empty() {
            form = form.push(field_label_pick(
                "Select from list",
                self.available_sequences.clone(),
                if seq.sequence_name.is_empty() { "" } else { &seq.sequence_name },
                EditorMsg::SubSeqSelectFromList,
            ));
        }

        // ── Input Mapping Section ──
        form = form.push(Space::new().height(12));
        form = form.push(
            container(
                row![
                    text("📥 Input Mapping").size(14),
                    Space::new().width(Length::Fill),
                    button(text("➕ Add").size(11)).padding(Padding::from([4, 8])).on_press(EditorMsg::AddInputMapping),
                ].align_y(iced::Alignment::Center)
            ).padding(8).width(Length::Fill).style(section_header_style)
        );

        // Display Info
        if self.subsequence_binding_keys.is_empty() {
            form = form.push(
                container(text("⚠️ SubSequence has no binding keys.").size(11))
                    .padding(8).width(Length::Fill).style(hint_style)
            );
        } else {
            let keys_str = self.subsequence_binding_keys.join(", ");
            form = form.push(
                container(text(format!("🔑 Required Inputs: {}", keys_str)).size(11))
                    .padding(8).width(Length::Fill).style(hint_style)
            );
        }

        if self.container_fields.is_empty() {
            form = form.push(
                container(text("⚠️ No previous Container.").size(11))
                    .padding(8).width(Length::Fill).style(hint_style)
            );
        } else {
            let fields_str: String = self.container_fields.iter()
                .map(|f| format!("{}", f.field_name))
                .collect::<Vec<_>>()
                .join(", ");
            form = form.push(
                container(text(format!("📦 Container Output: {}", fields_str)).size(11))
                    .padding(8).width(Length::Fill).style(hint_style)
            );
        }

        // Mapping rows - both are dropdowns
        for (idx, mapping) in self.input_mappings.iter().enumerate() {
            form = form.push(self.view_mapping_row(idx, mapping));
        }

        if self.input_mappings.is_empty() && !self.subsequence_binding_keys.is_empty() {
            form = form.push(
                container(text("Add mapping using the ➕ button").size(11))
                    .padding(8).style(hint_style)
            );
        }

        form = form.push(Space::new().height(8));
        form = form.push(
            container(text("💡 SubSequence is executed for each row of the Container").size(11))
                .padding(8).style(hint_style)
        );

        form.into()
    }

    /// Mapping row - Both are dropdowns!
    fn view_mapping_row<'a>(&self, idx: usize, mapping: &InputMapping) -> Element<'a, EditorMsg, Theme> {
        // Binding Key Dropdown Options
        let mut binding_key_options: Vec<String> = vec!["(none)".to_string()];
        binding_key_options.extend(self.subsequence_binding_keys.iter().cloned());
        
        // Container Output Dropdown Options
        let mut source_options: Vec<String> = vec!["(none)".to_string()];
        source_options.extend(self.container_fields.iter().map(|f| f.field_name.clone()));
        
        let selected_key = if mapping.binding_key.is_empty() { 
            "(none)".to_string() 
        } else { 
            mapping.binding_key.clone() 
        };
        
        let selected_source = mapping.source_field.clone().unwrap_or_else(|| "(none)".to_string());

        // Validity Display
        let is_valid = !mapping.binding_key.is_empty() && mapping.source_field.is_some();
        let status = if is_valid { text("✅").size(12) } else { text("⚠️").size(12) };

        let delete_btn = button(text("🗑").size(11))
            .padding(Padding::from([4, 8]))
            .style(button::danger)
            .on_press(EditorMsg::RemoveInputMapping(idx));

        container(
            row![
                status,
                Space::new().width(6),
                // Binding Key Dropdown
                column![
                    text("Binding Key").size(9),
                    pick_list(binding_key_options, Some(selected_key), move |s| EditorMsg::MappingBindingKeyChanged(idx, s))
                        .width(140),
                ].spacing(2),
                Space::new().width(6),
                text("←").size(14),
                Space::new().width(6),
                // Source Field Dropdown
                column![
                    text("Container Output").size(9),
                    pick_list(source_options, Some(selected_source), move |s| EditorMsg::MappingSourceChanged(idx, s))
                        .width(140),
                ].spacing(2),
                Space::new().width(Length::Fill),
                delete_btn,
            ].align_y(iced::Alignment::Center)
        ).padding(8).width(Length::Fill).style(styles::card_style).into()
    }
}

// =========================================================================
// Extract Binding Keys from SubSequence
// =========================================================================

/// Extract binding keys used in all Steps of the Sequence
pub fn extract_binding_keys_from_sequence(seq: &Sequence) -> Vec<String> {
    let mut keys = Vec::new();
    
    for handler in &seq.step_sequence {
        extract_binding_keys_from_handler(handler, &mut keys);
    }
    
    // Remove duplicates
    keys.sort();
    keys.dedup();
    keys
}

fn extract_binding_keys_from_handler(handler: &Handler, keys: &mut Vec<String>) {
    match handler {
        Handler::Item(Step::Act(action)) => {
            extract_from_action(action, keys);
        }
        Handler::Item(Step::Extract(ext)) => {
            extract_from_extraction(ext, keys);
        }
        Handler::Container { selector, steps, .. } => {
            if let Some(k) = &selector.binding {
                if !k.is_empty() { keys.push(k.clone()); }
            }
            for step in steps {
                if let Step::Extract(ext) = step {
                    extract_from_extraction(ext, keys);
                }
            }
        }
        Handler::SubSequence(inner_seq) => {
            // Recursively process inner sequences
            for h in &inner_seq.step_sequence {
                extract_binding_keys_from_handler(h, keys);
            }
        }
    }
}

fn extract_from_action(action: &Action, keys: &mut Vec<String>) {
    match action {
        Action::Click { selector_string } => {
            if let Some(k) = &selector_string.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::Navigate { url } => {
            if let Some(k) = &url.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::WaitFor { selector_string, time_ms } => {
            if let Some(k) = &selector_string.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &time_ms.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::ClickByText { selector_string, text } => {
            if let Some(k) = &selector_string.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &text.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::GetHTML { selector_string, time_ms } => {
            if let Some(k) = &selector_string.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &time_ms.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::SwitchToFrame { selector_string } => {
            if let Some(k) = &selector_string.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::NewTab { url } => {
            if let Some(k) = &url.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::SwitchTab { index } => {
            if let Some(k) = &index.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::Type { selector, text } => {
            if let Some(k) = &selector.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &text.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::ClearAndType { selector, text } => {
            if let Some(k) = &selector.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &text.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::NavigateHref { base, href } => {
            if let Some(k) = &base.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &href.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::ScrollDown { scroll } => {
            if let Some(k) = &scroll.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::ScrollUp { scroll } => {
            if let Some(k) = &scroll.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Action::Wait { time_ms } => {
            if let Some(k) = &time_ms.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        _ => {}
    }
}

fn extract_from_extraction(ext: &Extraction, keys: &mut Vec<String>) {
    match ext {
        Extraction::Text { selector_string, field_name } |
        Extraction::Count { selector_string, field_name } |
        Extraction::Exists { selector_string, field_name } |
        Extraction::MultipleText { selector_string, field_name } => {
            if let Some(k) = &selector_string.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &field_name.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
        Extraction::Attribute { selector_string, field_name, attr_str } => {
            if let Some(k) = &selector_string.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &field_name.binding { if !k.is_empty() { keys.push(k.clone()); } }
            if let Some(k) = &attr_str.binding { if !k.is_empty() { keys.push(k.clone()); } }
        }
    }
}

// =========================================================================
// Parse metadata
// =========================================================================

fn parse_mappings_from_metadata(metadata: &[String]) -> Vec<InputMapping> {
    metadata.iter().filter_map(|s| {
        if let Some(rest) = s.strip_prefix("mapping:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() == 2 {
                return Some(InputMapping {
                    binding_key: parts[0].to_string(),
                    source_field: Some(parts[1].to_string()),
                });
            }
        }
        None
    }).collect()
}

// =========================================================================
// BindValue Field Widget
// =========================================================================

fn bind_str_field<'a>(label: &'a str, bv: &BindValue<String>, fid: FieldId) -> Element<'a, EditorMsg, Theme> {
    let is_bound = bv.binding.is_some();
    let binding_key = bv.binding.clone().unwrap_or_default();

    let fid_val = fid.clone();
    let fid_key = fid.clone();
    let fid_tog = fid.clone();

    let bind_input: Element<'a, EditorMsg, Theme> = if is_bound {
        container(
            text_input("binding key", &binding_key)
                .on_input(move |s| EditorMsg::BindStrKeyChanged(fid_key.clone(), s))
                .padding(6).width(Length::Fill),
        ).width(Length::Fill).into()
    } else {
        Space::new().width(0).into()
    };

    let binding_row = row![
        checkbox(is_bound).on_toggle(move |v| EditorMsg::BindStrToggle(fid_tog.clone(), v)),
        Space::new().width(8),
        bind_input,
    ].align_y(iced::Alignment::Center);

    let input_area = container(
        scrollable(
            text_input(label, &bv.value)
                .on_input(move |s| EditorMsg::BindStrValueChanged(fid_val.clone(), s))
                .padding(8).width(Length::Fill),
        ).height(48),
    ).width(Length::Fill).style(input_area_style);

    column![text(label).size(12), input_area, binding_row].spacing(4).into()
}

fn bind_u64_field<'a>(label: &'a str, bv: &BindValue<u64>, fid: FieldId) -> Element<'a, EditorMsg, Theme> {
    let is_bound = bv.binding.is_some();
    let binding_key = bv.binding.clone().unwrap_or_default();
    let val_str = bv.value.to_string();

    let fid_val = fid.clone();
    let fid_key = fid.clone();
    let fid_tog = fid.clone();

    let bind_input: Element<'a, EditorMsg, Theme> = if is_bound {
        container(
            text_input("binding key", &binding_key)
                .on_input(move |s| EditorMsg::BindU64KeyChanged(fid_key.clone(), s))
                .padding(6).width(Length::Fill),
        ).width(Length::Fill).into()
    } else {
        Space::new().width(0).into()
    };

    let binding_row = row![
        checkbox(is_bound).on_toggle(move |v| EditorMsg::BindU64Toggle(fid_tog.clone(), v)),
        Space::new().width(8),
        bind_input,
    ].align_y(iced::Alignment::Center);

    let input_area = container(
        text_input(label, &val_str)
            .on_input(move |s| EditorMsg::BindU64ValueChanged(fid_val.clone(), s))
            .padding(8).width(Length::Fill),
    ).width(Length::Fill).style(input_area_style);

    column![text(label).size(12), input_area, binding_row].spacing(4).into()
}

fn field_label_pick<'a>(label: &'a str, options: Vec<String>, selected: &str, on_change: impl Fn(String) -> EditorMsg + 'a) -> Element<'a, EditorMsg, Theme> {
    column![
        text(label).size(12),
        pick_list(options, Some(selected.to_string()), on_change).width(Length::Fill),
    ].spacing(4).into()
}

fn field_label_input<'a>(label: &'a str, placeholder: &'a str, value: &str, on_change: impl Fn(String) -> EditorMsg + 'a) -> Element<'a, EditorMsg, Theme> {
    column![
        text(label).size(12),
        container(
            text_input(placeholder, value).on_input(on_change).padding(8).width(Length::Fill),
        ).width(Length::Fill).style(input_area_style),
    ].spacing(4).into()
}

// =========================================================================
// Extraction Fields inside Container
// =========================================================================

fn push_extraction_fields_indexed<'a>(mut col: iced::widget::Column<'a, EditorMsg, Theme>, ext: &Extraction, idx: usize) -> iced::widget::Column<'a, EditorMsg, Theme> {
    match ext {
        Extraction::Text { selector_string, field_name } |
        Extraction::Count { selector_string, field_name } |
        Extraction::Exists { selector_string, field_name } |
        Extraction::MultipleText { selector_string, field_name } => {
            col = col.push(indexed_bind_str("Selector", selector_string, idx, FieldId::Selector));
            col = col.push(indexed_bind_str("Field", field_name, idx, FieldId::FieldName));
        }
        Extraction::Attribute { selector_string, field_name, attr_str } => {
            col = col.push(indexed_bind_str("Selector", selector_string, idx, FieldId::Selector));
            col = col.push(indexed_bind_str("Field", field_name, idx, FieldId::FieldName));
            col = col.push(indexed_bind_str("Attr", attr_str, idx, FieldId::AttrStr));
        }
    }
    col
}

fn indexed_bind_str<'a>(label: &'a str, bv: &BindValue<String>, idx: usize, fid: FieldId) -> Element<'a, EditorMsg, Theme> {
    let is_bound = bv.binding.is_some();
    let binding_key = bv.binding.clone().unwrap_or_default();

    let fid_val = fid.clone();
    let fid_key = fid.clone();
    let fid_tog = fid.clone();

    let bind_input: Element<'a, EditorMsg, Theme> = if is_bound {
        container(
            text_input("key", &binding_key)
                .on_input(move |s| EditorMsg::ContainerStepBindStrKey(idx, fid_key.clone(), s))
                .padding(6).width(Length::Fill),
        ).width(Length::Fill).into()
    } else {
        Space::new().width(0).into()
    };

    let binding_row = row![
        checkbox(is_bound).on_toggle(move |v| EditorMsg::ContainerStepBindStrToggle(idx, fid_tog.clone(), v)),
        Space::new().width(8),
        bind_input,
    ].align_y(iced::Alignment::Center);

    let input_area = container(
        scrollable(
            text_input(label, &bv.value)
                .on_input(move |s| EditorMsg::ContainerStepBindStr(idx, fid_val.clone(), s))
                .padding(8).width(Length::Fill),
        ).height(48),
    ).width(Length::Fill).style(input_area_style);

    column![text(label).size(11), input_area, binding_row].spacing(3).into()
}

// =========================================================================
// BindValue mut Access Helper
// =========================================================================

fn find_bind_str<'a>(handler: &'a mut Handler, fid: &FieldId) -> Option<&'a mut BindValue<String>> {
    match handler {
        Handler::Item(Step::Act(action)) => find_bind_str_in_action(action, fid),
        Handler::Item(Step::Extract(ext)) => find_bind_str_in_extraction(ext, fid),
        Handler::Container { selector, .. } if *fid == FieldId::Selector => Some(selector),
        _ => None,
    }
}
fn find_bind_i64<'a>(handler: &'a mut Handler, fid: &FieldId) -> Option<&'a mut BindValue<i64>> {
    match handler {
        Handler::Item(Step::Act(action)) => match (action, fid) {
            (Action::ScrollDown { scroll }, FieldId::Scroll) => Some(scroll),
            (Action::ScrollUp { scroll }, FieldId::Scroll) => Some(scroll),
            _ => None,
        },
        _ => None,
    }
}
fn bind_i64_field<'a>(label: &'a str, bv: &BindValue<i64>, fid: FieldId) -> Element<'a, EditorMsg, Theme> {
    let is_bound = bv.binding.is_some();
    let binding_key = bv.binding.clone().unwrap_or_default();
    let val_str = bv.value.to_string();

    let fid_val = fid.clone();
    let fid_key = fid.clone();
    let fid_tog = fid.clone();

    let bind_input: Element<'a, EditorMsg, Theme> = if is_bound {
        container(
            text_input("binding key", &binding_key)
                .on_input(move |s| EditorMsg::BindI64KeyChanged(fid_key.clone(), s))
                .padding(6).width(Length::Fill),
        ).width(Length::Fill).into()
    } else {
        Space::new().width(0).into()
    };

    let binding_row = row![
        checkbox(is_bound).on_toggle(move |v| EditorMsg::BindI64Toggle(fid_tog.clone(), v)),
        Space::new().width(8),
        bind_input,
    ].align_y(iced::Alignment::Center);

    let input_area = container(
        text_input(label, &val_str)
            .on_input(move |s| EditorMsg::BindI64ValueChanged(fid_val.clone(), s))
            .padding(8).width(Length::Fill),
    ).width(Length::Fill).style(input_area_style);

    column![text(label).size(12), input_area, binding_row].spacing(4).into()
}
fn find_bind_str_in_action<'a>(action: &'a mut Action, fid: &FieldId) -> Option<&'a mut BindValue<String>> {
    match (action, fid) {
        (Action::Click { selector_string }, FieldId::Selector) => Some(selector_string),
        (Action::Navigate { url }, FieldId::Url) => Some(url),
        (Action::WaitFor { selector_string, .. }, FieldId::Selector) => Some(selector_string),
        (Action::ClickByText { selector_string, .. }, FieldId::Selector) => Some(selector_string),
        (Action::ClickByText { text, .. }, FieldId::Text) => Some(text),
        (Action::GetHTML { selector_string, .. }, FieldId::Selector) => Some(selector_string),
        (Action::SwitchToFrame { selector_string }, FieldId::Selector) => Some(selector_string),
        (Action::NewTab { url }, FieldId::Url) => Some(url),
        (Action::Type { selector, .. }, FieldId::Selector) => Some(selector),
        (Action::Type { text, .. }, FieldId::Text) => Some(text),
        (Action::ClearAndType { selector, .. }, FieldId::Selector) => Some(selector),
        (Action::ClearAndType { text, .. }, FieldId::Text) => Some(text),
        (Action::NavigateHref { base, .. }, FieldId::Base) => Some(base),
        (Action::NavigateHref { href, .. }, FieldId::Href) => Some(href),
        _ => None,
    }
}

fn find_bind_str_in_extraction<'a>(ext: &'a mut Extraction, fid: &FieldId) -> Option<&'a mut BindValue<String>> {
    match (ext, fid) {
        (Extraction::Text { selector_string, .. }, FieldId::Selector) => Some(selector_string),
        (Extraction::Text { field_name, .. }, FieldId::FieldName) => Some(field_name),
        (Extraction::Count { selector_string, .. }, FieldId::Selector) => Some(selector_string),
        (Extraction::Count { field_name, .. }, FieldId::FieldName) => Some(field_name),
        (Extraction::Attribute { selector_string, .. }, FieldId::Selector) => Some(selector_string),
        (Extraction::Attribute { field_name, .. }, FieldId::FieldName) => Some(field_name),
        (Extraction::Attribute { attr_str, .. }, FieldId::AttrStr) => Some(attr_str),
        (Extraction::Exists { selector_string, .. }, FieldId::Selector) => Some(selector_string),
        (Extraction::Exists { field_name, .. }, FieldId::FieldName) => Some(field_name),
        (Extraction::MultipleText { selector_string, .. }, FieldId::Selector) => Some(selector_string),
        (Extraction::MultipleText { field_name, .. }, FieldId::FieldName) => Some(field_name),
        _ => None,
    }
}

fn find_bind_u64<'a>(handler: &'a mut Handler, fid: &FieldId) -> Option<&'a mut BindValue<u64>> {
    match handler {
        Handler::Item(Step::Act(action)) => match (action, fid) {
            (Action::WaitFor { time_ms, .. }, FieldId::TimeMs) => Some(time_ms),
            (Action::GetHTML { time_ms, .. }, FieldId::TimeMs) => Some(time_ms),
            (Action::SwitchTab { index }, FieldId::Index) => Some(index),
            (Action::Wait { time_ms }, FieldId::TimeMs) => Some(time_ms),
            _ => None,
        },
        _ => None,
    }
}

// =========================================================================
// Variant Name ↔ Default Value
// =========================================================================


const EXTRACTION_VARIANTS: &[&str] = &["Text", "Count", "Attribute", "Exists", "MultipleText"];

const KEY_ACTION_NAMES: &[&str] = &["Enter", "Tab", "Escape", "Space", "Backspace", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"];

fn action_variant_name(action: &Action) -> &'static str {
    match action {
        Action::Click { .. } => "Click",
        Action::Navigate { .. } => "Navigate",
        Action::WaitFor { .. } => "WaitFor",
        Action::Wait { .. } => "Wait",
        Action::ScrollAll => "ScrollAll",
        Action::ClickByText { .. } => "ClickByText",
        Action::GetHTML { .. } => "GetHTML",
        Action::DismissPermission => "DismissPermission",
        Action::SwitchToDefaultContent => "SwitchToDefaultContent",
        Action::SwitchToFrame { .. } => "SwitchToFrame",
        Action::Refresh => "Refresh",
        Action::Forward => "Forward",
        Action::Backward => "Backward",
        Action::NewTab { .. } => "NewTab",
        Action::SwitchTab { .. } => "SwitchTab",
        Action::CloseTab => "CloseTab",
        Action::SwitchToLastTab => "SwitchToLastTab",
        Action::Type { .. } => "Type",
        Action::PressKey { .. } => "PressKey",
        Action::ClearAndType { .. } => "ClearAndType",
        Action::NavigateHref { .. } => "NavigateHref",
        Action::ScrollDown { .. } => "ScrollDown",
        Action::ScrollUp { .. } => "ScrollUp",


    }
}

fn extraction_variant_name(ext: &Extraction) -> &'static str {
    match ext {
        Extraction::Text { .. } => "Text",
        Extraction::Count { .. } => "Count",
        Extraction::Attribute { .. } => "Attribute",
        Extraction::Exists { .. } => "Exists",
        Extraction::MultipleText { .. } => "MultipleText",
    }
}

fn default_action_for_variant(variant: &str) -> Action {
    match variant {
        "Click" => Action::Click { selector_string: BindValue::new(String::new()) },
        "Navigate" => Action::Navigate { url: BindValue::new(String::new()) },
        "WaitFor" => Action::WaitFor { selector_string: BindValue::new(String::new()), time_ms: BindValue::new(500) },
        "Wait" => Action::Wait { time_ms: BindValue::new(1000) },
        "ScrollAll" => Action::ScrollAll,
        "ClickByText" => Action::ClickByText { selector_string: BindValue::new(String::new()), text: BindValue::new(String::new()) },
        "GetHTML" => Action::GetHTML { selector_string: BindValue::new(String::new()), time_ms: BindValue::new(500) },
        "DismissPermission" => Action::DismissPermission,
        "SwitchToDefaultContent" => Action::SwitchToDefaultContent,
        "SwitchToFrame" => Action::SwitchToFrame { selector_string: BindValue::new(String::new()) },
        "Refresh" => Action::Refresh,
        "Forward" => Action::Forward,
        "Backward" => Action::Backward,
        "NewTab" => Action::NewTab { url: BindValue::new(String::new()) },
        "SwitchTab" => Action::SwitchTab { index: BindValue::new(0) },
        "CloseTab" => Action::CloseTab,
        "SwitchToLastTab" => Action::SwitchToLastTab,
        "Type" => Action::Type { selector: BindValue::new(String::new()), text: BindValue::new(String::new()) },
        "PressKey" => Action::PressKey { key: KeyAction::Enter },
        "ClearAndType" => Action::ClearAndType { selector: BindValue::new(String::new()), text: BindValue::new(String::new()) },
        "NavigateHref" => Action::NavigateHref { 
            base: BindValue::new(String::new()), 
            href: BindValue::new(String::new()) 
        },
        "ScrollDown" => Action::ScrollDown { scroll: BindValue::new(500) },
        "ScrollUp" => Action::ScrollUp { scroll: BindValue::new(500) },
        _ => Action::Click { selector_string: BindValue::new(String::new()) },
    }
}

fn default_extraction_for_variant(variant: &str) -> Extraction {
    match variant {
        "Text" => Extraction::Text { selector_string: BindValue::new(String::new()), field_name: BindValue::new(String::new()) },
        "Count" => Extraction::Count { selector_string: BindValue::new(String::new()), field_name: BindValue::new(String::new()) },
        "Attribute" => Extraction::Attribute { selector_string: BindValue::new(String::new()), field_name: BindValue::new(String::new()), attr_str: BindValue::new(String::new()) },
        "Exists" => Extraction::Exists { selector_string: BindValue::new(String::new()), field_name: BindValue::new(String::new()) },
        "MultipleText" => Extraction::MultipleText { selector_string: BindValue::new(String::new()), field_name: BindValue::new(String::new()) },
        _ => Extraction::Text { selector_string: BindValue::new(String::new()), field_name: BindValue::new(String::new()) },
    }
}

fn key_action_name(key: &KeyAction) -> &'static str {
    match key {
        KeyAction::Enter => "Enter", KeyAction::Tab => "Tab", KeyAction::Escape => "Escape",
        KeyAction::Space => "Space", KeyAction::Backspace => "Backspace",
        KeyAction::ArrowUp => "ArrowUp", KeyAction::ArrowDown => "ArrowDown",
        KeyAction::ArrowLeft => "ArrowLeft", KeyAction::ArrowRight => "ArrowRight",
    }
}

fn parse_key_action(s: &str) -> KeyAction {
    match s {
        "Enter" => KeyAction::Enter, "Tab" => KeyAction::Tab, "Escape" => KeyAction::Escape,
        "Space" => KeyAction::Space, "Backspace" => KeyAction::Backspace,
        "ArrowUp" => KeyAction::ArrowUp, "ArrowDown" => KeyAction::ArrowDown,
        "ArrowLeft" => KeyAction::ArrowLeft, "ArrowRight" => KeyAction::ArrowRight,
        _ => KeyAction::Enter,
    }
}

// =========================================================================
// Styles
// =========================================================================

fn input_area_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.2))),
        border: Border { color: iced::Color::from_rgba(0.4, 0.4, 0.45, 0.5), width: 1.0, radius: 6.0.into() },
        ..Default::default()
    }
}

fn hint_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.3, 0.3, 0.4, 0.3))),
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn section_header_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.15, 0.2, 0.25, 0.6))),
        border: Border { color: iced::Color::from_rgba(0.3, 0.4, 0.5, 0.5), width: 1.0, radius: 6.0.into() },
        ..Default::default()
    }
}