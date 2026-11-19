import { createRoot } from 'react-dom/client';
import { Component } from './workspace';
import { ScrollPane } from './scroll-pane';
import './style.css';

createRoot(document.getElementById('root')!).render(
  <ScrollPane className="debug-page-scroll" viewport={{ 'aria-label': 'DevBox 工作台' }}>
    <Component />
  </ScrollPane>,
);
