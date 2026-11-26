// Every overlay kind. To add an overlay: create a file exporting an
// OverlayDef and list it here — the control panel picks it up automatically.

import type { OverlayDef } from '../lib/overlay';
import { battle } from './battle';
import { databox } from './databox';
import { delta } from './delta';
import { flags } from './flags';
import { fuel } from './fuel';
import { garage61 } from './garage61';
import { incidents } from './incidents';
import { inputGraph } from './inputGraph';
import { inputs } from './inputs';
import { monitor } from './monitor';
import { pit } from './pit';
import { relative } from './relative';
import { safety } from './safety';
import { session } from './session';
import { spotter } from './spotter';
import { standings } from './standings';
import { startLights } from './startLights';
import { forecast } from './forecast';
import { trackmap } from './trackmap';
import { tyres } from './tyres';
import { weather } from './weather';
import { laptimes } from './laptimes';
import { energy } from './energy';
import { radar } from './radar';
import { carsettings } from './carsettings';
import { tyredata } from './tyredata';
import { shiftlights } from './shiftlightsOverlay';
import { pitRejoin } from './pitRejoin';
import { cornerTimes } from './cornerTimes';
import { pitStrategy } from './pitStrategy';

const ALL: OverlayDef[] = [relative, standings, trackmap, inputs, shiftlights, inputGraph, laptimes, cornerTimes, fuel, pitRejoin, pitStrategy, energy, battle, session, flags, startLights, weather, forecast, tyres, tyredata, carsettings, databox, spotter, radar, safety, delta, pit, incidents, garage61, monitor];

/** Overlays held back for now: Garage 61 comes back once the integration is ready. They are
 *  not offered, listed or opened; their code stays. */
export const HIDDEN_KINDS = ['garage61'];
export const hiddenKind = (kind: string) => HIDDEN_KINDS.includes(kind);

/** The overlays offered in the settings. */
export const OVERLAYS: OverlayDef[] = ALL.filter((o) => !hiddenKind(o.kind));

export const byKind = (kind: string) => ALL.find((o) => o.kind === kind);
