import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import VocabularyManager from './VocabularyManager';
import * as api from '../api';

describe('VocabularyManager Component', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders loading state initially and then displays vocabulary list', async () => {
    const mockVocab = [
      { id: '1', word: 'concomitant', translation: '付随する' },
      { id: '2', word: 'ubiquitous', translation: '至る所にある' },
    ];
    vi.spyOn(api, 'fetchVocabulary').mockResolvedValueOnce(mockVocab);

    render(<VocabularyManager />);

    expect(screen.getByText('Loading...')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('concomitant')).toBeInTheDocument();
      expect(screen.getByText('付随する')).toBeInTheDocument();
      expect(screen.getByText('ubiquitous')).toBeInTheDocument();
      expect(screen.getByText('至る所にある')).toBeInTheDocument();
    });
    expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
  });

  it('displays empty message when no vocabulary items are stored', async () => {
    vi.spyOn(api, 'fetchVocabulary').mockResolvedValueOnce([]);

    render(<VocabularyManager />);

    await waitFor(() => {
      expect(screen.getByText('Your vocabulary list is empty.')).toBeInTheDocument();
    });
  });

  it('saves new vocabulary word and translation', async () => {
    const initialList = [{ id: '1', word: 'word1', translation: '訳1' }];
    const updatedList = [
      { id: '1', word: 'word1', translation: '訳1' },
      { id: '2', word: 'lucrative', translation: '利益の上がる' },
    ];

    vi.spyOn(api, 'fetchVocabulary')
      .mockResolvedValueOnce(initialList)
      .mockResolvedValueOnce(updatedList);

    const addVocabSpy = vi.spyOn(api, 'addVocabulary').mockResolvedValueOnce({ success: true } as any);
    const user = userEvent.setup();

    render(<VocabularyManager />);

    await waitFor(() => {
      expect(screen.getByText('word1')).toBeInTheDocument();
    });

    const wordInput = screen.getByPlaceholderText('English Word/Phrase');
    const transInput = screen.getByPlaceholderText('Japanese Translation');
    const submitButton = screen.getByRole('button', { name: 'Save Word' });

    await user.type(wordInput, 'lucrative');
    await user.type(transInput, '利益の上がる');
    await user.click(submitButton);

    expect(addVocabSpy).toHaveBeenCalledWith('lucrative', '利益の上がる');
    await waitFor(() => {
      expect(screen.getByText('lucrative')).toBeInTheDocument();
      expect(screen.getByText('利益の上がる')).toBeInTheDocument();
    });
    expect(wordInput).toHaveValue('');
    expect(transInput).toHaveValue('');
  });

  it('does not save when word or translation is missing', async () => {
    vi.spyOn(api, 'fetchVocabulary').mockResolvedValueOnce([]);
    const addVocabSpy = vi.spyOn(api, 'addVocabulary');
    const user = userEvent.setup();

    render(<VocabularyManager />);
    await waitFor(() => expect(screen.queryByText('Loading...')).not.toBeInTheDocument());

    const submitButton = screen.getByRole('button', { name: 'Save Word' });
    await user.click(submitButton);
    expect(addVocabSpy).not.toHaveBeenCalled();

    const wordInput = screen.getByPlaceholderText('English Word/Phrase');
    await user.type(wordInput, 'test');
    await user.click(submitButton);
    expect(addVocabSpy).not.toHaveBeenCalled();
  });

  it('deletes vocabulary item when confirmed', async () => {
    const initialList = [{ id: 'item-1', word: 'redundant', translation: '不要な' }];
    vi.spyOn(api, 'fetchVocabulary')
      .mockResolvedValueOnce(initialList)
      .mockResolvedValueOnce([]);

    const deleteSpy = vi.spyOn(api, 'deleteVocabulary').mockResolvedValueOnce({ success: true } as any);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    const user = userEvent.setup();

    render(<VocabularyManager />);

    await waitFor(() => {
      expect(screen.getByText('redundant')).toBeInTheDocument();
    });

    const deleteButton = screen.getByRole('button', { name: 'Delete' });
    await user.click(deleteButton);

    expect(deleteSpy).toHaveBeenCalledWith('item-1');
    await waitFor(() => {
      expect(screen.getByText('Your vocabulary list is empty.')).toBeInTheDocument();
    });
  });
});
