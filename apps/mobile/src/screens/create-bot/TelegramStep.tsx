import React from 'react';
import { Switch, Text, TextInput, View } from 'react-native';
import { lightTheme } from '../../theme';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';

export type TelegramStepProps = {
  telegramEnabled: boolean;
  setTelegramEnabled: (value: boolean) => void;
  telegramBotToken: string;
  setTelegramBotToken: (value: string) => void;
  telegramUserId: string;
  setTelegramUserId: (value: string) => void;
  telegramPairingCode: string;
  setTelegramPairingCode: (value: string) => void;
};

export function TelegramStep({
  telegramEnabled,
  setTelegramEnabled,
  telegramBotToken,
  setTelegramBotToken,
  telegramUserId,
  setTelegramUserId,
  telegramPairingCode,
  setTelegramPairingCode,
}: TelegramStepProps) {
  return (
    <View>
      <View style={styles.instructionBox}>
        <Text style={styles.instructionTitle}>Setup Guide</Text>
        <Text style={styles.instructionStep}>
          1. In Telegram, open <Text style={styles.inlineCode}>@BotFather</Text> and run{' '}
          <Text style={styles.inlineCode}>/newbot</Text>.
        </Text>
        <Text style={styles.instructionStep}>
          2. Copy the bot token BotFather returns and paste it below.
        </Text>
        <Text style={styles.instructionStep}>
          3. Send your bot a first message (for example:{' '}
          <Text style={styles.inlineCode}>/start</Text>).
        </Text>
        <Text style={styles.instructionStep}>
          4. The bot will reply with your Telegram user ID and a pairing code.
        </Text>
        <Text style={styles.instructionStep}>
          5. Reply with that pairing code in bot chat after deployment to finish linking.
        </Text>
      </View>

      <View style={styles.switchRow}>
        <View style={styles.switchCopy}>
          <Text style={styles.switchTitle}>Enable Telegram</Text>
          <Text style={styles.switchSubtitle}>
            Turn this on only if you want Telegram commands/alerts.
          </Text>
        </View>
        <Switch
          value={telegramEnabled}
          onValueChange={setTelegramEnabled}
          trackColor={{
            false: lightTheme.colors.wave[300],
            true: lightTheme.colors.primary[400],
          }}
        />
      </View>

      {telegramEnabled && (
        <>
          <Text style={styles.sectionLabel}>Telegram Bot Token</Text>
          <TextInput
            style={styles.input}
            value={telegramBotToken}
            onChangeText={setTelegramBotToken}
            placeholder="123456789:ABCdefGHI..."
            placeholderTextColor={lightTheme.colors.wave[400]}
            secureTextEntry
          />
          <Text style={styles.helperText}>
            Keep this private. We encrypt it at rest and only use it for your bot session.
          </Text>

          <Text style={styles.sectionLabel}>Telegram User ID</Text>
          <TextInput
            style={styles.input}
            value={telegramUserId}
            onChangeText={setTelegramUserId}
            placeholder="e.g. 123456789"
            placeholderTextColor={lightTheme.colors.wave[400]}
            keyboardType="number-pad"
          />

          <Text style={styles.sectionLabel}>Pairing Code</Text>
          <TextInput
            style={styles.input}
            value={telegramPairingCode}
            onChangeText={setTelegramPairingCode}
            placeholder="e.g. TRAWL-4821"
            placeholderTextColor={lightTheme.colors.wave[400]}
            autoCapitalize="characters"
            autoCorrect={false}
          />
          <Text style={styles.helperText}>
            We currently keep these values in setup flow for guided pairing.
          </Text>
        </>
      )}
    </View>
  );
}
