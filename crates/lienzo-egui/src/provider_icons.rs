//! Embedded provider identity artwork shared by native rendering and exports.
//!
//! SVGL is the primary source; MiniMax and Z.ai use the MIT-licensed Lobe
//! fallback. Provenance and exact upstream hashes
//! live beside the assets; providers never download icons while painting.

use lienzo::Icon;

pub(crate) fn svg_for_icon(icon: Icon) -> Option<&'static str> {
    Some(match icon {
        Icon::Codex => include_str!("../assets/providers/icons/codex.svg"),
        Icon::OpenAi => include_str!("../assets/providers/icons/openai.svg"),
        Icon::OpenRouter => include_str!("../assets/providers/icons/openrouter.svg"),
        Icon::OpenCode => include_str!("../assets/providers/icons/opencode.svg"),
        Icon::Nvidia => include_str!("../assets/providers/icons/nvidia.svg"),
        Icon::DeepSeek => include_str!("../assets/providers/icons/deepseek.svg"),
        Icon::MiniMax => include_str!("../assets/providers/icons/minimax.svg"),
        Icon::Zai => include_str!("../assets/providers/icons/zai.svg"),
        Icon::Anthropic => include_str!("../assets/providers/icons/anthropic.svg"),
        Icon::Groq => include_str!("../assets/providers/icons/groq.svg"),
        Icon::Mistral => include_str!("../assets/providers/icons/mistral.svg"),
        Icon::Xai => include_str!("../assets/providers/icons/xai.svg"),
        _ => return None,
    })
}

#[cfg(test)]
pub(crate) const PROVIDER_ICONS: [Icon; 12] = [
    Icon::Codex,
    Icon::OpenAi,
    Icon::OpenRouter,
    Icon::OpenCode,
    Icon::Nvidia,
    Icon::DeepSeek,
    Icon::MiniMax,
    Icon::Zai,
    Icon::Anthropic,
    Icon::Groq,
    Icon::Mistral,
    Icon::Xai,
];
