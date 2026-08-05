//! Bot command handlers.

mod help;
mod menu_locale;
mod privacy;
mod product_menus;
mod rename;
mod translate;
mod translate_all;
pub mod translate_lang;
mod translate_langs;
mod translate_me;
mod translate_service;
mod verify;

pub use help::{ExplainHandler, HelpHandler};
pub use privacy::PrivacyHandler;
pub use product_menus::{
    InChatMenuHandler, TranscriptionMenuHandler, TranscriptionPairingHandler,
    TranslationInChatMenuHandler, TranslationMenuHandler, TranslationThreadsMenuHandler,
};
pub use rename::RenameHandler;
pub use signal_bot_core::CommandHandler;
pub use translate::TranslateHandler;
pub use translate_all::TranslateAllHandler;
pub use translate_langs::TranslateLangsHandler;
pub use translate_me::TranslateMeHandler;
pub use verify::VerifyHandler;
