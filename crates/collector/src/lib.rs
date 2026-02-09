// crates/collector/src/lib.rs

pub mod workflow;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceFile {
    pub main_sequences: HashMap<String, workflow::Sequence>,
    pub sub_sequences: HashMap<String, workflow::Sequence>,
}

impl SequenceFile {
    pub fn new(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
    pub fn empty() -> Self {
        Self {
            main_sequences: HashMap::new(),
            sub_sequences: HashMap::new(),
        }
    }
    pub fn get_main(&self, name: &str) -> Option<&workflow::Sequence> {
        self.main_sequences.get(name)
    }

    pub fn get_sub(&self, name: &str) -> Option<&workflow::Sequence> {
        self.sub_sequences.get(name)
    }

    pub fn insert_main(&mut self, seq: workflow::Sequence) {
        self.main_sequences.insert(seq.sequence_name.clone(), seq);
    }

    pub fn insert_sub(&mut self, seq: workflow::Sequence) {
        self.sub_sequences.insert(seq.sequence_name.clone(), seq);
    }
    
    pub fn remove_main(&mut self, name: &str) -> Option<workflow::Sequence> {
        self.main_sequences.remove(name)
    }
}
// -------------------------------------------------------------------------
// [Modified] The modules below are compiled only when the 'ui' feature is enabled.
// This ensures no errors occur if 'iced' is missing; they are simply ignored.
// -------------------------------------------------------------------------
#[cfg(feature = "ui")]
pub mod styles;
#[cfg(feature = "ui")]
pub mod handler_card;
#[cfg(feature = "ui")]
pub mod handler_editor;
#[cfg(feature = "ui")]
pub mod handler_list;
#[cfg(feature = "ui")]
pub mod test_input_card;

// -------------------------------------------------------------------------
// Re-exports
// -------------------------------------------------------------------------

// [Modified] Re-exports are also done only when the 'ui' feature is present.
#[cfg(feature = "ui")]
pub use handler_card::{handler_card, empty_handler_card, compact_handler_card};

#[cfg(feature = "ui")]
pub use handler_list::{handler_list, compact_handler_list, empty_handler_list, mixed_handler_list};