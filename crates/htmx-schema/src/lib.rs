//! Declarative schema and documentation engine for HTMX 2.0.

pub mod attribute;
pub mod extensions;
pub mod registry;

pub use attribute::{
    AttributeCategory, AttributeValueType, HtmxAttributeDefinition, ModifierDefinition,
};
pub use extensions::{all_extensions, get_extension, ExtensionDefinition, EXTENSIONS};
pub use registry::{
    all_attributes, get_attribute, ATTRIBUTES, SWAP_MODIFIERS, SWAP_STYLES, TARGET_KEYWORDS,
    TRIGGER_EVENTS, TRIGGER_MODIFIERS,
};
