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
import { trackmap } from './trackmap';
import { tyres } from './tyres';
import { weather } from './weather';

export const OVERLAYS: OverlayDef[] = [relative, standings, trackmap, inputs, inputGraph, fuel, battle, session, flags, weather, tyres, databox, spotter, safety, delta, pit, incidents, garage61, monitor];

export const byKind = (kind: string) => OVERLAYS.find((o) => o.kind === kind);
