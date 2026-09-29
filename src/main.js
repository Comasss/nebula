import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

// Menú contextual del navegador desactivado salvo en campos de texto
document.addEventListener('contextmenu', (e) => {
  if (!e.target.closest('input, textarea, .selectable')) e.preventDefault();
});

export default mount(App, { target: document.getElementById('app') });
