// The buses' photos (the old app took them with its 3D renderer, which comes later): none yet,
// so the tiles show their quiet box.

/** A bus's photo: none yet (the promise answers null). */
export const photo = async () => null;
/** Ask for a photo to be taken: nothing to take it yet (answers the "leave it" function). */
export const wantPhoto = () => () => {};
