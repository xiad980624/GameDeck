import { createRoot } from 'react-dom/client';
import App from './App';
import './styles/index.css';

if (import.meta.env.DEV && import.meta.env.VITE_INPUT_PROBE === 'true') {
  void import('./input-smoke');
} else {
  createRoot(document.getElementById('root')!).render(<App />);
}
