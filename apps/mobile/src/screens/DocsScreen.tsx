import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ActivityIndicator,
  ImageBackground,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import type { DocsArticle, DocsCategory, TrackDocsEventRequest } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { lightTheme } from '../theme';
const DOCS_BG = require('../../../../assets/branding/tt-docs.png');
const HEADER_HEIGHT = 56;

type DocsCategoryId = string;

function matchesQuery(text: string, query: string): boolean {
  return text.toLowerCase().includes(query);
}

function articleMatchesQuery(article: DocsArticle, query: string): boolean {
  return (
    matchesQuery(article.title, query) ||
    matchesQuery(article.summary, query) ||
    article.content.some((line) => matchesQuery(line, query))
  );
}

export function DocsScreen() {
  const insets = useSafeAreaInsets();
  const contentTopPadding = insets.top + HEADER_HEIGHT + 10;
  const [categories, setCategories] = useState<DocsCategory[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedCategoryId, setSelectedCategoryId] = useState<DocsCategoryId | null>(null);
  const [selectedArticleId, setSelectedArticleId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  const searchTrackTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastTrackedSearchKey = useRef('');

  const normalizedQuery = searchQuery.trim().toLowerCase();

  const selectedCategory = useMemo(
    () => categories.find((category) => category.id === selectedCategoryId) ?? null,
    [categories, selectedCategoryId]
  );

  const selectedArticle = useMemo(
    () => selectedCategory?.articles.find((article) => article.id === selectedArticleId) ?? null,
    [selectedCategory, selectedArticleId]
  );

  // Single pass: filter categories and compute per-category match counts together.
  const { filteredOverviewCategories, filteredOverviewCounts } = useMemo(() => {
    const counts: Record<string, number> = {};
    if (!normalizedQuery) {
      for (const category of categories) {
        counts[category.id] = category.articles.length;
      }
      return { filteredOverviewCategories: categories, filteredOverviewCounts: counts };
    }

    const matched: typeof categories = [];
    for (const category of categories) {
      const matchingArticles = category.articles.filter((article) =>
        articleMatchesQuery(article, normalizedQuery)
      );
      const categoryMatches =
        matchesQuery(category.title, normalizedQuery) ||
        matchesQuery(category.description, normalizedQuery) ||
        matchingArticles.length > 0;
      if (categoryMatches) {
        matched.push(category);
        counts[category.id] = matchingArticles.length;
      }
    }
    return { filteredOverviewCategories: matched, filteredOverviewCounts: counts };
  }, [categories, normalizedQuery]);

  const filteredCategoryArticles = useMemo(() => {
    if (!selectedCategory) return [];
    if (!normalizedQuery) return selectedCategory.articles;
    return selectedCategory.articles.filter((article) => articleMatchesQuery(article, normalizedQuery));
  }, [selectedCategory, normalizedQuery]);

  const trackDocsEvent = useCallback(async (request: TrackDocsEventRequest) => {
    try {
      await api.docs.trackEvent(request);
    } catch {
      // Non-blocking analytics
    }
  }, []);

  const loadDocs = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await api.docs.getDocs();
      setCategories(response.categories);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load docs');
      setCategories([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadDocs();
  }, [loadDocs]);

  useEffect(() => {
    if (searchTrackTimer.current) {
      clearTimeout(searchTrackTimer.current);
    }

    if (normalizedQuery.length < 2) {
      return;
    }

    const searchScope = selectedCategory?.id ?? 'all';
    const trackKey = `${searchScope}::${normalizedQuery}`;
    if (trackKey === lastTrackedSearchKey.current) {
      return;
    }

    const resultsCount = selectedCategory ? filteredCategoryArticles.length : filteredOverviewCategories.length;

    searchTrackTimer.current = setTimeout(() => {
      trackDocsEvent({
        eventType: 'search',
        categoryId: selectedCategory?.id,
        query: normalizedQuery,
        resultsCount,
      });
      lastTrackedSearchKey.current = trackKey;
    }, 450);

    return () => {
      if (searchTrackTimer.current) {
        clearTimeout(searchTrackTimer.current);
      }
    };
  }, [
    filteredCategoryArticles.length,
    filteredOverviewCategories.length,
    normalizedQuery,
    selectedCategory,
    trackDocsEvent,
  ]);

  const openCategory = (categoryId: DocsCategoryId) => {
    setSelectedArticleId(null);
    setSelectedCategoryId(categoryId);
    trackDocsEvent({
      eventType: 'category_opened',
      categoryId,
    });
  };

  const openArticle = (articleId: string) => {
    setSelectedArticleId(articleId);
    trackDocsEvent({
      eventType: 'article_opened',
      categoryId: selectedCategory?.id,
      articleId,
    });
  };

  const backToOverview = () => {
    setSelectedArticleId(null);
    setSelectedCategoryId(null);
  };

  const backToCategory = () => {
    setSelectedArticleId(null);
  };

  const subtitle = selectedArticle
    ? selectedArticle.summary
    : selectedCategory
      ? selectedCategory.description
      : 'Browse setup guides, optimization playbooks, and support references.';

  return (
    <ImageBackground source={DOCS_BG} style={styles.bgFill} resizeMode="cover">
      <ScrollView contentContainerStyle={[styles.content, { paddingTop: contentTopPadding }]} keyboardShouldPersistTaps="handled">
        <Text style={styles.title}>Docs</Text>
        <Text style={styles.subtitle}>{subtitle}</Text>

        <View style={styles.searchCard}>
          <Text style={styles.searchLabel}>Search</Text>
          <TextInput
            style={styles.searchInput}
            value={searchQuery}
            onChangeText={setSearchQuery}
            placeholder={selectedCategory ? 'Search this category...' : 'Search all docs...'}
            placeholderTextColor={lightTheme.colors.wave[400]}
          />
        </View>

        {isLoading && (
          <View style={styles.centeredBlock}>
            <ActivityIndicator color={lightTheme.colors.primary[700]} />
          </View>
        )}

        {!isLoading && error && (
          <View style={styles.card}>
            <Text style={styles.cardTitle}>Unable to load docs</Text>
            <Text style={styles.cardDescription}>{error}</Text>
            <TouchableOpacity style={styles.actionButton} onPress={loadDocs}>
              <Text style={styles.actionButtonText}>Retry</Text>
            </TouchableOpacity>
          </View>
        )}

        {!isLoading && !error && !selectedCategory && (
          <View style={styles.stack}>
            {filteredOverviewCategories.map((category) => (
              <TouchableOpacity key={category.id} style={styles.card} onPress={() => openCategory(category.id)}>
                <Text style={styles.cardTitle}>{category.title}</Text>
                <Text style={styles.cardDescription}>{category.description}</Text>
                <Text style={styles.linkText}>{filteredOverviewCounts[category.id] ?? 0} articles</Text>
              </TouchableOpacity>
            ))}
            {filteredOverviewCategories.length === 0 && (
              <View style={styles.card}>
                <Text style={styles.cardTitle}>No matches</Text>
                <Text style={styles.cardDescription}>Try a different search term.</Text>
              </View>
            )}
          </View>
        )}

        {!isLoading && !error && selectedCategory && !selectedArticle && (
          <View style={styles.stack}>
            <TouchableOpacity style={styles.backButton} onPress={backToOverview}>
              <Text style={styles.backButtonText}>← Docs Overview</Text>
            </TouchableOpacity>

            {filteredCategoryArticles.map((article) => (
              <TouchableOpacity key={article.id} style={styles.card} onPress={() => openArticle(article.id)}>
                <Text style={styles.cardTitle}>{article.title}</Text>
                <Text style={styles.cardDescription}>{article.summary}</Text>
                <Text style={styles.linkText}>Read Article</Text>
              </TouchableOpacity>
            ))}

            {filteredCategoryArticles.length === 0 && (
              <View style={styles.card}>
                <Text style={styles.cardTitle}>No matches in {selectedCategory.title}</Text>
                <Text style={styles.cardDescription}>Adjust your search to find related articles.</Text>
              </View>
            )}
          </View>
        )}

        {!isLoading && !error && selectedCategory && selectedArticle && (
          <View style={styles.stack}>
            <TouchableOpacity style={styles.backButton} onPress={backToCategory}>
              <Text style={styles.backButtonText}>← {selectedCategory.title}</Text>
            </TouchableOpacity>

            <View style={styles.card}>
              <Text style={styles.articleTitle}>{selectedArticle.title}</Text>
              {selectedArticle.content.map((paragraph) => (
                <Text key={paragraph} style={styles.articleParagraph}>
                  {paragraph}
                </Text>
              ))}
            </View>
          </View>
        )}
      </ScrollView>
    </ImageBackground>
  );
}

const styles = StyleSheet.create({
  bgFill: {
    flex: 1,
  },
  content: {
    padding: 16,
    paddingBottom: 28,
  },
  title: {
    fontSize: 30,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
    fontFamily: lightTheme.typography.families.display,
  },
  subtitle: {
    marginTop: 8,
    fontSize: 14,
    color: lightTheme.colors.wave[600],
  },
  searchCard: {
    marginTop: 14,
    borderRadius: 16,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    backgroundColor: lightTheme.colors.surface,
    padding: 12,
  },
  searchLabel: {
    fontSize: 12,
    color: lightTheme.colors.wave[500],
    marginBottom: 6,
  },
  searchInput: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    borderRadius: 10,
    paddingHorizontal: 12,
    paddingVertical: 10,
    backgroundColor: '#fff',
    fontSize: 14,
    color: lightTheme.colors.wave[900],
  },
  centeredBlock: {
    paddingVertical: 24,
    alignItems: 'center',
  },
  stack: {
    marginTop: 12,
    gap: 10,
  },
  card: {
    borderRadius: 16,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    backgroundColor: lightTheme.colors.surface,
    padding: 14,
  },
  cardTitle: {
    fontSize: 17,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
  },
  cardDescription: {
    marginTop: 6,
    fontSize: 13,
    color: lightTheme.colors.wave[600],
    lineHeight: 19,
  },
  linkText: {
    marginTop: 10,
    fontSize: 13,
    fontWeight: '700',
    color: lightTheme.colors.primary[700],
  },
  backButton: {
    alignSelf: 'flex-start',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    paddingHorizontal: 10,
    paddingVertical: 6,
    backgroundColor: '#fff',
  },
  backButtonText: {
    fontSize: 13,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
  },
  articleTitle: {
    fontSize: 24,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
    fontFamily: lightTheme.typography.families.display,
  },
  articleParagraph: {
    marginTop: 12,
    fontSize: 15,
    color: lightTheme.colors.wave[800],
    lineHeight: 22,
  },
  actionButton: {
    marginTop: 12,
    borderRadius: 10,
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 10,
    alignItems: 'center',
  },
  actionButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '700',
  },
});
