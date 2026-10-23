// The company's pages of phase 3 (Trello "Tauri 3/6") by their tab (`company::TABS`): the
// planning, the depot, the concessions, the fleet map, the research, the workshop and the
// contracts. The company's frame (`company.js`) draws the one open through `MorePage`.

import { S } from '../store.js';
import { html, tr, setCo, Icon, Btn } from './kit.js';
import { MORE_TABS } from './more.js';
import { PlanningPage } from './planning.js';
import { DepotPage } from './depot.js';
import { ConcessionsPage } from './concessions.js';
import { FleetMapPage } from './fleetmap.js';
import { ResearchPage } from './research.js';
import { WorkshopPage } from './workshop.js';
import { ContractsPage } from './contracts.js';

export { MORE_TABS };

/** Each page's view (`more.js`'s `useMore`). */
const VIEWS = { 5: 'planning', 7: 'depot', 8: 'concessions', 9: 'fleetmap', 10: 'research', 11: 'workshop', 12: 'contracts' };

const PAGES = { 5: PlanningPage, 7: DepotPage, 8: ConcessionsPage, 9: FleetMapPage, 10: ResearchPage, 11: WorkshopPage, 12: ContractsPage };

/** The page of tab `tab`; over it, what went wrong when its view could not be read (a friend's
 * look at the boss's company that lost him, a company file that failed), with a way to ask again. */
export function MorePage({ tab }) {
  const Page = PAGES[tab];
  const err = S.co.moreError?.view === VIEWS[tab] ? S.co.moreError : null;
  if (!Page) return null;
  const again = () => setCo((c) => ({ moreError: null, bump: c.bump + 1 }));
  return html`${err && html`<div class="co-more-failed" role="alert"><${Icon} name="error" size=${18} /><span class="grow">${err.text}</span><${Btn} small icon="refresh" label=${tr('Try again')} onClick=${again} /></div>`}<${Page} />`;
}
