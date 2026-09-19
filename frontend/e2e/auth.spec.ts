import { test, expect } from '@playwright/test';
import { setupApiMocks } from './mocks';

test.describe('Authentication & Session Flow (E2E)', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
  });

  test('shows login screen on initial visit', async ({ page }) => {
    await page.goto('/');

    await expect(page.getByText('ENG-APP')).toBeVisible();
    await expect(page.getByText('Please enter your API Key to access the application.')).toBeVisible();
    await expect(page.getByPlaceholder('API Key')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Login' })).toBeVisible();
  });

  test('displays error message on invalid API key', async ({ page }) => {
    await page.goto('/');

    await page.getByPlaceholder('API Key').fill('invalid-key');
    await page.getByRole('button', { name: 'Login' }).click();

    await expect(page.getByText('Invalid API Key. Please try again.')).toBeVisible();
    // ダッシュボードには遷移していないこと
    await expect(page.getByRole('button', { name: 'English Generator' })).not.toBeVisible();
  });

  test('successfully logs in, persists session on reload, and logs out', async ({ page }) => {
    await page.goto('/');

    // 1. 正常なキーでログイン
    await page.getByPlaceholder('API Key').fill('valid-test-key');
    await page.getByRole('button', { name: 'Login' }).click();

    // 2. ダッシュボードが表示される
    await expect(page.getByRole('button', { name: 'English Generator' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Generate Text' })).toBeVisible();

    // 3. リロードしてもログイン状態が維持されている
    await page.reload();
    await expect(page.getByRole('button', { name: 'English Generator' })).toBeVisible();

    // 4. ログアウトボタン押下でログイン画面に戻る
    await page.getByRole('button', { name: 'Logout' }).click();
    await expect(page.getByPlaceholder('API Key')).toBeVisible();
    await expect(page.getByRole('button', { name: 'English Generator' })).not.toBeVisible();
  });
});
