// Stub for @cedros/pay-react-native i18n module
// The mobile app doesn't use i18n features, so we provide empty implementations

export const detectLocale = () => 'en';
export const loadLocale = async () => ({});
export const getAvailableLocales = () => ['en'];
export const createTranslator = () => (key) => key;
export const getLocalizedError = (error) => error.message || String(error);
