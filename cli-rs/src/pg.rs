use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf, sync::OnceLock, time::Duration, time::Instant};
use tokio::time::sleep;

// Global PostgreSQL instance that lives for the CLI lifetime
static DB_INSTANCE: OnceLock<postgresql_embedded::PostgreSQL> = OnceLock::new();

// /// Ensure CORE_DATABASE_URI is set to a local DSN.
// /// Today we only set a sensible default if not present.
// /// Follow-up: actually start embedded Postgres and provision DB/user.
// #[allow(dead_code)]
// pub fn ensure_core_dsn() -> Result<String> {
//     if let Ok(uri) = env::var("CORE_DATABASE_URI") {
//         if !uri.is_empty() {
//             return Ok(uri);
//         }
//     }
//     // Default DSN (match plan doc): postgres://dev:dev@127.0.0.1:54330/dust
//     let uri = "postgres://dev:dev@127.0.0.1:54330/dust".to_string();
//     env::set_var("CORE_DATABASE_URI", &uri);
//     Ok(uri)
// }

/// Doctor helper: report basic status for CORE_DATABASE_URI.
pub fn status() -> String {
    match env::var("CORE_DATABASE_URI") {
        Ok(v) if !v.is_empty() => format!("CORE_DATABASE_URI set → {}", v),
        _ => "CORE_DATABASE_URI not set (will attempt to use default 127.0.0.1:54330)".to_string(),
    }
}

/// Ensure embedded Postgres is running and set CORE_DATABASE_URI to a reachable DSN.
pub async fn ensure_running() -> Result<(String, Option<postgresql_embedded::PostgreSQL>)> {
    let timeout_ms: u64 = env::var("DUSTX_PG_READY_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000);

    // Check if DB_INSTANCE is already set (DB already running from this process)
    if DB_INSTANCE.get().is_some() {
        if let Some(dsn) = read_lock_dsn()? {
            env::set_var("CORE_DATABASE_URI", &dsn);
            return Ok((dsn, None));
        }
    }

    // Check if we already have a running embedded instance from another process
    if let Some(dsn) = read_lock_dsn()? {
        // Use a shorter timeout for checking existing DB
        if wait_ready(&dsn, 2000).await.is_ok() {
            // Ensure dev user has proper permissions even for existing DB
            if let Some(lock) = read_lock() {
                let admin_dsn = format!(
                    "postgres://postgres:dev@{}:{}/dust_api",
                    lock.host, lock.port
                );
                if let Ok((client, connection)) =
                    tokio_postgres::connect(&admin_dsn, tokio_postgres::NoTls).await
                {
                    tokio::spawn(async move {
                        let _ = connection.await;
                    });

                    // Create dev user if it doesn't exist (or alter if exists)
                    let _ = client
                        .execute("CREATE USER dev WITH PASSWORD 'dev' SUPERUSER", &[])
                        .await;
                    // If user already exists, make sure it's a superuser
                    let _ = client.execute("ALTER USER dev WITH SUPERUSER", &[]).await;
                }
            }

            env::set_var("CORE_DATABASE_URI", &dsn);
            return Ok((dsn, None)); // Already running, no instance to manage
        }
        // Fall through to restart if not ready in time
    }

    let (dsn, pg) = start_db(None, None).await?;

    // Store the PostgreSQL instance in the OnceLock to keep it alive for the program duration
    DB_INSTANCE
        .set(pg)
        .map_err(|_| anyhow!("Failed to set DB instance (shouldn't happen)"))?;

    wait_ready(&dsn, timeout_ms).await?;
    Ok((dsn, None))
}

async fn wait_ready(dsn: &str, timeout_ms: u64) -> Result<()> {
    let start = Instant::now();
    let step = Duration::from_millis(50);
    let timeout = Duration::from_millis(timeout_ms);
    let mut _attempts = 0;
    loop {
        _attempts += 1;

        // Try target database first
        match tokio_postgres::connect(dsn, tokio_postgres::NoTls).await {
            Ok((client, connection)) => {
                tokio::spawn(async move {
                    let _ = connection.await;
                });
                match client.simple_query("SELECT 1").await {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(_e) => {}
                }
            }
            Err(_e) => {}
        }

        if start.elapsed() > timeout {
            return Err(anyhow!("postgres not ready after {}ms", timeout_ms));
        }
        sleep(step).await;
    }
}

fn pick_port(start: u16, attempts: u16) -> Option<u16> {
    for i in 0..attempts {
        if portpicker::is_free(start + i) {
            return Some(start + i);
        }
    }
    None
}

fn dust_data_dir() -> Result<PathBuf> {
    if let Ok(dir) = env::var("DUSTX_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let cwd = env::current_dir().context("cannot read current dir")?;
    Ok(cwd.join(".dust"))
}

pub fn cli_data_dir() -> Result<PathBuf> {
    dust_data_dir()
}

fn install_dir() -> Result<PathBuf> {
    if let Ok(p) = env::var("DUSTX_PG_INSTALL_DIR") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(home) = env::var("HOME") {
        return Ok(PathBuf::from(home).join(".dust/postgres"));
    }
    Ok(dust_data_dir()?.join("postgres"))
}
fn default_data_dir() -> Result<PathBuf> {
    Ok(dust_data_dir()?.join("db"))
}
fn lock_path() -> Result<PathBuf> {
    Ok(default_data_dir()?.join("pg.lock.json"))
}

#[derive(Serialize, Deserialize)]
struct PgLock {
    host: String,
    port: u16,
    password: String,
}

fn write_lock(port: u16, password: &str) -> Result<()> {
    let lp = lock_path()?;
    if let Some(parent) = lp.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let lk = PgLock {
        host: "127.0.0.1".to_string(),
        port,
        password: password.to_string(),
    };
    fs::write(lp, serde_json::to_vec_pretty(&lk)?).context("write pg lockfile")
}

fn read_lock() -> Option<PgLock> {
    let Ok(lp) = lock_path() else {
        return None;
    };
    let Ok(bytes) = fs::read(lp) else {
        return None;
    };
    serde_json::from_slice::<PgLock>(&bytes).ok()
}

fn read_lock_dsn() -> Result<Option<String>> {
    Ok(read_lock().map(|l| format!("postgres://dev:dev@{}:{}/dust_api", l.host, l.port)))
}

// #[allow(dead_code)]
// fn postmaster_pid_exists() -> bool {
//     if let Ok(dir) = default_data_dir() {
//         return dir.join("postmaster.pid").exists();
//     }
//     false
// }

pub async fn start_db(
    data_dir: Option<&str>,
    port: Option<u16>,
) -> Result<(String, postgresql_embedded::PostgreSQL)> {
    use postgresql_embedded::{PostgreSQL, Settings};
    let install_dir = install_dir()?;
    let data_dir = data_dir.map(PathBuf::from).unwrap_or(default_data_dir()?);
    let _ = fs::create_dir_all(&install_dir);
    let _ = fs::create_dir_all(&data_dir);

    let mut settings = Settings::new();
    settings.installation_dir = install_dir;
    settings.data_dir = data_dir;
    settings.host = "127.0.0.1".to_string();
    settings.port = port.or_else(|| pick_port(54330, 20)).unwrap_or(54330);
    settings.username = "postgres".to_string(); // Initial setup uses postgres
    settings.password = "dev".to_string();
    settings.temporary = false;
    // Ensure the port is actually set in postgresql.conf
    settings
        .configuration
        .insert("port".to_string(), settings.port.to_string());

    let mut pg = PostgreSQL::new(settings.clone());
    pg.setup().await.context("embedded PG setup failed")?;
    pg.start().await.context("embedded PG start failed")?;
    // Create the dev user and database
    let ready_deadline = Instant::now() + Duration::from_millis(10_000);

    // First create the database
    loop {
        match pg.create_database("dust_api").await {
            Ok(_) => break,
            Err(e) => {
                // If already exists or not ready yet, tolerate and retry a few times
                let msg = format!("{}", e);
                if msg.contains("already exists") {
                    break;
                }
                if Instant::now() > ready_deadline {
                    break;
                }
                sleep(Duration::from_millis(50)).await;
            }
        }
    }

    // Now create the dev user and grant permissions
    let admin_dsn = format!(
        "postgres://postgres:dev@127.0.0.1:{}/dust_api",
        settings.port
    );
    while Instant::now() < ready_deadline {
        if let Ok((client, connection)) =
            tokio_postgres::connect(&admin_dsn, tokio_postgres::NoTls).await
        {
            tokio::spawn(async move {
                let _ = connection.await;
            });

            // Create dev user if it doesn't exist (or alter if exists)
            let _ = client
                .execute("CREATE USER dev WITH PASSWORD 'dev' SUPERUSER", &[])
                .await;

            // If user already exists, make sure it's a superuser
            let _ = client.execute("ALTER USER dev WITH SUPERUSER", &[]).await;

            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    write_lock(settings.port, &settings.password)?;

    let dsn = format!("postgres://dev:dev@127.0.0.1:{}/dust_api", settings.port);
    env::set_var("CORE_DATABASE_URI", &dsn);
    Ok((dsn, pg))
}

// #[allow(dead_code)]
// pub async fn stop_db() -> Result<()> {
//     use postgresql_embedded::{PostgreSQL, Settings};
//     // If there's no lock, attempt graceful stop via default dir
//     let lock = read_lock();
//     let mut settings = Settings::new();
//     settings.installation_dir = install_dir()?;
//     settings.data_dir = default_data_dir()?;
//     settings.host = "127.0.0.1".to_string();
//     if let Some(l) = lock {
//         settings.port = l.port;
//         settings.password = l.password;
//     }
//     settings.temporary = false;
//     let pg = PostgreSQL::new(settings);
//     let _ = pg.stop().await; // ignore errors; best-effort
//     Ok(())
// }

// #[allow(dead_code)]
// pub fn db_status() -> String {
//     if let Some(lk) = read_lock() {
//         // Try a quick TCP connect for liveness
//         let alive = TcpStream::connect_timeout(
//             &format!("{}:{}", lk.host, lk.port).parse().unwrap(),
//             Duration::from_millis(50),
//         )
//         .is_ok();
//             // || postmaster_pid_exists();
//         return format!(
//             "embedded PG: host={} port={} {}\nCORE_DATABASE_URI={}",
//             lk.host,
//             lk.port,
//             if alive {
//                 "(running)"
//             } else {
//                 "(unknown/stopped)"
//             },
//             env::var("CORE_DATABASE_URI").unwrap_or_default()
//         );
//     }
//     status()
// }

pub fn backup(out: &str) -> Result<()> {
    use std::io::Write;
    let dir = default_data_dir()?;
    let file = std::fs::File::create(out).with_context(|| format!("create {}", out))?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
    for entry in walkdir::WalkDir::new(&dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            let name = path
                .strip_prefix(&dir)
                .unwrap()
                .to_string_lossy()
                .to_string();
            zip.start_file(name, options)?;
            let bytes = std::fs::read(path)?;
            zip.write_all(&bytes)?;
        }
    }
    zip.finish()?;
    Ok(())
}

pub fn reset(keep_backup: bool) -> Result<()> {
    let dir = default_data_dir()?;
    if keep_backup {
        let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let bak = dir.with_extension(format!("bak-{}", ts));
        std::fs::rename(&dir, &bak).ok();
    } else {
        std::fs::remove_dir_all(&dir).ok();
    }
    Ok(())
}
