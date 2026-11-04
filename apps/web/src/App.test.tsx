import React from 'react';
import {it,expect,afterEach} from 'vitest';
import {render,screen,fireEvent,cleanup} from '@testing-library/react';
import {App} from './main';
afterEach(cleanup);
it('runs the fictitious revision flow end to end',()=>{
 render(<App/>);
 expect(screen.getByRole('heading',{level:1}).textContent).toContain('Tableau de bord');
 fireEvent.click(screen.getByRole('button',{name:'Commencer une révision ↗'}));
 const answer=screen.queryByLabelText('Ta réponse');
 if(answer)fireEvent.change(answer,{target:{value:'Une raison constante'}});
 else fireEvent.click(screen.getAllByRole('checkbox')[0]);
 fireEvent.click(screen.getByRole('button',{name:'Valider ma réponse'}));
 expect(screen.getByRole('heading',{name:'La correction'})).toBeTruthy();
 fireEvent.click(screen.getByRole('button',{name:'Compris'}));
 expect(JSON.parse(localStorage.getItem('study-demo-reviews')||'{}')).not.toEqual({});
});
it('shows honest disconnected state in the sync center',()=>{
 render(<App/>);fireEvent.click(screen.getByRole('button',{name:'Synchronisation'}));
 expect(screen.getByText(/Sans serveur configuré/)).toBeTruthy();
 expect((screen.getByRole('button',{name:'Synchroniser / relancer'}) as HTMLButtonElement).disabled).toBe(true);
});
