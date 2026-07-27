//! Localized `!help` and `!privacy` menu text.

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

pub fn help_menu(language: MenuLanguage) -> &'static str {
    match language {
        MenuLanguage::En => HELP_EN,
        MenuLanguage::Es => HELP_ES,
    }
}

pub fn privacy_menu(language: MenuLanguage) -> &'static str {
    match language {
        MenuLanguage::En => PRIVACY_EN,
        MenuLanguage::Es => PRIVACY_ES,
    }
}

const HELP_EN: &str = r#"Translation Threads:

Join a per-language Signal group bridged to this main chat.

**Commands:**
- !translate-me-on <lang>
- !translate-me-off
- !list-langs

example:
!translate-me-on es

**Default Language**
- !set-es — español
- !set-en — english

**Command Menus**
- !privacy — Privacy & TEE
- !help — Show this menu"#;

const HELP_ES: &str = r#"Hilos de traducción:

Únete a un grupo Signal por idioma, conectado a este chat principal.

**Commands:**
- !translate-me-on <lang>
- !translate-me-off
- !list-langs

ejemplo:
!translate-me-on es

**Idioma predeterminado**
- !set-es — español
- !set-en — english

**Menús de comandos**
- !privacy — Privacidad y TEE
- !help — Mostrar este menú"#;

const PRIVACY_EN: &str = r#"**Bread Coop AI** (Private & Verifiable)

**TEE Commands:**
- !models - List AI models
- !clear - Clear chat history
- !verify <challenge> - Get TEE attestation with your challenge

**Command Menus**
- !privacy - Show this message
- !help - Show feature menu

**Verification:**
`!verify my-random-text` to get cryptographic proof this bot runs in a TEE. Your challenge is embedded in the TDX quote, proving the attestation was generated fresh for you.

**Privacy:**
Your messages are end-to-end encrypted via Signal, processed in a verified TEE (Intel TDX), and sent to NEAR AI Cloud's private inference (NVIDIA GPU TEE).

Voice transcription runs locally in the TEE (Whisper). Translation uses NEAR AI on text only.

Neither the bot operator nor NEAR AI can read your messages."#;

const PRIVACY_ES: &str = r#"**Bread Coop AI** (Privado y verificable)

**Comandos TEE:**
- !models - Listar modelos de IA
- !clear - Borrar historial del chat
- !verify <challenge> - Obtener attestation TEE con tu challenge

**Menús de comandos**
- !privacy - Mostrar este mensaje
- !help - Mostrar menú de funciones

**Verificación:**
`!verify my-random-text` para obtener prueba criptográfica de que este bot corre en un TEE. Tu challenge se incluye en la cita TDX, demostrando que la attestation se generó en tiempo real para ti.

**Privacidad:**
Tus mensajes están cifrados de extremo a extremo con Signal, se procesan en un TEE verificado (Intel TDX) y se envían a la inferencia privada de NEAR AI Cloud (NVIDIA GPU TEE).

La transcripción de voz corre localmente en el TEE (Whisper). La traducción usa NEAR AI solo con texto.

Ni el operador del bot ni NEAR AI pueden leer tus mensajes."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_en_covers_sidecars_omits_legacy() {
        let h = help_menu(MenuLanguage::En);
        assert!(h.contains("!translate-me-on"));
        assert!(h.contains("!list-langs"));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!transcribe"));
        assert!(!h.contains("!translate-on"));
        assert!(!h.contains("list-langs-common"));
    }

    #[test]
    fn help_es_covers_sidecars() {
        let h = help_menu(MenuLanguage::Es);
        assert!(h.contains("!translate-me-on"));
        assert!(!h.contains("!ask"));
    }
}
