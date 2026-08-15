import { useState, useEffect } from 'react';
import { fetchTopics, addTopic, deleteTopic } from '../api';
import type { Topic } from '../api';

export default function TopicManager() {
  const [topics, setTopics] = useState<Topic[]>([]);
  const [newTopic, setNewTopic] = useState('');
  const [loading, setLoading] = useState(true);

  const loadTopics = async () => {
    setLoading(true);
    try {
      const data = await fetchTopics();
      setTopics(data);
    } catch (err) {
      console.error(err);
      alert('Failed to load topics.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTopics();
  }, []);

  const handleAdd = async (e: React.SyntheticEvent) => {
    e.preventDefault();
    if (!newTopic.trim()) return;
    try {
      await addTopic(newTopic);
      setNewTopic('');
      loadTopics();
    } catch (err) {
      console.error(err);
      alert('Failed to add topic.');
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Are you sure?')) return;
    try {
      await deleteTopic(id);
      loadTopics();
    } catch (err) {
      console.error(err);
      alert('Failed to delete topic.');
    }
  };

  return (
    <div className="glass-panel animate-fade-in">
      <h2>Topics Manager</h2>
      <p>Manage your custom topics for article generation.</p>

      <form onSubmit={handleAdd} style={{ display: 'flex', gap: '8px', marginBottom: '24px' }}>
        <input 
          type="text" 
          value={newTopic} 
          onChange={(e) => setNewTopic(e.target.value)}
          placeholder="New topic (e.g., technology)"
          style={{ flex: 1, padding: '10px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.2)', background: 'rgba(0,0,0,0.2)', color: 'white' }}
        />
        <button type="submit" className="btn-primary">Add Topic</button>
      </form>

      {loading ? (
        <p>Loading...</p>
      ) : (
        <ul style={{ listStyle: 'none', padding: 0, display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {topics.map(t => (
            <li key={t.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(255,255,255,0.05)', padding: '12px 16px', borderRadius: '8px' }}>
              <span>{t.name}</span>
              <button onClick={() => handleDelete(t.id)} className="btn-glass" style={{ padding: '6px 12px', borderColor: 'rgba(255,100,100,0.5)', color: '#ff8888' }}>Delete</button>
            </li>
          ))}
          {topics.length === 0 && <p style={{ textAlign: 'center', opacity: 0.5 }}>No topics found.</p>}
        </ul>
      )}
    </div>
  );
}
