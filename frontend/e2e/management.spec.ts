import { test, expect } from '@playwright/test';
import { setupApiMocks } from './mocks';

test.describe('Topic & Vocabulary Management Flow (E2E)', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);

    // ログイン状態にして開始
    await page.goto('/');
    await page.getByPlaceholder('API Key').fill('valid-test-key');
    await page.getByRole('button', { name: 'Login' }).click();
    await expect(page.getByRole('heading', { name: 'Generate Text' })).toBeVisible();
  });

  test('manages topics: views list, adds new topic, and deletes existing topic', async ({ page }) => {
    // 1. Topic Manager へ遷移
    await page.getByRole('button', { name: 'Topic Manager' }).click();
    await expect(page.getByRole('heading', { name: 'Topics Manager' })).toBeVisible();

    // 2. 既存トピックが表示されている
    await expect(page.getByText('Global Economy')).toBeVisible();
    await expect(page.getByText('Artificial Intelligence')).toBeVisible();

    // 3. 新規トピックを追加
    const topicInput = page.getByPlaceholder(/New topic/i);
    await topicInput.fill('Renewable Energy');
    await page.getByRole('button', { name: 'Add Topic' }).click();

    // 4. 追加されたトピックが表示される
    await expect(page.getByText('Renewable Energy')).toBeVisible();

    // 5. トピックを削除 (confirm ダイアログを自動承認)
    page.once('dialog', async (dialog) => {
      await dialog.accept();
    });
    const topicItem = page.locator('li').filter({ hasText: 'Global Economy' });
    await topicItem.getByRole('button', { name: 'Delete' }).click();

    // 6. 削除されたトピックが表示されなくなる
    await expect(page.getByText('Global Economy')).not.toBeVisible();
  });

  test('manages vocabulary: views list, adds new word, and deletes word', async ({ page }) => {
    // 1. Vocabulary List へ遷移
    await page.getByRole('button', { name: 'Vocabulary List' }).click();
    await expect(page.getByRole('heading', { name: 'Vocabulary List' })).toBeVisible();

    // 2. 既存単語が表示されている
    await expect(page.getByText('ubiquitous')).toBeVisible();
    await expect(page.getByText('至る所にある')).toBeVisible();

    // 3. 新規単語を登録
    await page.getByPlaceholder('English Word/Phrase').fill('paradigm');
    await page.getByPlaceholder('Japanese Translation').fill('規範・枠組み');
    await page.getByRole('button', { name: 'Save Word' }).click();

    // 4. 追加された単語が表示される
    await expect(page.getByText('paradigm')).toBeVisible();
    await expect(page.getByText('規範・枠組み')).toBeVisible();

    // 5. 単語を削除 (confirm ダイアログを自動承認)
    page.once('dialog', async (dialog) => {
      await dialog.accept();
    });
    const vocabItem = page.locator('li').filter({ hasText: 'ubiquitous' });
    await vocabItem.getByRole('button', { name: 'Delete' }).click();

    // 6. 削除された単語が表示されなくなる
    await expect(page.getByText('ubiquitous')).not.toBeVisible();
  });
});
