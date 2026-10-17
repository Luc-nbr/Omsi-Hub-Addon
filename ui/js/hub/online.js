// The online account's places in the shell (the bar's button, the invites over every page, the
// company's friends). The online port fills them; until then they draw nothing.

export const OnlineButton = () => null;
export const OnlineLayer = () => null;
/** The account as the company's pages ask for it: not signed in. */
export const useOnlineState = () => ({ signedIn: false, friends: [], invites: [] });
export function openOnline() {}
export async function inviteFriend() {
  return null;
}
