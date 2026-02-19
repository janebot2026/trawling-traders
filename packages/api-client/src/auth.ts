import type {
  BillingSummary,
  NameAvailability,
  UpdateUserSettingsRequest,
  User,
  UserSettings,
} from '@trawling-traders/types';
import { fetchApi } from './http';

export const userApi = {
  async getCurrentUser(): Promise<User> {
    return fetchApi('/me');
  },

  async checkDisplayNameAvailability(displayName: string): Promise<NameAvailability> {
    const response = await fetchApi(
      `/account/display-name-availability?display_name=${encodeURIComponent(displayName)}`
    );
    return {
      available: Boolean(response.available),
      normalizedName:
        response.normalizedName ?? response.normalized_name ?? displayName.trim(),
      suggestedName: response.suggestedName ?? response.suggested_name ?? undefined,
    };
  },

  async getSettings(): Promise<UserSettings> {
    const response = await fetchApi('/account/settings');
    return {
      id: response.id,
      email: response.email,
      displayName: response.displayName ?? response.display_name,
      defaultAssistantStyle:
        response.defaultAssistantStyle ?? response.default_assistant_style ?? undefined,
      picture: response.picture,
      authMethods: {
        emailPassword: Boolean(response.authMethods?.emailPassword ?? response.auth_methods?.email_password),
        google: Boolean(response.authMethods?.google ?? response.auth_methods?.google),
        apple: Boolean(response.authMethods?.apple ?? response.auth_methods?.apple),
      },
      createdAt: response.createdAt ?? response.created_at,
      updatedAt: response.updatedAt ?? response.updated_at,
    };
  },

  async updateSettings(request: UpdateUserSettingsRequest): Promise<UserSettings> {
    const response = await fetchApi('/account/settings', {
      method: 'PATCH',
      body: JSON.stringify({
        display_name: request.displayName,
        default_assistant_style: request.defaultAssistantStyle,
      }),
    });

    return {
      id: response.id,
      email: response.email,
      displayName: response.displayName ?? response.display_name,
      defaultAssistantStyle:
        response.defaultAssistantStyle ?? response.default_assistant_style ?? undefined,
      picture: response.picture,
      authMethods: {
        emailPassword: Boolean(response.authMethods?.emailPassword ?? response.auth_methods?.email_password),
        google: Boolean(response.authMethods?.google ?? response.auth_methods?.google),
        apple: Boolean(response.authMethods?.apple ?? response.auth_methods?.apple),
      },
      createdAt: response.createdAt ?? response.created_at,
      updatedAt: response.updatedAt ?? response.updated_at,
    };
  },

  async getBillingSummary(): Promise<BillingSummary> {
    const response = await fetchApi('/account/billing');
    return {
      status: response.status,
      planCode: response.planCode ?? response.plan_code,
      maxBots: Number(response.maxBots ?? response.max_bots ?? 1),
      botCount: Number(response.botCount ?? response.bot_count ?? 0),
      currentPeriodEnd: response.currentPeriodEnd ?? response.current_period_end ?? undefined,
    };
  },
};
