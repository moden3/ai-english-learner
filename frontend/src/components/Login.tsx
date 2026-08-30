import { useState } from 'react';
import { validateApiKey } from '../api';

interface LoginProps {
  onLogin: (key: string) => void;
}

export default function Login({ onLogin }: LoginProps) {
  const [key, setKey] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.SyntheticEvent) => {
    e.preventDefault();
    setError('');
    
    const trimmedKey = key.trim();
    if (!trimmedKey) return;
    
    setLoading(true);
    const isValid = await validateApiKey(trimmedKey);
    setLoading(false);
    
    if (isValid) {
      onLogin(trimmedKey);
    } else {
      setError('Invalid API Key. Please try again.');
    }
  };

  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', minHeight: '100vh', padding: '24px' }}>
      <div className="glass-panel animate-fade-in" style={{ width: '100%', maxWidth: '400px', textAlign: 'center' }}>
        <h1 className="gradient-text" style={{ marginBottom: '24px', fontSize: '32px' }}>ENG-APP</h1>
        <p style={{ marginBottom: '32px', opacity: 0.8 }}>Please enter your API Key to access the application.</p>
        
        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <input 
            type="password" 
            value={key}
            onChange={e => setKey(e.target.value)}
            placeholder="API Key"
            disabled={loading}
            style={{ 
              padding: '12px', 
              borderRadius: '8px', 
              border: '1px solid rgba(255,255,255,0.2)', 
              background: 'rgba(0,0,0,0.3)', 
              color: 'white',
              fontSize: '16px'
            }}
          />
          {error && <p style={{ color: '#ff8888', margin: 0, fontSize: '14px', textAlign: 'left' }}>{error}</p>}
          <button type="submit" className="btn-primary" disabled={loading} style={{ padding: '12px', fontSize: '16px', opacity: loading ? 0.7 : 1 }}>
            {loading ? 'Verifying...' : 'Login'}
          </button>
        </form>
      </div>
    </div>
  );
}
