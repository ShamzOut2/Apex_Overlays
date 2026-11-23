// Apex Overlays shared pit stop times.
//
// POST /v1/stops   {"game":"iracing","stops":[{carId,tyres,fuelL,fuelSecs,fuelDelay,stood}, ...]}
//                  Opted-in apps send the stops they measured. Only those six numbers are
//                  kept per stop (plus the day). Nothing about the sender is read or stored:
//                  no IP address, no headers, no identifiers.
// GET  /v1/stops?game=iracing&car=<id>
//                  The latest stops for one car (the same six numbers), for apps to work out
//                  fill rates and tyre times from.

const MAX_BATCH = 20;
const KEEP_PER_CAR = 500;
const GAMES = new Set(['iracing']);

const json = (body, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json', 'cache-control': status === 200 ? 'public, max-age=3600' : 'no-store' } });

const num = (v, lo, hi) => typeof v === 'number' && Number.isFinite(v) && v >= lo && v <= hi;

/** The same checks the app makes: a real stop, or nothing. */
function clean(s) {
  if (!s || typeof s !== 'object') return null;
  const { carId, tyres, fuelL, fuelSecs, fuelDelay, stood } = s;
  if (!Number.isInteger(carId) || carId <= 0 || carId > 1e7) return null;
  if (!Number.isInteger(tyres) || tyres < 0 || tyres > 4) return null;
  if (!num(fuelL, 0, 200) || !num(fuelSecs, 0, 200) || !num(fuelDelay, 0, 120) || !num(stood, 1.5, 300)) return null;
  if (fuelSecs > stood + 1) return null;
  if (fuelL >= 1 && (fuelSecs <= 0.5 || fuelL / fuelSecs < 0.3 || fuelL / fuelSecs > 15)) return null;
  const r = (v) => Math.round(v * 100) / 100;
  return { carId, tyres, fuelL: r(fuelL), fuelSecs: r(fuelSecs), fuelDelay: r(fuelDelay), stood: r(stood) };
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname !== '/v1/stops') return json({ error: 'not found' }, 404);

    if (request.method === 'GET') {
      const game = url.searchParams.get('game') ?? '';
      const car = Number(url.searchParams.get('car'));
      if (!GAMES.has(game) || !Number.isInteger(car) || car <= 0) return json({ error: 'game and car wanted' }, 400);
      const { results } = await env.DB.prepare(
        'SELECT car_id, tyres, fuel_l, fuel_secs, fuel_delay, stood FROM stops WHERE game = ? AND car_id = ? ORDER BY id DESC LIMIT ?',
      )
        .bind(game, car, KEEP_PER_CAR)
        .all();
      return json({
        stops: results.map((r) => ({ carId: r.car_id, tyres: r.tyres, fuelL: r.fuel_l, fuelSecs: r.fuel_secs, fuelDelay: r.fuel_delay, stood: r.stood })),
      });
    }

    if (request.method === 'POST') {
      const text = await request.text();
      if (text.length > 8192) return json({ error: 'too large' }, 413);
      let body;
      try {
        body = JSON.parse(text);
      } catch {
        return json({ error: 'not JSON' }, 400);
      }
      if (!GAMES.has(body?.game) || !Array.isArray(body.stops) || body.stops.length > MAX_BATCH) return json({ error: 'bad request' }, 400);
      const stops = body.stops.map(clean).filter(Boolean);
      const day = new Date().toISOString().slice(0, 10);
      const insert = env.DB.prepare('INSERT INTO stops (game, car_id, tyres, fuel_l, fuel_secs, fuel_delay, stood, day) VALUES (?, ?, ?, ?, ?, ?, ?, ?)');
      const statements = stops.map((s) => insert.bind(body.game, s.carId, s.tyres, s.fuelL, s.fuelSecs, s.fuelDelay, s.stood, day));
      // Keep the newest rows per car only.
      for (const car of new Set(stops.map((s) => s.carId))) {
        statements.push(
          env.DB.prepare('DELETE FROM stops WHERE game = ? AND car_id = ? AND id NOT IN (SELECT id FROM stops WHERE game = ? AND car_id = ? ORDER BY id DESC LIMIT ?)').bind(
            body.game,
            car,
            body.game,
            car,
            KEEP_PER_CAR,
          ),
        );
      }
      if (statements.length) await env.DB.batch(statements);
      return json({ kept: stops.length });
    }

    return json({ error: 'method not allowed' }, 405);
  },
};
