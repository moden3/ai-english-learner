import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  validateApiKey,
  fetchTopics,
  addTopic,
  deleteTopic,
  fetchVocabulary,
  addVocabulary,
  deleteVocabulary,
  generateText,
  analyzeText,
} from './api';

describe('API Client (api.ts)', () => {
  beforeEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  describe('validateApiKey', () => {
    it('returns true when the server responds with 200 OK', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(new Response('', { status: 200 }));

      const result = await validateApiKey('test-valid-key');
      expect(result).toBe(true);
      expect(fetch).toHaveBeenCalledWith(expect.stringContaining('/topics'), {
        method: 'GET',
        headers: { 'x-api-key': 'test-valid-key' },
      });
    });

    it('returns false when the server responds with 401/403 or error status', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(new Response('', { status: 401 }));

      const result = await validateApiKey('test-invalid-key');
      expect(result).toBe(false);
    });

    it('returns false when fetch throws a network error', async () => {
      vi.spyOn(globalThis, 'fetch').mockRejectedValueOnce(new Error('Network offline'));

      const result = await validateApiKey('test-key');
      expect(result).toBe(false);
    });
  });

  describe('fetchWithAuth behavior', () => {
    it('throws UNAUTHORIZED when no api_key exists in sessionStorage', async () => {
      await expect(fetchTopics()).rejects.toThrow('UNAUTHORIZED');
    });

    it('attaches x-api-key and Content-Type headers when authenticated', async () => {
      sessionStorage.setItem('api_key', 'my-auth-key');
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response(JSON.stringify([{ id: '1', name: 'tech' }]), { status: 200 })
      );

      const topics = await fetchTopics();
      expect(topics).toEqual([{ id: '1', name: 'tech' }]);

      expect(fetchSpy).toHaveBeenCalledTimes(1);
      const callOptions = fetchSpy.mock.calls[0][1];
      const headers = callOptions?.headers as Headers;
      expect(headers.get('x-api-key')).toBe('my-auth-key');
    });

    it('clears sessionStorage and reloads window when response status is 401', async () => {
      sessionStorage.setItem('api_key', 'revoked-key');
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response('Unauthorized', { status: 401 })
      );

      await expect(fetchTopics()).rejects.toThrow('UNAUTHORIZED');
      expect(sessionStorage.getItem('api_key')).toBeNull();
      expect(window.location.reload).toHaveBeenCalledTimes(1);
    });

    it('throws descriptive error on 500 server error', async () => {
      sessionStorage.setItem('api_key', 'my-auth-key');
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response('Internal Server Error', { status: 500 })
      );

      await expect(fetchTopics()).rejects.toThrow('API Error 500: Internal Server Error');
    });
  });

  describe('Topics API', () => {
    beforeEach(() => {
      sessionStorage.setItem('api_key', 'valid-key');
    });

    it('addTopic sends POST with JSON body', async () => {
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response(JSON.stringify({ success: true }), { status: 200 })
      );

      await addTopic('AI Ethics');
      expect(fetchSpy).toHaveBeenCalledWith(
        expect.stringContaining('/topics'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ name: 'AI Ethics' }),
        })
      );
    });

    it('deleteTopic sends DELETE request', async () => {
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response('', { status: 200 })
      );

      await deleteTopic('topic-123');
      expect(fetchSpy).toHaveBeenCalledWith(
        expect.stringContaining('/topics/topic-123'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });
  });

  describe('Vocabulary API', () => {
    beforeEach(() => {
      sessionStorage.setItem('api_key', 'valid-key');
    });

    it('fetchVocabulary retrieves word list', async () => {
      const mockList = [{ id: '1', word: 'ubiquitous', translation: '至る所にある' }];
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response(JSON.stringify(mockList), { status: 200 })
      );

      const res = await fetchVocabulary();
      expect(res).toEqual(mockList);
    });

    it('addVocabulary sends POST with word and translation', async () => {
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response(JSON.stringify({ success: true }), { status: 200 })
      );

      await addVocabulary('resilient', '回復力のある');
      expect(fetchSpy).toHaveBeenCalledWith(
        expect.stringContaining('/vocabulary'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ word: 'resilient', translation: '回復力のある' }),
        })
      );
    });

    it('deleteVocabulary sends DELETE request', async () => {
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response('', { status: 200 })
      );

      await deleteVocabulary('vocab-999');
      expect(fetchSpy).toHaveBeenCalledWith(
        expect.stringContaining('/vocabulary/vocab-999'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });
  });

  describe('Generate & Analyze API', () => {
    beforeEach(() => {
      sessionStorage.setItem('api_key', 'valid-key');
    });

    it('generateText sends topic_name and use_web_search flag', async () => {
      const mockResponse = { text: 'Sample AI generated text', source_url: 'https://news.example.com' };
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response(JSON.stringify(mockResponse), { status: 200 })
      );

      const result = await generateText('Semiconductors', true);
      expect(result).toEqual(mockResponse);
      expect(fetchSpy).toHaveBeenCalledWith(
        expect.stringContaining('/generate_text'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({
            topic_name: 'Semiconductors',
            use_web_search: true,
            action: 'generate',
          }),
        })
      );
    });

    it('analyzeText sends text to analyze', async () => {
      const mockAnalysis = {
        segments: [{ id: 1, text: 'Hello world', translation: 'こんにちは世界', grammar_note: '挨拶' }],
        keywords: [{ word: 'world', meaning: '世界', part_of_speech: 'noun', example: 'The world is big' }],
      };
      const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
        new Response(JSON.stringify(mockAnalysis), { status: 200 })
      );

      const result = await analyzeText('Hello world');
      expect(result).toEqual(mockAnalysis);
      expect(fetchSpy).toHaveBeenCalledWith(
        expect.stringContaining('/generate_text'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ action: 'analyze', text: 'Hello world' }),
        })
      );
    });
  });
});
