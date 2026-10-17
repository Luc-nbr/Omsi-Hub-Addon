// A tour's trips as the drive's steps read them: which trip a start at a time of day takes, a
// trip picked to start with, a time of day as the timetable writes it, and a map's picture.

import { S } from '../store.js';
import { bytesUrl } from '../api.js';
import { choose, realTime } from './drive.js';

/** "HH:MM" of seconds after midnight (a night tour's 25:10 stays 25:10). */
export function hms(seconds) {
  const m = Math.round((seconds || 0) / 60);
  return `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
}

/** The place of the trip a start at `now` (seconds after midnight) takes: the one leaving
 * within two minutes or later, else the last. A tour that runs past midnight counts its early
 * trips of the next day after 24:00, so at 03:25 its trip of 27:30 is the next one. */
export function tripAt(tour, now) {
  const trips = tour?.trips || [];
  if (!trips.length) return -1;
  const late = trips[trips.length - 1].departure;
  const nextDay = now < 86400 && trips[0].departure > now && late >= now + 86400 - 120;
  const at = nextDay ? now + 86400 : now;
  const k = trips.findIndex((x) => x.departure >= at - 120);
  return k >= 0 ? k : trips.length - 1;
}

/** Start the tour with trip `x`: the duty's time becomes its departure (a trip after midnight
 * of a night tour, 24:30, starts at 00:30), unless the duty follows the clock. */
export function pickTrip(x) {
  const c = S.choice;
  const time = realTime() ? c.time : Math.floor(x.departure / 60) % 1440;
  choose({ time, start_trip: [c.line, c.tour, x.index, time] });
}

/** The picture of a map (its folder's `picture.jpg`), through the app's bytes route. */
export const mapPicture = (file) => (file ? bytesUrl('hub_map_picture', { map: file }) : '');
