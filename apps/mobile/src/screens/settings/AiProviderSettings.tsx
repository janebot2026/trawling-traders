import React, { useState, useRef } from 'react';
import {
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import type { LlmProvider } from '@trawling-traders/types';
import { useSettingsStore } from '../../store';
import { lightTheme } from '../../theme';
import { LLM_MODELS } from '../../config/llmModels';

const PROVIDERS: { key: LlmProvider; label: string }[] = [
  { key: 'openai', label: 'OpenAI' },
  { key: 'anthropic', label: 'Anthropic' },
  { key: 'venice', label: 'Venice' },
  { key: 'openrouter', label: 'OpenRouter' },
];

/**
 * Returns a masked representation of an API key for safe display.
 * Shows only the last 4 characters to avoid exposing the full key
 * in screen recordings or React DevTools.
 */
function maskApiKey(key: string): string {
  if (!key) return '';
  if (key.length <= 4) return '****';
  return `****${key.slice(-4)}`;
}

function ProviderCard({ provider }: { provider: { key: LlmProvider; label: string } }) {
  const apiKeys = useSettingsStore((s) => s.apiKeys);
  const preferredModels = useSettingsStore((s) => s.preferredModels);
  const setApiKey = useSettingsStore((s) => s.setApiKey);
  const removeApiKey = useSettingsStore((s) => s.removeApiKey);
  const setPreferredModel = useSettingsStore((s) => s.setPreferredModel);

  const storedKey = apiKeys[provider.key] ?? '';
  const selectedModel = preferredModels[provider.key];
  const models = LLM_MODELS[provider.key];

  // Store masked value in state (safe for screen recordings / DevTools).
  // The actual key the user types is kept only in a ref so it never enters
  // React state and is not captured by screen-recording tools.
  const pendingKeyRef = useRef<string>('');
  const [keyDisplay, setKeyDisplay] = useState<string>(maskApiKey(storedKey));
  const [showKey, setShowKey] = useState(false);
  const [hasUnsavedKey, setHasUnsavedKey] = useState(false);

  const handleChangeText = (text: string) => {
    // Keep the actual value in a ref, not in state
    pendingKeyRef.current = text;
    // Display masked version; show full typed text only when "Show" is active
    setKeyDisplay(showKey ? text : maskApiKey(text));
    setHasUnsavedKey(text.trim() !== storedKey);
  };

  const handleToggleShow = () => {
    setShowKey((prev) => {
      const next = !prev;
      // Re-mask or reveal based on the toggled state
      if (next) {
        setKeyDisplay(pendingKeyRef.current || storedKey);
      } else {
        setKeyDisplay(maskApiKey(pendingKeyRef.current || storedKey));
      }
      return next;
    });
  };

  const saveKey = () => {
    const trimmed = pendingKeyRef.current.trim();
    if (trimmed) {
      setApiKey(provider.key, trimmed);
      pendingKeyRef.current = '';
      setKeyDisplay(maskApiKey(trimmed));
      setHasUnsavedKey(false);
    }
  };

  const clearKey = () => {
    removeApiKey(provider.key);
    pendingKeyRef.current = '';
    setKeyDisplay('');
    setHasUnsavedKey(false);
  };

  return (
    <View style={styles.card}>
      <Text style={styles.sectionTitle}>{provider.label}</Text>

      <Text style={styles.inputLabel}>API Key</Text>
      <View style={styles.keyRow}>
        <TextInput
          style={[styles.input, styles.keyInput]}
          value={keyDisplay}
          onChangeText={handleChangeText}
          placeholder="sk-..."
          placeholderTextColor={lightTheme.colors.wave[400]}
          secureTextEntry={!showKey}
          autoCapitalize="none"
          autoCorrect={false}
        />
        <Pressable style={styles.toggleButton} onPress={handleToggleShow}>
          <Text style={styles.toggleButtonText}>{showKey ? 'Hide' : 'Show'}</Text>
        </Pressable>
      </View>

      <View style={styles.keyActions}>
        <TouchableOpacity
          style={[styles.saveKeyButton, !hasUnsavedKey && styles.saveKeyButtonDisabled]}
          onPress={saveKey}
          disabled={!hasUnsavedKey}
        >
          <Text style={styles.saveKeyButtonText}>Save Key</Text>
        </TouchableOpacity>
        {storedKey ? (
          <TouchableOpacity style={styles.clearKeyButton} onPress={clearKey}>
            <Text style={styles.clearKeyButtonText}>Clear</Text>
          </TouchableOpacity>
        ) : null}
      </View>

      <Text style={[styles.inputLabel, { marginTop: 14 }]}>Preferred Model</Text>
      <View style={styles.chipRow}>
        {models.map((m) => {
          const active = selectedModel === m.value;
          return (
            <TouchableOpacity
              key={m.value}
              style={[styles.chip, active && styles.chipActive]}
              onPress={() => setPreferredModel(provider.key, m.value)}
            >
              <Text style={[styles.chipText, active && styles.chipTextActive]}>{m.label}</Text>
            </TouchableOpacity>
          );
        })}
      </View>
    </View>
  );
}

export function AiProviderSettings() {
  return (
    <View>
      <Text style={styles.helper}>Default keys pre-fill when creating new bots.</Text>
      {PROVIDERS.map((p) => (
        <ProviderCard key={p.key} provider={p} />
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  helper: {
    marginTop: 4,
    marginBottom: 4,
    fontSize: 12,
    color: lightTheme.colors.wave[500],
  },
  card: {
    marginTop: 14,
    borderRadius: 16,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    backgroundColor: lightTheme.colors.surface,
    padding: 14,
  },
  sectionTitle: {
    fontSize: 17,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
    marginBottom: 10,
  },
  inputLabel: {
    fontSize: 13,
    color: lightTheme.colors.wave[600],
    marginBottom: 6,
  },
  input: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    borderRadius: 10,
    paddingHorizontal: 12,
    paddingVertical: 10,
    backgroundColor: '#fff',
    fontSize: 14,
    color: lightTheme.colors.wave[900],
  },
  keyRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
  },
  keyInput: {
    flex: 1,
  },
  toggleButton: {
    paddingHorizontal: 12,
    paddingVertical: 10,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    backgroundColor: lightTheme.colors.wave[50],
  },
  toggleButtonText: {
    fontSize: 13,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
  },
  keyActions: {
    flexDirection: 'row',
    gap: 8,
    marginTop: 10,
  },
  saveKeyButton: {
    borderRadius: 10,
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 9,
    paddingHorizontal: 16,
    alignItems: 'center',
  },
  saveKeyButtonDisabled: {
    backgroundColor: lightTheme.colors.wave[300],
  },
  saveKeyButtonText: {
    color: '#fff',
    fontSize: 13,
    fontWeight: '700',
  },
  clearKeyButton: {
    borderRadius: 10,
    borderWidth: 1,
    borderColor: lightTheme.colors.lobster[300],
    paddingVertical: 9,
    paddingHorizontal: 16,
    alignItems: 'center',
    backgroundColor: lightTheme.colors.lobster[50],
  },
  clearKeyButtonText: {
    color: lightTheme.colors.lobster[700],
    fontSize: 13,
    fontWeight: '700',
  },
  chipRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  chip: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    borderRadius: 999,
    paddingHorizontal: 12,
    paddingVertical: 8,
    backgroundColor: '#fff',
  },
  chipActive: {
    borderColor: '#0b5ea8',
    backgroundColor: '#0b5ea8',
  },
  chipText: {
    color: lightTheme.colors.wave[800],
    fontSize: 13,
    fontWeight: '600',
  },
  chipTextActive: {
    color: '#ffffff',
    fontWeight: '700',
  },
});
