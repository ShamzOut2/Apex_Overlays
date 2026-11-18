# Packs the official outlines built by every_indexed_track (<dir>/out/<id>.json) into
# data/iracing-track-outlines.json: each layout's points (and pit lane dashes) as base64 of
# little-endian u16 pairs (0..65535 = 0..1).
import base64, glob, json, os, struct, sys
src, dst = sys.argv[1], sys.argv[2]
def pack(points):
    out = bytearray()
    for x, y in points:
        out += struct.pack('<HH', round(min(1, max(0, x)) * 65535), round(min(1, max(0, y)) * 65535))
    return base64.b64encode(bytes(out)).decode()
tracks = {}
for f in sorted(glob.glob(os.path.join(src, '*.json')), key=lambda p: int(os.path.basename(p)[:-5])):
    m = json.load(open(f))
    t = {'p': pack(m['points'])}
    if m.get('pit'):
        t['pit'] = pack([pt for dash in m['pit'] for pt in dash])
    tracks[str(m['trackId'])] = t
json.dump({'version': 2, 'tracks': tracks}, open(dst, 'w'), separators=(',', ':'))
print(len(tracks), 'tracks,', os.path.getsize(dst) // 1024, 'KB')
