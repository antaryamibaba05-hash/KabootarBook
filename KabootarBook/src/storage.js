import Database from '@tauri-apps/plugin-sql';

let db = null;
let writeQueue = Promise.resolve();
const KEY = 'kabootarbook.local.v1';

export async function openLocalStore() {
  try {
    db = await Database.load('sqlite:kabootarbook.db');
    const rows = await db.select('SELECT state_json FROM app_state WHERE id = 1');
    if (rows.length && rows[0].state_json) return JSON.parse(rows[0].state_json);
    let legacy = null;
    try { legacy = JSON.parse(localStorage.getItem(KEY) || 'null'); } catch { /* no usable browser preview state */ }
    if (legacy && typeof legacy === 'object') {
      await writeState(legacy);
      return legacy;
    }
    return null;
  } catch (error) {
    db = null;
    console.info('Native SQLite not active; using browser preview storage.', error);
    try { return JSON.parse(localStorage.getItem(KEY) || 'null'); } catch { return null; }
  }
}

async function writeState(snapshot) {
  if (!db) return;
  await db.execute(
    `INSERT INTO app_state(id, state_json, updated_at) VALUES(1, $1, $2)
     ON CONFLICT(id) DO UPDATE SET state_json = excluded.state_json, updated_at = excluded.updated_at`,
    [JSON.stringify(snapshot), new Date().toISOString()]
  );
}

export function persistState(snapshot) {
  try { localStorage.setItem(KEY, JSON.stringify(snapshot)); } catch { /* browser storage can be unavailable */ }
  if (!db) return;
  const stableCopy = structuredClone(snapshot);
  writeQueue = writeQueue.then(() => writeState(stableCopy)).catch(error => {
    console.error('Could not save KabootarBook data to SQLite.', error);
  });
}

export async function clearStoredData() {
  await writeQueue;
  try { localStorage.removeItem(KEY); } catch { /* ignore browser preview storage failures */ }
  if (db) await db.execute('DELETE FROM app_state WHERE id = 1');
}
