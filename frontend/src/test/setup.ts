import '@testing-library/jest-dom/vitest';
import { beforeEach, vi } from 'vitest';

// モック: window.alert, window.confirm, window.location.reload
beforeEach(() => {
  sessionStorage.clear();
  vi.restoreAllMocks();

  window.alert = vi.fn();
  window.confirm = vi.fn(() => true);
  
  // window.location の reload をスパイ可能にする
  Object.defineProperty(window, 'location', {
    writable: true,
    value: {
      ...window.location,
      reload: vi.fn(),
    },
  });
});
