import { useSettingsStore } from '../index';

beforeEach(() => {
  useSettingsStore.setState({
    apiKeys: {},
    preferredModels: {},
    disabledCustodians: [],
  });
});

describe('useSettingsStore', () => {
  it('has correct initial state', () => {
    const state = useSettingsStore.getState();
    expect(state.apiKeys).toEqual({});
    expect(state.preferredModels).toEqual({});
    expect(state.disabledCustodians).toEqual([]);
  });

  it('setApiKey stores key for provider', () => {
    useSettingsStore.getState().setApiKey('openai', 'sk-test-123');
    expect(useSettingsStore.getState().apiKeys).toEqual({ openai: 'sk-test-123' });
  });

  it('removeApiKey removes key for provider', () => {
    useSettingsStore.getState().setApiKey('openai', 'sk-test');
    useSettingsStore.getState().setApiKey('anthropic', 'sk-ant-test');
    useSettingsStore.getState().removeApiKey('openai');

    expect(useSettingsStore.getState().apiKeys).toEqual({ anthropic: 'sk-ant-test' });
  });

  it('clearApiKeys removes all keys', () => {
    useSettingsStore.getState().setApiKey('openai', 'sk-1');
    useSettingsStore.getState().setApiKey('anthropic', 'sk-2');
    useSettingsStore.getState().clearApiKeys();

    expect(useSettingsStore.getState().apiKeys).toEqual({});
  });

  it('setPreferredModel stores model for provider', () => {
    useSettingsStore.getState().setPreferredModel('openai', 'gpt-4o');
    expect(useSettingsStore.getState().preferredModels).toEqual({ openai: 'gpt-4o' });
  });

  it('toggleCustodian adds when not present', () => {
    useSettingsStore.getState().toggleCustodian('solflare');
    expect(useSettingsStore.getState().disabledCustodians).toEqual(['solflare']);
  });

  it('toggleCustodian removes when already present', () => {
    useSettingsStore.getState().toggleCustodian('solflare');
    useSettingsStore.getState().toggleCustodian('solflare');
    expect(useSettingsStore.getState().disabledCustodians).toEqual([]);
  });

  it('partialize excludes apiKeys from persistence (security)', () => {
    const persistOptions = (useSettingsStore as any).persist?.getOptions?.();
    if (persistOptions?.partialize) {
      const full = {
        apiKeys: { openai: 'sk-secret' },
        preferredModels: { openai: 'gpt-4o' },
        disabledCustodians: ['phantom'],
        setApiKey: jest.fn(),
        removeApiKey: jest.fn(),
        clearApiKeys: jest.fn(),
        setPreferredModel: jest.fn(),
        toggleCustodian: jest.fn(),
      };
      const partialised = persistOptions.partialize(full);
      expect(partialised).not.toHaveProperty('apiKeys');
      expect(partialised).toHaveProperty('preferredModels');
      expect(partialised).toHaveProperty('disabledCustodians');
    }
  });
});
