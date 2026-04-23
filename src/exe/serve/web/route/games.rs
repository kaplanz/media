//! Game routes.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use media::kind::game::{Body, Game};
use media::kind::{Kind, Meta, Record};
use sqlx::SqlitePool;
use uuid::Uuid;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(fetch).put(update).delete(remove))
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    tgdb: Option<i64>,
    title: String,
    system: Option<String>,
    owned: i64,
    rated: Option<i64>,
    created: i64,
    updated: i64,
}

impl Row {
    fn into_record(self, tags: Vec<String>) -> Record {
        Record {
            item: Kind::Game(Game {
                id: self.id,
                tgdb: self.tgdb,
                title: self.title,
                system: self.system,
                owned: self.owned,
                rated: self.rated,
            }),
            meta: Meta {
                created: self.created,
                updated: self.updated,
            },
            tags,
        }
    }
}

async fn list(State(db): State<SqlitePool>) -> Result<Json<Vec<Record>>, StatusCode> {
    let rows = sqlx::query_as::<_, Row>(
        "SELECT games.id, tgdb, title, system, owned, rated, \
         media.created, media.updated \
         FROM games JOIN media ON media.id = games.id",
    )
    .fetch_all(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let all_tags = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT tags.media, tags.label \
         FROM tags JOIN games ON games.id = tags.media \
         ORDER BY tags.label",
    )
    .fetch_all(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut tag_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (id, label) in all_tags {
        tag_map.entry(id).or_default().push(label);
    }

    let records = rows
        .into_iter()
        .map(|row| {
            let tags = tag_map.remove(&row.id).unwrap_or_default();
            row.into_record(tags)
        })
        .collect();

    Ok(Json(records))
}

async fn fetch(
    State(db): State<SqlitePool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Record>, StatusCode> {
    let row = sqlx::query_as::<_, Row>(
        "SELECT games.id, tgdb, title, system, owned, rated, \
         media.created, media.updated \
         FROM games JOIN media ON media.id = games.id \
         WHERE games.id = ?",
    )
    .bind(id)
    .fetch_optional(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let tags =
        sqlx::query_scalar::<_, String>("SELECT label FROM tags WHERE media = ? ORDER BY label")
            .bind(id)
            .fetch_all(&db)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(row.into_record(tags)))
}

async fn create(
    State(db): State<SqlitePool>,
    Json(body): Json<Body>,
) -> Result<(StatusCode, Json<Uuid>), StatusCode> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO games (id, tgdb, title, system, owned, rated) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(body.tgdb)
    .bind(&body.title)
    .bind(&body.system)
    .bind(body.owned.unwrap_or(0))
    .bind(body.rated)
    .execute(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map(|_| (StatusCode::CREATED, Json(id)))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update(
    State(db): State<SqlitePool>,
    Path(id): Path<Uuid>,
    Json(body): Json<Body>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query(
        "UPDATE games SET tgdb = ?, title = ?, system = ?, owned = ?, rated = ? WHERE id = ?",
    )
    .bind(body.tgdb)
    .bind(&body.title)
    .bind(&body.system)
    .bind(body.owned.unwrap_or(0))
    .bind(body.rated)
    .bind(id)
    .execute(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    .and_then(|res| {
        if res.rows_affected() > 0 {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    })
}

async fn remove(
    State(db): State<SqlitePool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM media WHERE id = ?")
        .bind(id)
        .execute(&db)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .and_then(|res| {
            if res.rows_affected() > 0 {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        })
}
