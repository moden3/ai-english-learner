import { useState } from 'react';

interface LoginProps {
  onLogin: (key: string) => void;
}

export default function Login({ onLogin }: LoginProps) {
  const [key, setKey] = useState('');

  const handleSubmit = (e: React.SyntheticEvent) => {
    e.preventDefault();
    if (key.trim()) {
      onLogin(key.trim());
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
            style={{ 
              padding: '12px', 
              borderRadius: '8px', 
              border: '1px solid rgba(255,255,255,0.2)', 
              background: 'rgba(0,0,0,0.3)', 
              color: 'white',
              fontSize: '16px'
            }}
          />
          <button type="submit" className="btn-primary" style={{ padding: '12px', fontSize: '16px' }}>
            Login
          </button>
        </form>
      </div>
    </div>
  );
}
