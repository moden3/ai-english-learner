import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import TextGenerator from './TextGenerator';
import * as api from '../api';

describe('TextGenerator Component', () => {
  const mockTopics = [
    { id: '1', name: 'Macroeconomics' },
    { id: '2', name: 'Quantum Computing' },
  ];

  const mockGeneratedResult = {
    text: 'Quantum computing is rapidly advancing in modern physics.',
    source_url: 'https://news.example.com/quantum',
  };

  const mockAnalysisResult = {
    segments: [
      {
        id: 1,
        text: 'Quantum computing is rapidly advancing',
        translation: '量子コンピューティングは急速に進歩している',
        grammar_note: '現在進行形 (is advancing)',
      },
      {
        id: 2,
        text: 'in modern physics.',
        translation: '現代物理学において。',
        grammar_note: '前置詞句',
      },
    ],
    keywords: [
      {
        word: 'advancing',
        meaning: '進歩している',
        part_of_speech: 'verb',
        example: 'Technology is advancing rapidly.',
      },
    ],
  };

  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(api, 'fetchTopics').mockResolvedValue(mockTopics);
  });

  it('renders topics in select dropdown and web search checkbox unchecked by default', async () => {
    render(<TextGenerator />);

    await waitFor(() => {
      expect(screen.getByRole('combobox')).toHaveValue('Macroeconomics');
    });

    const checkbox = screen.getByRole('checkbox', { name: /最新ニュースを参照して生成する/i });
    expect(checkbox).not.toBeChecked();
  });

  it('generates text without web search when checkbox is unchecked', async () => {
    const generateSpy = vi.spyOn(api, 'generateText').mockResolvedValueOnce(mockGeneratedResult);
    const user = userEvent.setup();

    render(<TextGenerator />);
    await waitFor(() => expect(screen.getByRole('combobox')).toHaveValue('Macroeconomics'));

    const generateButton = screen.getByRole('button', { name: /Generate Now/i });
    await user.click(generateButton);

    expect(generateSpy).toHaveBeenCalledWith('Macroeconomics', false);

    await waitFor(() => {
      expect(screen.getByText('Quantum computing is rapidly advancing in modern physics.')).toBeInTheDocument();
      expect(screen.getByText('https://news.example.com/quantum')).toBeInTheDocument();
    });
  });

  it('generates text with web search enabled when checkbox is checked', async () => {
    const generateSpy = vi.spyOn(api, 'generateText').mockResolvedValueOnce(mockGeneratedResult);
    const user = userEvent.setup();

    render(<TextGenerator />);
    await waitFor(() => expect(screen.getByRole('combobox')).toHaveValue('Macroeconomics'));

    const checkbox = screen.getByRole('checkbox', { name: /最新ニュースを参照して生成する/i });
    await user.click(checkbox);
    expect(checkbox).toBeChecked();

    const generateButton = screen.getByRole('button', { name: /Generate Now/i });
    await user.click(generateButton);

    expect(generateSpy).toHaveBeenCalledWith('Macroeconomics', true);

    await waitFor(() => {
      expect(screen.getByText(mockGeneratedResult.text)).toBeInTheDocument();
    });
  });

  it('handles tab switching and triggers analysis on first click', async () => {
    vi.spyOn(api, 'generateText').mockResolvedValueOnce(mockGeneratedResult);
    const analyzeSpy = vi.spyOn(api, 'analyzeText').mockResolvedValueOnce(mockAnalysisResult);
    const user = userEvent.setup();

    render(<TextGenerator />);
    await waitFor(() => expect(screen.getByRole('combobox')).toHaveValue('Macroeconomics'));

    // 英文生成
    await user.click(screen.getByRole('button', { name: /Generate Now/i }));
    await waitFor(() => expect(screen.getByText(mockGeneratedResult.text)).toBeInTheDocument());

    // 解析タブへ切り替え
    const analysisTab = screen.getByRole('button', { name: /Slash Reading & Analysis/i });
    await user.click(analysisTab);

    expect(analyzeSpy).toHaveBeenCalledWith(mockGeneratedResult.text);

    // 解析結果のセグメントとキーワードが表示される
    await waitFor(() => {
      expect(screen.getByText('Quantum computing is rapidly advancing')).toBeInTheDocument();
      expect(screen.getByText('in modern physics.')).toBeInTheDocument();
      expect(screen.getByText('advancing')).toBeInTheDocument();
      expect(screen.getByText('進歩している')).toBeInTheDocument();
    });

    // セグメントクリックでアコーディオン展開（和訳・文法解説）
    const segmentElement = screen.getByText('Quantum computing is rapidly advancing');
    await user.click(segmentElement);

    await waitFor(() => {
      expect(screen.getByText('量子コンピューティングは急速に進歩している')).toBeInTheDocument();
      expect(screen.getByText('現在進行形 (is advancing)')).toBeInTheDocument();
    });

    // キーワードを単語帳に追加
    const addVocabSpy = vi.spyOn(api, 'addVocabulary').mockResolvedValueOnce({ success: true } as any);
    const saveVocabButton = screen.getByRole('button', { name: '+ Save to Vocabulary' });
    await user.click(saveVocabButton);

    expect(addVocabSpy).toHaveBeenCalledWith('advancing', '進歩している');
    expect(window.alert).toHaveBeenCalledWith('Saved "advancing" to Vocabulary!');
  });

  it('alerts user if generate is clicked when no topics exist', async () => {
    vi.spyOn(api, 'fetchTopics').mockResolvedValueOnce([]);
    const alertSpy = vi.spyOn(window, 'alert');
    const user = userEvent.setup();

    render(<TextGenerator />);
    await waitFor(() => expect(screen.queryByText('Loading...')).not.toBeInTheDocument());

    const generateButton = screen.getByRole('button', { name: /Generate Now/i });
    await user.click(generateButton);

    expect(alertSpy).toHaveBeenCalledWith('Please create a topic first.');
  });
});
