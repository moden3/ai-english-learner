const API_URL = import.meta.env.VITE_API_URL || '';

if (!API_URL) {
  console.warn('VITE_API_URL is missing. Check your .env file.');
}

/**
 * バックエンド（API Gateway）への共通のFetch関数
 * 常に sessionStorage から取得した x-api-key ヘッダーを付与してリクエストを行います。
 */
async function fetchWithAuth(endpoint: string, options: RequestInit = {}) {
  const API_KEY = sessionStorage.getItem('api_key');
  if (!API_KEY) {
    throw new Error('UNAUTHORIZED');
  }

  const headers = new Headers(options.headers);
  headers.set('x-api-key', API_KEY);
  
  // GET以外のリクエストでContent-Typeが未指定の場合はapplication/jsonをセット
  if (!headers.has('Content-Type') && options.method && options.method !== 'GET') {
    headers.set('Content-Type', 'application/json');
  }

  const response = await fetch(`${API_URL}${endpoint}`, {
    ...options,
    headers,
  });

  // APIキーが間違っている場合は強制ログアウト
  if (response.status === 401 || response.status === 403) {
    sessionStorage.removeItem('api_key');
    window.location.reload();
    throw new Error('UNAUTHORIZED');
  }

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`API Error ${response.status}: ${errorText}`);
  }

  // Deleteなどのレスポンスが空の場合を考慮
  const text = await response.text();
  return text ? JSON.parse(text) : {};
}

// ==========================================
// Topics API
// ==========================================
export interface Topic {
  id: string;
  name: string;
}

export const fetchTopics = () => fetchWithAuth('/topics') as Promise<Topic[]>;
export const addTopic = (name: string) => fetchWithAuth('/topics', { method: 'POST', body: JSON.stringify({ name }) });
export const deleteTopic = (id: string) => fetchWithAuth(`/topics/${id}`, { method: 'DELETE' });

export const validateApiKey = async (key: string): Promise<boolean> => {
  try {
    const response = await fetch(`${API_URL}/topics`, {
      method: 'GET',
      headers: {
        'x-api-key': key
      }
    });
    return response.ok;
  } catch (err) {
    return false;
  }
};

// ==========================================
// Vocabulary API
// ==========================================
export interface Vocabulary {
  id: string;
  word: string;
  translation: string;
}

export const fetchVocabulary = () => fetchWithAuth('/vocabulary') as Promise<Vocabulary[]>;
export const addVocabulary = (word: string, translation: string) => fetchWithAuth('/vocabulary', { method: 'POST', body: JSON.stringify({ word, translation }) });
export const deleteVocabulary = (id: string) => fetchWithAuth(`/vocabulary/${id}`, { method: 'DELETE' });

// ==========================================
// Generate Text API (AI連携)
// ==========================================
export interface GenerateResult {
  text: string;
  source_url?: string;
}

export const generateText = (topic_name: string, use_lite_model: boolean = true) => 
  fetchWithAuth('/generate_text', { 
    method: 'POST', 
    body: JSON.stringify({ topic_name, use_lite_model, action: 'generate' }) 
  }) as Promise<GenerateResult>;

export interface AnalyzeSegment {
  id: number;
  text: string;
  translation: string;
  grammar_note: string;
}

export interface AnalyzeKeyword {
  word: string;
  meaning: string;
  part_of_speech: string;
  example: string;
}

export interface AnalyzeResult {
  segments: AnalyzeSegment[];
  keywords: AnalyzeKeyword[];
}

export const analyzeText = (text: string) => 
  fetchWithAuth('/generate_text', { 
    method: 'POST', 
    body: JSON.stringify({ action: 'analyze', text, use_lite_model: true }) 
  }) as Promise<AnalyzeResult>;
