import { useState, useEffect } from 'react';
import { fetchTopics, generateText, analyzeText, addVocabulary } from '../api';
import type { Topic, GenerateResult, AnalyzeResult } from '../api';

export default function TextGenerator() {
  const [topics, setTopics] = useState<Topic[]>([]);
  const [selectedTopic, setSelectedTopic] = useState<string>('');
  const [useLiteModel, setUseLiteModel] = useState<boolean>(true);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [result, setResult] = useState<GenerateResult | null>(null);
  const [analysisResult, setAnalysisResult] = useState<AnalyzeResult | null>(null);
  const [activeTab, setActiveTab] = useState<'original' | 'analysis'>('original');
  const [activeSegmentId, setActiveSegmentId] = useState<number | null>(null);

  useEffect(() => {
    fetchTopics().then(data => {
      setTopics(data);
      if (data.length > 0) setSelectedTopic(data[0].name);
    }).catch(err => console.error('Failed to load topics', err));
  }, []);

  const handleGenerate = async () => {
    let topicToUse = selectedTopic;
    if (!topicToUse) {
      if (topics.length > 0) {
        const randomIndex = Math.floor(Math.random() * topics.length);
        topicToUse = topics[randomIndex].name;
        setSelectedTopic(topicToUse);
      } else {
        alert('Please create a topic first.');
        return;
      }
    }
    
    setIsGenerating(true);
    setResult(null);
    setAnalysisResult(null);
    setActiveTab('original');
    try {
      const res = await generateText(topicToUse, useLiteModel);
      setResult(res);
    } catch (err) {
      console.error(err);
      alert('Failed to generate text. Check console for details.');
    } finally {
      setIsGenerating(false);
    }
  };

  const handleAnalyze = async () => {
    if (!result?.text) return;
    setIsAnalyzing(true);
    try {
      const res = await analyzeText(result.text);
      setAnalysisResult(res);
      setActiveTab('analysis');
    } catch (err) {
      console.error(err);
      alert('Failed to analyze text.');
    } finally {
      setIsAnalyzing(false);
    }
  };

  const handleSaveVocab = async (word: string, translation: string) => {
    try {
      await addVocabulary(word, translation);
      alert(`Saved "${word}" to Vocabulary!`);
    } catch (err) {
      console.error(err);
      alert('Failed to save vocabulary.');
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
          <option value="">Random (Auto-select)</option>
          {topics.map(t => (
            <option key={t.id} value={t.name}>{t.name}</option>
          ))}
        </select>
        <button 
          onClick={handleGenerate} 
          disabled={isGenerating}
          className="btn-primary"
          style={{ minWidth: '150px', padding: '12px 24px', opacity: isGenerating ? 0.5 : 1 }}
        >
          {isGenerating ? 'Generating...' : '✨ Generate Now'}
        </button>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '14px', color: 'rgba(255,255,255,0.8)' }}>
        <input 
          type="checkbox" 
          id="useLiteModel" 
          checked={useLiteModel} 
          onChange={e => setUseLiteModel(e.target.checked)}
          style={{ width: '16px', height: '16px', cursor: 'pointer' }}
        />
        <label htmlFor="useLiteModel" style={{ cursor: 'pointer' }}>
          Use Fast/Lite AI (No Search, 500 requests/day)
        </label>
      </div>

      {isGenerating && (
        <div style={{ padding: '40px', textAlign: 'center' }}>
          <div style={{ margin: '0 auto 16px auto', width: '40px', height: '40px', border: '4px solid rgba(255,255,255,0.1)', borderTop: '4px solid var(--accent-blue)', borderRadius: '50%', animation: 'spin 1s linear infinite' }}></div>
          <p className="gradient-text animate-pulse">AI is reading the latest news and writing your article...</p>
        </div>
      )}

      {result && (
        <div className="animate-fade-in" style={{ background: 'rgba(0,0,0,0.3)', borderRadius: '12px', border: '1px solid rgba(255,255,255,0.1)', overflow: 'hidden' }}>
          
          <div style={{ display: 'flex', borderBottom: '1px solid rgba(255,255,255,0.1)', background: 'rgba(0,0,0,0.2)' }}>
            <button 
              onClick={() => setActiveTab('original')}
              style={{ flex: 1, padding: '16px', background: activeTab === 'original' ? 'rgba(255,255,255,0.1)' : 'transparent', border: 'none', color: 'white', cursor: 'pointer', fontWeight: activeTab === 'original' ? 'bold' : 'normal' }}
            >
              Original Text
            </button>
            <button 
              onClick={() => {
                if (analysisResult) setActiveTab('analysis');
                else handleAnalyze();
              }}
              disabled={isAnalyzing}
              style={{ flex: 1, padding: '16px', background: activeTab === 'analysis' ? 'rgba(255,255,255,0.1)' : 'transparent', border: 'none', color: 'white', cursor: 'pointer', fontWeight: activeTab === 'analysis' ? 'bold' : 'normal' }}
            >
              {isAnalyzing ? 'Analyzing...' : 'Slash Reading & Analysis'}
            </button>
          </div>

          <div style={{ padding: '24px' }}>
            <h3 style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', paddingBottom: '12px', marginBottom: '16px', textTransform: 'capitalize' }}>
              Topic: {selectedTopic}
            </h3>

            {activeTab === 'original' && (
              <div style={{ fontSize: '16px', lineHeight: '1.8', whiteSpace: 'pre-wrap' }}>
                {result.text}
              </div>
            )}

            {activeTab === 'analysis' && analysisResult && (
              <div>
                <div style={{ marginBottom: '32px' }}>
                  <h4 style={{ marginBottom: '16px', color: 'var(--accent-blue)' }}>Slash Reading</h4>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                    {analysisResult.segments.map(seg => (
                      <div 
                        key={seg.id} 
                        onClick={() => setActiveSegmentId(activeSegmentId === seg.id ? null : seg.id)}
                        style={{ padding: '12px', background: activeSegmentId === seg.id ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.2)', borderRadius: '8px', cursor: 'pointer', transition: 'background 0.2s' }}
                      >
                        <div style={{ fontSize: '16px', fontWeight: '500' }}>{seg.text}</div>
                        {activeSegmentId === seg.id && (
                          <div className="animate-fade-in" style={{ marginTop: '12px', paddingTop: '12px', borderTop: '1px dashed rgba(255,255,255,0.1)', fontSize: '14px', color: '#ccc' }}>
                            <div style={{ marginBottom: '4px' }}><strong>和訳:</strong> {seg.translation}</div>
                            <div><strong>解説:</strong> {seg.grammar_note}</div>
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>

                <div>
                  <h4 style={{ marginBottom: '16px', color: 'var(--accent-green)' }}>Keywords</h4>
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))', gap: '16px' }}>
                    {analysisResult.keywords.map((kw, i) => (
                      <div key={i} style={{ background: 'rgba(0,0,0,0.2)', padding: '16px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.05)', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
                        <div>
                          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                            <span style={{ fontSize: '18px', fontWeight: 'bold' }}>{kw.word}</span>
                            <span style={{ fontSize: '12px', padding: '2px 8px', background: 'rgba(255,255,255,0.1)', borderRadius: '12px' }}>{kw.part_of_speech}</span>
                          </div>
                          <div style={{ fontSize: '14px', color: '#ddd', marginBottom: '8px' }}>{kw.meaning}</div>
                          <div style={{ fontSize: '12px', color: '#aaa', fontStyle: 'italic', marginBottom: '16px' }}>"{kw.example}"</div>
                        </div>
                        <button 
                          onClick={() => handleSaveVocab(kw.word, kw.meaning)}
                          style={{ padding: '8px', background: 'rgba(255,255,255,0.1)', border: 'none', borderRadius: '4px', color: 'white', cursor: 'pointer', fontSize: '12px', transition: 'background 0.2s' }}
                          onMouseOver={e => e.currentTarget.style.background = 'rgba(255,255,255,0.2)'}
                          onMouseOut={e => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
                        >
                          + Save to Vocabulary
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            )}
            
            {result.source_url && (
              <div style={{ marginTop: '24px', paddingTop: '16px', borderTop: '1px dashed rgba(255,255,255,0.2)' }}>
                <span style={{ opacity: 0.7, fontSize: '14px', marginRight: '8px' }}>Reference Source:</span>
                <a href={result.source_url} target="_blank" rel="noopener noreferrer" style={{ color: 'var(--accent-blue)', textDecoration: 'underline', wordBreak: 'break-all' }}>
                  {result.source_url}
                </a>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
