import React from 'react';
import type {
  AIAssistantOption,
  AlgorithmFactor,
  AlgorithmMode,
  AssetFocus,
  LlmModel,
  LlmProvider,
  NameAvailability,
  Persona,
  Strictness,
  TradeableAsset,
  TradingMode,
} from '@trawling-traders/types';
import type { Option, StrategyType } from './wizardShared';
import { NameStep } from './NameStep';
import { PersonaStep } from './PersonaStep';
import { AssetsStep } from './AssetsStep';
import { StrategyStep } from './StrategyStep';
import { AlgorithmStep } from './AlgorithmStep';
import { RiskStep } from './RiskStep';
import { LlmStep } from './LlmStep';
import { TelegramStep } from './TelegramStep';
import { SummaryStep } from './SummaryStep';

export type WizardStep = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8;

type CreateBotWizardStepsProps = {
  step: WizardStep;
  name: string;
  setName: (value: string) => void;
  nameAvailability: NameAvailability | null;
  nameCheckLoading: boolean;
  assistantStyle: Persona;
  setAssistantStyle: (value: Persona) => void;
  assistantOptions: AIAssistantOption[];
  assistantOptionsLoading: boolean;
  assetFocus: AssetFocus;
  setAssetFocus: (value: AssetFocus) => void;
  tradeableAssets: TradeableAsset[];
  selectedAssets: string[];
  setSelectedAssets: (value: string[]) => void;
  assetsLoading: boolean;
  strategyOptions: Option<StrategyType>[];
  strategyType: StrategyType;
  setStrategyType: (value: StrategyType) => void;
  algorithmMode: AlgorithmMode;
  strictness: Strictness;
  setStrictness: (value: Strictness) => void;
  factorCatalog: Option<string>[];
  algorithmFactors: AlgorithmFactor[];
  setAlgorithmFactors: (value: AlgorithmFactor[]) => void;
  tradingMode: TradingMode;
  setTradingMode: (value: TradingMode) => void;
  maxPositionSize: string;
  setMaxPositionSize: (value: string) => void;
  maxTradesPerDay: string;
  setMaxTradesPerDay: (value: string) => void;
  maxDailyLoss: string;
  setMaxDailyLoss: (value: string) => void;
  maxDrawdown: string;
  setMaxDrawdown: (value: string) => void;
  llmProvider: LlmProvider;
  setLlmProvider: (value: LlmProvider) => void;
  llmModel: LlmModel;
  setLlmModel: (value: LlmModel) => void;
  llmApiKey: string;
  setLlmApiKey: (value: string) => void;
  telegramEnabled: boolean;
  setTelegramEnabled: (value: boolean) => void;
  telegramBotToken: string;
  setTelegramBotToken: (value: string) => void;
  telegramUserId: string;
  setTelegramUserId: (value: string) => void;
  telegramPairingCode: string;
  setTelegramPairingCode: (value: string) => void;
  modelsForProvider: { value: LlmModel; label: string }[];
  llmModels: Record<LlmProvider, { value: LlmModel; label: string }[]>;
  assetChoices: Option<AssetFocus>[];
  strictnessOptions: Option<Strictness>[];
};

export function CreateBotWizardSteps(props: CreateBotWizardStepsProps) {
  const { step } = props;

  if (step === 0) {
    return (
      <NameStep
        name={props.name}
        setName={props.setName}
        nameAvailability={props.nameAvailability}
        nameCheckLoading={props.nameCheckLoading}
        tradingMode={props.tradingMode}
        setTradingMode={props.setTradingMode}
      />
    );
  }

  if (step === 1) {
    return (
      <PersonaStep
        assistantStyle={props.assistantStyle}
        setAssistantStyle={props.setAssistantStyle}
        assistantOptions={props.assistantOptions}
        assistantOptionsLoading={props.assistantOptionsLoading}
      />
    );
  }

  if (step === 2) {
    return (
      <AssetsStep
        assetFocus={props.assetFocus}
        setAssetFocus={props.setAssetFocus}
        tradeableAssets={props.tradeableAssets}
        selectedAssets={props.selectedAssets}
        setSelectedAssets={props.setSelectedAssets}
        assetsLoading={props.assetsLoading}
        assetChoices={props.assetChoices}
      />
    );
  }

  if (step === 3) {
    return (
      <StrategyStep
        strategyOptions={props.strategyOptions}
        strategyType={props.strategyType}
        setStrategyType={props.setStrategyType}
      />
    );
  }

  if (step === 4) {
    return (
      <AlgorithmStep
        strategyType={props.strategyType}
        factorCatalog={props.factorCatalog}
        algorithmFactors={props.algorithmFactors}
        setAlgorithmFactors={props.setAlgorithmFactors}
      />
    );
  }

  if (step === 5) {
    return (
      <RiskStep
        maxPositionSize={props.maxPositionSize}
        setMaxPositionSize={props.setMaxPositionSize}
        maxTradesPerDay={props.maxTradesPerDay}
        setMaxTradesPerDay={props.setMaxTradesPerDay}
        maxDailyLoss={props.maxDailyLoss}
        setMaxDailyLoss={props.setMaxDailyLoss}
        maxDrawdown={props.maxDrawdown}
        setMaxDrawdown={props.setMaxDrawdown}
        strictness={props.strictness}
        setStrictness={props.setStrictness}
        strictnessOptions={props.strictnessOptions}
      />
    );
  }

  if (step === 6) {
    return (
      <LlmStep
        llmProvider={props.llmProvider}
        setLlmProvider={props.setLlmProvider}
        llmModel={props.llmModel}
        setLlmModel={props.setLlmModel}
        llmApiKey={props.llmApiKey}
        setLlmApiKey={props.setLlmApiKey}
        modelsForProvider={props.modelsForProvider}
        llmModels={props.llmModels}
      />
    );
  }

  if (step === 7) {
    return (
      <TelegramStep
        telegramEnabled={props.telegramEnabled}
        setTelegramEnabled={props.setTelegramEnabled}
        telegramBotToken={props.telegramBotToken}
        setTelegramBotToken={props.setTelegramBotToken}
        telegramUserId={props.telegramUserId}
        setTelegramUserId={props.setTelegramUserId}
        telegramPairingCode={props.telegramPairingCode}
        setTelegramPairingCode={props.setTelegramPairingCode}
      />
    );
  }

  return (
    <SummaryStep
      name={props.name}
      assistantStyle={props.assistantStyle}
      assistantOptions={props.assistantOptions}
      strategyType={props.strategyType}
      strictness={props.strictness}
      assetFocus={props.assetFocus}
      selectedAssets={props.selectedAssets}
      tradingMode={props.tradingMode}
      maxPositionSize={props.maxPositionSize}
      maxDailyLoss={props.maxDailyLoss}
      maxDrawdown={props.maxDrawdown}
      maxTradesPerDay={props.maxTradesPerDay}
      algorithmFactors={props.algorithmFactors}
      llmProvider={props.llmProvider}
      llmModel={props.llmModel}
      telegramEnabled={props.telegramEnabled}
      telegramUserId={props.telegramUserId}
      telegramPairingCode={props.telegramPairingCode}
    />
  );
}
