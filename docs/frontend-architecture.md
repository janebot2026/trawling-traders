# Trawling Traders - Frontend Architecture

*Last updated: 2026-02-02*

## Overview

React Native mobile app for managing AI trading bots on DigitalOcean VPS.

## Project Structure

```
trawling-traders/
├── apps/
│   └── mobile/              # React Native app (Expo)
│       ├── src/
│       │   ├── screens/     # 6 MVP screens
│       │   ├── navigation/  # React Navigation setup
│       │   ├── api/         # (future) API integration layer
│       │   ├── hooks/       # (future) Custom React hooks
│       │   ├── store/       # (future) Zustand state management
│       │   ├── types/       # (future) Local type extensions
│       │   └── utils/       # (future) Utility functions
│       └── package.json
├── packages/
│   ├── types/               # Shared TypeScript types
│   └── api-client/          # API client for Rust backend
└── package.json             # Workspace root
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | React Native 0.72.6 + Expo ~49.0.15 |
| Navigation | React Navigation v6 (stack + bottom tabs) |
| State Management | Zustand (planned) |
| Charts | react-native-chart-kit (planned) |
| Styling | React Native StyleSheet |
| Animations | React Native Animated + react-native-gesture-handler |

## Screens (6 total)

### 1. AuthScreen
- **Route:** `Auth`
- **Purpose:** Cedros Login integration
- **State:** Loading check → Login button → Navigate to Subscribe
- **UI:** Centered logo, "Sign in with Cedros" button, secure hint

### 2. SubscribeScreen
- **Route:** `Subscribe`
- **Purpose:** Cedros Pay subscription flow
- **State:** Single-tier display → Subscribe button → Navigate to Main
- **UI:** Plan card (Trader Pro $29/mo), feature list, secure payment badge

### 3. BotsListScreen
- **Route:** `Main` (tab)
- **Purpose:** Dashboard showing up to 4 bots
- **State:** List of bots, refresh control, create button (if < 4 bots)
- **UI:** 
  - Header: "My Bots" + count
  - Create Bot button (conditional)
  - Bot cards: name, status badge, persona, today PnL, last heartbeat
  - Pull-to-refresh
  - Empty state for new users

### 4. CreateBotScreen
- **Route:** `CreateBot`
- **Purpose:** Create new trading bot with full configuration
- **State:** Multi-section form with validation
- **Sections:**
  1. **Identity:** Name input
  2. **Persona:** Beginner / Hands-on / Quant-lite (cards with descriptions)
  3. **Asset Focus:** Majors / Memes / Custom (chips)
  4. **Algorithm:** Trend / Mean Reversion / Breakout (cards with descriptions)
  5. **Risk Caps:** Max position %, max daily loss, max drawdown, max trades/day
  6. **Trading Mode:** Paper (default) / Live toggle with warning
  7. **LLM Config:** Provider selector (OpenAI/Anthropic/Venice/OpenRouter) + API key
- **Validation:** Name required, API key required
- **Submit:** POST /bots → Navigate to Main

### 5. BotDetailScreen
- **Route:** `BotDetail`
- **Purpose:** View bot status, performance, events, actions
- **Params:** `botId: string`
- **State:** Bot data, events list, metrics (future), action loading states
- **UI:**
  - Header: Name, status dot + text, config pending badge, settings button, last heartbeat
  - Performance: Today PnL, Total PnL, chart placeholder
  - Actions: Pause/Resume, Redeploy, Destroy (row of buttons)
  - Events: Recent events list (type, time, message)
  - Infrastructure: Region, droplet ID, IP (when available)

### 6. BotSettingsScreen
- **Route:** `BotSettings`
- **Purpose:** Edit existing bot configuration
- **Params:** `botId: string`
- **State:** Pre-filled form from current config, config status (pending/applied/failed)
- **UI:** Mirror of CreateBotScreen but:
  - Pre-populated fields
  - "Config Status" indicator (Pending / Applied / Failed)
  - Save creates new config version → shows pending until bot acks

## Navigation Flow

```
Auth → Subscribe → Main (Tabs)
                    ├── Bots → CreateBot
                    │          └── (back to Bots)
                    └── BotDetail → BotSettings
                                   └── (back to BotDetail)
```

## API Integration Points

All API calls go through `@trawling-traders/api-client` singleton:

```typescript
// Initialize (in App entry)
import { initializeApi } from '@trawling-traders/api-client';

initializeApi({
  baseUrl: 'https://api.trawling-traders.com/v1',
  getAuthToken: () => getSecureStorage('cedros_token'),
});

// Use in components
import { getApi } from '@trawling-traders/api-client';

const { bots } = await getApi().listBots();
const bot = await getApi().createBot({ ... });
await getApi().botAction(botId, 'pause');
```

## Type Definitions (Shared)

See `/packages/types/src/index.ts` for full definitions.

Key types:
- `Bot` - Bot entity with status, config state, timestamps
- `BotConfig` - Full configuration (identity, algorithm, risk, secrets)
- `BotStatus` - 'provisioning' | 'online' | 'offline' | 'paused' | 'error' | 'destroying'
- `Persona` - 'beginner' | 'tweaker' | 'quant-lite'
- `AlgorithmMode` - 'trend' | 'mean-reversion' | 'breakout'
- `BotEvent` - Events from bot (trade_opened, config_applied, error, etc.)
- `MetricPoint` - Time-series data for charts

## Backend API Contract (Expected)

### Endpoints (App → Server)

```
GET    /me                    → User
GET    /bots                  → { bots: Bot[], total: number }
POST   /bots                  → Bot (creates, triggers provisioning)
GET    /bots/:id              → { bot: Bot, config: BotConfig }
PATCH  /bots/:id/config       → BotConfig (creates new version)
POST   /bots/:id/actions      → { action: 'pause'|'resume'|'redeploy'|'destroy' }
GET    /bots/:id/metrics?range=7d|30d → { metrics: MetricPoint[], range }
GET    /bots/:id/events?cursor=...    → { events: BotEvent[], nextCursor? }
```

### Endpoints (Bot → Server)

```
POST   /bot/register          → Bot registration on first boot
POST   /bot/heartbeat         → Status ping
GET    /bot/:id/config        → Pull desired config
POST   /bot/:id/config_ack    → Ack config applied
POST   /bot/:id/metrics_batch → Push metrics
POST   /bot/:id/events_batch  → Push events
```

## State Management (Planned)

Zustand stores:

```typescript
// Auth store
interface AuthStore {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  login: (token: string) => Promise<void>;
  logout: () => void;
}

// Bots store
interface BotsStore {
  bots: Bot[];
  selectedBot: Bot | null;
  isLoading: boolean;
  fetchBots: () => Promise<void>;
  createBot: (config: CreateBotRequest) => Promise<void>;
  selectBot: (id: string) => void;
  refreshBot: (id: string) => Promise<void>;
}

// Events store (per bot)
interface EventsStore {
  events: Record<string, BotEvent[]>; // botId -> events
  fetchEvents: (botId: string) => Promise<void>;
}
```

## Styling Approach

- All styles in `StyleSheet.create()` per screen/component
- Consistent spacing: 8px base unit (8, 16, 24, 32)
- Colors:
  - Primary: `#007AFF` (iOS blue)
  - Success: `#4CAF50`
  - Warning: `#FFC107`
  - Error: `#F44336`
  - Neutral: `#9E9E9E`
  - Background: `#f5f5f5`
  - Card: `#fff`

## Animation Targets (Professional UX)

- [ ] Screen transitions (slide from right)
- [ ] Bot card press (scale 0.98 + shadow)
- [ ] Status badge pulse (when provisioning)
- [ ] Pull-to-refresh spinner
- [ ] Chart entry animation (line draw)
- [ ] Form field focus states
- [ ] Button press feedback
- [ ] Config pending → applied transition
- [ ] List item entrance (stagger fade in)

## Next Steps

1. **Wire real API calls** (waiting for backend)
2. **Add Zustand stores** for state management
3. **Implement chart integration** with real metrics
4. **Polish animations** for professional feel
5. **Add error handling** and retry logic
6. **Test on device** (iOS/Android)

## Integration Notes for Backend

### Required for Day 1
- `POST /bots` must accept `CreateBotRequest` and return `Bot`
- `GET /bots` must return list for dashboard
- Bot status must update from 'provisioning' → 'online' when droplet ready

### Required for Day 2-3
- `PATCH /bots/:id/config` creates immutable version
- Config reconciliation: bot pulls → applies → acks
- `GET /bots/:id/config` returns desired config for bot

### Required for Day 4
- `GET /bots/:id/metrics` returns time-series data
- `GET /bots/:id/events` returns event log
- Bot pushes metrics/events via batch endpoints

### Required for Day 5-7
- Cedros Login integration (JWT tokens)
- Cedros Pay subscription gating
- Rate limiting, auth checks

---

*Document maintained by Jane. Update when API contracts change.*
