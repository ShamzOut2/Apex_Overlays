# Shared pit stop times

A small Cloudflare Worker with a D1 database. Apex Overlays users who opt in (Pit rejoin →
"Share your stop times") send each stop they make as six numbers: the iRacing car id, tyres
changed, litres added, seconds the fuel took, seconds before the fuel started, and seconds
stood in the box. Everyone (opted in or not, with "Use shared stop times" on) downloads the
stops for the car they are in, so the pit rejoin can time a stop before they have made one.

Privacy: no name, customer id, licence key, install id, IP address, session or track is sent
or stored. The worker reads only the request body; the date a stop arrived is kept to allow old
rows to be dropped. The newest 500 stops per car are kept.

## Deploy (once)

    npm install -g wrangler
    wrangler login
    wrangler d1 create apex-pit-data          # put the printed database_id in wrangler.toml
    wrangler d1 execute apex-pit-data --remote --file=schema.sql
    wrangler deploy

The app expects it at `https://apex-pit-data.<account>.workers.dev` (see `SERVICE` in
`crates/core/src/pitshare.rs`).
