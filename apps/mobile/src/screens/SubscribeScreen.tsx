import React from 'react';
import { View, Text, StyleSheet, Button } from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';

type SubscribeScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Subscribe'>;

export function SubscribeScreen() {
  const navigation = useNavigation<SubscribeScreenNavigationProp>();

  const handleSubscribe = () => {
    // TODO: Integrate Cedros Pay
    // For now, just navigate to main app
    navigation.navigate('Main');
  };

  return (
    <View style={styles.container}>
      <Text style={styles.title}>Choose Your Plan</Text>
      
      <View style={styles.planCard}>
        <Text style={styles.planName}>Trader Pro</Text>
        <Text style={styles.planPrice}>$29/month</Text>
        <View style={styles.features}>
          <Text style={styles.feature}>• Up to 4 trading bots</Text>
          <Text style={styles.feature}>• Paper & Live trading</Text>
          <Text style={styles.feature}>• Real-time metrics</Text>
          <Text style={styles.feature}>• Priority support</Text>
        </View>
        
        <Button title="Subscribe Now" onPress={handleSubscribe} />
      </View>
      
      <Text style={styles.secure}>🔒 Secure payment via Cedros Pay</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    padding: 20,
    backgroundColor: '#f5f5f5',
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
    marginBottom: 30,
    textAlign: 'center',
  },
  planCard: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 24,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 3,
  },
  planName: {
    fontSize: 24,
    fontWeight: 'bold',
    marginBottom: 8,
  },
  planPrice: {
    fontSize: 32,
    color: '#007AFF',
    marginBottom: 20,
  },
  features: {
    marginBottom: 24,
  },
  feature: {
    fontSize: 16,
    marginBottom: 8,
    color: '#333',
  },
  secure: {
    marginTop: 20,
    textAlign: 'center',
    color: '#666',
  },
});
