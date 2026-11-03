use chrono::{DateTime, Utc};
use jni::{
    objects::{JObject, JString},
    sys::jstring,
    JNIEnv,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Pigeon {
    id: String,
    ring: String,
    name: String,
    breed: String,
    colour: String,
    loft: String,
    sex: String,
    hatch_date: String,
    notes: String,
    custom: Value,
    archived: bool,
    archive_reason: Option<String>,
    archived_at: Option<String>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Flight {
    id: String,
    pigeon_ids: Vec<String>,
    #[serde(default)]
    pigeon_returned_at: std::collections::HashMap<String, String>,
    started_at: String,
    ended_at: String,
    notes: String,
    conditions: String,
    location: String,
    custom: Value,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct State {
    pigeons: Vec<Pigeon>,
    flights: Vec<Flight>,
    active: Option<Value>,
    settings: Value,
}

fn with_db<T>(f: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
    let lock = DB
        .get()
        .ok_or("Database is not initialized")?
        .lock()
        .map_err(|e| e.to_string())?;
    f(&lock)
}
fn set_error(e: impl std::fmt::Display) -> String {
    json!({"error":e.to_string()}).to_string()
}
fn read_string(env: &mut JNIEnv<'_>, s: JString<'_>) -> Result<String, String> {
    env.get_string(&s)
        .map(|x| x.into())
        .map_err(|e| e.to_string())
}
fn out(env: &mut JNIEnv<'_>, s: String) -> jstring {
    env.new_string(s)
        .unwrap_or_else(|_| {
            env.new_string("{\"error\":\"JNI string conversion failed\"}")
                .unwrap()
        })
        .into_raw()
}
fn get_or<T: rusqlite::types::FromSql>(
    r: &rusqlite::Row<'_>,
    i: usize,
    default: T,
) -> rusqlite::Result<T> {
    r.get::<_, Option<T>>(i).map(|v| v.unwrap_or(default))
}
fn migrate(db: &Connection) -> Result<(), String> {
    let legacy_exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='app_state')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY,applied_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS pigeons(id TEXT PRIMARY KEY,ring TEXT NOT NULL UNIQUE COLLATE NOCASE,name TEXT NOT NULL DEFAULT '',breed TEXT NOT NULL DEFAULT '',colour TEXT NOT NULL DEFAULT '',loft TEXT NOT NULL DEFAULT '',sex TEXT NOT NULL DEFAULT '',hatch_date TEXT NOT NULL DEFAULT '',notes TEXT NOT NULL DEFAULT '',custom_json TEXT NOT NULL DEFAULT '{}',archived INTEGER NOT NULL DEFAULT 0,archive_reason TEXT,archived_at TEXT); CREATE TABLE IF NOT EXISTS flights(id TEXT PRIMARY KEY,started_at TEXT NOT NULL,ended_at TEXT NOT NULL,notes TEXT NOT NULL DEFAULT '',conditions TEXT NOT NULL DEFAULT '',location TEXT NOT NULL DEFAULT '',custom_json TEXT NOT NULL DEFAULT '{}'); CREATE TABLE IF NOT EXISTS flight_pigeons(flight_id TEXT NOT NULL REFERENCES flights(id) ON DELETE CASCADE,pigeon_id TEXT NOT NULL REFERENCES pigeons(id) ON DELETE RESTRICT,returned_at TEXT,PRIMARY KEY(flight_id,pigeon_id)); CREATE TABLE IF NOT EXISTS active_flight(id INTEGER PRIMARY KEY CHECK(id=1),session_json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS app_settings(id INTEGER PRIMARY KEY CHECK(id=1),settings_json TEXT NOT NULL); CREATE INDEX IF NOT EXISTS idx_flights_started_at ON flights(started_at); CREATE INDEX IF NOT EXISTS idx_flight_pigeons_pigeon ON flight_pigeons(pigeon_id);").map_err(|e|e.to_string())?;
    let done: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=1)",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !done {
        if legacy_exists {
            let legacy: Option<String> = db
                .query_row("SELECT state_json FROM app_state WHERE id=1", [], |r| {
                    r.get(0)
                })
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some(raw) = legacy {
                import_legacy(db, &raw)?;
            }
        }
        db.execute(
            "INSERT INTO schema_migrations(version,applied_at) VALUES(1,?1)",
            [Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
    }
    let has_returned_at: bool = db
        .prepare("PRAGMA table_info(flight_pigeons)")
        .map_err(|e| e.to_string())?
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
        .iter()
        .any(|name| name == "returned_at");
    if !has_returned_at {
        db.execute("ALTER TABLE flight_pigeons ADD COLUMN returned_at TEXT", [])
            .map_err(|e| e.to_string())?;
    }
    db.execute("UPDATE flight_pigeons SET returned_at=(SELECT ended_at FROM flights WHERE flights.id=flight_pigeons.flight_id) WHERE returned_at IS NULL",[]).map_err(|e|e.to_string())?;
    db.execute(
        "INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES(2,?1)",
        [Utc::now().to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
fn save_pigeon(db: &Connection, p: &Pigeon) -> Result<(), String> {
    if p.ring.trim().is_empty() {
        return Err("Ring number is required".into());
    }
    db.execute("INSERT INTO pigeons(id,ring,name,breed,colour,loft,sex,hatch_date,notes,custom_json,archived,archive_reason,archived_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13) ON CONFLICT(id) DO UPDATE SET ring=excluded.ring,name=excluded.name,breed=excluded.breed,colour=excluded.colour,loft=excluded.loft,sex=excluded.sex,hatch_date=excluded.hatch_date,notes=excluded.notes,custom_json=excluded.custom_json,archived=excluded.archived,archive_reason=excluded.archive_reason,archived_at=excluded.archived_at",params![p.id,p.ring.trim(),p.name,p.breed,p.colour,p.loft,p.sex,p.hatch_date,p.notes,if p.custom.is_null(){"{}".to_owned()}else{p.custom.to_string()},p.archived as i64,p.archive_reason,p.archived_at]).map_err(|e|e.to_string())?;
    Ok(())
}
fn import_legacy(db: &Connection, raw: &str) -> Result<(), String> {
    let state: State = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let mut rings = std::collections::HashSet::new();
    let mut ids = std::collections::HashSet::new();
    for p in &state.pigeons {
        if p.id.is_empty() || p.ring.trim().is_empty() {
            return Err("Backup has a pigeon missing its ID or ring number".into());
        }
        if !ids.insert(p.id.clone()) {
            return Err(format!("Backup repeats pigeon ID {}", p.id));
        }
        if !rings.insert(p.ring.trim().to_lowercase()) {
            return Err(format!("Backup repeats ring number {}", p.ring));
        }
    }
    let valid_pigeons = ids.clone();
    for f in &state.flights {
        if f.id.is_empty() || !ids.insert(f.id.clone()) {
            return Err(format!(
                "Backup has missing or duplicate flight ID {}",
                f.id
            ));
        }
        let start = DateTime::parse_from_rfc3339(&f.started_at)
            .map_err(|_| format!("Flight {} has invalid release time", f.id))?;
        let end = DateTime::parse_from_rfc3339(&f.ended_at)
            .map_err(|_| format!("Flight {} has invalid return time", f.id))?;
        if end < start {
            return Err(format!("Flight {} returns before release", f.id));
        }
        if f.pigeon_ids.is_empty() || f.pigeon_ids.iter().any(|id| !valid_pigeons.contains(id)) {
            return Err(format!("Flight {} refers to an unknown pigeon", f.id));
        }
    }
    if let Some(active) = &state.active {
        if active["startedAt"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .is_none()
            || active["pigeonIds"].as_array().is_none_or(|ids| {
                ids.is_empty()
                    || ids
                        .iter()
                        .any(|x| x.as_str().is_none_or(|id| !valid_pigeons.contains(id)))
            })
        {
            return Err("Backup active flight has invalid time or pigeon links".into());
        }
    }
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM flights", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM pigeons", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM active_flight", [])
        .map_err(|e| e.to_string())?;
    for p in &state.pigeons {
        save_pigeon(&tx, p)?
    }
    for f in &state.flights {
        tx.execute("INSERT OR IGNORE INTO flights(id,started_at,ended_at,notes,conditions,location,custom_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![f.id,f.started_at,f.ended_at,f.notes,f.conditions,f.location,if f.custom.is_null(){"{}".to_owned()}else{f.custom.to_string()}]).map_err(|e|e.to_string())?;
        for pid in &f.pigeon_ids {
            tx.execute("INSERT OR IGNORE INTO flight_pigeons(flight_id,pigeon_id) SELECT ?1,?2 WHERE EXISTS(SELECT 1 FROM pigeons WHERE id=?2)",params![f.id,pid]).map_err(|e|e.to_string())?;
        }
    }
    if let Some(active) = state.active {
        tx.execute(
            "INSERT OR REPLACE INTO active_flight(id,session_json) VALUES(1,?1)",
            [active.to_string()],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "INSERT OR REPLACE INTO app_settings(id,settings_json) VALUES(1,?1)",
        [if state.settings.is_null() {
            "{}".into()
        } else {
            state.settings.to_string()
        }],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}
fn load_state() -> Result<String, String> {
    with_db(|db| {
        let mut s = State {
            settings: json!({}),
            ..State::default()
        };
        let mut q=db.prepare("SELECT id,ring,name,breed,colour,loft,sex,hatch_date,notes,custom_json,archived,archive_reason,archived_at FROM pigeons ORDER BY ring COLLATE NOCASE").map_err(|e|e.to_string())?;
        s.pigeons = q
            .query_map([], |r| {
                let custom: String = get_or(r, 9, "{}".into())?;
                Ok(Pigeon {
                    id: r.get(0)?,
                    ring: r.get(1)?,
                    name: r.get(2)?,
                    breed: r.get(3)?,
                    colour: r.get(4)?,
                    loft: r.get(5)?,
                    sex: r.get(6)?,
                    hatch_date: r.get(7)?,
                    notes: r.get(8)?,
                    custom: serde_json::from_str(&custom).unwrap_or(json!({})),
                    archived: r.get::<_, i64>(10)? != 0,
                    archive_reason: r.get(11)?,
                    archived_at: r.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut q=db.prepare("SELECT id,started_at,ended_at,notes,conditions,location,custom_json FROM flights ORDER BY started_at DESC").map_err(|e|e.to_string())?;
        s.flights = q
            .query_map([], |r| {
                let id: String = r.get(0)?;
                let mut p = db
                    .prepare("SELECT pigeon_id,returned_at FROM flight_pigeons WHERE flight_id=?1")
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let pairs = p
                    .query_map([&id], |x| {
                        Ok((x.get::<_, String>(0)?, x.get::<_, Option<String>>(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                let ids = pairs.iter().map(|x| x.0.clone()).collect::<Vec<_>>();
                let returned_at = pairs
                    .into_iter()
                    .filter_map(|(id, time)| time.map(|time| (id, time)))
                    .collect::<std::collections::HashMap<_, _>>();
                let custom: String = get_or(r, 6, "{}".into())?;
                Ok(Flight {
                    id,
                    pigeon_ids: ids,
                    pigeon_returned_at: returned_at,
                    started_at: r.get(1)?,
                    ended_at: r.get(2)?,
                    notes: r.get(3)?,
                    conditions: r.get(4)?,
                    location: r.get(5)?,
                    custom: serde_json::from_str(&custom).unwrap_or(json!({})),
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let active: Option<String> = db
            .query_row(
                "SELECT session_json FROM active_flight WHERE id=1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        s.active = active.and_then(|v| serde_json::from_str(&v).ok());
        let settings: Option<String> = db
            .query_row(
                "SELECT settings_json FROM app_settings WHERE id=1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        s.settings = settings
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or(json!({}));
        serde_json::to_string(&s).map_err(|e| e.to_string())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeInit(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    path: JString<'_>,
) -> jstring {
    let result = (|| {
        let p = read_string(&mut env, path)?;
        let db = Connection::open(Path::new(&p)).map_err(|e| e.to_string())?;
        migrate(&db)?;
        if DB.set(Mutex::new(db)).is_err() {
            return load_state();
        }
        load_state()
    })();
    out(&mut env, result.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeLoad(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
) -> jstring {
    let r = load_state().unwrap_or_else(set_error);
    out(&mut env, r)
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeSavePigeon(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    raw: JString<'_>,
) -> jstring {
    let r = (|| {
        let text = read_string(&mut env, raw)?;
        let p: Pigeon = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        with_db(|db| save_pigeon(db, &p).and_then(|_| load_state()))
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeSetPigeonStatus(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    jid: JString<'_>,
    jreason: JString<'_>,
) -> jstring {
    let r = (|| {
        let id = read_string(&mut env, jid)?;
        let reason = read_string(&mut env, jreason)?;
        with_db(|db| {
            if reason.is_empty() {
                db.execute("UPDATE pigeons SET archived=0,archive_reason=NULL,archived_at=NULL WHERE id=?1",[id]).map_err(|e|e.to_string())?;
            } else {
                if !["sold", "died", "deleted"].contains(&reason.as_str()) {
                    return Err("Invalid archive reason".into());
                }
                let active_raw: Option<String> = db
                    .query_row(
                        "SELECT session_json FROM active_flight WHERE id=1",
                        [],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                let in_flight = active_raw
                    .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                    .is_some_and(|active| {
                        active["pigeonIds"]
                            .as_array()
                            .is_some_and(|ids| ids.iter().any(|x| x.as_str() == Some(&id)))
                            && !active["pigeonReturnedAt"].get(&id).is_some()
                    });
                if in_flight {
                    return Err("Record this pigeon's return before archiving it".into());
                }
                db.execute(
                    "UPDATE pigeons SET archived=1,archive_reason=?1,archived_at=?2 WHERE id=?3",
                    params![reason, Utc::now().to_rfc3339(), id],
                )
                .map_err(|e| e.to_string())?;
            }
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeDeletePigeon(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    jid: JString<'_>,
) -> jstring {
    let r = (|| {
        let id = read_string(&mut env, jid)?;
        with_db(|db| {
            let attached: bool = db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM flight_pigeons WHERE pigeon_id=?1)",
                    [&id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if attached {
                return Err(
                    "This pigeon has flight history; keep it in the archive to preserve its links"
                        .into(),
                );
            }
            db.execute("DELETE FROM pigeons WHERE id=?1 AND archived=1", [id])
                .map_err(|e| e.to_string())?;
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeSaveSettings(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    raw: JString<'_>,
) -> jstring {
    let r = (|| {
        let text = read_string(&mut env, raw)?;
        let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        with_db(|db| {
            db.execute(
                "INSERT OR REPLACE INTO app_settings(id,settings_json) VALUES(1,?1)",
                [value.to_string()],
            )
            .map_err(|e| e.to_string())?;
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeStartFlight(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    raw: JString<'_>,
) -> jstring {
    let r = (|| {
        let text = read_string(&mut env, raw)?;
        let mut value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let start = value["startedAt"]
            .as_str()
            .ok_or("Missing start timestamp")?;
        if DateTime::parse_from_rfc3339(start).is_err() {
            return Err("Invalid start timestamp".into());
        }
        let ids = value["pigeonIds"]
            .as_array()
            .ok_or("Choose at least one pigeon")?
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Err("Choose at least one pigeon".into());
        }
        let flight_id = value["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or("active-flight")
            .to_owned();
        value["id"] = json!(flight_id);
        with_db(|db| {
            let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
            let occupied: bool = tx
                .query_row("SELECT EXISTS(SELECT 1 FROM active_flight)", [], |r| {
                    r.get(0)
                })
                .map_err(|e| e.to_string())?;
            if occupied {
                return Err("A flight is already active".into());
            }
            for id in &ids {
                let exists: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM pigeons WHERE id=?1 AND archived=0)",
                        [id.as_str()],
                        |r| r.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                if !exists {
                    return Err("A selected pigeon is not active in the loft".into());
                }
            }
            tx.execute(
                "INSERT INTO active_flight(id,session_json) VALUES(1,?1)",
                [value.to_string()],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeReturnPigeon(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    jid: JString<'_>,
    jend: JString<'_>,
) -> jstring {
    let r = (|| {
        let id = read_string(&mut env, jid)?;
        let end = read_string(&mut env, jend)?;
        let end_dt = DateTime::parse_from_rfc3339(&end).map_err(|_| "Invalid return timestamp")?;
        with_db(|db| {
            let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
            let raw: String = tx
                .query_row(
                    "SELECT session_json FROM active_flight WHERE id=1",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .ok_or("No active flight")?;
            let mut active: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            let start = DateTime::parse_from_rfc3339(
                active["startedAt"].as_str().ok_or("Missing start time")?,
            )
            .map_err(|e| e.to_string())?;
            if end_dt < start {
                return Err("Return time cannot be earlier than release".into());
            }
            let ids = active["pigeonIds"]
                .as_array()
                .ok_or("No pigeons in active flight")?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if !ids.iter().any(|x| x == &id) {
                return Err("Pigeon is not part of this flight".into());
            }
            if !active["pigeonReturnedAt"].is_object() {
                active["pigeonReturnedAt"] = json!({});
            }
            let returned = active["pigeonReturnedAt"]
                .as_object_mut()
                .ok_or("Could not update return list")?;
            if returned.contains_key(&id) {
                return Err("This pigeon is already recorded as returned".into());
            }
            returned.insert(id.clone(), json!(end));
            tx.execute(
                "UPDATE active_flight SET session_json=?1 WHERE id=1",
                [active.to_string()],
            )
            .map_err(|e| e.to_string())?;
            let returned_ids = active["pigeonReturnedAt"]
                .as_object()
                .ok_or("Could not read return list")?;
            let remaining = ids.iter().any(|pid| !returned_ids.contains_key(pid));
            if !remaining {
                let f: Flight =
                    serde_json::from_value(active.clone()).map_err(|e| e.to_string())?;
                let finish = f
                    .pigeon_ids
                    .iter()
                    .filter_map(|pid| f.pigeon_returned_at.get(pid))
                    .filter_map(|t| DateTime::parse_from_rfc3339(t).ok())
                    .max()
                    .ok_or("No valid pigeon returns")?
                    .to_rfc3339();
                tx.execute("INSERT INTO flights(id,started_at,ended_at,notes,conditions,location,custom_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![f.id,f.started_at,finish,f.notes,f.conditions,f.location,f.custom.to_string()]).map_err(|e|e.to_string())?;
                for pid in &f.pigeon_ids {
                    tx.execute("INSERT INTO flight_pigeons(flight_id,pigeon_id,returned_at) VALUES(?1,?2,?3)",params![f.id,pid,f.pigeon_returned_at.get(pid)]).map_err(|e|e.to_string())?;
                }
                tx.execute("DELETE FROM active_flight WHERE id=1", [])
                    .map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeStopFlight(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    jend: JString<'_>,
) -> jstring {
    let r = (|| {
        let end = read_string(&mut env, jend)?;
        let end_dt = DateTime::parse_from_rfc3339(&end).map_err(|e| e.to_string())?;
        with_db(|db| {
            let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
            let raw: Option<String> = tx
                .query_row(
                    "SELECT session_json FROM active_flight WHERE id=1",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            let raw = raw.ok_or("No active flight")?;
            let mut active: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            let start = active["startedAt"].as_str().ok_or("Missing start time")?;
            if end_dt < DateTime::parse_from_rfc3339(start).map_err(|e| e.to_string())? {
                return Err("Return time cannot be earlier than release".into());
            }
            active["endedAt"] = json!(end);
            let ids = active["pigeonIds"]
                .as_array()
                .ok_or("No pigeons in flight")?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let returned = active["pigeonReturnedAt"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            let mut times = returned
                .values()
                .filter_map(Value::as_str)
                .map(|x| DateTime::parse_from_rfc3339(x).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            if times.len() != returned.len() || ids.iter().any(|pid| !returned.contains_key(pid)) {
                return Err("Record a return for every flying pigeon first".into());
            }
            times.sort();
            active["endedAt"] = json!(times
                .last()
                .ok_or("No pigeon return timestamps")?
                .to_rfc3339());
            let f: Flight = serde_json::from_value(active.clone()).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO flights(id,started_at,ended_at,notes,conditions,location,custom_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![f.id,f.started_at,f.ended_at,f.notes,f.conditions,f.location,f.custom.to_string()]).map_err(|e|e.to_string())?;
            for pid in f.pigeon_ids {
                tx.execute(
                    "INSERT INTO flight_pigeons(flight_id,pigeon_id,returned_at) VALUES(?1,?2,?3)",
                    params![f.id, pid, f.pigeon_returned_at.get(&pid)],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.execute("DELETE FROM active_flight WHERE id=1", [])
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeAddFlight(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    raw: JString<'_>,
) -> jstring {
    let r = (|| {
        let text = read_string(&mut env, raw)?;
        let f: Flight = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let start = DateTime::parse_from_rfc3339(&f.started_at).map_err(|e| e.to_string())?;
        let end = DateTime::parse_from_rfc3339(&f.ended_at).map_err(|e| e.to_string())?;
        if end < start {
            return Err("Return time cannot be earlier than release".into());
        }
        if f.pigeon_ids.is_empty() {
            return Err("Choose at least one pigeon".into());
        }
        with_db(|db| {
            let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO flights(id,started_at,ended_at,notes,conditions,location,custom_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![f.id,f.started_at,f.ended_at,f.notes,f.conditions,f.location,if f.custom.is_null(){"{}".into()}else{f.custom.to_string()}]).map_err(|e|e.to_string())?;
            for pid in &f.pigeon_ids {
                let returned = f.pigeon_returned_at.get(pid).or(Some(&f.ended_at));
                tx.execute("INSERT INTO flight_pigeons(flight_id,pigeon_id,returned_at) SELECT ?1,?2,?3 WHERE EXISTS(SELECT 1 FROM pigeons WHERE id=?2)",params![f.id,pid,returned]).map_err(|e|e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeDeleteFlight(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    jid: JString<'_>,
) -> jstring {
    let r = (|| {
        let id = read_string(&mut env, jid)?;
        with_db(|db| {
            db.execute("DELETE FROM flights WHERE id=?1", [id])
                .map_err(|e| e.to_string())?;
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}
#[no_mangle]
pub extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeImportLegacy(
    mut env: JNIEnv<'_>,
    _: JObject<'_>,
    raw: JString<'_>,
) -> jstring {
    let r = (|| {
        let text = read_string(&mut env, raw)?;
        with_db(|db| {
            import_legacy(db, &text)?;
            load_state()
        })
    })();
    out(&mut env, r.unwrap_or_else(set_error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        migrate(&db).unwrap();
        db
    }
    fn pigeon(id: &str, ring: &str) -> Pigeon {
        Pigeon {
            id: id.into(),
            ring: ring.into(),
            ..Pigeon::default()
        }
    }
    #[test]
    fn rejects_duplicate_ring_numbers_without_case_sensitivity() {
        let db = test_db();
        save_pigeon(&db, &pigeon("a", "PK-1")).unwrap();
        assert!(save_pigeon(&db, &pigeon("b", "pk-1")).is_err());
    }
    #[test]
    fn legacy_import_preserves_active_flight_and_shared_flight_links() {
        let db = test_db();
        let raw = r#"{"pigeons":[{"id":"p1","ring":"R1"},{"id":"p2","ring":"R2"}],"flights":[{"id":"f1","pigeonIds":["p1","p2"],"startedAt":"2025-01-01T08:00:00Z","endedAt":"2025-01-01T09:00:00Z"}],"active":{"id":"active-1","pigeonIds":["p2"],"startedAt":"2025-01-02T08:00:00Z"},"settings":{"theme":"dark"}}"#;
        import_legacy(&db, raw).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM flight_pigeons WHERE flight_id='f1'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        assert!(db
            .query_row(
                "SELECT session_json FROM active_flight WHERE id=1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap()
            .contains("active-1"));
        assert!(db
            .query_row(
                "SELECT settings_json FROM app_settings WHERE id=1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap()
            .contains("dark"));
    }
    #[test]
    fn per_pigeon_return_map_survives_flight_json_roundtrip() {
        let raw = r#"{"id":"f1","pigeonIds":["p1","p2"],"startedAt":"2025-06-01T09:00:00Z","endedAt":"2025-06-01T10:30:00Z","pigeonReturnedAt":{"p1":"2025-06-01T09:45:00Z","p2":"2025-06-01T10:30:00Z"}}"#;
        let f: Flight = serde_json::from_str(raw).unwrap();
        assert_eq!(
            f.pigeon_returned_at.get("p1").unwrap(),
            "2025-06-01T09:45:00Z"
        );
        assert_eq!(
            serde_json::from_str::<Flight>(&serde_json::to_string(&f).unwrap())
                .unwrap()
                .pigeon_returned_at,
            f.pigeon_returned_at
        );
    }
    #[test]
    fn timestamp_arithmetic_handles_midnight_and_rejects_reversed_returns() {
        let start = DateTime::parse_from_rfc3339("2025-05-01T23:30:00Z").unwrap();
        let end = DateTime::parse_from_rfc3339("2025-05-02T00:15:00Z").unwrap();
        assert_eq!(end.signed_duration_since(start).num_minutes(), 45);
        assert!(end > start);
    }
    #[test]
    fn backup_restore_replaces_records_only_after_validating_everything() {
        let db = test_db();
        save_pigeon(&db, &pigeon("old", "OLD")).unwrap();
        let invalid = r#"{"pigeons":[{"id":"new","ring":"NEW"}],"flights":[{"id":"bad","pigeonIds":["missing"],"startedAt":"2025-01-01T10:00:00Z","endedAt":"2025-01-01T09:00:00Z"}]}"#;
        assert!(import_legacy(&db, invalid).is_err());
        assert_eq!(
            db.query_row("SELECT ring FROM pigeons WHERE id='old'", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "OLD"
        );
        let valid = r#"{"pigeons":[{"id":"new","ring":"NEW"}],"flights":[],"active":null,"settings":{"theme":"dark"}}"#;
        import_legacy(&db, valid).unwrap();
        assert!(
            db.query_row(
                "SELECT EXISTS(SELECT 1 FROM pigeons WHERE id='old')",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap()
                == false
        );
        assert!(db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pigeons WHERE id='new')",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap());
    }
    #[test]
    fn migration_adds_return_time_without_losing_legacy_flight_rows() {
        let db = test_db();
        db.execute_batch("DROP TABLE flight_pigeons; CREATE TABLE flight_pigeons(flight_id TEXT NOT NULL REFERENCES flights(id) ON DELETE CASCADE,pigeon_id TEXT NOT NULL REFERENCES pigeons(id) ON DELETE RESTRICT,PRIMARY KEY(flight_id,pigeon_id)); INSERT INTO pigeons(id,ring) VALUES('p1','R-1'); INSERT INTO flights(id,started_at,ended_at) VALUES('f1','2025-01-01T09:00:00Z','2025-01-01T10:00:00Z'); INSERT INTO flight_pigeons(flight_id,pigeon_id) VALUES('f1','p1');").unwrap();
        migrate(&db).unwrap();
        let returned: String = db
            .query_row(
                "SELECT returned_at FROM flight_pigeons WHERE flight_id='f1' AND pigeon_id='p1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(returned, "2025-01-01T10:00:00Z");
        assert!(db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=2)",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap());
    }
    #[test]
    fn schema_keeps_shared_flight_foreign_keys_and_archived_reason() {
        let db = test_db();
        let tables: Vec<String> = db
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(tables.contains(&"flight_pigeons".into()));
        let columns: Vec<String> = db
            .prepare("PRAGMA table_info(pigeons)")
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(columns.contains(&"archive_reason".into()));
    }
}
