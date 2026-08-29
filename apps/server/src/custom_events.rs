//! Casos de uso do Event Builder, separados por responsabilidade.

#[path = "custom_events/core.rs"]
mod core;
#[path = "custom_events/deletion.rs"]
mod deletion;
#[path = "custom_events/draft.rs"]
mod draft;
#[path = "custom_events/draft_view.rs"]
mod draft_view;
#[path = "custom_events/item_types.rs"]
mod item_types;
#[path = "custom_events/items.rs"]
mod items;
#[path = "custom_events/metadata.rs"]
mod metadata;
#[path = "custom_events/options.rs"]
mod options;
#[path = "custom_events/publication.rs"]
mod publication;

pub use core::{BuilderDraft, BuilderItem, BuilderOption, BuilderOptionLink, BuilderVersion};
pub use deletion::{delete, delete_admin, EventDeletionResult};
pub use draft::{available, create, get, mine};
pub use draft_view::draft;
pub use item_types::{add_multiple_choice_item, add_numeric_item};
pub use items::{add_item, delete_item, move_item, update_item};
pub use metadata::update_metadata;
pub use options::{add_option, delete_option, move_option, update_option, update_option_media};
pub use publication::publish;
