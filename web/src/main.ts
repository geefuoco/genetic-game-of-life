import '@fontsource-variable/inter';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

const target = document.getElementById('app');
if (!target) throw new Error('the page has no #app element to mount into');
mount(App, { target });
