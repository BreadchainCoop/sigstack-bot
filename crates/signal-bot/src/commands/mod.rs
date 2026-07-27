//! Bot command handlers.

mod help;
mod menu_locale;
mod models;
mod privacy;
mod product_menus;
mod set_language;
mod translate;
mod translate_all;
pub mod translate_lang;
mod translate_langs;
mod translate_me;
mod translate_parallel;
mod translate_service;
mod verify;

pub use help::HelpHandler;
pub use models::ModelsHandler;
pub use privacy::PrivacyHandler;
pub use product_menus::{
    InChatMenuHandler, ParallelMenuHandler, TranscriptionStubHandler, TranslationMenuHandler,
};
pub use set_language::SetLanguageHandler;
pub use signal_bot_core::CommandHandler;
pub use translate::TranslateHandler;
pub use translate_all::TranslateAllHandler;
pub use translate_langs::TranslateLangsHandler;
pub use translate_me::TranslateMeHandler;
pub use translate_parallel::TranslateParallelHandler;
pub use verify::VerifyHandler;
