// The cloud saves' places in the company's chooser. The online port fills them; until then a
// company opens as it is, from this computer.

export const CloudBadge = () => null;
/** Before a company opens: nothing to fetch from the cloud. */
export async function cloudBeforeOpen() {
  return true;
}
export const useCloud = () => null;
