/** Pre-game menu: title screen (PLAY) → team choice (red/blue). Resolves with
 * the chosen team (0 red, 1 blue) once the player has clicked through. */
export function chooseTeam(): Promise<number> {
  return new Promise((resolve) => {
    const menu = document.getElementById('menu') as HTMLElement;
    const title = document.getElementById('menu-title') as HTMLElement;
    const team = document.getElementById('menu-team') as HTMLElement;
    const play = document.getElementById('btn-play') as HTMLButtonElement;
    menu.classList.remove('hidden');
    title.classList.remove('hidden');
    team.classList.add('hidden');
    const onStart = (t: number) => {
      menu.classList.add('hidden');
      resolve(t);
    };
    play.onclick = () => {
      title.classList.add('hidden');
      team.classList.remove('hidden');
    };
    (document.getElementById('btn-team-red') as HTMLButtonElement).onclick = () => onStart(0);
    (document.getElementById('btn-team-blue') as HTMLButtonElement).onclick = () => onStart(1);
  });
}
