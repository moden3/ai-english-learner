import { test, expect } from '@playwright/test';
import { setupApiMocks, mockGenerateResult } from './mocks';

test.describe('Article Generation & Analysis Flow (E2E)', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);

    // ログイン状態にして開始
    await page.goto('/');
    await page.getByPlaceholder('API Key').fill('valid-test-key');
    await page.getByRole('button', { name: 'Login' }).click();
    await expect(page.getByRole('heading', { name: 'Generate Text' })).toBeVisible();
  });

  test('generates article, switches to slash reading, expands grammar notes, and saves vocabulary', async ({ page }) => {
    // 1. トピック選択
    const topicSelect = page.getByRole('combobox');
    await expect(topicSelect).toBeVisible();
    await topicSelect.selectOption('Artificial Intelligence');

    // 2. Web検索チェックボックスをONにする
    const webSearchCheckbox = page.getByLabel('🌐 最新ニュースを参照して生成する (Tavily Web検索)');
    await expect(webSearchCheckbox).not.toBeChecked();
    await webSearchCheckbox.check();
    await expect(webSearchCheckbox).toBeChecked();

    // 3. 記事生成ボタンをクリック
    await page.getByRole('button', { name: /Generate Now/i }).click();

    // 4. 生成結果（英文テキストと参照元URL）が表示される
    await expect(page.getByText(mockGenerateResult.text)).toBeVisible();
    await expect(page.getByRole('link', { name: mockGenerateResult.source_url })).toBeVisible();

    // 5. 「Slash Reading & Analysis」タブに切り替える
    await page.getByRole('button', { name: 'Slash Reading & Analysis' }).click();

    // 6. スラッシュリーディングのセグメントとキーワードが表示される
    const segment = page.getByText('Artificial intelligence is revolutionizing');
    await expect(segment).toBeVisible();
    await expect(page.getByText('revolutionizing', { exact: true })).toBeVisible();

    // 7. セグメントをクリックしてアコーディオンを展開
    await segment.click();
    await expect(page.getByText('人工知能は変革をもたらしている')).toBeVisible();
    await expect(page.getByText('主語(S) + 現在進行形(V)')).toBeVisible();

    // 8. キーワードの単語帳保存ボタンをクリック
    const dialogPromise = page.waitForEvent('dialog');
    await page.getByRole('button', { name: '+ Save to Vocabulary' }).click();
    const dialog = await dialogPromise;
    expect(dialog.message()).toContain('Saved "revolutionizing" to Vocabulary!');
    await dialog.accept();
  });
});
