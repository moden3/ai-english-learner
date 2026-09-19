import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from './App';
import * as api from './api';

describe('App Component (Routing & Auth Integration)', () => {
  beforeEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
    vi.spyOn(api, 'fetchTopics').mockResolvedValue([]);
    vi.spyOn(api, 'fetchVocabulary').mockResolvedValue([]);
  });

  it('renders Login screen when unauthenticated', () => {
    render(<App />);

    expect(screen.getByText('Please enter your API Key to access the application.')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('API Key')).toBeInTheDocument();
    expect(screen.queryByText('English Generator')).not.toBeInTheDocument();
  });

  it('renders main application when api_key exists in sessionStorage', async () => {
    sessionStorage.setItem('api_key', 'existing-secret-key');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'ENG-APP' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'English Generator' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Topic Manager' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Vocabulary List' })).toBeInTheDocument();
    });
  });

  it('transitions from Login to Main App on successful login', async () => {
    vi.spyOn(api, 'validateApiKey').mockResolvedValueOnce(true);
    const user = userEvent.setup();

    render(<App />);

    const input = screen.getByPlaceholderText('API Key');
    const loginButton = screen.getByRole('button', { name: 'Login' });

    await user.type(input, 'my-super-secret-key');
    await user.click(loginButton);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'ENG-APP' })).toBeInTheDocument();
      expect(sessionStorage.getItem('api_key')).toBe('my-super-secret-key');
    });
  });

  it('navigates between tabs via sidebar buttons', async () => {
    sessionStorage.setItem('api_key', 'valid-key');
    const user = userEvent.setup();

    render(<App />);

    // 初期タブ: Generate Text
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Generate Text' })).toBeInTheDocument();
    });

    // Topic Manager へ切り替え
    const topicsTab = screen.getByRole('button', { name: 'Topic Manager' });
    await user.click(topicsTab);
    expect(screen.getByRole('heading', { name: 'Topics Manager' })).toBeInTheDocument();

    // Vocabulary List へ切り替え
    const vocabTab = screen.getByRole('button', { name: 'Vocabulary List' });
    await user.click(vocabTab);
    expect(screen.getByRole('heading', { name: 'Vocabulary List' })).toBeInTheDocument();

    // English Generator に戻る
    const genTab = screen.getByRole('button', { name: 'English Generator' });
    await user.click(genTab);
    expect(screen.getByRole('heading', { name: 'Generate Text' })).toBeInTheDocument();
  });

  it('logs out and returns to Login screen when Logout is clicked', async () => {
    sessionStorage.setItem('api_key', 'valid-key');
    const user = userEvent.setup();

    render(<App />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Logout' })).toBeInTheDocument();
    });

    const logoutButton = screen.getByRole('button', { name: 'Logout' });
    await user.click(logoutButton);

    expect(sessionStorage.getItem('api_key')).toBeNull();
    expect(screen.getByText('Please enter your API Key to access the application.')).toBeInTheDocument();
  });
});
