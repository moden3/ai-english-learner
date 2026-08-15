import { useState, useEffect } from 'react';
import { fetchTopics, generateText } from '../api';
import type { Topic, GenerateResult } from '../api';

export default function TextGenerator() {
  const [topics, setTopics] = useState<Topic[]>([]);
  const [selectedTopic, setSelectedTopic] = useState<string>('');
  const [isGenerating, setIsGenerating] = useState(false);
  const [result, setResult] = useState<GenerateResult | null>(null);

  useEffect(() => {
    fetchTopics().then(data => {
      setTopics(data);
      if (data.length > 0) setSelectedTopic(data[0].name);
    }).catch(err => console.error('Failed to load topics', err));
  }, []);

  const handleGenerate = async () => {
    if (!selectedTopic) {
      alert('Please select or create a topic first.');
      return;
    }
    setIsGenerating(true);
    setResult(null);
    try {
      const res = await generateText(selectedTopic);
      setResult(res);
    } catch (err) {
      console.error(err);
      alert('Failed to generate text. Check console for details.');
    } finally {
      setIsGenerating(false);
    }
  };

  return (
    <div className="glass-panel animate-fade-in" style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <div>
        <h2>Generate Text</h2>
        <p>Select a topic and let the AI generate an english article using real-time news.</p>
      </div>

      <div style={{ display: 'flex', gap: '16px', alignItems: 'center', flexWrap: 'wrap' }}>
        <select 
          value={selectedTopic}
          onChange={e => setSelectedTopic(e.target.value)}
          style={{ padding: '12px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.2)', background: 'rgba(0,0,0,0.5)', color: 'white', flex: 1, minWidth: '200px' }}
        >
          <option value="" disabled>Select a topic...</option>
          {topics.map(t => (
            <option key={t.id} value={t.name}>{t.name}</option>
          ))}
        </select>
        <button 
          onClick={handleGenerate} 
          disabled={isGenerating || !selectedTopic}
          className="btn-primary"
          style={{ minWidth: '150px', padding: '12px 24px', opacity: (isGenerating || !selectedTopic) ? 0.5 : 1 }}
        >
          {isGenerating ? 'Generating...' : '✨ Generate Now'}
        </button>
      </div>

      {isGenerating && (
        <div style={{ padding: '40px', textAlign: 'center' }}>
          <div style={{ margin: '0 auto 16px auto', width: '40px', height: '40px', border: '4px solid rgba(255,255,255,0.1)', borderTop: '4px solid var(--accent-blue)', borderRadius: '50%', animation: 'spin 1s linear infinite' }}></div>
          <p className="gradient-text animate-pulse">AI is reading the latest news and writing your article...</p>
        </div>
      )}

      {result && (
        <div className="animate-fade-in" style={{ background: 'rgba(0,0,0,0.3)', padding: '24px', borderRadius: '12px', border: '1px solid rgba(255,255,255,0.1)' }}>
          <h3 style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', paddingBottom: '12px', marginBottom: '16px', textTransform: 'capitalize' }}>
            Topic: {selectedTopic}
          </h3>
          <div style={{ fontSize: '16px', lineHeight: '1.8', whiteSpace: 'pre-wrap' }}>
            {result.text}
          </div>
          
          {result.source_url && (
            <div style={{ marginTop: '24px', paddingTop: '16px', borderTop: '1px dashed rgba(255,255,255,0.2)' }}>
              <span style={{ opacity: 0.7, fontSize: '14px', marginRight: '8px' }}>Reference Source:</span>
              <a href={result.source_url} target="_blank" rel="noopener noreferrer" style={{ color: 'var(--accent-blue)', textDecoration: 'underline', wordBreak: 'break-all' }}>
                {result.source_url}
              </a>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
