import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import TopicManager from './TopicManager';
import * as api from '../api';

describe('TopicManager Component', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders loading state initially and then shows topic list', async () => {
    const mockTopics = [
      { id: '1', name: 'Artificial Intelligence' },
      { id: '2', name: 'Global Economy' },
    ];
    vi.spyOn(api, 'fetchTopics').mockResolvedValueOnce(mockTopics);

    render(<TopicManager />);

    expect(screen.getByText('Loading...')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('Artificial Intelligence')).toBeInTheDocument();
      expect(screen.getByText('Global Economy')).toBeInTheDocument();
    });
    expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
  });

  it('shows empty message when no topics exist', async () => {
    vi.spyOn(api, 'fetchTopics').mockResolvedValueOnce([]);

    render(<TopicManager />);

    await waitFor(() => {
      expect(screen.getByText('No topics found.')).toBeInTheDocument();
    });
  });

  it('adds a new topic when submitted and reloads list', async () => {
    const initialTopics = [{ id: '1', name: 'Topic 1' }];
    const afterAddTopics = [{ id: '1', name: 'Topic 1' }, { id: '2', name: 'New Science Topic' }];

    vi.spyOn(api, 'fetchTopics')
      .mockResolvedValueOnce(initialTopics)
      .mockResolvedValueOnce(afterAddTopics);

    const addTopicSpy = vi.spyOn(api, 'addTopic').mockResolvedValueOnce({ success: true } as any);
    const user = userEvent.setup();

    render(<TopicManager />);

    await waitFor(() => {
      expect(screen.getByText('Topic 1')).toBeInTheDocument();
    });

    const input = screen.getByPlaceholderText(/New topic/i);
    const addButton = screen.getByRole('button', { name: 'Add Topic' });

    await user.type(input, 'New Science Topic');
    await user.click(addButton);

    expect(addTopicSpy).toHaveBeenCalledWith('New Science Topic');
    await waitFor(() => {
      expect(screen.getByText('New Science Topic')).toBeInTheDocument();
    });
    expect(input).toHaveValue('');
  });

  it('does not add topic when input is empty or whitespace', async () => {
    vi.spyOn(api, 'fetchTopics').mockResolvedValueOnce([]);
    const addTopicSpy = vi.spyOn(api, 'addTopic');
    const user = userEvent.setup();

    render(<TopicManager />);
    await waitFor(() => expect(screen.queryByText('Loading...')).not.toBeInTheDocument());

    const addButton = screen.getByRole('button', { name: 'Add Topic' });
    await user.click(addButton);

    expect(addTopicSpy).not.toHaveBeenCalled();
  });

  it('deletes a topic when Delete is clicked and user confirms', async () => {
    const initialTopics = [{ id: 'del-1', name: 'Topic to delete' }];
    vi.spyOn(api, 'fetchTopics')
      .mockResolvedValueOnce(initialTopics)
      .mockResolvedValueOnce([]);

    const deleteSpy = vi.spyOn(api, 'deleteTopic').mockResolvedValueOnce({ success: true } as any);
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const user = userEvent.setup();

    render(<TopicManager />);

    await waitFor(() => {
      expect(screen.getByText('Topic to delete')).toBeInTheDocument();
    });

    const deleteButton = screen.getByRole('button', { name: 'Delete' });
    await user.click(deleteButton);

    expect(confirmSpy).toHaveBeenCalledWith('Are you sure?');
    expect(deleteSpy).toHaveBeenCalledWith('del-1');

    await waitFor(() => {
      expect(screen.getByText('No topics found.')).toBeInTheDocument();
    });
  });

  it('cancels deletion when user clicks Cancel on confirm dialog', async () => {
    const initialTopics = [{ id: 'keep-1', name: 'Keep this topic' }];
    vi.spyOn(api, 'fetchTopics').mockResolvedValueOnce(initialTopics);
    const deleteSpy = vi.spyOn(api, 'deleteTopic');
    vi.spyOn(window, 'confirm').mockReturnValue(false);
    const user = userEvent.setup();

    render(<TopicManager />);

    await waitFor(() => {
      expect(screen.getByText('Keep this topic')).toBeInTheDocument();
    });

    const deleteButton = screen.getByRole('button', { name: 'Delete' });
    await user.click(deleteButton);

    expect(deleteSpy).not.toHaveBeenCalled();
    expect(screen.getByText('Keep this topic')).toBeInTheDocument();
  });
});
