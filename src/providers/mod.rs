pub mod traits;
pub mod registry;
pub mod openai_compatible;
pub mod anthropic;
pub mod ollama;

pub use registry::ProviderRegistry;
pub use openai_compatible::OpenAICompatibleProvider;
pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;
