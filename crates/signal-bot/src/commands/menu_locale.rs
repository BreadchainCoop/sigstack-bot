//! Localized `!help`, product, and `!privacy` menu text.
//!
//! Command-list layout follows the Signal mobile menu standard:
//! [`docs/solutions/signal-mobile-menus.md`](../../../../docs/solutions/signal-mobile-menus.md).

use crate::config::BotRole;
use crate::group_preferences_store::GroupPreferencesStore;
use crate::menu_language::MenuLanguage;
use signal_client::BotMessage;

pub fn menu_language_for_message(
    message: &BotMessage,
    group_prefs: &GroupPreferencesStore,
) -> MenuLanguage {
    message
        .group_id
        .as_deref()
        .map(|group_id| group_prefs.get_menu_language(group_id))
        .unwrap_or_default()
}

pub fn help_menu(language: MenuLanguage, role: BotRole) -> &'static str {
    match (role, language) {
        (BotRole::Transcription, MenuLanguage::En) => HELP_TRANSCRIPTION_EN,
        (BotRole::Transcription, MenuLanguage::Es) => HELP_TRANSCRIPTION_ES,
        (BotRole::Translation, MenuLanguage::En) => HELP_HUB_EN,
        (BotRole::Translation, MenuLanguage::Es) => HELP_HUB_ES,
    }
}

pub fn translation_products_menu(
    language: MenuLanguage,
    translate_all_enabled: bool,
) -> &'static str {
    match (language, translate_all_enabled) {
        (MenuLanguage::En, true) => TRANSLATION_MENU_EN,
        (MenuLanguage::En, false) => TRANSLATION_MENU_AUTO_DISABLED_EN,
        (MenuLanguage::Es, true) => TRANSLATION_MENU_ES,
        (MenuLanguage::Es, false) => TRANSLATION_MENU_AUTO_DISABLED_ES,
    }
}

pub fn transcription_unavailable(language: MenuLanguage) -> &'static str {
    match language {
        MenuLanguage::En => TRANSCRIPTION_UNAVAILABLE_EN,
        MenuLanguage::Es => TRANSCRIPTION_UNAVAILABLE_ES,
    }
}

pub fn transcription_invited(language: MenuLanguage) -> &'static str {
    match language {
        MenuLanguage::En => TRANSCRIPTION_INVITED_EN,
        MenuLanguage::Es => TRANSCRIPTION_INVITED_ES,
    }
}

pub fn transcription_group_only(language: MenuLanguage) -> &'static str {
    match language {
        MenuLanguage::En => TRANSCRIPTION_GROUP_ONLY_EN,
        MenuLanguage::Es => TRANSCRIPTION_GROUP_ONLY_ES,
    }
}

pub fn privacy_menu(language: MenuLanguage, role: BotRole) -> &'static str {
    match (role, language) {
        (BotRole::Transcription, MenuLanguage::En) => PRIVACY_TRANSCRIPTION_EN,
        (BotRole::Transcription, MenuLanguage::Es) => PRIVACY_TRANSCRIPTION_ES,
        (BotRole::Translation, MenuLanguage::En) => PRIVACY_TRANSLATION_EN,
        (BotRole::Translation, MenuLanguage::Es) => PRIVACY_TRANSLATION_ES,
    }
}

/// Exact command match (avoids `!translation` matching `!translation-on`).
pub fn is_exact_command(text: &str, command: &str) -> bool {
    text.trim() == command
}

const HELP_TRANSCRIPTION_EN: &str = r#"Voice transcription

Voice notes in this chat are transcribed to text (Whisper, inside the TEE).

!transcription
  This menu
!transcribe-on / !transcribe-off
  Toggle auto transcription
!transcribe
  Quote a voice note to transcribe
!privacy
  Privacy & TEE
!help
  Show this menu
!verify <challenge>
  TEE attestation"#;

const HELP_TRANSCRIPTION_ES: &str = r#"Transcripción de voz

Las notas de voz en este chat se transcriben a texto (Whisper, dentro del TEE).

!transcription
  Este menú
!transcribe-on / !transcribe-off
  Activar/desactivar auto-transcripción
!transcribe
  Cita una nota de voz para transcribir
!privacy
  Privacidad y TEE
!help
  Mostrar este menú
!verify <challenge>
  Attestation TEE"#;

const HELP_HUB_EN: &str = r#"Sigstack

!translation
  Translation
!transcription
  Voice transcription
!privacy
  Privacy & TEE
!set-en / !set-es
  Menu language
!help
  Show this menu"#;

const HELP_HUB_ES: &str = r#"Sigstack

!translation
  Traducción
!transcription
  Transcripción de voz
!privacy
  Privacidad y TEE
!set-en / !set-es
  Idioma del menú
!help
  Mostrar este menú"#;

const TRANSLATION_MENU_EN: &str = r#"Translation

Language Threads (recommended)
Multilingual main + language sidecars.

!translate-me-on <lang>
  Join/create a Language Thread (from main)
!translate-me-off
  Leave your Language Thread
!list-langs
  Language codes

In-chat (same group only)
Stay in this thread; auto or quote one message.

!translate-on <lang1> <lang2>
  e.g. !translate-on es en
!translate-off
  Stop auto-translate
!translate <lang>
  Reply to a message

!models
  List AI models
!verify <challenge>
  TEE attestation
!help
  Main menu"#;

const TRANSLATION_MENU_ES: &str = r#"Traducción

Language Threads (recomendado)
Principal multilingüe + sidecars por idioma.

!translate-me-on <lang>
  Unirte/crear Language Thread (principal)
!translate-me-off
  Salir de tu Language Thread
!list-langs
  Códigos de idioma

En el chat (solo este grupo)
Quédate en este hilo; auto o cita un mensaje.

!translate-on <lang1> <lang2>
  ej. !translate-on es en
!translate-off
  Detener auto-traducción
!translate <lang>
  Responde a un mensaje

!models
  Listar modelos de IA
!verify <challenge>
  Attestation TEE
!help
  Menú principal"#;

const TRANSLATION_MENU_AUTO_DISABLED_EN: &str = r#"Translation

Language Threads (recommended)
Multilingual main + language sidecars.

!translate-me-on <lang>
  Join/create a Language Thread (from main)
!translate-me-off
  Leave your Language Thread
!list-langs
  Language codes

In-chat (same group only)
Auto-translate is disabled on this bot (!translate-on).

!translate <lang>
  Reply to a message

!models
  List AI models
!verify <challenge>
  TEE attestation
!help
  Main menu"#;

const TRANSLATION_MENU_AUTO_DISABLED_ES: &str = r#"Traducción

Language Threads (recomendado)
Principal multilingüe + sidecars por idioma.

!translate-me-on <lang>
  Unirte/crear Language Thread (principal)
!translate-me-off
  Salir de tu Language Thread
!list-langs
  Códigos de idioma

En el chat (solo este grupo)
La auto-traducción está desactivada en este bot (!translate-on).

!translate <lang>
  Responde a un mensaje

!models
  Listar modelos de IA
!verify <challenge>
  Attestation TEE
!help
  Menú principal"#;

const TRANSCRIPTION_UNAVAILABLE_EN: &str = r#"Voice transcription is currently unavailable.

The transcription bot is not paired with this group yet. Meanwhile, try translation:

!translation
  Translation

!help
  Main menu"#;

const TRANSCRIPTION_UNAVAILABLE_ES: &str = r#"La transcripción de voz no está disponible por ahora.

El bot de transcripción aún no está emparejado con este grupo. Mientras tanto, prueba la traducción:

!translation
  Traducción

!help
  Menú principal"#;

const TRANSCRIPTION_INVITED_EN: &str = r#"Invited the transcription bot to this group.

Accept the Signal invite on that number, then send !transcription again (the transcription bot will answer with its menu).

!help
  Main menu"#;

const TRANSCRIPTION_INVITED_ES: &str = r#"Se invitó al bot de transcripción a este grupo.

Acepta la invitación de Signal en ese número y luego envía !transcription de nuevo (el bot de transcripción responderá con su menú).

!help
  Menú principal"#;

const TRANSCRIPTION_GROUP_ONLY_EN: &str = r#"Voice transcription pairing works in a Signal group.

Add both bots to a group, then send !transcription there.

!help
  Main menu"#;

const TRANSCRIPTION_GROUP_ONLY_ES: &str = r#"El emparejamiento de transcripción funciona en un grupo de Signal.

Añade ambos bots a un grupo y envía !transcription allí.

!help
  Menú principal"#;

const PRIVACY_TRANSCRIPTION_EN: &str = r#"**Sigstack transcription** (Private & Verifiable)

**TEE Commands:**
!verify <challenge>
  Get TEE attestation with your challenge

**Privacy:**
Voice notes are decrypted by Signal CLI inside this TEE and transcribed with Whisper in the same CVM. Text transcripts are posted back to Signal.

Neither the bot operator nor the host can read decrypted audio or text in TEE memory.

Pair with the translation bot in the same group if you also want translation."#;

const PRIVACY_TRANSCRIPTION_ES: &str = r#"**Sigstack transcripción** (Privado y verificable)

**Comandos TEE:**
!verify <challenge>
  Obtener attestation TEE con tu challenge

**Privacidad:**
Las notas de voz se descifran con Signal CLI dentro de este TEE y se transcriben con Whisper en el mismo CVM. El texto se publica de nuevo en Signal.

Ni el operador del bot ni el host pueden leer el audio o texto descifrado en la memoria del TEE.

Empareja con el bot de traducción en el mismo grupo si también quieres traducción."#;

const PRIVACY_TRANSLATION_EN: &str = r#"**Sigstack translation** (Private & Verifiable)

**TEE Commands:**
!models
  List AI models
!verify <challenge>
  Get TEE attestation with your challenge

**Verification:**
`!verify my-random-text` to get cryptographic proof this bot runs in a TEE. Your challenge is embedded in the TDX quote.

**Privacy:**
Messages are end-to-end encrypted via Signal, processed in a verified TEE (Intel TDX), and translated via NEAR AI Cloud private inference (NVIDIA GPU TEE).

Voice transcription is a separate bot/CVM. This bot only acts on text (including transcripts posted by the transcription bot).

Neither the bot operator nor NEAR AI can read your messages in plaintext outside the TEEs.

!help
  Main menu"#;

const PRIVACY_TRANSLATION_ES: &str = r#"**Sigstack traducción** (Privado y verificable)

**Comandos TEE:**
!models
  Listar modelos de IA
!verify <challenge>
  Obtener attestation TEE con tu challenge

**Verificación:**
`!verify my-random-text` para obtener prueba criptográfica de que este bot corre en un TEE. Tu challenge se incluye en la cita TDX.

**Privacidad:**
Los mensajes van cifrados de extremo a extremo con Signal, se procesan en un TEE verificado (Intel TDX) y se traducen vía inferencia privada de NEAR AI Cloud (NVIDIA GPU TEE).

La transcripción de voz es un bot/CVM aparte. Este bot solo actúa sobre texto (incluidas las transcripciones del bot de transcripción).

Ni el operador del bot ni NEAR AI pueden leer tus mensajes en texto plano fuera de los TEEs.

!help
  Menú principal"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_translation_en_is_hub() {
        let h = help_menu(MenuLanguage::En, BotRole::Translation);
        assert!(h.contains("!translation"));
        assert!(h.contains("!transcription"));
        assert!(h.contains("!privacy"));
        assert!(!h.contains("!translate-me-on"));
        assert!(!h.contains("!transcribe-on"));
        assert!(
            h.contains("!translation\n  Translation"),
            "hub commands should use stacked layout"
        );
        assert!(h.contains("!set-en / !set-es\n  Menu language"));
        assert!(h.contains("!help\n  Show this menu"));
        assert!(!h.contains("!translation —"));
        assert!(!h.contains("Menu language:"));
        assert!(!h.contains("!help —"));
    }

    #[test]
    fn translation_menu_leads_with_language_threads() {
        let h = translation_products_menu(MenuLanguage::En, true);
        assert!(h.contains("Language Threads (recommended)"));
        assert!(h.contains("!translate-me-on"));
        assert!(h.contains("!translate-me-off"));
        assert!(h.contains("!translate-on"));
        assert!(h.contains("!translate <lang>"));
        assert!(!h.contains("!parallel"));
        assert!(!h.contains("!in-chat"));
        let lt = h.find("Language Threads").expect("lt");
        let in_chat = h.find("In-chat").expect("in-chat section");
        assert!(
            lt < in_chat,
            "Language Threads should appear before In-chat"
        );
        assert!(
            h.contains("!translate-me-on <lang>\n  "),
            "translation menu should use stacked layout"
        );
        assert!(!h.contains("!translate-me-on <lang> —"));
    }

    #[test]
    fn translation_menu_auto_disabled_hides_translate_on() {
        let h = translation_products_menu(MenuLanguage::En, false);
        assert!(h.contains("!translate-me-on"));
        assert!(h.contains("Auto-translate is disabled"));
        assert!(!h.contains("!translate-on <lang1>"));
        assert!(h.contains("!translate <lang>"));
        assert!(h.contains("!translate <lang>\n  "));
    }

    #[test]
    fn exact_command_does_not_match_prefixed() {
        assert!(is_exact_command("!translation", "!translation"));
        assert!(!is_exact_command("!translation-on es en", "!translation"));
        assert!(is_exact_command("!in-chat", "!in-chat"));
        assert!(!is_exact_command("!in-chat-extra", "!in-chat"));
    }

    #[test]
    fn help_transcription_en_covers_voice() {
        let h = help_menu(MenuLanguage::En, BotRole::Transcription);
        assert!(h.contains("!transcribe"));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!translate-me-on"));
        assert!(h.contains("!transcribe-on / !transcribe-off\n  "));
        assert!(!h.contains("!transcribe-on / !transcribe-off —"));
    }

    #[test]
    fn help_es_hub() {
        let h = help_menu(MenuLanguage::Es, BotRole::Translation);
        assert!(h.contains("!translation"));
        assert!(!h.contains("!ask"));
        assert!(h.contains("!translation\n  Traducción"));
    }

    #[test]
    fn privacy_menus_cover_roles_and_languages() {
        let en = privacy_menu(MenuLanguage::En, BotRole::Transcription);
        assert!(en.contains("Sigstack transcription"));
        assert!(en.contains("!verify <challenge>\n  "));
        assert!(!en.contains("!verify <challenge> -"));
        let es = privacy_menu(MenuLanguage::Es, BotRole::Translation);
        assert!(es.contains("Sigstack traducción"));
        assert!(es.contains("!models\n  "));
    }

    #[test]
    fn transcription_unavailable_offers_translation() {
        let m = transcription_unavailable(MenuLanguage::En);
        assert!(m.contains("unavailable"));
        assert!(m.contains("!translation"));
        assert!(m.contains("!translation\n  Translation"));
        assert!(!m.contains("!translation —"));
    }

    #[test]
    fn menu_language_for_message_uses_group_pref_or_default() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_menu_language("g-es", MenuLanguage::Es);

        let dm = BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: "!help".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        };
        assert_eq!(menu_language_for_message(&dm, &store), MenuLanguage::En);

        let mut group = dm.clone();
        group.is_group = true;
        group.group_id = Some("g-es".into());
        assert_eq!(menu_language_for_message(&group, &store), MenuLanguage::Es);
    }
}
