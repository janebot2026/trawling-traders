import React, { useEffect } from 'react';
import { View, Text, StyleSheet, Button, ActivityIndicator } from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';

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
    }, 1000);
  }, []);

  const handleLogin = () => {
    // TODO: Integrate Cedros Login
    // For now, just navigate to Subscribe as a placeholder
    navigation.navigate('Subscribe');
  };

  if (isLoading) {
    return (
      <View style={styles.container}>
        <ActivityIndicator size="large" />
      </View>
    );
  }

  return (
    <View style={styles.container}>
      <Text style={styles.title}>Trawling Traders</Text>
      <Text style={styles.subtitle}>Automated Trading with AI</Text>
      
      <View style={styles.buttonContainer}>
        <Button title="Sign In with Cedros" onPress={handleLogin} />
      </View>
      
      <Text style={styles.hint}>Secure authentication powered by Cedros</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 20,
    backgroundColor: '#fff',
  },
  title: {
    fontSize: 32,
    fontWeight: 'bold',
    marginBottom: 10,
  },
  subtitle: {
    fontSize: 18,
    color: '#666',
    marginBottom: 40,
  },
  buttonContainer: {
    width: '100%',
    maxWidth: 300,
    marginBottom: 20,
  },
  hint: {
    fontSize: 12,
    color: '#999',
  },
});
