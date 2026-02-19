import type {
  DocsCategory,
  EmailCsvReportRequest,
  EmailCsvReportResponse,
  GetDocsResponse,
  TrackDocsEventRequest,
  TrackDocsEventResponse,
} from '@trawling-traders/types';
import { fetchApi } from './http';
import { RawDocsArticle, RawDocsCategory } from './raw-types';

export const docsApi = {
  async getDocs(): Promise<GetDocsResponse> {
    const response = await fetchApi('/docs');

    const categories = (response.categories || []).map((category: RawDocsCategory): DocsCategory => ({
      id: category.id,
      title: category.title,
      description: category.description ?? '',
      articles: (category.articles || []).map((article: RawDocsArticle) => ({
        id: article.id,
        title: article.title,
        summary: article.summary ?? '',
        content: Array.isArray(article.content) ? article.content.map((line: unknown) => String(line)) : [],
      })),
    }));

    return { categories };
  },

  async trackEvent(request: TrackDocsEventRequest): Promise<TrackDocsEventResponse> {
    const response = await fetchApi('/docs/analytics', {
      method: 'POST',
      body: JSON.stringify({
        event_type: request.eventType,
        category_id: request.categoryId,
        article_id: request.articleId,
        query: request.query,
        results_count: request.resultsCount,
      }),
    });

    return {
      success: Boolean(response.success),
    };
  },
};

export const reportsApi = {
  async requestEmailCsv(request: EmailCsvReportRequest): Promise<EmailCsvReportResponse> {
    const response = await fetchApi('/reports/email-csv', {
      method: 'POST',
      body: JSON.stringify({
        report_kind: request.reportKind,
        timeframe: request.timeframe,
      }),
    });

    return {
      success: Boolean(response.success),
      message: response.message,
      deliveredTo: response.deliveredTo ?? response.delivered_to,
      rowsIncluded: Number(response.rowsIncluded ?? response.rows_included ?? 0),
    };
  },
};
