import './style.css';
import { initCore } from './wasm';
import { Game } from './game';

async function main(): Promise<void> {
  await initCore();
  const host = document.getElementById('app');
  if (!host) throw new Error('#app element missing');
  const game = await Game.create(host, Date.now() % 0x7fffffff);
  game.start();
}

void main();
