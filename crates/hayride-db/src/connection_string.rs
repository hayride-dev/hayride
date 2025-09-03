use anyhow::Result;
use url::Url;

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    PostgreSQL,
    SQLite,
    MySQL,
    Unknown,
}

pub struct ConnectionStringParser {
    connection_string: String,
}

impl ConnectionStringParser {
    pub fn new(connection_string: &str) -> Self {
        Self {
            connection_string: connection_string.trim().to_string(),
        }
    }

    pub fn get_database_type(&self) -> Result<DatabaseType> {
        let conn_str = self.connection_string.trim();

        // 1) Try URL parse first
        if let Ok(url) = Url::parse(conn_str) {
            let scheme = url.scheme().to_ascii_lowercase();
            return Ok(match scheme.as_str() {
                // Postgres schemes
                "postgres" | "postgresql" => DatabaseType::PostgreSQL,

                // MySQL / MariaDB schemes
                "mysql" | "mariadb" | "mysqlx" => DatabaseType::MySQL,

                // SQLite schemes (URL or file:)
                "sqlite" => DatabaseType::SQLite,
                "file" => {
                    // file: URLs used with SQLite
                    DatabaseType::SQLite
                }

                // Unknown scheme -> fallback heuristics below
                _ => {
                    // e.g., scheme-only that some drivers accept; keep checking
                    self.fallback_detect(conn_str)
                }
            });
        }

        // 2) Not a URL: use fallback heuristics
        Ok(self.fallback_detect(conn_str))
    }

    fn fallback_detect(&self, conn_str: &str) -> DatabaseType {
        let s = conn_str;

        // Case-insensitive helper (for scheme-like prefixes)
        let lower = s.to_ascii_lowercase();

        // ---- SQLite ----
        // In-memory forms
        if lower == "sqlite::memory:" || lower.starts_with("file::memory:") {
            return DatabaseType::SQLite;
        }
        // Common SQLite file extensions or typical path-like strings.
        // Accept absolute/relative and Windows drive paths.
        let looks_like_path = s.starts_with("./")
            || s.starts_with("../")
            || s.starts_with('/')
            || s.contains('\\') && s.chars().nth(1) == Some(':'); // e.g., C:\...
        if lower.ends_with(".db")
            || lower.ends_with(".sqlite")
            || lower.ends_with(".sqlite3")
            || looks_like_path
        {
            return DatabaseType::SQLite;
        }

        // ---- PostgreSQL libpq keyword string ----
        // libpq accepts space-separated key=val; keys include user, password, host, port, dbname, etc.
        if seems_like_libpq_keywords(s) {
            return DatabaseType::PostgreSQL;
        }

        // ---- MySQL Go DSN style ----
        // user:pass@tcp(host:3306)/dbname?param=...
        if lower.contains("@tcp(") && lower.contains(")/") {
            return DatabaseType::MySQL;
        }

        // MySQL-ish: `mariadb://` etc. when URL parsing failed (rare), or driver-specific aliases
        if lower.starts_with("mariadb://")
            || lower.starts_with("mysql://")
            || lower.starts_with("mysqlx://")
        {
            return DatabaseType::MySQL;
        }

        // Final: Unknown
        DatabaseType::Unknown
    }
}

fn seems_like_libpq_keywords(s: &str) -> bool {
    // very light detection: space-separated tokens with '=', known-ish keys
    // Accepts: user=, password=, host=, port=, dbname=, application_name=, sslmode=, etc.
    let keys = [
        "user",
        "password",
        "host",
        "port",
        "dbname",
        "application_name",
        "sslmode",
        "options",
    ];
    let mut has_eq_tokens = false;
    for tok in s.split_whitespace() {
        if let Some((k, _v)) = tok.split_once('=') {
            has_eq_tokens = true;
            let k = k.to_ascii_lowercase();
            // If we find at least one recognizable libpq key, that's enough
            if keys.contains(&k.as_str()) {
                return true;
            }
        }
    }
    // Strings with key=value but unknown keys: still likely libpq
    has_eq_tokens
}
