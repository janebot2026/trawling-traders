import React, { useEffect } from 'react';
import { 
  View, 
  Text, 
  StyleSheet, 
  Button, 
  ActivityIndicator,
  Image,
  ScrollView,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';

// LOB Avatar - our lobster mascot
const LOB_AVATAR = require('../../assets/lob-avatar.png');

type AuthScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Auth'>;

export function AuthScreen() {
  const navigation = useNavigation<AuthScreenNavigationProp>();
  const [isLoading, setIsLoading] = React.useState(true);

  useEffect(() => {
    // TODO: Check for existing session
    // If logged in, navigate to Main
    // If not, show Cedros Login
    
    setTimeout(() => {
      setIsLoading(false);
    }, 1500); // Slightly longer to show off the mascot
  }, []);

  const handleLogin = () => {
    // TODO: Integrate Cedros Login
    navigation.navigate('Subscribe');
  };

  if (isLoading) {
    return (
      <View style={styles.container}>
        <Image source={LOB_AVATAR} style={styles.loadingAvatar} />
        <Text style={styles.loadingText}>Loading...</Text>
      </View>
    );
  }

  return (
    <ScrollView contentContainerStyle={styles.scrollContainer}>
      <View style={styles.container}>
        {/* LOB Mascot */}
        <View style={styles.mascotContainer}>
          <Image source={LOB_AVATAR} style={styles.avatar} />
          <View style={styles.badge}>
            <Text style={styles.badgeText}>#4821</Text>
          </View>
        </View>

        <Text style={styles.greeting}>Ahoy, Trader! 🦞</Text>
        <Text style={styles.title}>Trawling Traders</Text>
        <Text style={styles.subtitle}>
          Deploy AI-powered trading bots that trawl the markets 24/7
        </Text>

        {/* Feature highlights */}
        <View style={styles.features}>
          <View style={styles.feature}>
            <Text style={styles.featureIcon}>🤖</Text>
            <Text style={styles.featureText}>Up to 4 AI bots</Text>
          </View>
          <View style={styles.feature}>
            <Text style={styles.featureIcon}>📈</Text>
            <Text style={styles.featureText}>xStocks & crypto</Text>
          </View>
          <View style={styles.feature}>
            <Text style={styles.featureIcon}>🔒</Text>
            <Text style={styles.featureText}>Your own VPS</Text>
          </View>
        </View>
        
        <View style={styles.buttonContainer}>
          <Button title="Sign In with Cedros" onPress={handleLogin} />
        </View>
        
        <Text style={styles.hint}>Secure authentication powered by Cedros</Text>
        
        {/* Lobster footer */}
        <Text style={styles.footer}>
          Meet LOB — your trading companion
        </Text>
      </View>
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  scrollContainer: {
    flexGrow: 1,
  },
  container: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 20,
    backgroundColor: '#fff',
  },
  loadingAvatar: {
    width: 120,
    height: 120,
    borderRadius: 60,
  },
  loadingText: {
    marginTop: 16,
    fontSize: 16,
    color: '#666',
  },
  mascotContainer: {
    position: 'relative',
    marginBottom: 20,
  },
  avatar: {
    width: 150,
    height: 150,
    borderRadius: 75,
    borderWidth: 4,
    borderColor: '#007AFF',
  },
  badge: {
    position: 'absolute',
    bottom: 0,
    right: 0,
    backgroundColor: '#22c55e',
    paddingHorizontal: 12,
    paddingVertical: 4,
    borderRadius: 12,
    borderWidth: 3,
    borderColor: '#fff',
  },
  badgeText: {
    color: '#fff',
    fontWeight: 'bold',
    fontSize: 14,
  },
  greeting: {
    fontSize: 24,
    fontWeight: '600',
    color: '#007AFF',
    marginBottom: 8,
  },
  title: {
    fontSize: 32,
    fontWeight: 'bold',
    marginBottom: 10,
  },
  subtitle: {
    fontSize: 16,
    color: '#666',
    textAlign: 'center',
    marginBottom: 30,
    maxWidth: 300,
  },
  features: {
    flexDirection: 'row',
    marginBottom: 30,
    gap: 20,
  },
  feature: {
    alignItems: 'center',
  },
  featureIcon: {
    fontSize: 28,
    marginBottom: 4,
  },
  featureText: {
    fontSize: 12,
    color: '#666',
  },
  buttonContainer: {
    width: '100%',
    maxWidth: 300,
    marginBottom: 20,
  },
  hint: {
    fontSize: 12,
    color: '#999',
    marginBottom: 20,
  },
  footer: {
    fontSize: 14,
    color: '#999',
    fontStyle: 'italic',
  },
});
