import React from 'react';
import { ImageBackground, Pressable, StyleSheet, Text, View } from 'react-native';
import { DrawerActions } from '@react-navigation/native';
import { NavigationContainer } from '@react-navigation/native';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { createDrawerNavigator, DrawerContentScrollView } from '@react-navigation/drawer';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { lightTheme } from '../theme';
import { AppHeader } from '../components/AppHeader';
import { AppTabBar } from '../components/AppTabBar';

import { AuthScreen } from '../screens/AuthScreen';
import { SubscribeScreen } from '../screens/SubscribeScreen';
import { HomeOverviewScreen } from '../screens/HomeOverviewScreen';
import { CreateBotScreen } from '../screens/CreateBotScreen';
import { BotDetailScreen } from '../screens/BotDetailScreen';
import { BotStrategyConfigScreen } from '../screens/BotStrategyConfigScreen';
import { BotBehaviorConfigScreen } from '../screens/BotBehaviorConfigScreen';
import { BotSettingsScreen } from '../screens/BotSettingsScreen';
import { ProfileScreen } from '../screens/ProfileScreen';
import { ChatScreen } from '../screens/ChatScreen';
import { ReportsScreen } from '../screens/ReportsScreen';
import { DocsScreen } from '../screens/DocsScreen';
import { SettingsScreen } from '../screens/SettingsScreen';
import { BillingScreen } from '../screens/BillingScreen';
import { DepositScreen } from '../screens/DepositScreen';
import { WithdrawScreen } from '../screens/WithdrawScreen';
import { ResearchScreen } from '../screens/ResearchScreen';
import { LeaderboardScreen } from '../screens/LeaderboardScreen';
import { CommunityScreen } from '../screens/CommunityScreen';

const SHIP_LOCKER_BG = require('../../../../assets/branding/tt-panel.png');
const PANEL_BUTTON_OFF = require('../../../../assets/branding/tt-panel-button-off-cropped.png');
const PANEL_BUTTON_HOVER = require('../../../../assets/branding/tt-panel-button-hover-cropped.png');
const PANEL_BUTTON_ON = require('../../../../assets/branding/tt-panel-button-on-cropped.png');

export type RootStackParamList = {
  Auth: undefined;
  Subscribe: undefined;
  Main: undefined;
  CreateBot: undefined;
  BotDetail: { botId: string };
  BotStrategyConfig: { botId: string };
  BotBehaviorConfig: { botId: string };
  BotSettings: { botId: string };
  Profile: undefined;
  Settings: undefined;
  Billing: undefined;
  Deposit: undefined;
  Withdraw: undefined;
};

export type MainDrawerParamList = {
  Home: undefined;
  Docs: undefined;
  Reports: undefined;
  Chat: undefined;
};

const Stack = createNativeStackNavigator<RootStackParamList>();
const Drawer = createDrawerNavigator<MainDrawerParamList>();
const ProfileDrawer = createDrawerNavigator<{ App: undefined }>();
const Tab = createBottomTabNavigator();

function MainTabs() {
  return (
    <Tab.Navigator
      tabBar={(props) => <AppTabBar {...props} />}
      screenOptions={{
        headerShown: false,
      }}
    >
      <Tab.Screen name="Bots" component={HomeOverviewScreen} />
      <Tab.Screen name="Research" component={ResearchScreen} />
      <Tab.Screen name="Leaderboard" component={LeaderboardScreen} />
      <Tab.Screen name="Community" component={CommunityScreen} />
    </Tab.Navigator>
  );
}

function MainDrawer() {
  return (
    <Drawer.Navigator
      drawerContent={(props) => <MainDrawerContent {...props} />}
      screenOptions={({ navigation }) => ({
        headerShown: true,
        drawerActiveTintColor: lightTheme.colors.primary[700],
        header: () => (
          <AppHeader
            title="Trawling Traders"
            onMenu={() => navigation.dispatch(DrawerActions.toggleDrawer())}
            onProfile={() => navigation.getParent()?.getParent()?.dispatch(DrawerActions.toggleDrawer())}
          />
        ),
      })}
    >
      <Drawer.Screen
        name="Home"
        component={MainTabs}
        options={({ navigation }) => ({
          headerTransparent: true,
          header: () => (
            <AppHeader
              title="Trawling Traders"
              transparent
              onMenu={() => navigation.dispatch(DrawerActions.toggleDrawer())}
              onProfile={() => navigation.getParent()?.getParent()?.dispatch(DrawerActions.toggleDrawer())}
            />
          ),
        })}
      />
      <Drawer.Screen
        name="Docs"
        component={DocsScreen}
        options={({ navigation }) => ({
          headerTransparent: true,
          header: () => (
            <AppHeader
              title="Docs"
              transparent
              onMenu={() => navigation.dispatch(DrawerActions.toggleDrawer())}
              onProfile={() => navigation.getParent()?.getParent()?.dispatch(DrawerActions.toggleDrawer())}
            />
          ),
        })}
      />
      <Drawer.Screen
        name="Reports"
        component={ReportsScreen}
        options={({ navigation }) => ({
          headerTransparent: true,
          header: () => (
            <AppHeader
              title="Reports"
              transparent
              onMenu={() => navigation.dispatch(DrawerActions.toggleDrawer())}
              onProfile={() => navigation.getParent()?.getParent()?.dispatch(DrawerActions.toggleDrawer())}
            />
          ),
        })}
      />
      <Drawer.Screen name="Chat" component={ChatScreen} />
    </Drawer.Navigator>
  );
}

function MainDrawerContent(props: any) {
  const insets = useSafeAreaInsets();
  const nav = props.navigation;
  const activeRoute = props.state?.routeNames?.[props.state?.index] ?? 'Home';
  const items: { key: string; label: string; active?: boolean; onPress: () => void }[] = [
    {
      key: 'home',
      label: 'Home',
      active: activeRoute === 'Home',
      onPress: () => {
        nav.closeDrawer();
        nav.navigate('Home');
      },
    },
    {
      key: 'deposit',
      label: 'Deposit',
      onPress: () => {
        nav.closeDrawer();
        nav.getParent()?.navigate('Deposit');
      },
    },
    {
      key: 'withdraw',
      label: 'Withdraw',
      onPress: () => {
        nav.closeDrawer();
        nav.getParent()?.navigate('Withdraw');
      },
    },
    {
      key: 'docs',
      label: 'Docs',
      active: activeRoute === 'Docs',
      onPress: () => {
        nav.closeDrawer();
        nav.navigate('Docs');
      },
    },
    {
      key: 'reports',
      label: 'Reports',
      active: activeRoute === 'Reports',
      onPress: () => {
        nav.closeDrawer();
        nav.navigate('Reports');
      },
    },
    {
      key: 'chat',
      label: 'Chat',
      active: activeRoute === 'Chat',
      onPress: () => {
        nav.closeDrawer();
        nav.navigate('Chat');
      },
    },
  ];
  return (
    <DrawerContentScrollView
      {...props}
      style={styles.drawerScrollContainer}
      contentContainerStyle={styles.drawerScroll}
    >
      <ImageBackground source={SHIP_LOCKER_BG} style={styles.drawerBg} resizeMode="cover">
        <View style={[styles.drawerItemsWrap, { paddingTop: insets.top + 128 }]}>
          {items.map((item) => (
            <DrawerLabelButton
              key={item.key}
              label={item.label}
              active={item.active}
              onPress={item.onPress}
            />
          ))}
        </View>
      </ImageBackground>
    </DrawerContentScrollView>
  );
}

function AppStack() {
  return (
    <Stack.Navigator
      initialRouteName="Auth"
      screenOptions={{
        header: ({ navigation, route, options, back }) => (
          <AppHeader
            title={typeof options.title === 'string' ? options.title : route.name}
            showBack={!!back}
            transparent={Boolean(options.headerTransparent)}
            onBack={back ? navigation.goBack : undefined}
            onMenu={back ? undefined : () => navigation.dispatch(DrawerActions.toggleDrawer())}
            onProfile={() => navigation.getParent()?.dispatch(DrawerActions.toggleDrawer())}
          />
        ),
      }}
    >
      <Stack.Screen name="Auth" component={AuthScreen} options={{ headerShown: false }} />
      <Stack.Screen name="Subscribe" component={SubscribeScreen} options={{ title: 'Subscribe', headerRight: () => null }} />
      <Stack.Screen name="Main" component={MainDrawer} options={{ headerShown: false }} />
      <Stack.Screen
        name="CreateBot"
        component={CreateBotScreen}
        options={({ navigation }) => ({
          title: 'Create Bot',
          headerTransparent: true,
          header: () => (
            <AppHeader
              title="Create Bot"
              showBack
              transparent
              onBack={navigation.goBack}
              onProfile={() => navigation.getParent()?.dispatch(DrawerActions.toggleDrawer())}
            />
          ),
        })}
      />
      <Stack.Screen name="BotDetail" component={BotDetailScreen} options={{ title: 'Bot Details' }} />
      <Stack.Screen name="BotStrategyConfig" component={BotStrategyConfigScreen} options={{ title: 'Strategy Config' }} />
      <Stack.Screen name="BotBehaviorConfig" component={BotBehaviorConfigScreen} options={{ title: 'Behavior Config' }} />
      <Stack.Screen name="BotSettings" component={BotSettingsScreen} options={{ title: 'Bot Settings' }} />
      <Stack.Screen name="Profile" component={ProfileScreen} options={{ title: 'Profile', headerTransparent: true }} />
      <Stack.Screen name="Settings" component={SettingsScreen} options={{ title: 'Settings', headerTransparent: true }} />
      <Stack.Screen
        name="Billing"
        component={BillingScreen}
        options={{ title: 'Billing', headerTransparent: true }}
      />
      <Stack.Screen
        name="Deposit"
        component={DepositScreen}
        options={{ title: 'Fuel your fleet', headerTransparent: true }}
      />
      <Stack.Screen
        name="Withdraw"
        component={WithdrawScreen}
        options={{ title: 'Withdraw', headerTransparent: true }}
      />
    </Stack.Navigator>
  );
}

function ProfileDrawerContent(props: any) {
  const insets = useSafeAreaInsets();
  const nav = props.navigation;
  const getFocusedRouteName = (state: any): string | null => {
    if (!state?.routes || typeof state.index !== 'number') {
      return null;
    }
    const current = state.routes[state.index];
    if (!current) {
      return null;
    }
    if (current.state) {
      return getFocusedRouteName(current.state);
    }
    return current.name ?? null;
  };

  const currentRouteName = getFocusedRouteName(nav.getState()) ?? '';

  const items: { key: string; label: string; active?: boolean; onPress: () => void }[] = [
    {
      key: 'profile',
      label: 'Profile',
      active: currentRouteName === 'Profile',
      onPress: () => {
        nav.closeDrawer();
        nav.navigate('App', { screen: 'Profile' });
      },
    },
    {
      key: 'billing',
      label: 'Billing',
      active: currentRouteName === 'Billing',
      onPress: () => {
        nav.closeDrawer();
        nav.navigate('App', { screen: 'Billing' });
      },
    },
    {
      key: 'settings',
      label: 'Settings',
      active: currentRouteName === 'Settings',
      onPress: () => {
        nav.closeDrawer();
        nav.navigate('App', { screen: 'Settings' });
      },
    },
    {
      key: 'logout',
      label: 'Log out',
      onPress: () => {
        nav.closeDrawer();
        nav.navigate('App', { screen: 'Auth' });
      },
    },
  ];
  return (
    <DrawerContentScrollView
      {...props}
      style={styles.drawerScrollContainer}
      contentContainerStyle={styles.drawerScroll}
    >
      <ImageBackground source={SHIP_LOCKER_BG} style={styles.drawerBg} resizeMode="cover">
        <View style={[styles.drawerItemsWrap, { paddingTop: insets.top + 128 }]}>
          {items.map((item) => (
            <DrawerLabelButton
              key={item.key}
              label={item.label}
              active={item.active}
              onPress={item.onPress}
            />
          ))}
        </View>
      </ImageBackground>
    </DrawerContentScrollView>
  );
}

function DrawerLabelButton({
  label,
  onPress,
  active = false,
}: {
  label: string;
  onPress: () => void;
  active?: boolean;
}) {
  const getButtonSource = (pressed: boolean) => {
    if (active) return PANEL_BUTTON_ON;
    if (pressed) return PANEL_BUTTON_HOVER;
    return PANEL_BUTTON_OFF;
  };

  return (
    <Pressable
      style={styles.drawerLabel}
      onPress={onPress}
    >
      {({ pressed }) => (
        <>
          <ImageBackground
            source={getButtonSource(pressed)}
            style={styles.drawerLabelBg}
            imageStyle={styles.drawerLabelBgImage}
            resizeMode="stretch"
          />
          <Text style={styles.drawerLabelText}>{label}</Text>
        </>
      )}
    </Pressable>
  );
}

export function AppNavigator() {
  return (
    <NavigationContainer>
      <ProfileDrawer.Navigator
        screenOptions={{
          headerShown: false,
          drawerPosition: 'right',
          drawerType: 'front',
          swipeEnabled: false,
        }}
        drawerContent={(props) => <ProfileDrawerContent {...props} />}
      >
        <ProfileDrawer.Screen name="App" component={AppStack} />
      </ProfileDrawer.Navigator>
    </NavigationContainer>
  );
}

const styles = StyleSheet.create({
  drawerScrollContainer: {
    marginTop: 0,
    paddingTop: 0,
  },
  drawerScroll: {
    flexGrow: 1,
    paddingTop: 0,
  },
  drawerBg: {
    flex: 1,
    justifyContent: 'flex-start',
    paddingHorizontal: 0,
  },
  drawerItemsWrap: {
    gap: 9,
    paddingBottom: 20,
  },
  drawerLabel: {
    marginHorizontal: 0,
    borderRadius: 16,
    minHeight: 72,
    paddingVertical: 14,
    paddingHorizontal: 62,
    position: 'relative',
    overflow: 'hidden',
    justifyContent: 'center',
  },
  drawerLabelBg: {
    ...StyleSheet.absoluteFillObject,
  },
  drawerLabelBgImage: {
    borderRadius: 16,
  },
  drawerLabelText: {
    color: '#eef6ff',
    fontSize: 17,
    fontFamily: 'BNRumble',
    textAlign: 'left',
    letterSpacing: 0.25,
  },
});
