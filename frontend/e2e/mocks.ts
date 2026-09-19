import type { Page } from '@playwright/test';

export const mockTopics = [
  { id: 'topic-1', name: 'Global Economy' },
  { id: 'topic-2', name: 'Artificial Intelligence' },
];

export const mockVocabulary = [
  { id: 'vocab-1', word: 'ubiquitous', translation: '至る所にある' },
  { id: 'vocab-2', word: 'lucrative', translation: '利益の上がる' },
];

export const mockGenerateResult = {
  text: 'Artificial intelligence is revolutionizing modern manufacturing and supply chains across the globe.',
  source_url: 'https://news.example.com/ai-industry',
};

export const mockAnalyzeResult = {
  segments: [
    {
      id: 1,
      text: 'Artificial intelligence is revolutionizing',
      translation: '人工知能は変革をもたらしている',
      grammar_note: '主語(S) + 現在進行形(V)',
    },
    {
      id: 2,
      text: 'modern manufacturing and supply chains',
      translation: '現代の製造業とサプライチェーンに',
      grammar_note: '目的語(O)',
    },
    {
      id: 3,
      text: 'across the globe.',
      translation: '世界中で。',
      grammar_note: '前置詞句(prep)',
    },
  ],
  keywords: [
    {
      word: 'revolutionizing',
      meaning: '大改革をもたらしている',
      part_of_speech: 'verb',
      example: 'AI is revolutionizing the tech industry.',
    },
  ],
};

/**
 * すべてのAPIエンドポイントをインターセプトしてモックレスポンスを返すセットアップ関数
 * これによりバックエンド未起動でも、外部APIトークン消費ゼロで実ブラウザE2Eテストを実行可能。
 */
export async function setupApiMocks(page: Page, options: { topics?: typeof mockTopics; vocab?: typeof mockVocabulary } = {}) {
  let currentTopics = options.topics ? [...options.topics] : [...mockTopics];
  let currentVocab = options.vocab ? [...options.vocab] : [...mockVocabulary];

  // Topics API
  await page.route(/\/topics/, async (route) => {
    const method = route.request().method();
    const url = route.request().url();

    if (method === 'GET') {
      const apiKey = route.request().headers()['x-api-key'];
      if (apiKey === 'invalid-key') {
        return route.fulfill({ status: 401, body: 'Unauthorized' });
      }
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(currentTopics) });
    }

    if (method === 'POST') {
      const postData = route.request().postDataJSON();
      const newTopic = { id: `topic-${Date.now()}`, name: postData.name };
      currentTopics.push(newTopic);
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(newTopic) });
    }

    if (method === 'DELETE') {
      const pathname = new URL(url).pathname;
      const id = pathname.split('/').filter(Boolean).pop();
      currentTopics = currentTopics.filter(t => t.id !== id);
      return route.fulfill({ status: 200, body: '' });
    }

    return route.continue();
  });

  // Vocabulary API
  await page.route(/\/vocabulary/, async (route) => {
    const method = route.request().method();
    const url = route.request().url();

    if (method === 'GET') {
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(currentVocab) });
    }

    if (method === 'POST') {
      const postData = route.request().postDataJSON();
      const newVocab = { id: `vocab-${Date.now()}`, word: postData.word, translation: postData.translation };
      currentVocab.push(newVocab);
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(newVocab) });
    }

    if (method === 'DELETE') {
      const pathname = new URL(url).pathname;
      const id = pathname.split('/').filter(Boolean).pop();
      currentVocab = currentVocab.filter(v => v.id !== id);
      return route.fulfill({ status: 200, body: '' });
    }

    return route.continue();
  });

  // Generate Text API
  await page.route(/\/generate_text/, async (route) => {
    const postData = route.request().postDataJSON();
    if (postData?.action === 'analyze') {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockAnalyzeResult),
      });
    }

    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockGenerateResult),
    });
  });
}
