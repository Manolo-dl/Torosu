use std::collections::HashMap;
use sqlx::{AssertSqlSafe, Column, Row, prelude::FromRow};
use crate::DbState;

#[derive(FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub name: String,
}

#[derive(FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    pub cid: i64,
    pub name: String,
    #[sqlx(rename = "type")]
    pub column_type: String,
    pub notnull: i64,
    pub dflt_value: Option<String>,
    pub pk: i64,
}

#[tauri::command]
pub async fn get_database_tables(db_state: tauri::State<'_, DbState>) -> Result<Vec<TableInfo>, String> {

    let pool = &db_state.pool;

    let tables = sqlx::query_as::<_, TableInfo>(
        "SELECT name FROM sqlite_master WHERE type='table'"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(tables)
}

#[tauri::command]
pub async fn get_table_columns(db_state: tauri::State<'_, DbState>, table_name: String) -> Result<Vec<ColumnInfo>, String> {

    let pool = &db_state.pool;

    let query = format!("PRAGMA table_info({})", table_name);

    let columns = sqlx::query_as::<_, ColumnInfo>(AssertSqlSafe(query))
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(columns)
}

#[tauri::command]
pub async fn execute_query(db_state: tauri::State<'_, DbState>, query: String) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {

    let pool = &db_state.pool;

    let rows = sqlx::query(AssertSqlSafe(query))
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let result = rows.iter().map(|row| {
        let mut map = HashMap::new();

        for (i, column) in row.columns().iter().enumerate() {
            let value: serde_json::Value = if let Ok(v) = row.try_get::<i64, _>(i) {
                serde_json::json!(v)
            } else if let Ok(v) = row.try_get::<f64, _>(i) {
                serde_json::json!(v)
            } else if let Ok(v) = row.try_get::<String, _>(i) {
                serde_json::json!(v)
            } else {
                serde_json::Value::Null
            };

            map.insert(column.name().to_string(), value);
        }

        map
    }).collect();

    Ok(result)
}

