//! Book routes.

use std::collections::HashMap;
use std::fmt::Write as _;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use media::kind::book::{Body, Book};
use media::kind::{Kind, Meta, Record};
use sqlx::SqlitePool;
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use super::query::Order;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .routes(routes!(list, create))
        .routes(routes!(fetch, update, remove))
}

/// Sort field for books.
#[derive(Clone, Copy, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
enum Sort {
    /// Sort by title.
    Title,
    /// Sort by creation time.
    #[default]
    Created,
    /// Sort by last update time.
    Updated,
}

impl Sort {
    fn as_col(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Created => "media.created",
            Self::Updated => "media.updated",
        }
    }
}

/// Query parameters for listing books.
#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::IntoParams)]
struct Params {
    /// Search title (case-insensitive substring).
    q: Option<String>,
    /// Filter by tag.
    tag: Option<String>,
    /// Field to sort by.
    sort: Option<Sort>,
    /// Sort direction.
    order: Option<Order>,
    /// Maximum number of results.
    limit: Option<i64>,
    /// Number of results to skip.
    offset: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    isbn: Option<String>,
    hcid: Option<i64>,
    title: String,
    cover: Option<String>,
    about: Option<String>,
    color: Option<String>,
    created: i64,
    updated: i64,
}

impl Row {
    fn into_record(self, tags: Vec<String>) -> Record {
        Record {
            item: Kind::Book(Book {
                id: self.id,
                isbn: self.isbn,
                hcid: self.hcid,
                title: self.title,
                cover: self.cover,
                about: self.about,
                color: self.color,
            }),
            meta: Meta {
                created: self.created,
                updated: self.updated,
            },
            tags,
        }
    }
}

#[utoipa::path(get, path = "/", tag = "books",
    params(Params),
    responses((status = 200, body = Vec<Record>)))]
async fn list(
    State(db): State<SqlitePool>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<Record>>, StatusCode> {
    let sort_col = params.sort.unwrap_or_default().as_col();
    let order = params.order.unwrap_or_default().as_str();

    let mut sql = String::from(
        "SELECT books.id, isbn, hcid, title, cover, about, color, \
         media.created, media.updated \
         FROM books JOIN media ON media.id = books.id",
    );
    let mut conds: Vec<&str> = Vec::new();
    if params.q.is_some() {
        conds.push("title LIKE '%' || ? || '%'");
    }
    if params.tag.is_some() {
        conds.push(
            "EXISTS (SELECT 1 FROM tags \
             WHERE tags.media = books.id AND tags.label = ?)",
        );
    }
    if !conds.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conds.join(" AND "));
    }
    write!(sql, " ORDER BY {sort_col} {order}").unwrap();
    if params.limit.is_some() {
        sql.push_str(" LIMIT ? OFFSET ?");
    }

    let mut stmt = sqlx::query_as::<_, Row>(&sql);
    if let Some(ref q) = params.q {
        stmt = stmt.bind(q.as_str());
    }
    if let Some(ref tag) = params.tag {
        stmt = stmt.bind(tag.as_str());
    }
    if let Some(limit) = params.limit {
        stmt = stmt.bind(limit).bind(params.offset.unwrap_or(0));
    }

    let rows = stmt
        .fetch_all(&db)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let all_tags = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT tags.media, tags.label \
         FROM tags JOIN books ON books.id = tags.media \
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

#[utoipa::path(get, path = "/{id}", tag = "books",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Record), (status = 404)))]
async fn fetch(
    State(db): State<SqlitePool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Record>, StatusCode> {
    let row = sqlx::query_as::<_, Row>(
        "SELECT books.id, isbn, hcid, title, cover, about, color, \
         media.created, media.updated \
         FROM books JOIN media ON media.id = books.id \
         WHERE books.id = ?",
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

#[utoipa::path(post, path = "/", tag = "books",
    request_body = Body,
    responses((status = 201, body = Uuid), (status = 500)))]
async fn create(
    State(db): State<SqlitePool>,
    Json(body): Json<Body>,
) -> Result<(StatusCode, Json<Uuid>), StatusCode> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO books (id, isbn, hcid, title, cover, about, color) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&body.isbn)
    .bind(body.hcid)
    .bind(&body.title)
    .bind(&body.cover)
    .bind(&body.about)
    .bind(&body.color)
    .execute(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map(|_| (StatusCode::CREATED, Json(id)))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[utoipa::path(put, path = "/{id}", tag = "books",
    params(("id" = Uuid, Path)),
    request_body = Body,
    responses((status = 204), (status = 404)))]
async fn update(
    State(db): State<SqlitePool>,
    Path(id): Path<Uuid>,
    Json(body): Json<Body>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query(
        "UPDATE books \
         SET isbn = ?, hcid = ?, title = ?, cover = ?, about = ?, color = ? \
         WHERE id = ?",
    )
    .bind(&body.isbn)
    .bind(body.hcid)
    .bind(&body.title)
    .bind(&body.cover)
    .bind(&body.about)
    .bind(&body.color)
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

#[utoipa::path(delete, path = "/{id}", tag = "books",
    params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404)))]
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
