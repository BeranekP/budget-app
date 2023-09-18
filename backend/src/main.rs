use std::{default, f64::consts::E};

use axum::{
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::{
    types::chrono::{DateTime, NaiveDate, NaiveDateTime, Utc},
    Decode, Encode, Error, Executor, FromRow, PgPool, Postgres, QueryBuilder, Type,
};

use tower_http::cors::CorsLayer;

use rust_decimal;

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://postgres:7622043385@localhost:5432/postgres"
    )]
    pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    pool.execute(include_str!("../schema.sql"))
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin("http://127.0.0.1:5173".parse::<HeaderValue>().unwrap());

    let router = Router::new()
        .route("/items", post(create_item))
        .route("/items", get(get_items))
        .route("/items/:id", get(get_item))
        .route("/balance", get(get_balance))
        .route("/categories", get(get_categories))
        .layer(cors_layer)
        .with_state(pool);

    Ok(router.into())
}

async fn create_item(
    State(pool): State<PgPool>,
    Json(request_data): Json<RequestData>,
) -> impl IntoResponse {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO budget_items (name, amount, description, type_id, category_id)",
    );

    // process category, check if exists or create new
    let new_category = request_data.category;
    let new_item = request_data.item;
    let cat_result: Option<Category> = sqlx::query_as("SELECT * FROM category WHERE category=($1)")
        .bind(new_category.category.trim())
        .fetch_one(&pool)
        .await
        .map_err(|_| {})
        .ok();

    let query_category = cat_result;
    let category_id;
    match query_category {
        Some(c) => {
            println!("{:?}", c);
            category_id = c.category_id
        }
        None => {
            println!("Category {} does not yet exists", new_category.category);
            let result: (i32,) =
                sqlx::query_as("INSERT INTO category(category) VALUES ($1) RETURNING category_id")
                    .bind(new_category.category.trim())
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            category_id = result.0;
        }
    }
    // push budget item to db
    query_builder.push_values([new_item], |mut b, item| {
        b.push_bind(item.name)
            .push_bind(item.amount)
            .push_bind(item.description)
            .push_bind(item.type_id)
            .push_bind(category_id);
    });

    let result = query_builder.build().execute(&pool).await;

    match result {
        Ok(_) => (StatusCode::OK, "Article created".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error creating article: {}", e.to_string()),
        ),
    }
}

async fn get_item(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<Json<Item>, (StatusCode, String)> {
    let item: Item = sqlx::query_as("SELECT * FROM budget_items JOIN category ON category.category_id = budget_items.category_id JOIN item_types ON item_types.type_id=budget_items.type_id WHERE budget_items.id = $1;")
        .bind(id)
        .fetch_one(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Item {} not found", id),
            )
        })?;
    Ok(Json(item))
}

async fn get_items(State(pool): State<PgPool>) -> Result<Json<Vec<Item>>, (StatusCode, String)> {
    println!("Getting items");
    let items: Vec<Item> = sqlx::query_as("SELECT * FROM budget_items JOIN category ON category.category_id=budget_items.category_id JOIN item_types ON item_types.type_id=budget_items.type_id;")
        .fetch_all(&pool)
        .await
        .unwrap();
    println!("{:?}", items);
    Ok(Json(items))
}

async fn get_balance(State(pool): State<PgPool>) -> Result<Json<String>, (StatusCode, String)> {
    let amounts: Vec<Amount> =
        sqlx::query_as("SELECT *  FROM  (SELECT budget_items.type_id, SUM(amount) FROM budget_items GROUP BY budget_items.type_id) AS sums JOIN item_types ON item_types.type_id=sums.type_id;")
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error getttin balance\n{}", e),
                )
            })?;
    println!("{:?}", amounts);
    let mut result = rust_decimal::Decimal::new(0, 1);
    for amount in amounts {
        if amount.item_type == "expense" {
            result -= amount.sum;
        } else if amount.item_type == "income" {
            result += amount.sum
        } else {
            result;
        }
    }
    Ok(Json(result.to_string()))
}
async fn get_categories(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Category>>, (StatusCode, String)> {
    let cagories: Vec<Category> = sqlx::query_as("SELECT * FROM category;")
        .fetch_all(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error retrieving cateogories"),
            )
        })?;
    println!("{:?}", cagories);
    Ok(Json(cagories))
}

#[derive(Deserialize, Serialize, FromRow, Debug)]
struct Item {
    #[serde(skip_deserializing)]
    id: i32,
    name: String,
    amount: rust_decimal::Decimal,
    description: String,
    type_id: i32,
    #[serde(skip_deserializing)]
    category_id: i32,
    #[serde(skip_deserializing)]
    date: NaiveDateTime,
    #[serde(skip_serializing_if = "String::is_empty", skip_deserializing)]
    category: String,
    #[serde(skip_serializing_if = "String::is_empty", skip_deserializing)]
    item_type: String,
}
#[derive(Deserialize, Serialize, Debug, FromRow)]
struct Category {
    category: String,
    #[serde(skip_deserializing)]
    category_id: i32,
}

#[derive(Deserialize)]
struct RequestData {
    item: Item,
    category: Category,
}

#[derive(FromRow, Serialize, Debug)]
struct Amount {
    sum: rust_decimal::Decimal,
    type_id: i32,
    item_type: String,
}
