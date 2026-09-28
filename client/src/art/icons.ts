// The activity-indicator icon atlas (admin-panel wave). Hand-authored
// 16×16 pixel art, one grid per intent + the blocked "!" — drawn to a
// Canvas2D and baked into nearest-neighbour textures so they stay crisp at
// any zoom. This file IS the art spec: palette + grids live here, nothing
// else knows what the icons look like.
//
// Visual language: one glyph per intent, readable at ~15 px on screen.
// The harvest droplet bakes pure white and is tinted per resource at draw
// time (RES_COLORS) — one glyph, six resource colors. The user's rule:
// icons are the FAR-ZOOM information layer; the procedural rigs carry the
// detail up close.

import { Texture, ImageSource } from 'pixi.js';

/** Icon keys — one per drawable glyph (intents map onto these). */
export type IconGlyph = 'off' | 'dig' | 'haulDirt' | 'haulHome' | 'fight' | 'medic' | 'feeder' | 'follow' | 'produce' | 'blocked' | 'harvest';

const SIZE = 16;

/** Char → color, per icon ('.' = transparent). */
const PALETTES: Record<IconGlyph, Record<string, string>> = {
  off: { z: '#b8c4d8', Z: '#8a96ac' },
  dig: { s: '#c8d0dc', S: '#8a94a6', w: '#8a5a2a' },
  haulDirt: { u: '#f0e8da', d: '#8a6a4a', D: '#6a4e34' },
  haulHome: { r: '#c07840', w: '#e8dcc8', u: '#f0d878', d: '#5c4033' },
  fight: { s: '#d8e0ec', S: '#98a4b6', h: '#d9a83a' },
  medic: { a: '#e4e8dc', c: '#e8544f' },
  feeder: { g: '#d9b32b', G: '#a8862a', j: '#e8544f' },
  follow: { s: '#d9c27a', S: '#a89468', c: '#f0e8da' },
  produce: { a: '#d9b32b', A: '#a8862a', s: '#f0e0a0' },
  blocked: { r: '#e8544f', R: '#a83430' },
  harvest: { w: '#ffffff' },
};

const GRIDS: Record<IconGlyph, string[]> = {
  // off-duty: stacked Zz, drifting toward the corner
  off: [
    '................',
    '................',
    '...zzzzz........',
    '.......z........',
    '...zzzzz........',
    '...z............',
    '...zzzzz........',
    '................',
    '.........ZZZ....',
    '..........Z.....',
    '.........ZZZ....',
    '................',
    '................',
    '................',
    '................',
    '................',
  ],
  // dig: pickaxe — steel head arc, wooden diagonal handle
  dig: [
    '................',
    '.........ssss...',
    '.......ss....ss.',
    '......s........s',
    '.....s.......s..',
    '....s....ww..s..',
    '...s....ww......',
    '.......ww.......',
    '......ww........',
    '.....ww.........',
    '....ww..........',
    '...ww...........',
    '..ww............',
    '................',
    '................',
    '................',
  ],
  // spoil hauled out: dirt mound + up arrow
  haulDirt: [
    '................',
    '.......u........',
    '......uuu.......',
    '.......u........',
    '......uuu.......',
    '................',
    '................',
    '.....dDDd.......',
    '....dDDDDd......',
    '...dDDDDDDd.....',
    '...dDDDDDDd.....',
    '..dDDDDDDDDd....',
    '..dddddddddd....',
    '................',
    '................',
    '................',
  ],
  // haul home: cabin with a lit doorway and a descending arrow
  haulHome: [
    '................',
    '......rrrr......',
    '.....rrrrrr.....',
    '....rrrrrrrr....',
    '...rrrrrrrrrr...',
    '..rrrrrrrrrrrr..',
    '..rrrrrrrrrrrr..',
    '..wwwwwwwwwwww..',
    '..ww...dd...ww..',
    '..ww..uddu..ww..',
    '..ww...uu...ww..',
    '..ww..wddw..ww..',
    '..wwwwwwwwwwww..',
    '................',
    '................',
    '................',
  ],
  // fight: crossed blades with gold hilts
  fight: [
    '................',
    '..s..........s..',
    '..ss........ss..',
    '...ss......ss...',
    '....ss....ss....',
    '.....ss..ss.....',
    '......ssss......',
    '......ssss......',
    '.....ss..ss.....',
    '....S......S....',
    '...hS......Sh...',
    '..hh........hh..',
    '..hh........hh..',
    '................',
    '................',
    '................',
  ],
  // medic: pale roundel with a red cross
  medic: [
    '................',
    '................',
    '.....aaaaaa.....',
    '...aa......aa...',
    '..a....cc....a..',
    '..a....cc....a..',
    '..a..cccccc..a..',
    '..a..cccccc..a..',
    '..a....cc....a..',
    '..a....cc....a..',
    '...aa......aa...',
    '.....aaaaaa.....',
    '................',
    '................',
    '................',
    '................',
  ],
  // feeder: the queen's crown — gold, three points, red jewel
  feeder: [
    '................',
    '................',
    '..g...g....g....',
    '..g...g....g....',
    '..g..g.g..g.g...',
    '..g.gg.g.g.g....',
    '..ggg.g.gg.g....',
    '..gggggggggg....',
    '..gjgggggggg....',
    '..gggggggggg....',
    '..GGGGGGGGGG....',
    '................',
    '................',
    '................',
    '................',
    '................',
  ],
  // following: reticle — ring, ticks, center dot
  follow: [
    '................',
    '.......ss.......',
    '.......ss.......',
    '....s..ss..s....',
    '....s.s..s.s....',
    '....s.s..s.s....',
    '.......cc.......',
    '......sccs......',
    '......sccs......',
    '.......cc.......',
    '....s.s..s.s....',
    '....s.s..s.s....',
    '....s..ss..s....',
    '.......SS.......',
    '.......SS.......',
    '................',
  ],
  // honey production: amber droplet with rising sparkles
  produce: [
    '................',
    '......s...s.....',
    '................',
    '.......aa.......',
    '.......aa.......',
    '......aaaa......',
    '.....aaaaaa.....',
    '....aaaaaaaa....',
    '...aaaaaaaaaa...',
    '...aAAaaaaAAa...',
    '...aaaaaaaaaa...',
    '....aaaaaaaa....',
    '................',
    '................',
    '................',
    '................',
  ],
  // blocked: bold red exclamation (the reason rides in text)
  blocked: [
    '................',
    '......rrr.......',
    '......rRr.......',
    '......rRr.......',
    '......rRr.......',
    '......rRr.......',
    '......rRr.......',
    '......rRr.......',
    '......rRr.......',
    '................',
    '......rRr.......',
    '......rRr.......',
    '......rrr.......',
    '................',
    '................',
    '................',
  ],
  // harvest: white droplet, tinted per resource at draw time
  harvest: [
    '................',
    '.......ww.......',
    '.......ww.......',
    '......wwww......',
    '......wwww......',
    '.....wwwwww.....',
    '.....wwwwww.....',
    '....wwwwwwww....',
    '....wwwwwwww....',
    '....wwwwwwww....',
    '....wwwwwwww....',
    '.....wwwwww.....',
    '................',
    '................',
    '................',
    '................',
  ],
};

export class IconAtlas {
  private textures = new Map<IconGlyph, Texture>();

  private constructor() {
    for (const key of Object.keys(GRIDS) as IconGlyph[]) {
      const canvas = document.createElement('canvas');
      canvas.width = SIZE;
      canvas.height = SIZE;
      const ctx = canvas.getContext('2d');
      if (ctx === null) throw new Error('icons: no 2d context');
      const grid = GRIDS[key];
      const pal = PALETTES[key];
      if (grid.length !== SIZE) {
        throw new Error(`icons: ${key} has ${grid.length} rows (need ${SIZE})`);
      }
      grid.forEach((row, y) => {
        if (row.length !== SIZE) {
          throw new Error(`icons: ${key} row ${y} has ${row.length} cols (need ${SIZE})`);
        }
        for (let x = 0; x < SIZE; x++) {
          const c = row[x];
          if (c === '.') continue;
          const color = pal[c];
          if (color === undefined) throw new Error(`icons: ${key} has no color for '${c}'`);
          ctx.fillStyle = color;
          ctx.fillRect(x, y, 1, 1);
        }
      });
      const source = new ImageSource({ resource: canvas, scaleMode: 'nearest' });
      this.textures.set(key, new Texture({ source, label: `icon-${key}` }));
    }
  }

  static create(): IconAtlas {
    return new IconAtlas();
  }

  get(key: IconGlyph): Texture {
    const t = this.textures.get(key);
    if (t === undefined) throw new Error(`icons: no texture for ${key}`);
    return t;
  }

  destroy(): void {
    for (const t of this.textures.values()) t.destroy(true);
    this.textures.clear();
  }
}
