import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Modal,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import type { Bot, BotChatMessage } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { OceanBackground } from '../components/OceanBackground';
import { lightTheme } from '../theme';

export function ChatScreen() {
  const [bots, setBots] = useState<Bot[]>([]);
  const [selectedBotId, setSelectedBotId] = useState<string | null>(null);
  const [messages, setMessages] = useState<BotChatMessage[]>([]);
  const [draft, setDraft] = useState('');
  const [isLoadingBots, setIsLoadingBots] = useState(true);
  const [isLoadingMessages, setIsLoadingMessages] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isBotMenuOpen, setIsBotMenuOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectedBot = useMemo(
    () => bots.find((bot) => bot.id === selectedBotId) || null,
    [bots, selectedBotId]
  );

  const loadBots = useCallback(async () => {
    setError(null);
    try {
      const response = await api.bot.listBots();
      setBots(response.bots);
      // Use functional update to read current selectedBotId without adding it
      // to the dependency array, which would cause a refetch on every selection.
      setSelectedBotId((current) => {
        if (!current && response.bots.length > 0) {
          return response.bots[0].id;
        }
        if (response.bots.length === 0) {
          return null;
        }
        return current;
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load bots');
    } finally {
      setIsLoadingBots(false);
    }
  }, []);

  const loadMessages = useCallback(async (botId: string) => {
    setIsLoadingMessages(true);
    setError(null);
    try {
      const response = await api.bot.getChatMessages(botId);
      setMessages(response.messages);
    } catch (err) {
      setMessages([]);
      setError(err instanceof Error ? err.message : 'Failed to load chat history');
    } finally {
      setIsLoadingMessages(false);
      setIsRefreshing(false);
    }
  }, []);

  useEffect(() => {
    loadBots();
  }, [loadBots]);

  useEffect(() => {
    if (selectedBotId) {
      loadMessages(selectedBotId);
    } else {
      setMessages([]);
    }
  }, [selectedBotId, loadMessages]);

  const refresh = async () => {
    setIsRefreshing(true);
    await loadBots();
    if (selectedBotId) {
      await loadMessages(selectedBotId);
    } else {
      setIsRefreshing(false);
    }
  };

  const sendMessage = async () => {
    const botId = selectedBotId;
    const content = draft.trim();
    if (!botId || !content || isSending) return;

    setIsSending(true);
    setDraft('');

    const optimisticUser: BotChatMessage = {
      id: `local-user-${Date.now()}`,
      botId,
      role: 'user',
      content,
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, optimisticUser]);

    try {
      const response = await api.bot.postChatMessage(botId, { content });
      setMessages((prev) => {
        const withoutOptimistic = prev.filter((message) => message.id !== optimisticUser.id);
        return [...withoutOptimistic, response.userMessage, response.assistantMessage];
      });
    } catch (err) {
      setMessages((prev) => prev.filter((message) => message.id !== optimisticUser.id));
      setDraft(content);
      setError(err instanceof Error ? err.message : 'Failed to send message');
    } finally {
      setIsSending(false);
    }
  };

  if (isLoadingBots) {
    return (
      <OceanBackground>
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={lightTheme.colors.primary[700]} />
        </View>
      </OceanBackground>
    );
  }

  return (
    <OceanBackground>
      <View style={styles.container}>
        <View style={styles.headerCard}>
          <Text style={styles.headerLabel}>Chat Bot</Text>
          <TouchableOpacity
            style={styles.botSelectButton}
            onPress={() => setIsBotMenuOpen(true)}
            disabled={bots.length === 0}
          >
            <Text style={styles.botSelectText}>
              {selectedBot ? selectedBot.name : bots.length === 0 ? 'No bots available' : 'Select a bot'}
            </Text>
            <Text style={styles.botSelectChevron}>▾</Text>
          </TouchableOpacity>
        </View>

        <ScrollView
          style={styles.chatArea}
          contentContainerStyle={styles.chatAreaContent}
          refreshControl={
            <RefreshControl
              refreshing={isRefreshing}
              onRefresh={refresh}
              tintColor={lightTheme.colors.primary[700]}
            />
          }
        >
          {isLoadingMessages ? (
            <View style={styles.centeredInline}>
              <ActivityIndicator color={lightTheme.colors.primary[700]} />
            </View>
          ) : !selectedBotId ? (
            <View style={styles.emptyState}>
              <Text style={styles.emptyTitle}>No bot selected</Text>
              <Text style={styles.emptyText}>Create a boat first, then choose it from the dropdown above.</Text>
            </View>
          ) : messages.length === 0 ? (
            <View style={styles.emptyState}>
              <Text style={styles.emptyTitle}>Start the conversation</Text>
              <Text style={styles.emptyText}>Ask about strategy, risk, or recent market behavior.</Text>
            </View>
          ) : (
            messages.slice(-60).map((message) => (
              <View
                key={message.id}
                style={[styles.chatBubble, message.role === 'user' ? styles.userBubble : styles.assistantBubble]}
              >
                <Text style={styles.chatRole}>{message.role === 'user' ? 'You' : selectedBot?.name || 'Bot'}</Text>
                <Text style={styles.chatText}>{message.content}</Text>
                <Text style={styles.chatTimestamp}>{new Date(message.timestamp).toLocaleTimeString()}</Text>
              </View>
            ))
          )}

          {error && <Text style={styles.errorText}>{error}</Text>}
        </ScrollView>

        <View style={styles.composerContainer}>
          <TextInput
            style={styles.chatInput}
            placeholder={selectedBotId ? 'Message your bot...' : 'Select a bot to chat'}
            placeholderTextColor={lightTheme.colors.wave[400]}
            value={draft}
            onChangeText={setDraft}
            multiline
            editable={!!selectedBotId && !isSending}
          />
          <TouchableOpacity
            style={[styles.sendButton, (!selectedBotId || isSending) && styles.sendButtonDisabled]}
            onPress={sendMessage}
            disabled={!selectedBotId || isSending}
          >
            <Text style={styles.sendButtonText}>{isSending ? '...' : 'Send'}</Text>
          </TouchableOpacity>
        </View>

        <Modal
          visible={isBotMenuOpen}
          transparent
          animationType="fade"
          onRequestClose={() => setIsBotMenuOpen(false)}
        >
          <TouchableOpacity style={styles.modalBackdrop} activeOpacity={1} onPress={() => setIsBotMenuOpen(false)}>
            <View style={styles.modalCard}>
              <Text style={styles.modalTitle}>Select a bot</Text>
              <ScrollView style={styles.modalScroll}>
                {bots.map((bot) => (
                  <TouchableOpacity
                    key={bot.id}
                    style={[styles.modalItem, selectedBotId === bot.id && styles.modalItemSelected]}
                    onPress={() => {
                      setSelectedBotId(bot.id);
                      setIsBotMenuOpen(false);
                    }}
                  >
                    <Text style={[styles.modalItemText, selectedBotId === bot.id && styles.modalItemTextSelected]}>
                      {bot.name}
                    </Text>
                    <Text style={styles.modalItemSubtext}>{bot.assistantStyle}</Text>
                  </TouchableOpacity>
                ))}
              </ScrollView>
            </View>
          </TouchableOpacity>
        </Modal>
      </View>
    </OceanBackground>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    padding: 16,
    paddingBottom: 12,
  },
  centered: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  centeredInline: {
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 24,
  },
  headerCard: {
    borderRadius: 14,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    backgroundColor: lightTheme.colors.surface,
    padding: 12,
    marginBottom: 10,
  },
  headerLabel: {
    fontSize: 12,
    color: lightTheme.colors.wave[500],
    marginBottom: 6,
  },
  botSelectButton: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    borderRadius: 10,
    paddingHorizontal: 12,
    paddingVertical: 10,
    backgroundColor: '#fff',
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  botSelectText: {
    color: lightTheme.colors.wave[800],
    fontSize: 15,
    fontWeight: '600',
    flex: 1,
  },
  botSelectChevron: {
    color: lightTheme.colors.wave[500],
    marginLeft: 8,
  },
  chatArea: {
    flex: 1,
  },
  chatAreaContent: {
    paddingBottom: 8,
  },
  emptyState: {
    marginTop: 36,
    alignItems: 'center',
    paddingHorizontal: 18,
  },
  emptyTitle: {
    fontSize: 18,
    fontWeight: '700',
    color: lightTheme.colors.wave[700],
  },
  emptyText: {
    marginTop: 6,
    fontSize: 13,
    color: lightTheme.colors.wave[500],
    textAlign: 'center',
  },
  chatBubble: {
    borderRadius: 12,
    padding: 10,
    marginBottom: 8,
  },
  userBubble: {
    backgroundColor: lightTheme.colors.primary[100],
  },
  assistantBubble: {
    backgroundColor: lightTheme.colors.wave[100],
  },
  chatRole: {
    fontSize: 10,
    fontWeight: '700',
    color: lightTheme.colors.wave[500],
    marginBottom: 3,
  },
  chatText: {
    fontSize: 14,
    color: lightTheme.colors.wave[900],
  },
  chatTimestamp: {
    marginTop: 4,
    fontSize: 10,
    color: lightTheme.colors.wave[500],
  },
  errorText: {
    color: lightTheme.colors.lobster[600],
    marginTop: 10,
    marginBottom: 6,
    fontSize: 12,
  },
  composerContainer: {
    borderTopWidth: 1,
    borderColor: lightTheme.colors.wave[200],
    paddingTop: 10,
    flexDirection: 'row',
    alignItems: 'flex-end',
    gap: 8,
  },
  chatInput: {
    flex: 1,
    minHeight: 44,
    maxHeight: 100,
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    borderRadius: 10,
    backgroundColor: '#fff',
    color: lightTheme.colors.wave[900],
    paddingHorizontal: 10,
    paddingTop: 10,
  },
  sendButton: {
    borderRadius: 10,
    backgroundColor: lightTheme.colors.primary[700],
    paddingHorizontal: 14,
    paddingVertical: 10,
  },
  sendButtonDisabled: {
    opacity: 0.5,
  },
  sendButtonText: {
    color: '#fff',
    fontWeight: '700',
  },
  modalBackdrop: {
    flex: 1,
    backgroundColor: 'rgba(0,0,0,0.35)',
    justifyContent: 'center',
    padding: 22,
  },
  modalCard: {
    borderRadius: 14,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    backgroundColor: lightTheme.colors.surface,
    maxHeight: '70%',
    padding: 12,
  },
  modalTitle: {
    fontSize: 16,
    fontWeight: '700',
    color: lightTheme.colors.wave[800],
    marginBottom: 8,
  },
  modalScroll: {
    maxHeight: 360,
  },
  modalItem: {
    borderRadius: 10,
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[200],
    backgroundColor: '#fff',
    paddingHorizontal: 10,
    paddingVertical: 9,
    marginBottom: 8,
  },
  modalItemSelected: {
    borderColor: lightTheme.colors.primary[600],
    backgroundColor: lightTheme.colors.primary[50],
  },
  modalItemText: {
    fontSize: 14,
    color: lightTheme.colors.wave[800],
    fontWeight: '600',
  },
  modalItemTextSelected: {
    color: lightTheme.colors.primary[700],
  },
  modalItemSubtext: {
    marginTop: 2,
    fontSize: 11,
    color: lightTheme.colors.wave[500],
    textTransform: 'capitalize',
  },
});
