import { NavigationContainer } from '@react-navigation/native';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';

// Screens
import { AuthScreen } from '../screens/AuthScreen';
import { SubscribeScreen } from '../screens/SubscribeScreen';
import { BotsListScreen } from '../screens/BotsListScreen';
import { CreateBotScreen } from '../screens/CreateBotScreen';
import { BotDetailScreen } from '../screens/BotDetailScreen';
import { BotSettingsScreen } from '../screens/BotSettingsScreen';
import { ProfileScreen } from '../screens/ProfileScreen';

// Types
export type RootStackParamList = {
  Auth: undefined;
  Subscribe: undefined;
  Main: { refresh?: boolean } | undefined;
  CreateBot: undefined;
  BotDetail: { botId: string };
  BotSettings: { botId: string };
};

export type MainTabParamList = {
  Bots: undefined;
  Profile: undefined;
};

const Stack = createNativeStackNavigator<RootStackParamList>();
const Tab = createBottomTabNavigator<MainTabParamList>();

// Main tab navigator (after auth)
function MainTabs() {
  return (
    <Tab.Navigator>
      <Tab.Screen 
        name="Bots" 
        component={BotsListScreen} 
        options={{ title: 'My Bots' }}
      />
      <Tab.Screen 
        name="Profile" 
        component={ProfileScreen} 
        options={{ title: 'Account' }}
      />
    </Tab.Navigator>
  );
}

// Root navigator with auth flow
export function AppNavigator() {
  return (
    <NavigationContainer>
      <Stack.Navigator initialRouteName="Auth">
        <Stack.Screen 
          name="Auth" 
          component={AuthScreen}
          options={{ headerShown: false }}
        />
        <Stack.Screen 
          name="Subscribe" 
          component={SubscribeScreen}
          options={{ title: 'Subscribe' }}
        />
        
        <Stack.Screen 
          name="Main" 
          component={MainTabs}
          options={{ headerShown: false }}
        />
        
        <Stack.Screen 
          name="CreateBot" 
          component={CreateBotScreen}
          options={{ title: 'Create Bot' }}
        />
        
        <Stack.Screen 
          name="BotDetail" 
          component={BotDetailScreen}
          options={{ title: 'Bot Details' }}
        />
        
        <Stack.Screen 
          name="BotSettings" 
          component={BotSettingsScreen}
          options={{ title: 'Bot Settings' }}
        />
      </Stack.Navigator>
    </NavigationContainer>
  );
}
