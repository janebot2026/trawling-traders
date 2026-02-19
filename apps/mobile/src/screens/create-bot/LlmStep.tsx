import React from 'react';
import { Text, TextInput, View } from 'react-native';
import type { LlmModel, LlmProvider } from '@trawling-traders/types';
import { lightTheme } from '../../theme';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import { renderChip } from './WizardChip';
import type { LlmModelsMap, ModelsForProvider } from './wizardShared';

export type LlmStepProps = {
  llmProvider: LlmProvider;
  setLlmProvider: (value: LlmProvider) => void;
  llmModel: LlmModel;
  setLlmModel: (value: LlmModel) => void;
  llmApiKey: string;
  setLlmApiKey: (value: string) => void;
  modelsForProvider: ModelsForProvider;
  llmModels: LlmModelsMap;
};

export function LlmStep({
  llmProvider,
  setLlmProvider,
  llmModel,
  setLlmModel,
  llmApiKey,
  setLlmApiKey,
  modelsForProvider,
  llmModels,
}: LlmStepProps) {
  return (
    <View>
      <Text style={styles.sectionLabel}>LLM Provider</Text>
      {renderChip(
        (['openai', 'anthropic', 'venice', 'openrouter'] as LlmProvider[]).map(
          (value) => ({ value, label: value })
        ),
        llmProvider,
        (provider) => {
          setLlmProvider(provider);
          setLlmModel(llmModels[provider][0].value);
        }
      )}

      <Text style={styles.sectionLabel}>Model</Text>
      {renderChip(modelsForProvider, llmModel, setLlmModel)}

      <Text style={styles.sectionLabel}>API Key</Text>
      <TextInput
        style={styles.input}
        value={llmApiKey}
        onChangeText={setLlmApiKey}
        placeholder="sk-..."
        placeholderTextColor={lightTheme.colors.wave[400]}
        secureTextEntry
      />
      <Text style={styles.helperText}>
        Stored securely and only used for this boat.
      </Text>
    </View>
  );
}
