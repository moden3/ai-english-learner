import { useState, useEffect } from 'react'
import './index.css'
import TopicManager from './components/TopicManager'
import VocabularyManager from './components/VocabularyManager'
import TextGenerator from './components/TextGenerator'
import Login from './components/Login'

function App() {
  const [activeTab, setActiveTab] = useState('generate')
  const [isAuthenticated, setIsAuthenticated] = useState(false)

  // 起動時にsessionStorageからAPIキーを確認
  useEffect(() => {
    const key = sessionStorage.getItem('api_key')
    if (key) {
      setIsAuthenticated(true)
    }
  }, [])

  const handleLogin = (key: string) => {
    sessionStorage.setItem('api_key', key)
    setIsAuthenticated(true)
  }

  const handleLogout = () => {
    sessionStorage.removeItem('api_key')
    setIsAuthenticated(false)
  }

  if (!isAuthenticated) {
    return <Login onLogin={handleLogin} />
  }

  return (
    <div className="app-container">
      {/* Sidebar Navigation */}
      <aside className="sidebar glass-panel" style={{ margin: '24px', borderRadius: '24px' }}>
        <h1 className="gradient-text" style={{ fontSize: '24px', letterSpacing: '-0.5px' }}>
          ENG-APP
        </h1>
        
        <nav style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
          <button 
            className={`btn-glass ${activeTab === 'generate' ? 'active' : ''}`}
            onClick={() => setActiveTab('generate')}
            style={activeTab === 'generate' ? { background: 'rgba(255,255,255,0.1)', borderColor: 'rgba(255,255,255,0.3)' } : {}}
          >
            English Generator
          </button>
          <button 
            className={`btn-glass ${activeTab === 'topics' ? 'active' : ''}`}
            onClick={() => setActiveTab('topics')}
            style={activeTab === 'topics' ? { background: 'rgba(255,255,255,0.1)', borderColor: 'rgba(255,255,255,0.3)' } : {}}
          >
            Topic Manager
          </button>
          <button 
            className={`btn-glass ${activeTab === 'vocabulary' ? 'active' : ''}`}
            onClick={() => setActiveTab('vocabulary')}
            style={activeTab === 'vocabulary' ? { background: 'rgba(255,255,255,0.1)', borderColor: 'rgba(255,255,255,0.3)' } : {}}
          >
            Vocabulary List
          </button>

          {/* Logout Button */}
          <button 
            className="btn-glass"
            onClick={handleLogout}
            style={{ marginTop: 'auto', borderColor: 'rgba(255,100,100,0.3)', color: '#ff8888' }}
          >
            Logout
          </button>
        </nav>

        <div className="sidebar-footer" style={{ marginTop: '24px' }}>
          <p style={{ fontSize: '12px', textAlign: 'center', margin: 0 }}>
            Powered by AWS & Gemini
          </p>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="main-content">
        {activeTab === 'generate' && <TextGenerator />}
        {activeTab === 'topics' && <TopicManager />}
        {activeTab === 'vocabulary' && <VocabularyManager />}
      </main>
    </div>
  )
}

export default App
