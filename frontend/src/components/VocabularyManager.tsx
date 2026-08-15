import { useState, useEffect } from 'react';
import { fetchVocabulary, addVocabulary, deleteVocabulary } from '../api';
import type { Vocabulary } from '../api';

export default function VocabularyManager() {
  const [vocab, setVocab] = useState<Vocabulary[]>([]);
  const [word, setWord] = useState('');
  const [translation, setTranslation] = useState('');
  const [loading, setLoading] = useState(true);

  const loadVocab = async () => {
    setLoading(true);
    try {
      const data = await fetchVocabulary();
      setVocab(data);
    } catch (err) {
      console.error(err);
      alert('Failed to load vocabulary.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadVocab();
  }, []);

  const handleAdd = async (e: React.SyntheticEvent) => {
    e.preventDefault();
    if (!word.trim() || !translation.trim()) return;
    try {
      await addVocabulary(word, translation);
      setWord('');
      setTranslation('');
      loadVocab();
    } catch (err) {
      console.error(err);
      alert('Failed to add vocabulary.');
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Are you sure?')) return;
    try {
      await deleteVocabulary(id);
      loadVocab();
    } catch (err) {
      console.error(err);
      alert('Failed to delete vocabulary.');
    }
  };

  return (
    <div className="glass-panel animate-fade-in">
      <h2>Vocabulary List</h2>
      <p>Review the words and phrases you have saved.</p>

      <form onSubmit={handleAdd} style={{ display: 'flex', gap: '8px', marginBottom: '24px', flexWrap: 'wrap' }}>
        <input 
          type="text" 
          value={word} 
          onChange={(e) => setWord(e.target.value)}
          placeholder="English Word/Phrase"
          style={{ flex: 1, minWidth: '150px', padding: '10px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.2)', background: 'rgba(0,0,0,0.2)', color: 'white' }}
        />
        <input 
          type="text" 
          value={translation} 
          onChange={(e) => setTranslation(e.target.value)}
          placeholder="Japanese Translation"
          style={{ flex: 1, minWidth: '150px', padding: '10px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.2)', background: 'rgba(0,0,0,0.2)', color: 'white' }}
        />
        <button type="submit" className="btn-primary">Save Word</button>
      </form>

      {loading ? (
        <p>Loading...</p>
      ) : (
        <ul style={{ listStyle: 'none', padding: 0, display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {vocab.map(v => (
            <li key={v.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(255,255,255,0.05)', padding: '12px 16px', borderRadius: '8px' }}>
              <div>
                <strong style={{ fontSize: '18px' }}>{v.word}</strong> <span style={{ opacity: 0.5, margin: '0 12px' }}>|</span> <span style={{ color: 'var(--text-secondary)' }}>{v.translation}</span>
              </div>
              <button onClick={() => handleDelete(v.id)} className="btn-glass" style={{ padding: '6px 12px', borderColor: 'rgba(255,100,100,0.5)', color: '#ff8888' }}>Delete</button>
            </li>
          ))}
          {vocab.length === 0 && <p style={{ textAlign: 'center', opacity: 0.5 }}>Your vocabulary list is empty.</p>}
        </ul>
      )}
    </div>
  );
}
