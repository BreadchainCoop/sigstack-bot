//! Localized `!help` and `!privacy` menu text.

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
        (BotRole::Translation, MenuLanguage::En) => HELP_TRANSLATION_EN,
        (BotRole::Translation, MenuLanguage::Es) => HELP_TRANSLATION_ES,
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

const HELP_TRANSCRIPTION_EN: &str = r#"Voice transcription:

Voice notes in this chat are transcribed to text (Whisper, inside the TEE).

**Commands:**
- !transcribe-on / !transcribe-off — toggle auto transcription
- !transcribe — quote a voice note to transcribe it
- !privacy — Privacy & TEE
- !help — Show this menu
- !verify <challenge> — TEE attestation"#;

const HELP_TRANSCRIPTION_ES: &str = r#"Transcripción de voz:

Las notas de voz en este chat se transcriben a texto (Whisper, dentro del TEE).

**Comandos:**
- !transcribe-on / !transcribe-off — activar/desactivar transcripción automática
- !transcribe — cita una nota de voz para transcribirla
- !privacy — Privacidad y TEE
- !help — Mostrar este menú
- !verify <challenge> — attestation TEE"#;

const HELP_TRANSLATION_EN: &str = r#"Translation:

**In-chat (main thread):**
- !translate-on <lang1> <lang2>
- !translate-off

**Language Threads (sidecar groups):**
- !translate-me-on <lang>
- !translate-me-off
- !list-langs

example:
!translate-me-on es

**Quote translate:**
- !translate <lang> (reply to a message)

**Menu language**
- !set-es — español
- !set-en — english

**Other**
- !privacy — Privacy & TEE
- !help — Show this menu
- !models — List NEAR AI models
- !verify <challenge> — TEE attestation"#;

const HELP_TRANSLATION_ES: &str = r#"Traducción:

**En el hilo principal:**
- !translate-on <lang1> <lang2>
- !translate-off

**Hilos de idioma (grupos sidecar):**
- !translate-me-on <lang>
- !translate-me-off
- !list-langs

ejemplo:
!translate-me-on es

**Traducir citando:**
- !translate <lang> (responde a un mensaje)

**Idioma del menú**
- !set-es — español
- !set-en — english

**Otros**
- !privacy — Privacidad y TEE
- !help — Mostrar este menú
- !models — Listar modelos NEAR AI
- !verify <challenge> — attestation TEE"#;

const PRIVACY_TRANSCRIPTION_EN: &str = r#"**Sigstack transcription** (Private & Verifiable)

**TEE Commands:**
- !verify <challenge> - Get TEE attestation with your challenge

**Privacy:**
Voice notes are decrypted by Signal CLI inside this TEE and transcribed with Whisper in the same CVM. Text transcripts are posted back to Signal.

Neither the bot operator nor the host can read decrypted audio or text in TEE memory.

Pair with the translation bot in the same group if you also want translation."#;

const PRIVACY_TRANSCRIPTION_ES: &str = r#"**Sigstack transcripción** (Privado y verificable)

**Comandos TEE:**
- !verify <challenge> - Obtener attestation TEE con tu challenge

**Privacidad:**
Las notas de voz se descifran con Signal CLI dentro de este TEE y se transcriben con Whisper en el mismo CVM. El texto se publica de nuevo en Signal.

Ni el operador del bot ni el host pueden leer el audio o texto descifrado en la memoria del TEE.

Empareja con el bot de traducción en el mismo grupo si también quieres traducción."#;

const PRIVACY_TRANSLATION_EN: &str = r#"**Sigstack translation** (Private & Verifiable)

**TEE Commands:**
- !models - List AI models
- !verify <challenge> - Get TEE attestation with your challenge

**Verification:**
`!verify my-random-text` to get cryptographic proof this bot runs in a TEE. Your challenge is embedded in the TDX quote.

**Privacy:**
Messages are end-to-end encrypted via Signal, processed in a verified TEE (Intel TDX), and translated via NEAR AI Cloud private inference (NVIDIA GPU TEE).

Voice transcription is a separate bot/CVM. This bot only acts on text (including transcripts posted by the transcription bot).

Neither the bot operator nor NEAR AI can read your messages in plaintext outside the TEEs."#;

const PRIVACY_TRANSLATION_ES: &str = r#"**Sigstack traducción** (Privado y verificable)

**Comandos TEE:**
- !models - Listar modelos de IA
- !verify <challenge> - Obtener attestation TEE con tu challenge

**Verificación:**
`!verify my-random-text` para obtener prueba criptográfica de que este bot corre en un TEE. Tu challenge se incluye en la cita TDX.

**Privacidad:**
Los mensajes están cifrados de extremo a extremo con Signal, se procesan en un TEE verificado (Intel TDX) y se traducen vía inferencia privada de NEAR AI Cloud (NVIDIA GPU TEE).

La transcripción de voz es un bot/CVM aparte. Este bot solo actúa sobre texto (incluidas las transcripciones del bot de transcripción).

Ni el operador del bot ni NEAR AI pueden leer tus mensajes en texto plano fuera de los TEEs."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_translation_en_covers_products() {
        let h = help_menu(MenuLanguage::En, BotRole::Translation);
        assert!(h.contains("!translate-me-on"));
        assert!(h.contains("!list-langs"));
        assert!(h.contains("!translate-on"));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!transcribe"));
    }

    #[test]
    fn help_transcription_en_covers_voice() {
        let h = help_menu(MenuLanguage::En, BotRole::Transcription);
        assert!(h.contains("!transcribe"));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!translate-me-on"));
    }

    #[test]
    fn help_es_translation_covers_sidecars() {
        let h = help_menu(MenuLanguage::Es, BotRole::Translation);
        assert!(h.contains("!translate-me-on"));
        assert!(!h.contains("!ask"));
    }

    #[test]
    fn privacy_menus_cover_roles_and_languages() {
        let en = privacy_menu(MenuLanguage::En, BotRole::Transcription);
        assert!(en.contains("Sigstack transcription"));
        let es = privacy_menu(MenuLanguage::Es, BotRole::Translation);
        assert!(es.contains("Sigstack traducción"));
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
