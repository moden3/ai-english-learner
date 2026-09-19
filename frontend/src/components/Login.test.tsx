import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Login from './Login';
import * as api from '../api';

describe('Login Component', () => {
  it('renders title, input, and login button', () => {
    render(<Login onLogin={vi.fn()} />);

    expect(screen.getByText('ENG-APP')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('API Key')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Login' })).toBeInTheDocument();
  });

  it('does not submit when key is empty or whitespace only', async () => {
    const onLoginMock = vi.fn();
    const validateSpy = vi.spyOn(api, 'validateApiKey');
    const user = userEvent.setup();

    render(<Login onLogin={onLoginMock} />);
    const button = screen.getByRole('button', { name: 'Login' });

    await user.click(button);
    expect(validateSpy).not.toHaveBeenCalled();
    expect(onLoginMock).not.toHaveBeenCalled();

    const input = screen.getByPlaceholderText('API Key');
    await user.type(input, '   ');
    await user.click(button);
    expect(validateSpy).not.toHaveBeenCalled();
    expect(onLoginMock).not.toHaveBeenCalled();
  });

  it('displays error message when validateApiKey returns false', async () => {
    vi.spyOn(api, 'validateApiKey').mockResolvedValueOnce(false);
    const onLoginMock = vi.fn();
    const user = userEvent.setup();

    render(<Login onLogin={onLoginMock} />);
    const input = screen.getByPlaceholderText('API Key');
    const button = screen.getByRole('button', { name: 'Login' });

    await user.type(input, 'wrong-key');
    await user.click(button);

    await waitFor(() => {
      expect(screen.getByText('Invalid API Key. Please try again.')).toBeInTheDocument();
    });
    expect(onLoginMock).not.toHaveBeenCalled();
  });

  it('calls onLogin with trimmed key when validation succeeds', async () => {
    vi.spyOn(api, 'validateApiKey').mockResolvedValueOnce(true);
    const onLoginMock = vi.fn();
    const user = userEvent.setup();

    render(<Login onLogin={onLoginMock} />);
    const input = screen.getByPlaceholderText('API Key');
    const button = screen.getByRole('button', { name: 'Login' });

    await user.type(input, '  correct-secret-key  ');
    await user.click(button);

    await waitFor(() => {
      expect(onLoginMock).toHaveBeenCalledWith('correct-secret-key');
    });
    expect(screen.queryByText('Invalid API Key. Please try again.')).not.toBeInTheDocument();
  });

  it('shows loading state while verifying key', async () => {
    let resolveValidation: (value: boolean) => void;
    const validationPromise = new Promise<boolean>((resolve) => {
      resolveValidation = resolve;
    });
    vi.spyOn(api, 'validateApiKey').mockReturnValueOnce(validationPromise);

    const user = userEvent.setup();
    render(<Login onLogin={vi.fn()} />);

    const input = screen.getByPlaceholderText('API Key');
    const button = screen.getByRole('button', { name: 'Login' });

    await user.type(input, 'test-key');
    await user.click(button);

    expect(screen.getByRole('button', { name: 'Verifying...' })).toBeDisabled();
    expect(input).toBeDisabled();

    // 検証完了
    resolveValidation!(true);
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: 'Verifying...' })).not.toBeInTheDocument();
    });
  });
});
