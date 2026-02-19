import type { LlmModel, LlmProvider } from '@trawling-traders/types';

/** Shared LLM model options, keyed by provider. Used in CreateBotScreen and AiProviderSettings. */
export const LLM_MODELS: Record<LlmProvider, { value: LlmModel; label: string }[]> = {
  openai: [
    { value: 'gpt-4o', label: 'GPT-4o (Recommended)' },
    { value: 'gpt-4o-mini', label: 'GPT-4o Mini' },
    { value: 'gpt-4-turbo', label: 'GPT-4 Turbo' },
  ],
  anthropic: [
    { value: 'claude-3-5-sonnet', label: 'Claude 3.5 Sonnet (Recommended)' },
    { value: 'claude-3-opus', label: 'Claude 3 Opus' },
    { value: 'claude-3-haiku', label: 'Claude 3 Haiku' },
  ],
  venice: [{ value: 'llama-3.1-405b', label: 'Llama 3.1 405B' }],
  openrouter: [{ value: 'auto', label: 'Auto (Best Available)' }],
};
